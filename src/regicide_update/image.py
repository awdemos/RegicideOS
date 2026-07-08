#!/usr/bin/env python3
"""Helpers for fetching and installing RegicideOS release images."""

import hashlib
import os
import urllib.request
from pathlib import Path
from regicide_update import common as rc


CACHE_DIR = Path("/var/cache/regicide-image")


def ensure_cache() -> None:
    CACHE_DIR.mkdir(parents=True, exist_ok=True)


def fetch(url: str) -> Path:
    ensure_cache()
    name = os.path.basename(url)
    if not name:
        rc.die(f"Cannot determine filename from URL: {url}")
    dest = CACHE_DIR / name
    rc.info(f"Downloading {url} ...")
    urllib.request.urlretrieve(url, dest)
    return dest


def verify_checksum(image: Path, checksum_url: str | None) -> bool:
    if checksum_url is None:
        rc.warn("No checksum URL provided; skipping verification.")
        return True
    sum_file = CACHE_DIR / "checksums.sha256"
    rc.info(f"Downloading checksums from {checksum_url} ...")
    urllib.request.urlretrieve(checksum_url, sum_file)
    expected: str | None = None
    with open(sum_file) as f:
        for line in f:
            parts = line.strip().split()
            if len(parts) == 2 and parts[1] == image.name:
                expected = parts[0]
    if not expected:
        rc.die(f"No checksum found for {image.name}")
    actual = hashlib.sha256(image.read_bytes()).hexdigest()
    if actual != expected:
        rc.die(f"Checksum mismatch for {image.name}")
    rc.info("Checksum verified.")
    return True


def install_tarball(image: Path, roots_mount: str, reseed: bool = True) -> None:
    if not os.path.ismount(roots_mount):
        rc.die(f"{roots_mount} is not mounted")
    rc.info(f"Extracting {image} into {roots_mount}")
    flags = "-xpJf" if str(image).endswith(".xz") else "-xpf"
    rc.execute(f"tar -C {roots_mount} {flags} {image}")
    if reseed:
        seed_script = os.path.join(
            roots_mount, "usr", "lib", "regicide-update", "seed-overlays.sh"
        )
        if os.path.isfile(seed_script):
            rc.execute(f"bash {seed_script} {roots_mount} /overlay")
    rc.info("Tarball install complete.")
