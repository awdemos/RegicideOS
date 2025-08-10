#!/usr/bin/env python3
import argparse
import os
import sys
import subprocess
import requests
import re
import tomllib
import urllib.request
from dataclasses import dataclass
from time import sleep

# ===== COMMON FUNCTIONS =====
PRETEND = False

@dataclass
class Colours:
    """This is a class to hold the ascii escape sequences for printing colours."""
    red: str = "\033[31m"
    endc: str = "\033[m"
    green: str = "\033[32m"
    yellow: str = "\033[33m"
    blue: str = "\033[34m"

def die(message: str) -> None:
    """This is a function to exit the program with a die message."""
    print(f"{Colours.red}[ERROR]{Colours.endc} {message}", file=sys.stderr)
    exit(1)

def info(message: str) -> None:
    print(f"{Colours.blue}[INFO]{Colours.endc} {message}")

def warn(message: str) -> None:
    print(f"{Colours.yellow}[WARN]{Colours.endc} {message}")

def execute(command_string: str, override: bool = False) -> str:
    if not PRETEND or override:
        command = subprocess.Popen(command_string, stdout=subprocess.PIPE, shell=True)
        out, _ = command.communicate()
        return out
    print(f"[COMMAND]\n{command_string}")

def get_drive_size(drive: str) -> int:
    drive_size = (
        execute(f"lsblk -bo SIZE {drive} | grep -v -m 1 SIZE", override=True)
        .strip()
        .decode("UTF-8")
    )
    return int(drive_size) if drive_size != "" else 0

def check_drive_size(value: str = "") -> bool:
    return get_drive_size(value) > 12884901888

def get_drives(value: str = "") -> list:
    """The function to get all possible drives for installation."""
    return [
        f"/dev/{item}"
        for item in next(os.walk("/sys/block"))[1]
        if check_drive_size(f"/dev/{item}")
    ]

def get_fs(value: str = "") -> list:
    return list(LAYOUTS.keys())

# Hardcoded package sets instead of reading from system.toml
def get_package_sets(value: str = "") -> list:
    return ["recommended", "minimal"]

def get_flatpak_packages(applications_set: str) -> str:
    """Return flatpak packages for the given application set."""
    package_sets = {
        "recommended": [
            # add whatever you want here
            # "io.gitlab.librewolf-community", 
            # "org.mozilla.Thunderbird", 
            # "org.libreoffice.LibreOffice"
        ],
        "minimal": []
    }
    
    return " ".join(package_sets.get(applications_set, []))

def check_url(value: str) -> bool:
    try:
        response = requests.head(f"{value}Manifest.toml")
        if response.status_code == 200:
            return True
        warn("URL entered is not reachable, or there is no Manifest.toml available. Please try again.")
    except:
        warn("URL entered is not valid - did you forget 'https://'?")
    return False

def check_username(value: str) -> bool:
    if value == "":
        return True
    matches = re.match(r"[a-z_][a-z0-9_]{0,30}", value)
    return matches is not None and matches[0] == value

def chroot(command: str) -> None:
    execute(f'chroot /mnt/root /bin/bash <<"EOT"\n{command}\nEOT')

# ===== DRIVE FUNCTIONS =====
LAYOUTS = {
    "btrfs": [
        {"size": "512M", "label": "EFI", "format": "vfat", "type": "uefi"},
        {
            "size": True,
            "label": "ROOTS",
            "format": "btrfs",
            "subvolumes": [
                "/home",
                "/overlay",
                "/overlay/etc",
                "/overlay/var",
                "/overlay/usr",
            ],
            "type": "linux",
        },
    ],
    "btrfs_encryption_dev": [
        {"size": "512M", "label": "EFI", "format": "vfat", "type": "uefi"},
        {"size": "8G", "label": "ROOTS", "format": "ext4", "type": "linux"},
        {
            "size": True,
            "label": "XENIA",
            "format": "luks",
            "inside": {
                "size": True,
                "format": "btrfs",
                "subvolumes": [
                    "/home",
                    "/overlay",
                    "/overlay/etc",
                    "/overlay/var",
                    "/overlay/usr",
                ],
            },
            "type": "linux",
        },
    ],
}

def human_to_bytes(size) -> int:
    sizes = {
        "B": 1,
        "K": 1024,
        "M": 1024**2,
        "G": 1024**3,
        "T": 1024**4,
        "P": 1024**5,
    }
    return int(size[:-1]) * sizes[size[-1]]

def is_efi() -> bool:
    return os.path.isdir("/sys/firmware/efi")

