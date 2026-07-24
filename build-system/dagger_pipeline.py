#!/usr/bin/env python3.12
"""RegicideOS Build Pipeline - Dagger orchestration for stage4 builds.

Dagger is used here as an orchestration layer, not a replacement for the
Gentoo-based build logic. The actual OS build is done by build-manual.sh
which uses bubblewrap for an unprivileged chroot, avoiding loop devices
and root-only operations that fail inside nested containers.

Dagger provides:
  - Reproducible CI/CD builds
  - Multi-arch orchestration
  - Artifact caching and management
  - Clean environment isolation

Usage:
  # Main pipeline (binary packages): stage2+ reuse seeded binpkgs from the
  # cache volume via --usepkg, so rebuilds after small changes are fast.
  DAGGER_PROGRESS=plain dagger run python build-system/dagger_pipeline.py --plain

  # From-source pipeline: no --usepkg, everything compiles from source.
  # Still writes fresh binpkgs into the cache volume (FEATURES=buildpkg),
  # keeping it warm for the main pipeline.
  REGICIDE_USE_BINPKGS=0 DAGGER_PROGRESS=plain \
      dagger run python build-system/dagger_pipeline.py --plain

  DAGGER_PROGRESS=plain dagger run python build-system/dagger_pipeline.py --plain --encrypt
"""

import argparse
import asyncio
import getpass
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import dagger


def _dagger_cloud_org() -> str:
    """Return the Dagger Cloud organization name configured for this pipeline."""
    return os.environ.get("DAGGER_CLOUD_ORG", "RegicideOS")


def _cpu_count() -> int:
    """Return the number of host CPUs to expose to the build container."""
    return os.cpu_count() or 4


