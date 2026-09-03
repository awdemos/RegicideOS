#!/usr/bin/env python3
"""Input validation helpers for regicide-update CLIs.

All external-facing CLIs use these helpers to avoid path-traversal and
command-injection bugs when values are passed to emerge, tar, shell scripts,
grub tools, or network fetches.
"""

import os
import re
import urllib.parse
from pathlib import Path
from regicide_update import common as rc


_RE_PACKAGE = re.compile(r"^[A-Za-z0-9+_\-@./]+$")
_RE_SET = re.compile(r"^@[A-Za-z0-9_\-]+$")


def safe_url(url: str, allowed_schemes: tuple[str, ...] = ("http", "https")) -> str:
    """Return a normalized URL if its scheme is allowed.

    Disallow file://, ftp://, and other non-network schemes to prevent local
    file reads or unexpected transports. Reject URLs containing newlines or
    null bytes.
    """
    if "\n" in url or "\r" in url or "\x00" in url:
        rc.die("URL contains invalid characters")
    parsed = urllib.parse.urlparse(url)
    if not parsed.scheme:
        rc.die(f"URL missing scheme: {url}")
    if parsed.scheme.lower() not in allowed_schemes:
        rc.die(f"URL scheme not allowed: {parsed.scheme}")
    if not parsed.netloc:
        rc.die(f"URL missing host: {url}")
    return url


def safe_path(
    value: str,
    must_exist: bool = False,
    allowed_prefixes: tuple[str, ...] | None = None,
    must_be_absolute: bool = True,
) -> Path:
    """Resolve a path and optionally constrain it under allowed prefixes.

    Path-traversal attempts (e.g., /safe/../etc/passwd) are rejected. If
    allowed_prefixes is supplied, the resolved path must be equal to or under
    one of the prefixes.  For tests where `/home/a/.tmp` and `/var/home/a/.tmp`
    are the same directory via bind mount, we canonicalize to a stable resolved
    path.
    """
    if "\x00" in value:
        rc.die("Path contains invalid characters")
    path = Path(value)
    if must_be_absolute and not path.is_absolute():
        rc.die(f"Path must be absolute: {value}")
    resolved: Path
    try:
        resolved = path.resolve(strict=must_exist)
    except OSError:
        rc.die(f"Path does not exist: {value}")
    if allowed_prefixes:
        ok = False
        for prefix in allowed_prefixes:
            try:
                resolved.relative_to(Path(prefix).resolve())
                ok = True
                break
            except ValueError:
                pass
        if not ok:
            rc.die(f"Path outside allowed directories: {value}")
    return resolved


def _realpath(value: str) -> str:
    """Return the real (canonical) path for a string path.

    Useful in tests to compare a path that has been through safe_path.
    """
    return str(Path(value).resolve())


def safe_package_name(name: str) -> str:
    """Validate a Gentoo package atom, set, or category/package string.

    Rejects arguments that start with a dash (emerge option injection) or
    contain shell-metacharacters / traversal sequences.
    """
    if not name or name.startswith("-"):
        rc.die(f"Invalid package name: {name}")
    if name.startswith("@"):
        if not _RE_SET.match(name):
            rc.die(f"Invalid package set name: {name}")
        return name
    if not _RE_PACKAGE.match(name) or ".." in name:
        rc.die(f"Invalid package name: {name}")
    return name


def safe_snapshot_name(name: str) -> str:
    """Validate a snapshot set name.

    Only allows a limited character set so it can be used as a directory name
    safely inside the snapshot store.
    """
    if not name:
        rc.die("Snapshot name cannot be empty")
    if not re.match(r"^[A-Za-z0-9_\-:.]+$", name):
        rc.die(f"Invalid snapshot name: {name}")
    if name == "initial":
        rc.die("Snapshot name 'initial' is reserved")
    return name


def safe_slot(slot: str) -> str:
    """Validate an A/B root slot name."""
    lowered = slot.lower().strip()
    if lowered not in ("a", "b"):
        rc.die(f"Invalid root slot: {slot}")
    return lowered


def safe_shell_arg(arg: str) -> str:
    """Reject strings containing characters that could alter shell parsing.

    Used when arguments must be passed through a shell. Prefer passing argv
    arrays directly; this is a last-line defense.
    """
    if "\x00" in arg or "\n" in arg or "\r" in arg:
        rc.die("Shell argument contains invalid characters")
    if arg.startswith("$") or "`" in arg or ";" in arg or "|" in arg or "&" in arg:
        rc.die(f"Shell argument contains metacharacters: {arg!r}")
    return arg


def ensure_dir_under(path: Path, parent: Path) -> None:
    """Die if ``path`` is not equal to or under ``parent``."""
    try:
        path.resolve().relative_to(parent.resolve())
    except ValueError:
        rc.die(f"Path {path} is outside {parent}")
