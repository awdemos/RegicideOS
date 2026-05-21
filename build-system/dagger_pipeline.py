#!/usr/bin/env python3
"""RegicideOS Build Pipeline - Dagger orchestration for Catalyst builds.

Dagger is used here as an orchestration layer, not a replacement for Catalyst.
The actual OS build is done by Catalyst (proven Gentoo-native tooling).
Dagger provides:
  - Reproducible CI/CD builds
  - Multi-arch orchestration
  - Artifact caching and management
  - Clean environment isolation

Usage:
  dagger run python build-system/dagger_pipeline.py
  # Or with Dagger CLI:
  dagger call --source=. build-cosmic --arch=amd64
"""

import asyncio
import sys
from pathlib import Path

import dagger


async def build_cosmic(
    client: dagger.Client,
    arch: str = "amd64",
    variant: str = "systemd",
) -> dagger.Container:
    """Build RegicideOS COSMIC variant using Catalyst in a Gentoo container."""

    src = client.host().directory(
        ".",
        exclude=[
            "build-system/catalyst/tmp/",
            "target/",
            "*.img",
            "*.tar.xz",
        ],
    )

    gentoo = (
        client.container()
        .from_("gentoo/stage3:amd64-systemd")
        .with_exec(["emerge-webrsync"])
        .with_exec([
            "sh", "-c",
            "mkdir -p /etc/portage/package.accept_keywords /etc/portage/package.use && "
            "echo 'dev-util/catalyst ~amd64' > /etc/portage/package.accept_keywords/catalyst && "
            "echo 'sys-apps/util-linux python' > /etc/portage/package.use/catalyst && "
            "echo 'sys-boot/grub grub_platforms_efi-32' >> /etc/portage/package.use/catalyst"
        ])
        .with_exec(["emerge", "-qv", "dev-util/catalyst", "app-arch/pixz", "dev-vcs/git"])
        .with_exec(["mkdir", "-p", "/var/tmp/catalyst/builds/default", "/var/tmp/catalyst/snapshots"])
        .with_exec([
            "sh", "-c",
            "cd /var/tmp/catalyst/builds/default && "
            "STAGE3=$(curl -s https://distfiles.gentoo.org/releases/amd64/autobuilds/current-stage3-amd64-systemd-mergedusr/latest-stage3-amd64-systemd-mergedusr.txt | awk '/stage3/{print $1}') && "
            "curl -O \"https://distfiles.gentoo.org/releases/amd64/autobuilds/${STAGE3}\" && "
            "ln -s $(basename ${STAGE3}) stage3-amd64-systemd-mergedusr.tar.xz"
        ])
        .with_exec([
            "sh", "-c",
            "git config --global --add safe.directory /var/db/repos/gentoo && "
            "cd /var/db/repos/gentoo && git init && git add -A && git commit -m 'snapshot' && "
            "git rev-parse HEAD > /tmp/snapshot_hash"
        ])
        .with_exec(["mkdir", "-p", "/var/tmp/catalyst/config/stages"])
        .with_directory("/src", src)
    )

    build = (
        gentoo
        .with_workdir("/src/build-system/catalyst")
        .with_exec(["./build.sh"], insecure_root_capabilities=True)
    )

    return build


async def build_iso(
    client: dagger.Client,
    tarball: dagger.File,
) -> dagger.File:
    """Create a SquashFS image from a Catalyst tarball for live ISO use."""

    builder = (
        client.container()
        .from_("alpine:latest")
        .with_exec(["apk", "add", "squashfs-tools", "tar"])
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


async def main():
    config = dagger.Config(log_output=sys.stdout)
    async with dagger.Connection(config) as client:
        print("Building RegicideOS COSMIC stage4...")
        build_container = await build_cosmic(client)

        tarball = build_container.file(
            "/var/tmp/catalyst/builds/default/stage4-amd64-systemd-cosmic.tar.xz"
        )

        print("Creating SquashFS image...")
        iso_image = await build_iso(client, tarball)

        await iso_image.export("regicide-cosmic.img")
        print("Output: regicide-cosmic.img")


if __name__ == "__main__":
    asyncio.run(main())