async def build_cosmic(
    client: dagger.Client,
    arch: str = "amd64",
    variant: str = "systemd",
) -> dagger.Container:
    """Build RegicideOS COSMIC variant in a Gentoo container using cacheable stages.

    arch may be "amd64" (native x86_64) or "arm64" (aarch64, executed under
    qemu-user binfmt on an x86_64 host).
    """
    image_tag = {
        "amd64": "gentoo/stage3:amd64-systemd",
        "arm64": "gentoo/stage3:arm64-desktop-systemd",
    }[arch]
    # Cache volume names are arch-specific so amd64 and arm64 content never mix.
    vol = (lambda name: name) if arch == "amd64" else (lambda name: f"regicide-arm64-{name.removeprefix('regicide-')}")

    src = client.host().directory(
        ".",
        exclude=[
            ".git/",
            "build-system/catalyst/tmp/",
            "build-system/catalyst/output/",
            "target/",
            "*.img",
            "*.tar.xz",
            "*.qcow2",
        ],
    )

    # Cache volumes preserve downloaded distfiles and compiled binary
    # packages across runs, but are only attached for dedicated cheap sync
    # execs. Dagger >=0.21 never caches an exec that has a cache mount
    # attached, so mounting them on the base container would poison every
    # downstream vertex (observed: full @world rebuilds on every run).
    distfiles_cache = client.cache_volume(vol("regicide-distfiles-v5"))
    binpkgs_cache = client.cache_volume(vol("regicide-binpkgs-v5"))

    base = (
        client.container()
        .from_(image_tag)
        .with_env_variable("REGICIDE_ARCH", arch)
        .with_env_variable("GENTOO_MIRRORS", os.environ.get("GENTOO_MIRRORS", "https://distfiles.gentoo.org"))
        # REGICIDE_USE_BINPKGS=0 forces full source builds, bypassing the
        # local binpkg cache consumed via --usepkg.
        .with_env_variable("REGICIDE_USE_BINPKGS", os.environ.get("REGICIDE_USE_BINPKGS", "1"))
        # Point the chroot PKGDIR at a container-level dir shared by all stage
        # execs; the binpkgs seed/save execs sync it with the cache volume.
        .with_env_variable("REGICIDE_BINPKGS_DIR", os.environ.get("REGICIDE_BINPKGS_DIR", "/var/cache/binpkgs"))
    )

    # Prepare the build tooling.
    with_portage = base.with_exec(["emerge-webrsync"])
    with_tools = with_portage.with_exec(
        ["emerge", "-qv", "sys-apps/bubblewrap", "dev-vcs/git", "app-arch/tar", "net-misc/curl"]
    )

    # Mount only the files each stage needs, and mount them just before the
    # stage runs.  This keeps Dagger's cache keys stable: changing one stage
    # script (e.g. stage6-finalize.sh) only invalidates that stage and later
    # work, not the heavy Portage emerges in stages 1-5.
    #
    # The rootfs lives in the container overlay (not a cache volume) because
    # Dagger's cache-volume snapshot commit fails on multi-gigabyte rootfs
    # volumes.  The cosmic-overlay is cloned fresh into the rootfs by stage4a
    # (no cache volume) so that stage stays content-cacheable too.
    with_build_dir = (
        with_tools
        .with_exec(["mkdir", "-p", "/var/tmp/regicide-build/stage3"])
        .with_env_variable("REGICIDE_BUILD_DIR", "/var/tmp/regicide-build")
        .with_env_variable("REGICIDE_OUTPUT_DIR", "/var/tmp/regicide-build/output")
        .with_workdir("/src/build-system/catalyst")
    )

    # Seed the rootfs distfiles AND binpkgs from the cache volumes with
    # dedicated cheap execs, detaching each volume immediately afterwards so
    # every subsequent stage exec stays content-cacheable.  When a volume is
    # unchanged the seed output is byte-identical, so stage1 and everything
    # downstream still cache-hits.  stage2's make.conf enables --usepkg (see
    # REGICIDE_USE_BINPKGS), so seeded binpkgs make rebuilds fast; the
    # from-source pipeline (REGICIDE_USE_BINPKGS=0) ignores them but still
    # produces fresh binpkgs via FEATURES=buildpkg, keeping the volume warm.
    build = (
        with_build_dir
        .with_mounted_cache("/cache/distfiles", distfiles_cache)
        .with_exec([
            "sh", "-c",
            "mkdir -p /var/tmp/regicide-build/rootfs/var/cache/distfiles"
            " && cp -an /cache/distfiles/. /var/tmp/regicide-build/rootfs/var/cache/distfiles/"
            " 2>/dev/null || true",
        ])
        .without_mount("/cache/distfiles")
        .with_mounted_cache("/cache/binpkgs", binpkgs_cache)
        .with_exec([
            "sh", "-c",
            "mkdir -p /var/cache/binpkgs"
            " && cp -an /cache/binpkgs/. /var/cache/binpkgs/"
            " 2>/dev/null || true",
        ])
        .without_mount("/cache/binpkgs")
    )

    stages_path = "/src/build-system/catalyst/stages"
    overlays_path = "/src/overlays"
    catalyst_path = "/src/build-system/catalyst"
    repo_path = "/src"

    # Mount the shared helper once.
    build = with_build_dir.with_mounted_file(
        f"{stages_path}/common.sh",
        src.file("build-system/catalyst/stages/common.sh"),
    )

    # Stage 4a copies the local overlays into the rootfs; mount them too.
    build = (
        build
        .with_directory(f"{catalyst_path}/overlay", src.directory("build-system/catalyst/overlay"))
        .with_directory(f"{catalyst_path}/cosmic-overlay", src.directory("build-system/catalyst/cosmic-overlay"))
        .with_directory(f"{overlays_path}/regicide-rust", src.directory("overlays/regicide-rust"))
        # stage6-finalize.sh copies src/regicide_update into the rootfs.
        .with_directory(f"{repo_path}/src", src.directory("src"))
    )

    # Split long Portage emerges into cacheable withExec layers to limit
    # per-operation rootfs snapshots and avoid Dagger engine strain.
    stage_scripts = [
        "stages/stage1-setup.sh",
        "stages/stage2-sync.sh",
        "stages/stage3-base-a.sh",
        "stages/stage3-base-b.sh",
        "stages/stage3-base-c.sh",
        "stages/stage3-base-d.sh",
        "stages/stage3-base-e.sh",
        "stages/stage3-base-f.sh",
        "stages/stage4-cosmic-a.sh",
        "stages/stage4-cosmic-b.sh",
        "stages/stage5-regicide.sh",
        "stages/stage6-finalize.sh",
    ]
    for script in stage_scripts:
        # stage_scripts entries include the "stages/" prefix so the exec command
        # matches the repository layout.  Strip that prefix for the in-container
        # mount path.
        script_basename = script.removeprefix("stages/")
        build = build.with_mounted_file(
            f"{stages_path}/{script_basename}",
            src.file(f"build-system/catalyst/{script}"),
        )
        if script_basename == "stage6-finalize.sh":
            # stage6-finalize.sh stages the regicide-update source tree from
            # the repo root (REPO_ROOT=/src in the container).  Mount the
            # extra inputs it copies only now, just before stage6 runs, so
            # the cache keys for stages 1-5 stay stable.
            build = (
                build
                .with_mounted_file(f"{repo_path}/pyproject.toml", src.file("pyproject.toml"))
                .with_mounted_file(
                    f"{catalyst_path}/seed-overlays.sh",
                    src.file("build-system/catalyst/seed-overlays.sh"),
                )
                .with_directory(f"{repo_path}/data", src.directory("data"))
            )
        build = build.with_exec([f"./{script}"], insecure_root_capabilities=True)
        if script_basename in ("stage3-base-f.sh", "stage4-cosmic-b.sh", "stage5-regicide.sh"):
            # Persist newly downloaded distfiles and newly built binpkgs to
            # the cache volumes, then detach again so later stages stay
            # content-cacheable.
            build = (
                build
                .with_mounted_cache("/cache/distfiles", distfiles_cache)
                .with_exec([
                    "sh", "-c",
                    "cp -au /var/tmp/regicide-build/rootfs/var/cache/distfiles/. /cache/distfiles/"
                    " 2>/dev/null || true",
                ])
                .without_mount("/cache/distfiles")
                .with_mounted_cache("/cache/binpkgs", binpkgs_cache)
                .with_exec([
                    "sh", "-c",
                    "cp -au /var/cache/binpkgs/. /cache/binpkgs/"
                    " 2>/dev/null || true",
                ])
                .without_mount("/cache/binpkgs")
            )

    tarball_name = f"stage4-{arch}-systemd-cosmic.tar.xz"
    build = build.with_exec([
        "mkdir", "-p", f"{catalyst_path}/output",
    ]).with_exec([
        "cp",
        f"/var/tmp/regicide-build/output/{tarball_name}",
        f"{catalyst_path}/output/{tarball_name}",
    ])

    return build.with_workdir(catalyst_path)


