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
  dagger run python build-system/dagger_pipeline.py
  dagger run python build-system/dagger_pipeline.py --encrypt
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
    """Build RegicideOS COSMIC variant in a Gentoo container using cacheable stages."""

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

    # Cache volumes survive across Dagger runs, preserving downloaded distfiles,
    # binary packages, the stage3 seed tarball, the portage snapshot, and the
    # cosmic-overlay git clone.
    distfiles_cache = client.cache_volume("regicide-distfiles-v5")
    binpkgs_cache = client.cache_volume("regicide-binpkgs-v5")
    build_cache = client.cache_volume("regicide-manual-build-v5")
    cosmic_overlay_cache = client.cache_volume("regicide-cosmic-overlay-v5")

    base = (
        client.container()
        .from_("gentoo/stage3:amd64-systemd")
        .with_env_variable("GENTOO_MIRRORS", os.environ.get("GENTOO_MIRRORS", "https://distfiles.gentoo.org"))
        .with_mounted_cache("/var/cache/distfiles", distfiles_cache)
        .with_mounted_cache("/var/cache/binpkgs", binpkgs_cache)
    )

    # Prepare the build tooling.
    with_portage = base.with_exec(["emerge-webrsync"])
    with_tools = with_portage.with_exec(
        ["emerge", "-qv", "sys-apps/bubblewrap", "dev-vcs/git", "app-arch/tar", "net-misc/curl"]
    )

    # Build state and cosmic-overlay clone live on cache volumes.
    with_build_dir = (
        with_tools
        .with_mounted_cache("/var/tmp/regicide-build", build_cache)
        .with_mounted_cache("/var/cache/cosmic-overlay", cosmic_overlay_cache)
        .with_exec(["mkdir", "-p", "/var/tmp/regicide-build/stage3"])
    )

    with_source = (
        with_build_dir
        .with_directory("/src", src)
        .with_workdir("/src/build-system/catalyst")
        .with_env_variable("REGICIDE_BUILD_DIR", "/var/tmp/regicide-build")
        .with_env_variable("REGICIDE_OUTPUT_DIR", "/var/tmp/regicide-build/output")
        .with_env_variable("REGICIDE_COSMIC_OVERLAY_DIR", "/var/cache/cosmic-overlay")
    )

    # Run each stage as a separate withExec so Dagger can cache the result of
    # each stage independently. Stages 1-5 are idempotent: re-running them on
    # an already-populated rootfs is a cheap no-op.
    stage_scripts = [
        "stages/stage1-setup.sh",
        "stages/stage2-sync.sh",
        "stages/stage3-base.sh",
        "stages/stage4-cosmic.sh",
        "stages/stage5-regicide.sh",
        "stages/stage6-finalize.sh",
    ]
    build = with_source
    for script in stage_scripts:
        build = build.with_exec([f"./{script}"], insecure_root_capabilities=True)

    tarball_name = "stage4-amd64-systemd-cosmic.tar.xz"
    build = build.with_exec([
        "mkdir", "-p", "/src/build-system/catalyst/output",
    ]).with_exec([
        "cp",
        f"/var/tmp/regicide-build/output/{tarball_name}",
        f"/src/build-system/catalyst/output/{tarball_name}",
    ])

    return build.with_workdir("/src/build-system/catalyst")


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
        "--encrypt",
        action="store_true",
        help="Also build an encrypted QCOW2 disk image and prompt for a LUKS passphrase",
    )
    parser.add_argument(
        "--qcow2-size",
        default="20G",
        help="Disk size for the optional QCOW2 image (default: 20G)",
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
    args = parser.parse_args()

    tarball_path: Path | None = None
    squashfs_path: Path | None = None
    if args.from_tarball:
        tarball_path = args.from_tarball.resolve()
        if not tarball_path.is_file():
            print(f"Error: --from-tarball file not found: {tarball_path}", file=sys.stderr)
            sys.exit(1)
    if args.from_squashfs:
        squashfs_path = args.from_squashfs.resolve()
        if not squashfs_path.is_file():
            print(f"Error: --from-squashfs file not found: {squashfs_path}", file=sys.stderr)
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
            print("Building RegicideOS COSMIC stage4...")
            build_container = await build_cosmic(client)
            tarball = build_container.file(
                "/src/build-system/catalyst/output/stage4-amd64-systemd-cosmic.tar.xz"
            )
        else:
            print(f"Using existing stage4 tarball: {tarball_path}")
            tarball = client.host().file(str(tarball_path))

        if squashfs_path is None:
            print("Creating SquashFS image...")
            iso_image = await build_iso(client, tarball)
            await iso_image.export("regicide-cosmic.img")
            print("Output: regicide-cosmic.img")
        else:
            print(f"Using existing SquashFS image: {squashfs_path}")

        if args.encrypt:
            if tarball_path is None:
                # Export the stage4 tarball so the local image builder can use it.
                tarball_host_path = Path(
                    tempfile.mkdtemp(prefix="regicide-build-")
                ) / "stage4.tar.xz"
                print(f"Exporting stage4 tarball to {tarball_host_path}...")
                await tarball.export(str(tarball_host_path))
            else:
                tarball_host_path = tarball_path

            try:
                await build_qcow2_locally(
                    tarball_path=tarball_host_path,
                    output_path=Path(args.qcow2_output).resolve(),
                    disk_size=args.qcow2_size,
                    encrypt=True,
                )
            finally:
                if tarball_path is None:
                    tarball_host_path.unlink(missing_ok=True)
                    tarball_host_path.parent.rmdir()


if __name__ == "__main__":
    asyncio.run(main())
