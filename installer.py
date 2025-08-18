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

def get_package_sets(value: str = "") -> list:
    """Get package sets from system.toml equivalent"""
    # Hardcoded package sets instead of reading from system.toml
    return ["recommended", "minimal"]

def get_flatpak_packages(applications_set: str) -> str:
    """Return flatpak packages for the given application set."""
    package_sets = {
        "recommended": [
            "io.gitlab.librewolf-community", 
            "org.mozilla.Thunderbird", 
            "org.gnome.TextEditor", 
            "org.gnome.Rhythmbox3", 
            "org.gnome.Calculator", 
            "org.gnome.Totem", 
            "org.gnome.Loupe", 
            "org.libreoffice.LibreOffice"
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
    """
    Parses config file and presents an interactive mode if empty config is specified.
    """
    VALIDITY: dict = {
        "drive": {"func": get_drives, "mode": "execute", "default": ""},
        "repository": {
            "func": check_url,
            "mode": "check",
            "default": "https://repo.xenialinux.com/releases/",
            "valid_text": "a URL that points to a Xenia repository (remember the trailing slash)",
        },
        "flavour": {
            "func": get_flavours,
            "mode": "execute",
            "default": "gnome-systemd",
            "return": True,
        },
        "release_branch": {
            "func": get_releases,
            "mode": "execute",
            "default": "main",
            "return": True,
        },
        "filesystem": {"func": get_fs, "mode": "execute", "default": "btrfs"},
        "username": {
            "func": check_username,
            "mode": "check",
            "default": "",
            "valid_text": "a username to use for the main account (leave empty for none)",
        },
        "applications": {
            "func": get_package_sets,
            "mode": "execute",
            "default": "recommended",
        },
    }

    for key in VALIDITY:
        default = VALIDITY[key]["default"]

        match VALIDITY[key]["mode"]:
            case "execute":
                if "return" in VALIDITY[key].keys():
                    if VALIDITY[key]["return"] == True:
                        valid = VALIDITY[key]["func"](ret=config_file) 
                    else:
                        valid = VALIDITY[key]["func"]()
                else:
                    valid = VALIDITY[key]["func"]()

                if key not in config_file or config_file[key] not in valid:
                    config_file = fix_config(
                        config_file, interactive, key, valid, default
                    )

            case "check":
                do_return = False
                if "return" in VALIDITY[key].keys():
                    do_return = VALIDITY[key]["return"]

                if do_return:
                    if key not in config_file or not VALIDITY[key]["func"](
                        value=config_file[key], ret=config_file
                    ):
                        config_file = fix_config(
                            config_file,
                            interactive,
                            key,
                            VALIDITY[key]["valid_text"],
                            default,
                        )
                else:
                    if key not in config_file or not VALIDITY[key]["func"](
                        value=config_file[key]
                    ):
                        config_file = fix_config(
                            config_file,
                            interactive,
                            key,
                            VALIDITY[key]["valid_text"],
                            default,
                        )

    return config_file

# ===== SYSTEM FUNCTIONS =====
def mount_roots() -> None:
    if not os.path.exists("/mnt/gentoo"):
        os.mkdir("/mnt/gentoo")

    info("Mounting roots on /mnt/gentoo")
    execute("mount -L ROOTS /mnt/gentoo")

def mount() -> None:
    if not os.path.exists("/mnt/root"):
        os.mkdir("/mnt/root")

    info("Mounting root.img on /mnt/root")
    execute("mount -o ro,loop -t squashfs /mnt/gentoo/root.img /mnt/root")

    info("Mounting ESP on /mnt/root/boot/efi")
    execute("mount -L EFI /mnt/root/boot/efi")

    info("Mounting special filesystems")
    execute("mount -t proc /proc /mnt/root/proc")
    execute("mount --rbind /dev /mnt/root/dev")
    execute("mount --rbind /sys /mnt/root/sys")
    execute("mount --bind /run /mnt/root/run")
    execute("mount --make-slave /mnt/root/run")

def download_root(url: str) -> None:
    if os.path.exists("/mnt/gentoo/root.img"):
        os.remove("/mnt/gentoo/root.img")

    urllib.request.urlretrieve(url, "/mnt/gentoo/root.img")

def install_bootloader(platform, device="/dev/vda") -> None:
    # Check for grub binary, see if its grub2-install or grub-install
    if not os.path.exists("/mnt/root/usr/bin/grub-install"):
        grub = "grub2"
    else:
        grub = "grub"

    if "efi" in platform:
        chroot(
            f"""{grub}-install --force --target="{platform}" --efi-directory="/boot/efi" --boot-directory="/boot/efi"
{grub}-mkconfig -o /boot/efi/{grub}/grub.cfg"""
        )
    else:
        chroot(
            f"""{grub}-install --force --target="{platform}" --boot-directory="/boot/efi" {device}
{grub}-mkconfig -o /boot/efi/{grub}/grub.cfg"""
        )

def post_install(config: dict) -> None:
    layout_name = config["filesystem"]
    info("Mounting overlays & home")

    etc_path = "/mnt/root/overlay/etc"
    var_path = "/mnt/root/overlay/var"
    usr_path = "/mnt/root/overlay/usr"

    match layout_name:
        case "btrfs":
            execute("mount -L ROOTS -o subvol=overlay /mnt/root/overlay")
            execute("mount -L ROOTS -o subvol=home /mnt/root/home")
        case "btrfs_encryption_dev":
            execute(
                "mount /dev/mapper/xenia -o subvol=overlay /mnt/root/overlay"
            )
            execute("mount /dev/mapper/xenia -o subvol=home /mnt/root/home")
        case _:
            execute("mount -L OVERLAY /mnt/root/overlay")
            execute("mount -L HOME /mnt/root/home")

            etc_path = "/mnt/root/overlay"
            var_path = "/mnt/root/overlay"
            usr_path = "/mnt/root/overlay"

    for path in [
        etc_path + "/etc",
        etc_path + "/etcw",
        var_path + "/var",
        var_path + "/varw",
        usr_path + "/usr",
        usr_path + "/usrw",
    ]:
        if not os.path.isdir(path):
            os.mkdir(path)

    execute(
        f"mount -t overlay overlay -o lowerdir=/mnt/root/usr,upperdir={usr_path}/usr,workdir={usr_path}/usrw,ro /mnt/root/usr"
    )
    execute(
        f"mount -t overlay overlay -o lowerdir=/mnt/root/etc,upperdir={etc_path}/etc,workdir={etc_path}/etcw,rw /mnt/root/etc"
    )
    execute(
        f"mount -t overlay overlay -o lowerdir=/mnt/root/var,upperdir={var_path}/var,workdir={var_path}/varw,rw /mnt/root/var"
    )

    if config["username"] != "":
        info("Creating user")
        chroot(f"useradd -m {config['username']}")

        valid = False
        while not valid:
            try:
                subprocess.run(
                    f"chroot /mnt/root /bin/bash -c 'passwd {config['username']}'",
                    shell=True,
                    check=True,
                )
            except subprocess.CalledProcessError:
                valid = False
            else:
                valid = True

        chroot(f"usermod -aG wheel,video {config['username']}")

    flatpaks = get_flatpak_packages(config["applications"])

    if len(flatpaks) != 0:
        chroot(f"touch /etc/declare && echo '{flatpaks}' > /etc/declare/flatpak")

        if not os.path.exists("/mnt/root/usr/bin/rc-service"):
            chroot("systemctl enable declareflatpak")
        else:
            chroot("rc-update add declareflatpak")

# ===== MAIN FUNCTION =====
def parse_args() -> str:
    """This is a function to handle the parsing of command line args passed to the program."""
    parser = argparse.ArgumentParser(
        prog="RegicideOS Installer", description="Program to install RegicideOS."
    )
    parser.add_argument(
        "-c",
        "--config",
        dest="config_file",
        help="Run the installer automated from a toml config file.",
        default="",
        action="store",
    )

    args = parser.parse_args()

    return args.config_file

def main():
    config_file = parse_args()
    interactive = True

    info("BIOS detected." if not is_efi() else "EFI detected.")

    if config_file != "":
        interactive = False

        if not os.path.isfile(config_file):
            die(f"Config file {config_file} does not exist.")

        with open(config_file, "rb") as file:
            config_file = tomllib.load(file)

    info(
        f"Entering interactive mode. Default values are shown wrapped in square brackets like {Colours.blue}[this]{Colours.endc}. Press enter to accept the default.\n"
        if interactive
        else "Checking config"
    )

    config_parsed = parse_config(
        config_file if config_file != "" else {}, interactive=interactive
    )

    info(f"Done checking config")

    if interactive:
        warn(
            f"Drive partitioning is about to start. After this process, drive {config_parsed['drive']} will be erased. Press enter to continue."
        )
        input("")

    info(f"Partitioning drive {config_parsed['drive']}")
    partition_drive(
        config_parsed["drive"], LAYOUTS[config_parsed["filesystem"]]
    )

    info(f"Formatting drive {config_parsed['drive']}")
    format_drive(
        config_parsed["drive"], LAYOUTS[config_parsed["filesystem"]]
    )

    info("Starting installation")
    mount_roots()

    info("Downloading root image")
    root_url = get_url(config_parsed)
    download_root(root_url)
    mount()

    info("Installing bootloader")
    install_bootloader("x86_64-efi" if is_efi() else "i386-pc", device=config_parsed["drive"])

    info("Starting post-installation tasks")
    post_install(config_parsed)

if __name__ == "__main__":
    main()