async def build_iso(
    client: dagger.Client,
    tarball: dagger.File,
) -> dagger.File:
    """Create a SquashFS image from a stage4 tarball for live ISO use."""

    builder = (
        client.container()
        .from_("alpine:latest")
        .with_exec(["apk", "add", "squashfs-tools", "tar", "xz"])
        .with_file("/tmp/stage4.tar.xz", tarball)
        .with_exec(["mkdir", "-p", "/tmp/rootfs"])
        .with_exec([
            "tar", "-C", "/tmp/rootfs", "-xpJf", "/tmp/stage4.tar.xz",
        ])
        .with_exec([
            "mksquashfs", "/tmp/rootfs", "/tmp/regicide-cosmic.img",
            "-comp", "zstd", "-Xcompression-level", "19",
        ])
    )

    return builder.file("/tmp/regicide-cosmic.img")


def _with_cosign(container: dagger.Container) -> dagger.Container:
    """Install cosign v2.4.0 into a Linux container from the official release."""
    cosign_url = (
        "https://github.com/sigstore/cosign/releases/download/"
        "v2.4.0/cosign-linux-amd64"
    )
    expected_sha256 = "cd7636b3586a3bdac2d9c8f3b421ed119edcb20499107887fd929211110e8418"
    return (
        container
        .with_exec(["apk", "add", "--no-cache", "curl", "ca-certificates", "coreutils"])
        .with_exec([
            "sh", "-c",
            f"curl -sL -o /usr/local/bin/cosign '{cosign_url}' && "
            f"echo '{expected_sha256}  /usr/local/bin/cosign' | sha256sum -c - && "
            "chmod +x /usr/local/bin/cosign",
        ])
    )