def partition_drive(drive: str, layout: list) -> bool:
    execute(f"umount -ql {drive}?*")
    vgs = execute("vgs | awk '{ print $1 }' | grep -vw VG")
    
    if vgs is not None:
        vgs = [line.strip().decode("UTF-8") for line in vgs.splitlines()]
        for vg in vgs:
            execute(f"vgchange -an {vg}")

    command: str = f"cat <<EOF | sfdisk -q --wipe always --force {drive}\nlabel: gpt"
    drive_size: int = get_drive_size(drive)
    running_drive_size: int = drive_size - 1048576  # for BIOS systems, -1M so there is space for bios boot

    for partition in layout:
        size: str = ""
        if partition["size"] is True:
            if not is_efi():
                size = f"size={(running_drive_size/1024)}K, "  # convert to Kibibites instead of using sectors
        elif partition["size"][-1] == "%":
            partition_size: float = drive_size * (float(partition["size"][:-1]) / 100)
            partition_size = round(partition_size, 0)
            running_drive_size -= partition_size
            size = f"size={partition_size}, "
        else:
            running_drive_size -= human_to_bytes(partition["size"])
            size = f"size={partition['size']}, "
        command += f"\n{size}type={partition['type']}"

    if not is_efi():
        command += "\ntype=21686148-6449-6E6F-744E-656564454649"

    command += "\nEOF"
    execute(command)
    sleep(2)
    execute(f"partprobe {drive}")

def format_drive(drive: str, layout: list) -> None:
    name: str = "/dev/" + execute(
        f"lsblk -o NAME --list | grep -m 1 '{drive.split('/')[-1]}.'",
        override=True,
    ).strip().decode("UTF-8")

    noNum = False
    if name == "/dev/":  # the drive passed in doesnt have partitions/numbers at the end (luks inside partition)
        name = drive
        noNum = True
    else:
        name = name.replace("-", "/")
        number = int(name[-1:])

    for i, partition in enumerate(layout):
        if not noNum:
            name = name[:-1] + str(number)  # enumerates partitions
            number += 1

        if partition["format"] == "vfat":
            if "label" in partition:
                execute(f"mkfs.vfat -I -F 32 -n {partition['label']} {name}")
            else:
                execute(f"mkfs.vfat -I -F 32 {name}")
        elif partition["format"] == "ext4":
            if "label" in partition:
                execute(f"mkfs.ext4 -q -L {partition['label']} {name}")
            else:
                execute(f"mkfs.ext4 -q {name}")
        elif partition["format"] == "btrfs":
            if "label" in partition:
                execute(f"mkfs.btrfs -q -f -L {partition['label']} {name}")
            else:
                execute(f"mkfs.btrfs -q -f {name}")
            if "subvolumes" in partition:
                if not os.path.exists("/mnt/temp"):
                    os.mkdir("/mnt/temp")
                execute(f"mount {name} /mnt/temp")
                for subvolume in partition["subvolumes"]:
                    execute(f"btrfs subvolume create /mnt/temp{subvolume}")
                execute(f"umount {name}")
        elif partition["format"] == "lvm":
            execute(f"yes | pvcreate -ff -q {name}")
            execute(f"vgcreate -ff -q {partition['name']} {name}")
            for i, lv in enumerate(partition["lvs"]):
                if lv["size"] is True:
                    execute(f"lvcreate -q -l 100%FREE -n lv{i} {partition['name']}")
                elif lv["size"][-1] == "%":
                    execute(f"lvcreate -q -l {lv['size']}FREE -n lv{i} {partition['name']}")
                else:
                    execute(f"lvcreate -q -L {lv['size']} -n lv{i} {partition['name']}")
            format_drive(f"/dev/mapper/{partition['name']}-", partition["lvs"])
        elif partition["format"] == "luks":
            execute(f"cryptsetup -q luksFormat {name}")
            execute(f"cryptsetup -q config {name} --label {partition['label']}")
            execute(f"cryptsetup luksOpen /dev/disk/by-label/{partition['label']} xenia")
            format_drive(f"/dev/mapper/xenia", [partition["inside"]])

# ===== CONFIG FUNCTIONS =====
ARCH_TABLE = {
    "x86_64": "amd64"
}

arch = ARCH_TABLE[subprocess.run(["uname", "-m"], capture_output=True, text=True).stdout.strip()]

def get_flavours(ret):
    manifest = tomllib.load(urllib.request.urlopen(f"{ret['repository']}Manifest.toml"))
    return [flavour for flavour in manifest.keys() if arch in manifest[flavour]["arch"]]

def get_releases(ret):
    manifest = tomllib.load(urllib.request.urlopen(f"{ret['repository']}Manifest.toml"))
    return [release for release in manifest[ret["flavour"]]["versions"] if arch in manifest[ret["flavour"]]["versions"][release]["arch"]]
    
def get_url(config_parsed):
    manifest = tomllib.load(
        urllib.request.urlopen(f"{config_parsed['repository']}Manifest.toml")
    )
    filename = manifest[config_parsed["flavour"]]["versions"][
        config_parsed["release_branch"]
    ]["filename"]
    return f"{config_parsed['repository']}{arch}/{config_parsed['release_branch']}/{filename}"

def fix_config(config_file: dict, interactive: bool, key: str, valid: str, default: str):
    if not interactive:
        die(f"The value specified for '{key}' in the configuration file is not valid or missing. Valid options are: {valid}.")
    
    new_value = input(
        f"Enter value for '{key}'. Valid options are {valid}.\n{Colours.blue}[{default}]{Colours.endc}: "
    )
    config_file[key] = new_value if new_value != "" else default
    return parse_config(config_file, interactive=True)

def parse_config(config_file: dict, interactive: bool = False) -> dict:
