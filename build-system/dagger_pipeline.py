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
  # Requires a rootful Docker or Podman runtime (rootless Podman is unsupported).
  DAGGER_PROGRESS=plain dagger run python build-system/dagger_pipeline.py --plain

  # From-source pipeline: no --usepkg, everything compiles from source.
  # Still writes fresh binpkgs into the cache volume (FEATURES=buildpkg),
  # keeping it warm for the main pipeline.
  REGICIDE_USE_BINPKGS=0 DAGGER_PROGRESS=plain \
      dagger run python build-system/dagger_pipeline.py --plain

  # Encrypted output (rootful runtime required).
  DAGGER_PROGRESS=plain dagger run python build-system/dagger_pipeline.py --plain --encrypt

  # If your default docker endpoint is rootless Podman, use the rootful socket:
  #   sudo systemctl enable --now podman.socket
  #   DOCKER_HOST=unix:///run/podman/podman.sock sudo -E \
  #       dagger run python build-system/dagger_pipeline.py --plain
"""

import argparse
import asyncio
import getpass
import os
import re
import shlex
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


def _check_container_runtime() -> None:
    """Fail fast if the local container runtime is rootless Podman.

    Dagger's engine image must create the 'dagger0' bridge, which rootless
    Podman cannot do. The Dagger SDK/CLI retry the connection for ~10 minutes
    before surfacing the real error, so an explicit up-front check saves time.
    If docker is unavailable or its info cannot be read, we skip the check:
    the user may be targeting a remote engine via _EXPERIMENTAL_DAGGER_RUNNER_HOST.
    """
    if os.environ.get("_EXPERIMENTAL_DAGGER_RUNNER_HOST"):
        return
    try:
        result = subprocess.run(
            ["docker", "info"],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return
    if result.returncode != 0:
        return
    output = result.stdout.lower()
    # Podman rootless reports a standalone "rootless" line under Security Options.
    # Docker rootless reports "rootless: true".
    if re.search(r"^\s*rootless\s*$", output, re.MULTILINE) or "rootless: true" in output:
        print(
            "ERROR: Rootless Podman is not supported by the Dagger engine.\n"
            "The engine needs to create the 'dagger0' bridge, which requires root.\n"
            "Start the rootful Podman socket and re-run with:\n\n"
            "  sudo systemctl enable --now podman.socket\n"
            "  DOCKER_HOST=unix:///run/podman/podman.sock sudo -E \\\n"
            "      dagger run python build-system/dagger_pipeline.py --plain\n",
            file=sys.stderr,
        )
        sys.exit(1)


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
        # Skip COSMIC desktop packages and greeter when building a headless or
        # alternative-desktop image. Stage scripts read this in stage4/6.
        .with_env_variable("REGICIDE_SKIP_COSMIC", os.environ.get("REGICIDE_SKIP_COSMIC", "0"))
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

    # Split long Portage emerges into cacheable withExec layers to limit
    # per-operation rootfs snapshots and avoid Dagger engine strain.
    #
    # Mount inputs just before the stage that consumes them so that edits to
    # one file only invalidate the relevant stages:
    #   - common.sh is mounted per-stage because every stage sources it.
    #     A change to common.sh still invalidates stage1 onwards (it defines
    #     the Gentoo profile and make.conf variables), but not the tooling
    #     setup before stage1.
    #   - catalyst/overlay, catalyst/cosmic-overlay, and overlays/regicide-rust
    #     are mounted just before stage4-cosmic-a.sh, which copies them into
    #     the rootfs.  Changing an overlay ebuild no longer restarts stage1-3.
    #   - src/, pyproject.toml, seed-overlays.sh, and data/ are mounted just
    #     before stage6-finalize.sh, which copies regicide-update into the
    #     rootfs.  Changing the update tooling no longer restarts stage1-5.
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
            f"{stages_path}/common.sh",
            src.file("build-system/catalyst/stages/common.sh"),
        )
        build = build.with_mounted_file(
            f"{stages_path}/{script_basename}",
            src.file(f"build-system/catalyst/{script}"),
        )
        if script_basename == "stage4-cosmic-a.sh":
            build = (
                build
                .with_directory(
                    f"{catalyst_path}/overlay",
                    src.directory("build-system/catalyst/overlay"),
                )
                .with_directory(
                    f"{catalyst_path}/cosmic-overlay",
                    src.directory("build-system/catalyst/cosmic-overlay"),
                )
                .with_directory(
                    f"{overlays_path}/regicide-rust",
                    src.directory("overlays/regicide-rust"),
                )
            )
        if script_basename == "stage6-finalize.sh":
            # stage6-finalize.sh stages the regicide-update source tree from
            # the repo root (REPO_ROOT=/src in the container).  Mount the extra
            # inputs it copies only now, just before stage6 runs, so the cache
            # keys for stages 1-5 stay stable.
            build = (
                build
                .with_directory(f"{repo_path}/src", src.directory("src"))
                .with_mounted_file(f"{repo_path}/pyproject.toml", src.file("pyproject.toml"))
                .with_mounted_file(
                    f"{catalyst_path}/seed-overlays.sh",
                    src.file("build-system/catalyst/seed-overlays.sh"),
                )
                .with_directory(f"{repo_path}/data", src.directory("data"))
            )
        build = build.with_exec([f"./{script}"], insecure_root_capabilities=True)
        if script_basename in (
            "stage2-sync.sh",
            "stage3-base-f.sh",
            "stage4-cosmic-b.sh",
            "stage5-regicide.sh",
        ):
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


async def build_live_iso(
    client: dagger.Client,
    tarball: dagger.File,
    squashfs: dagger.File,
    arch: str,
) -> dagger.File:
    """Create a bootable live ISO (GRUB + dracut dmsquash-live).

    The ISO boots the stage4 kernel with an initramfs that mounts the
    SquashFS as a read-only live root (dracut's dmsquash-live module),
    so the whole desktop can be tried or used for installation without
    touching a disk.
    """
    base_image = {
        "amd64": "gentoo/stage3:amd64-systemd",
        "arm64": "gentoo/stage3:arm64-desktop-systemd",
    }[arch]

    # 1. Generate the live initramfs inside the extracted stage4 rootfs so it
    # matches the exact kernel and userland being shipped.
    initrd_builder = (
        client.container()
        .from_(base_image)
        .with_file("/tmp/stage4.tar.xz", tarball)
        .with_exec(["sh", "-c", "tar -C / -xpJf /tmp/stage4.tar.xz --exclude=./proc --exclude=./sys --exclude=./dev --exclude=./opt --exclude=./etc/hosts --exclude=./etc/resolv.conf && rm /tmp/stage4.tar.xz"])
        .with_exec([
            "sh", "-c",
            "set -e; mkdir -p /work; kver=$(ls /lib/modules | head -1); "
            "cp /boot/vmlinuz /work/vmlinuz; "
            "dracut --force --no-hostonly --add 'dmsquash-live' /work/initramfs.img ${kver}",
        ], insecure_root_capabilities=True)
    )

    # 2. Assemble the ISO tree and run grub-mkrescue. amd64 uses a quick
    # alpine assembler; arm64 needs Gentoo's grub (alpine only ships x86_64
    # grub modules), so the arm64 stage3 container doubles as assembler.
    grub_cfg = """set timeout=5
set default=0
menuentry "RegicideOS COSMIC (live)" {
    linux /boot/vmlinuz root=live:CDLABEL=REGICIDEOS rd.live.image rd.live.dir=/live rd.live.squashimg=rootfs.img console=tty0 console=ttyS0,115200n8
    initrd /boot/initramfs.img
}
menuentry "RegicideOS COSMIC (live, verbose)" {
    linux /boot/vmlinuz root=live:CDLABEL=REGICIDEOS rd.live.image rd.live.dir=/live rd.live.squashimg=rootfs.img console=tty0 console=ttyS0,115200n8 rd.debug
    initrd /boot/initramfs.img
}
"""
    iso_name = f"regicide-live-{arch}.iso"
    if arch == "amd64":
        iso_builder = (
            client.container()
            .from_("alpine:3.21")
            .with_exec(["apk", "add", "xorriso", "grub-efi", "grub-bios", "mtools"])
        )
    else:
        iso_builder = (
            client.container()
            .from_(base_image)
            .with_exec([
                "sh", "-c",
                "GRUB_PLATFORMS=arm64-efi emerge -qv sys-boot/grub app-cdr/xorriso",
            ])
        )
    iso_builder = (
        iso_builder
        .with_exec(["mkdir", "-p", "/iso/boot/grub", "/iso/live"])
        .with_file("/iso/boot/vmlinuz", initrd_builder.file("/work/vmlinuz"))
        .with_file("/iso/boot/initramfs.img", initrd_builder.file("/work/initramfs.img"))
        .with_file("/iso/live/rootfs.img", squashfs)
        .with_new_file("/iso/boot/grub/grub.cfg", grub_cfg)
        # The squashfs exceeds ISO9660's 4GiB file limit; force iso-level 3
        # (multi-extent). -iso-level is only valid in mkisofs-emulation mode,
        # so the wrapper injects it only after "-as mkisofs" (plain
        # "xorriso -version" must keep working for grub-mkrescue's probe).
        .with_new_file("/usr/local/bin/xorriso", '#!/bin/sh\nif [ "$1" = "-as" ] && [ "$2" = "mkisofs" ]; then\n  shift 2\n  exec /usr/bin/xorriso -as mkisofs -iso-level 3 "$@"\nfi\nexec /usr/bin/xorriso "$@"\n')
        .with_exec(["chmod", "+x", "/usr/local/bin/xorriso"])
        .with_exec([
            "grub-mkrescue",
            "-V", "REGICIDEOS",
            "-o", f"/{iso_name}", "/iso",
        ])
    )

    return iso_builder.file(f"/{iso_name}")


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


# Small built-in wordlists for memorable adjective-noun passphrases.
_ADJECTIVES = (
    "able", "apt", "avid", "bare", "bold", "brisk", "calm", "cool", "curt",
    "deft", "dire", "dual", "even", "fair", "fast", "firm", "fond", "free",
    "full", "gale", "glib", "good", "grim", "hardy", "huge", "hush", "iron",
    "jade", "jolly", "keen", "kind", "lax", "lean", "lush", "mere", "mild",
    "mute", "neat", "nice", "nimble", "open", "pale", "plucky", "prime",
    "quiet", "quick", "rapid", "rare", "raw", "real", "rich", "rough", "rugged",
    "safe", "sage", "sharp", "sleek", "slow", "smooth", "soft", "solid",
    "sound", "spry", "stark", "stout", "swift", "tame", "tart", "taut", "tidy",
    "trim", "true", "vast", "warm", "wild", "wiry", "wise", "witty", "zany",
)
_NOUNS = (
    "almond", "anchor", "arrow", "bison", "bronco", "canoe", "canyon",
    "cedar", "chisel", "cobalt", "comet", "copper", "crane", "crystal",
    "delta", "eagle", "elm", "falcon", "fjord", "flint", "fox", "gale",
    "gecko", "glacier", "grape", "harbor", "hawk", "heron", "ibis",
    "iron", "jackal", "jade", "koala", "lark", "lemon", "lotus", "lynx",
    "maple", "mesa", "mint", "moose", "newt", "oasis", "onion", "opal",
    "orca", "panda", "pearl", "pilot", "plum", "quartz", "rabbit", "raven",
    "reef", "ridge", "river", "robin", "rock", "sage", "salmon", "scorpion",
    "shark", "shore", "sparrow", "stone", "summit", "swan", "talon", "thorn",
    "tiger", "topaz", "valley", "violet", "wolf", "wren", "zest", "zinc",
)


def _generate_memorable_passphrase() -> str:
    """Return a memorable adjective-adjective-noun passphrase."""
    import secrets
    return "-".join([
        secrets.choice(_ADJECTIVES),
        secrets.choice(_ADJECTIVES),
        secrets.choice(_NOUNS),
    ])


def _generate_otp_style_passphrase(length: int = 32) -> str:
    """Return a long numeric string reminiscent of an OTP token."""
    import secrets
    return "".join(secrets.choice("0123456789") for _ in range(length))


def _generate_random_passphrase() -> str:
    """Return a high-entropy random passphrase."""
    import secrets
    return secrets.token_urlsafe(24)


def _get_luks_passphrase(
    *,
    passphrase_file: Path | None = None,
    memorable: bool = False,
    otp_style: bool = False,
) -> str:
    """Return the LUKS passphrase from the most secure available source.

    Priority:
      1. Explicit --luks-passphrase-file contents (CI secret mode).
      2. REGICIDE_LUKS_PASSPHRASE environment variable (CI secret mode).
      3. Auto-generated passphrase printed once to stderr.

    The passphrase is printed exactly once on stderr so CI logs can capture it
    for the operator, but it is not emitted inside any Dagger container exec.
    """
    if passphrase_file is not None:
        raw = passphrase_file.read_text(encoding="utf-8")
        return raw.rstrip("\n")

    env_pass = os.environ.get("REGICIDE_LUKS_PASSPHRASE")
    if env_pass:
        return env_pass

    if memorable:
        passphrase = _generate_memorable_passphrase()
    elif otp_style:
        passphrase = _generate_otp_style_passphrase()
    else:
        passphrase = _generate_random_passphrase()

    print(
        "\n!!! ENCRYPTED IMAGE PASSPHRASE (copy before continuing) !!!\n"
        f"{passphrase}\n"
        "!!! This is the only time this passphrase is displayed. !!!\n",
        file=sys.stderr,
    )
    return passphrase


def _secure_wipe(path: Path) -> None:
    """Best-effort overwrite of a file before unlinking it."""
    try:
        with open(path, "rb+") as f:
            size = f.seek(0, os.SEEK_END)
            f.seek(0)
            f.write(b"\x00" * size)
            f.flush()
            os.fsync(f.fileno())
    except OSError:
        pass
    try:
        path.unlink()
    except FileNotFoundError:
        pass


async def build_qcow2_locally(
    tarball_path: Path,
    output_path: Path,
    disk_size: str,
    encrypt: bool,
    arch: str = "amd64",
    passphrase_file: Path | None = None,
    memorable: bool = False,
    otp_style: bool = False,
    squashfs_path: Path | None = None,
) -> None:
    """Build a bootable QCOW2 image from a stage4 tarball on the host.

    The image is built inside a KVM VM using the stage4 rootfs SquashFS, so no
    host loop devices or passwordless sudo are required.  This is necessary
    because the build environment does not expose usable loop devices.
    """
    script = Path(__file__).parent / "catalyst" / "build-vm-image.sh"
    env = os.environ.copy()
    env["REGICIDE_ARCH"] = arch
    if not script.exists():
        raise FileNotFoundError(f"VM image builder script not found: {script}")

    cmd: list[str] = [
        str(script),
        str(tarball_path),
        str(output_path),
        disk_size,
    ]
    if squashfs_path is not None:
        cmd[1:1] = ["--squashfs", str(squashfs_path)]

    generated_passphrase_file: Path | None = None
    if encrypt:
        passphrase = _get_luks_passphrase(
            passphrase_file=passphrase_file,
            memorable=memorable,
            otp_style=otp_style,
        )
        # Keep the passphrase in ram-backed storage and never leak it to child
        # processes via environment variables. Write it without a trailing newline
        # because cryptsetup --key-file consumes the file verbatim.
        fd, passphrase_tmp = tempfile.mkstemp(prefix="regicide-luks-", dir="/dev/shm")
        generated_passphrase_file = Path(passphrase_tmp)
        os.fchmod(fd, 0o600)
        with os.fdopen(fd, "wb") as f:
            f.write(passphrase.encode("utf-8"))
        cmd[1:1] = ["--encrypt", "--passphrase-file", str(generated_passphrase_file)]
        print(f"Building encrypted QCOW2 image: {output_path}")
        # Do not let the passphrase escape into the builder's environment.
        env.pop("REGICIDE_LUKS_PASSPHRASE", None)

    try:
        subprocess.run(cmd, check=True, env=env)
    finally:
        if generated_passphrase_file is not None:
            _secure_wipe(generated_passphrase_file)

    print(f"QCOW2 image complete: {output_path}")


CHUNK_SIZE = "2G"
_CHUNK_RETRIES = 3


async def _export_file_with_retry(file: dagger.File, dest: Path) -> None:
    """Export a single file, retrying transient Dagger transport errors."""
    for attempt in range(_CHUNK_RETRIES):
        try:
            await file.export(str(dest))
            return
        except dagger.TransportError as exc:
            if attempt == _CHUNK_RETRIES - 1:
                raise
            print(
                f"  Export failed for {dest.name} ({exc}); retrying "
                f"({attempt + 1}/{_CHUNK_RETRIES})...",
                file=sys.stderr,
            )
            await asyncio.sleep(2 ** attempt)


async def export_tarball_in_chunks(
    container: dagger.Container,
    container_tar_path: str,
    local_path: Path,
) -> Path:
    """Export a large tarball in chunks to avoid Dagger fsync deadlocks/timeouts.

    The tarball is split inside the container into fixed-size pieces, each piece
    is exported separately, and the pieces are reassembled on the host.  This
    avoids the single multi-GB `File.export()` call that triggers
    `Server error '502 Bad Gateway'` under Podman and BuildKit's fsutil
    export path.
    """
    local_path = local_path.resolve()
    local_path.parent.mkdir(parents=True, exist_ok=True)
    chunk_dir = "/tmp/regicide-tarball-chunks"
    stem = local_path.name

    split_container = container.with_exec(
        [
            "sh",
            "-c",
            f"set -e; rm -rf {chunk_dir}; mkdir -p {chunk_dir}; "
            f"split -b {CHUNK_SIZE} -a 4 -d "
            f"{shlex.quote(container_tar_path)} "
            f"{shlex.quote(f'{chunk_dir}/{stem}.chunk_')}",
        ]
    )

    chunk_names = sorted(await split_container.directory(chunk_dir).entries())
    if not chunk_names:
        raise RuntimeError(f"no tarball chunks produced in {chunk_dir}")

    chunk_files: list[Path] = []
    for name in chunk_names:
        local_chunk = local_path.parent / name
        await _export_file_with_retry(split_container.file(f"{chunk_dir}/{name}"), local_chunk)
        chunk_files.append(local_chunk)

    # Reassemble on the host in a streaming fashion.
    with local_path.open("wb") as out:
        for chunk in chunk_files:
            with chunk.open("rb") as src:
                while True:
                    data = src.read(8 * 1024 * 1024)
                    if not data:
                        break
                    out.write(data)

    # Clean up chunk files once the tarball is complete and verified.
    total_size = local_path.stat().st_size
    for chunk in chunk_files:
        chunk.unlink()

    print(f"Exported {len(chunk_files)} chunks ({total_size} bytes total) -> {local_path}")
    return local_path


def assemble_tarball_chunks(chunk_paths: list[Path], output: Path) -> None:
    """Reassemble tarball chunks into a single tarball on the host."""
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("wb") as out:
        for chunk in chunk_paths:
            with chunk.open("rb") as src:
                while True:
                    data = src.read(8 * 1024 * 1024)
                    if not data:
                        break
                    out.write(data)


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
        help="Also build an encrypted QCOW2 disk image; auto-generate a passphrase if no secret source is provided",
    )
    parser.add_argument(
        "--luks-passphrase-file",
        type=Path,
        default=None,
        help="Path to a file containing the LUKS passphrase (CI secret mode; no terminal prompt)",
    )
    parser.add_argument(
        "--memorable-passphrase",
        action="store_true",
        help="Generate a human-readable adjective-adjective-noun passphrase instead of a random token",
    )
    parser.add_argument(
        "--otp-style-passphrase",
        action="store_true",
        help="Generate a long numeric passphrase reminiscent of an OTP token",
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
        "--iso",
        action="store_true",
        help="Also build a bootable live ISO (GRUB + dracut dmsquash-live) from the artifacts",
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

    _check_container_runtime()

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
            print("Exporting stage4 tarball (chunked)...")
            tarball_path = await export_tarball_in_chunks(
                build_container,
                f"/src/build-system/catalyst/output/stage4-{args.arch}-systemd-cosmic.tar.xz",
                out_dir / f"stage4-{args.arch}-systemd-cosmic.tar.xz",
            )
            print(f"Output: build-system/catalyst/output/stage4-{args.arch}-systemd-cosmic.tar.xz")

        print("Loading SBOM for signing...")
        sbom_env = os.environ.copy()
        sbom_env["REGICIDE_ARCH"] = args.arch
        subprocess.run(
            ["./build-system/catalyst/stages/stage7-sbom.sh"],
            check=True,
            env=sbom_env,
        )
        sbom_path = out_dir / "sbom.spdx.json"

        squashfs_path = out_dir / "regicide-cosmic.img"
        squashfs_file: dagger.File | None = None
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
                # Use /var/tmp for the extracted rootfs so large artifacts do not
                # exhaust the tmpfs-backed /tmp filesystem. Quote all paths to
                # avoid shell injection from artifact names.
                squash_root = "/var/tmp/regicide-squashfs-root"
                subprocess.run(
                    [
                        "sh", "-c",
                        "set -euo pipefail; "
                        f"rm -f {shlex.quote(str(squashfs_path))}; "
                        f"rm -rf {shlex.quote(squash_root)}; "
                        f"mkdir -p {shlex.quote(squash_root)}; "
                        f"df -h {shlex.quote(squash_root)}; "
                        f"tar -C {shlex.quote(squash_root)} -xpJf {shlex.quote(str(tarball_path))}; "
                        f"mksquashfs {shlex.quote(squash_root)} {shlex.quote(str(squashfs_path))} "
                        "-comp zstd -Xcompression-level 19 -noappend; "
                        f"chown {shlex.quote(f'{os.getuid()}:{os.getgid()}')} {shlex.quote(str(squashfs_path))}; "
                        f"unsquashfs -s {shlex.quote(str(squashfs_path))} >/dev/null; "
                        f"rm -rf {shlex.quote(squash_root)} || true",
                    ],
                    check=True,
                )
        print(f"Output: {squashfs_path}")

        if args.iso:
            print(f"Building bootable live ISO (--iso, {args.arch})...")
            squashfs_for_iso = squashfs_file if squashfs_file is not None else client.host().file(str(squashfs_path))
            iso_file = await build_live_iso(client, tarball, squashfs_for_iso, arch=args.arch)
            await iso_file.export(str(out_dir / f"regicide-cosmic-{args.arch}.iso"))
            print(f"Output: build-system/catalyst/output/regicide-cosmic-{args.arch}.iso")

        print("Running stage7 verification on host artifacts...")
        verify_env = os.environ.copy()
        verify_env["REGICIDE_ARCH"] = args.arch
        subprocess.run(
            ["./build-system/catalyst/stages/stage7-verify.sh"],
            check=True,
            env=verify_env,
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
                arch=args.arch,
                passphrase_file=args.luks_passphrase_file,
                memorable=args.memorable_passphrase,
                otp_style=args.otp_style_passphrase,
                squashfs_path=squashfs_path,
            )

        if args.run_vm_test:
            print("Building unencrypted QCOW2 for post-install VM test...")
            qcow2_path = Path("build-system/catalyst/output/regicide-cosmic-vm-test.qcow2").resolve()
            await build_qcow2_locally(
                tarball_path=tarball_path,
                output_path=qcow2_path,
                disk_size=args.qcow2_size,
                encrypt=False,
                arch=args.arch,
                squashfs_path=squashfs_path,
            )
            print("Running stage8 post-install VM test...")
            subprocess.run(
                ["./build-system/catalyst/stages/stage8-vm-test.sh", str(qcow2_path)],
                check=True,
            )


if __name__ == "__main__":
    asyncio.run(main())