async def sign_artifacts(
    client: dagger.Client,
    squashfs: dagger.File,
    sbom: dagger.File,
    identity: str,
) -> tuple[dagger.File, dagger.File | None, dagger.File, dagger.File, dagger.File | None, dagger.File, dagger.File]:
    """Sign the SquashFS image and SBOM, and attest the SBOM to the image.

    Returns (squashfs_sig, squashfs_cert, squashfs_bundle, sbom_sig, sbom_cert, sbom_bundle, attestation_bundle).
    In key-based mode the certificate files are None.
    """
    signer = client.container().from_("alpine:latest")
    signer = _with_cosign(signer)

    signer = (
        signer
        .with_file("/artifacts/regicide-cosmic.img", squashfs)
        .with_file("/artifacts/sbom.spdx.json", sbom)
        .with_env_variable("COSIGN_EXPERIMENTAL", "1")
    )

    key_path = os.environ.get("COSIGN_KEY_PATH")
    if key_path:
        signer = signer.with_mounted_file("/secrets/cosign.key", client.host().file(key_path))
        signer = signer.with_env_variable("COSIGN_PASSWORD", os.environ.get("COSIGN_PASSWORD", ""))

        signer = signer.with_exec([
            "cosign", "sign-blob",
            "--key=/secrets/cosign.key",
            "--tlog-upload=false",
            "--output-signature=/artifacts/regicide-cosmic.img.sig",
            "--output-certificate=/artifacts/regicide-cosmic.img.cert",
            "--bundle=/artifacts/regicide-cosmic.img.bundle",
            "/artifacts/regicide-cosmic.img",
        ])
        signer = signer.with_exec([
            "cosign", "sign-blob",
            "--key=/secrets/cosign.key",
            "--tlog-upload=false",
            "--output-signature=/artifacts/sbom.spdx.json.sig",
            "--output-certificate=/artifacts/sbom.spdx.json.cert",
            "--bundle=/artifacts/sbom.spdx.json.bundle",
            "/artifacts/sbom.spdx.json",
        ])
        signer = signer.with_exec([
            "cosign", "attest-blob",
            "--key=/secrets/cosign.key",
            "--tlog-upload=false",
            "--predicate=/artifacts/sbom.spdx.json",
            "--type=spdx",
            "--output-attestation=/artifacts/regicide-cosmic.img.att",
            "/artifacts/regicide-cosmic.img",
        ])
        return (
            signer.file("/artifacts/regicide-cosmic.img.sig"),
            None,
            signer.file("/artifacts/regicide-cosmic.img.bundle"),
            signer.file("/artifacts/sbom.spdx.json.sig"),
            None,
            signer.file("/artifacts/sbom.spdx.json.bundle"),
            signer.file("/artifacts/regicide-cosmic.img.att"),
        )

    # Keyless OIDC signing uses the ambient OIDC token.  cosign v2.4.0's
    # sign-blob takes --output-signature/--output-certificate but not
    # --certificate-identity/--certificate-oidc-issuer; those are for `cosign
    # sign` on OCI images.  The certificate returned by Fulcio still carries
    # the identity claims and is verified by the standard verify-blob flow.
    signer = signer.with_env_variable("COSIGN_YES", "true")

    signer = signer.with_exec([
        "sh", "-c",
        "cosign sign-blob "
        "--output-signature=/artifacts/regicide-cosmic.img.sig "
        "--output-certificate=/artifacts/regicide-cosmic.img.cert "
        "--bundle=/artifacts/regicide-cosmic.img.bundle "
        "/artifacts/regicide-cosmic.img",
    ])

    signer = signer.with_exec([
        "sh", "-c",
        "cosign sign-blob "
        "--output-signature=/artifacts/sbom.spdx.json.sig "
        "--output-certificate=/artifacts/sbom.spdx.json.cert "
        "--bundle=/artifacts/sbom.spdx.json.bundle "
        "/artifacts/sbom.spdx.json",
    ])

    signer = signer.with_exec([
        "sh", "-c",
        "cosign attest-blob "
        "--predicate=/artifacts/sbom.spdx.json "
        "--type=spdx "
        "--output-attestation=/artifacts/regicide-cosmic.img.att "
        "/artifacts/regicide-cosmic.img",
    ])

    return (
        signer.file("/artifacts/regicide-cosmic.img.sig"),
        signer.file("/artifacts/regicide-cosmic.img.cert"),
        signer.file("/artifacts/regicide-cosmic.img.bundle"),
        signer.file("/artifacts/sbom.spdx.json.sig"),
        signer.file("/artifacts/sbom.spdx.json.cert"),
        signer.file("/artifacts/sbom.spdx.json.bundle"),
        signer.file("/artifacts/regicide-cosmic.img.att"),
    )


def _get_luks_passphrase() -> str:
    """Return the LUKS passphrase from the environment or prompt twice."""
    env_pass = os.environ.get("REGICIDE_LUKS_PASSPHRASE")
    if env_pass:
        return env_pass

    while True:
        first = getpass.getpass("Enter LUKS passphrase for encrypted image: ")
        if not first:
            print("Passphrase cannot be empty.")
            continue
        second = getpass.getpass("Confirm LUKS passphrase: ")
        if first != second:
            print("Passphrases do not match. Try again.")
            continue
        return first


async def build_qcow2_locally(
    tarball_path: Path,
    output_path: Path,
    disk_size: str,
    encrypt: bool,
) -> None:
    """Build a bootable QCOW2 image from a stage4 tarball on the host.

    The image is built inside a KVM VM using the stage4 rootfs SquashFS, so no
    host loop devices or passwordless sudo are required.  This is necessary
    because the build environment does not expose usable loop devices.
    """
    script = Path(__file__).parent / "catalyst" / "build-vm-image.sh"
    if not script.exists():
        raise FileNotFoundError(f"VM image builder script not found: {script}")

    cmd: list[str] = [
        str(script),
        str(tarball_path),
        str(output_path),
        disk_size,
    ]

    passphrase_file: Path | None = None
    if encrypt:
        passphrase = _get_luks_passphrase()
        fd, passphrase_tmp = tempfile.mkstemp(prefix="regicide-luks-", text=True)
        passphrase_file = Path(passphrase_tmp)
        with os.fdopen(fd, "w") as f:
            f.write(passphrase + "\n")
        passphrase_file.chmod(0o600)
        cmd[1:1] = ["--encrypt", "--passphrase-file", str(passphrase_file)]
        print(f"Building encrypted QCOW2 image: {output_path}")

    try:
        subprocess.run(cmd, check=True)
    finally:
        if passphrase_file is not None:
            try:
                passphrase_file.unlink()
            except FileNotFoundError:
                pass

    print(f"QCOW2 image complete: {output_path}")


async def main() -> None:
    parser = argparse.ArgumentParser(
        description="Build RegicideOS COSMIC stage4, SquashFS, and optional encrypted QCOW2."
    )
    parser.add_argument(
        "--arch",
        choices=["amd64", "arm64"],
        default="amd64",
        help="Target architecture (arm64 builds run under qemu-user binfmt)",
    )
    parser.add_argument(
        "--plain",
        action="store_true",
        help="Use plain Dagger progress output (useful for logs and CI)",
    )
    parser.add_argument(
        "--encrypt",
        action="store_true",
        help="Also build an encrypted QCOW2 disk image and prompt for a LUKS passphrase",
    )
    parser.add_argument(
        "--qcow2-size",
        default="30G",
        help="Disk size for the optional QCOW2 image (default: 30G)",
    )
    parser.add_argument(
        "--qcow2-output",
        default="regicide-cosmic.qcow2",
        help="Output path for the optional QCOW2 image (default: regicide-cosmic.qcow2)",
    )
    parser.add_argument(
        "--from-tarball",
        type=Path,
        default=None,
        help="Reuse an existing stage4 tarball instead of rebuilding it in Dagger",
    )
    parser.add_argument(
        "--from-squashfs",
        type=Path,
        default=None,
        help="Reuse an existing SquashFS image instead of rebuilding it in Dagger",
    )
    parser.add_argument(
        "--run-vm-test",
        action="store_true",
        help="Build an unencrypted QCOW2 from the stage4 tarball and run stage8-vm-test.sh",
    )
    parser.add_argument(
        "--skip-sign",
        action="store_true",
        help="Skip Sigstore signing (useful for local test builds without cosign credentials)",
    )
    args = parser.parse_args()

    if args.plain:
        os.environ["DAGGER_PROGRESS"] = "plain"

    tarball_path: Path | None = None
    squashfs_input: Path | None = None
    if args.from_tarball:
        tarball_path = args.from_tarball.resolve()
        if not tarball_path.is_file():
            print(f"Error: --from-tarball file not found: {tarball_path}", file=sys.stderr)
            sys.exit(1)
    if args.from_squashfs:
        squashfs_input = args.from_squashfs.resolve()
        if not squashfs_input.is_file():
            print(f"Error: --from-squashfs file not found: {squashfs_input}", file=sys.stderr)
            sys.exit(1)

    config = dagger.Config(log_output=sys.stdout)
    os.environ.setdefault("DAGGER_CLOUD_ORG", _dagger_cloud_org())
    # DAGGER_CLOUD_TOKEN selects the Dagger Cloud organization; ensure it points
    # to the RegicideOS org rather than any previously-configured org.
    if "DAGGER_CLOUD_TOKEN" not in os.environ:
        print(
            "WARNING: DAGGER_CLOUD_TOKEN is not set; Dagger Cloud traces will not be sent.",
            file=sys.stderr,
        )
    async with dagger.Connection(config) as client:
        if tarball_path is None:
            print(f"Building RegicideOS COSMIC stage4 ({args.arch})...")
            build_container = await build_cosmic(client, arch=args.arch)
            tarball = build_container.file(
                f"/src/build-system/catalyst/output/stage4-{args.arch}-systemd-cosmic.tar.xz"
            )
        else:
            print(f"Using existing stage4 tarball: {tarball_path}")
            tarball = client.host().file(str(tarball_path))



        out_dir = Path("build-system/catalyst/output")

        if tarball_path is None:
            print("Exporting stage4 tarball...")
            await tarball.export(str(out_dir / f"stage4-{args.arch}-systemd-cosmic.tar.xz"))
            print(f"Output: build-system/catalyst/output/stage4-{args.arch}-systemd-cosmic.tar.xz")
            tarball_path = out_dir / f"stage4-{args.arch}-systemd-cosmic.tar.xz"

        print("Loading SBOM for signing...")
        subprocess.run(
            ["./build-system/catalyst/stages/stage7-sbom.sh"],
            check=True,
        )
        sbom_path = out_dir / "sbom.spdx.json"

        squashfs_path = out_dir / "regicide-cosmic.img"
        if squashfs_input is not None:
            print(f"Using existing SquashFS image: {squashfs_input}")
            if squashfs_input.resolve() != squashfs_path.resolve():
                subprocess.run(
                    ["cp", "-f", str(squashfs_input), str(squashfs_path)],
                    check=True,
                )
            else:
                print("SquashFS input path matches output path; reusing in place.")
        else:
            if os.geteuid() != 0:
                # Not root: build the SquashFS inside the Dagger engine (which
                # is privileged) instead of requiring passwordless host sudo.
                # This matches the RegicideOSArch pipeline flow.
                print("Creating SquashFS image in Dagger (not running as root)...")
                squashfs_file = await build_iso(client, tarball)
                await squashfs_file.export(str(squashfs_path))
            else:
                print("Creating SquashFS image locally...")
                # Creating a faithful SquashFS that preserves setuid binaries and
                # mixed ownership requires root privileges. Use sudo when not root.
                subprocess.run(
                    [
                        "sh", "-c",
                        # Use /var/tmp for the extracted rootfs so large artifacts do
                        # not exhaust the tmpfs-backed /tmp filesystem. Run tar and
                        # mksquashfs as root when available so setuid/ownership are
                        # preserved, and make cleanup tolerant of root-owned files.
                        "set -euo pipefail; "
                        "SQUASH_ROOT=/var/tmp/regicide-squashfs-root; "
                        f"rm -f '{squashfs_path}'; "
                        "rm -rf \"$SQUASH_ROOT\"; "
                        "mkdir -p \"$SQUASH_ROOT\"; "
                        "df -h \"$SQUASH_ROOT\"; "
                        f"tar -C \"$SQUASH_ROOT\" -xpJf '{tarball_path}'; "
                        f"mksquashfs \"$SQUASH_ROOT\" '{squashfs_path}' "
                        "-comp zstd -Xcompression-level 19 -noappend; "
                        f"chown {os.getuid()}:{os.getgid()} '{squashfs_path}'; "
                        f"unsquashfs -s '{squashfs_path}' >/dev/null; "
                        "rm -rf \"$SQUASH_ROOT\" || true",
                    ],
                    check=True,
                )
        print(f"Output: {squashfs_path}")

        print("Running stage7 verification on host artifacts...")
        subprocess.run(
            ["./build-system/catalyst/stages/stage7-verify.sh"],
            check=True,
        )

        if not args.skip_sign:
            identity = os.environ.get(
                "REGICIDE_SIGN_IDENTITY",
                "https://github.com/RegicideOS/RegicideOS/.github/workflows/release.yml@refs/heads/main",
            )
            print(f"Signing artifacts with identity: {identity}")
            iso_image = client.host().file(str(out_dir / "regicide-cosmic.img"))
            sbom_file = client.host().file(str(sbom_path))
            (
                img_sig,
                img_cert,
                img_bundle,
                sbom_sig,
                sbom_cert,
                sbom_bundle,
                attestation,
            ) = await sign_artifacts(client, iso_image, sbom_file, identity)

            await img_sig.export(str(out_dir / "regicide-cosmic.img.sig"))
            await img_bundle.export(str(out_dir / "regicide-cosmic.img.bundle"))
            await sbom_sig.export(str(out_dir / "sbom.spdx.json.sig"))
            await sbom_bundle.export(str(out_dir / "sbom.spdx.json.bundle"))
            await attestation.export(str(out_dir / "regicide-cosmic.img.att"))
            if img_cert is not None:
                await img_cert.export(str(out_dir / "regicide-cosmic.img.cert"))
                await sbom_cert.export(str(out_dir / "sbom.spdx.json.cert"))

            print("Output: build-system/catalyst/output/regicide-cosmic.img.sig")
            print("Output: build-system/catalyst/output/regicide-cosmic.img.bundle")
            if img_cert is not None:
                print("Output: build-system/catalyst/output/regicide-cosmic.img.cert")
            print("Output: build-system/catalyst/output/sbom.spdx.json.sig")
            print("Output: build-system/catalyst/output/sbom.spdx.json.bundle")
            if sbom_cert is not None:
                print("Output: build-system/catalyst/output/sbom.spdx.json.cert")
            print("Output: build-system/catalyst/output/regicide-cosmic.img.att")
        else:
            print("Skipping Sigstore signing (--skip-sign)")

        if args.encrypt:
            await build_qcow2_locally(
                tarball_path=tarball_path,
                output_path=Path(args.qcow2_output).resolve(),
                disk_size=args.qcow2_size,
                encrypt=True,
            )

        if args.run_vm_test:
            print("Building unencrypted QCOW2 for post-install VM test...")
            qcow2_path = Path("build-system/catalyst/output/regicide-cosmic-vm-test.qcow2").resolve()
            await build_qcow2_locally(
                tarball_path=tarball_path,
                output_path=qcow2_path,
                disk_size=args.qcow2_size,
                encrypt=False,
            )
            print("Running stage8 post-install VM test...")
            subprocess.run(
                ["./build-system/catalyst/stages/stage8-vm-test.sh", str(qcow2_path)],
                check=True,
            )


if __name__ == "__main__":
    asyncio.run(main())
