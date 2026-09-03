#!/usr/bin/env python3
"""Helpers for fetching and installing RegicideOS release images."""

import hashlib
import os
import urllib.error
import urllib.request
from pathlib import Path
from regicide_update import common as rc
from regicide_update import validation


CACHE_DIR = Path("/var/cache/regicide-image")


def ensure_cache() -> None:
    CACHE_DIR.mkdir(parents=True, exist_ok=True)


def _download(url: str, dest: Path, timeout: float = 300) -> None:
    """Stream ``url`` to ``dest`` with a bounded timeout.

    ``urllib.request.urlretrieve`` has no ``timeout`` keyword, so we use
    ``urlopen`` and copy in chunks. This both avoids a TypeError and actually
    bounds the hang the original timeout intended to fix.
    """
    try:
        with urllib.request.urlopen(url, timeout=timeout) as response:
            with open(dest, "wb") as out:
                while True:
                    chunk = response.read(1024 * 1024)
                    if not chunk:
                        break
                    out.write(chunk)
    except urllib.error.URLError as e:
        if dest.exists():
            dest.unlink(missing_ok=True)
        rc.die(f"Failed to download {url}: {e}")
    except TimeoutError as e:
        if dest.exists():
            dest.unlink(missing_ok=True)
        rc.die(f"Timed out downloading {url}: {e}")


def fetch(url: str) -> Path:
    ensure_cache()
    validated = validation.safe_url(url)
    name = os.path.basename(validated)
    if not name:
        rc.die(f"Cannot determine filename from URL: {validated}")
    dest = CACHE_DIR / name
    rc.info(f"Downloading {validated} ...")
    _download(validated, dest, timeout=300)
    return dest


def verify_checksum(image: Path, checksum_url: str | None) -> bool:
    if checksum_url is None:
        rc.warn("No checksum URL provided; skipping verification.")
        return True
    validated_url = validation.safe_url(checksum_url)
    sum_file = CACHE_DIR / f"checksums-{image.name}.sha256"
    rc.info(f"Downloading checksums from {validated_url} ...")
    _download(validated_url, sum_file, timeout=60)
    expected: str | None = None
    with open(sum_file) as f:
        for line in f:
            parts = line.strip().split()
            if len(parts) == 2 and parts[1] == image.name:
                expected = parts[0]
    if not expected:
        rc.die(f"No checksum found for {image.name}")
    hasher = hashlib.sha256()
    with open(image, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            hasher.update(chunk)
    if hasher.hexdigest() != expected:
        rc.die(f"Checksum mismatch for {image.name}")
    rc.info("Checksum verified.")
    return True


def install_tarball(image: Path, roots_mount: str, reseed: bool = True) -> None:
    roots_path = validation.safe_path(roots_mount, must_be_absolute=True)
    if not rc.is_btrfs(str(roots_path)):
        rc.die(f"{roots_path} is not a btrfs filesystem")
    rc.info(f"Extracting {image} into {roots_path}")
    flags = ["-x", "-p", "-J", "-f"] if str(image).endswith(".xz") else ["-x", "-p", "-f"]
    rc.execute("tar", ["-C", str(roots_path), *flags, str(image)])
    if reseed:
        seed_script = os.path.join(
            str(roots_path), "usr", "lib", "regicide-update", "seed-overlays.sh"
        )
        if os.path.isfile(seed_script):
            rc.execute("bash", [seed_script, str(roots_path), "/overlay"])
    rc.info("Tarball install complete.")
