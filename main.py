#!/usr/bin/env python3
import argparse
import os
import subprocess
import urllib.request
import sys
import tomllib
from dataclasses import dataclass

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
    sys.exit(1)

def parse_args() -> list:
    """This is a function to handle the parsing of command line args passed to the program."""

    parser = argparse.ArgumentParser(
        prog='Xenia Installer',
        description='Program to install Xenia Linux.'
    )
    parser.add_argument(
        '-a', '--automated',
        action='append',
        nargs=1,
        dest='config_name',
        help='Run the installer automated from a toml config file.'
    )

    args = parser.parse_args()

    if args.config_name is not None:
        if len(args.config_name) > 1:
            die("Mutiple config files provided - only 1 can be parsed.")
        config_file: list = args.config_name[0]
    else:
        config_file: list = []

    print(config_file) # debugging
    print(type(config_file)) # debugging

    return config_file

    # print(args.config_name[0])

class VarChecker:
    """
    This is a class to ensure the validity of variables passed to the script.

    Attributes:
        var_type      (string): Contains the type of variable to check.
        check_var (string): Contains the contents of the variable to check.
    """

    def __init__(self, var_type: str, check_var: str) -> None:
        """
        The constructors for the VarChecker class.

        Parameters:
            var_type  (string): Contains the type of variable to check.
            check_var (string): Contains the contents of the variable to check.
        """

        self.var_type: str = var_type
        self.check_var: str = check_var

    def checker(self) -> bool:
        """The function to sort the type of variable to sort, and call the correct function."""

        match self.var_type:
            case "drive":
                return self.drive()
            case other:
                return False

    def drive(self) -> bool:
        """The function to validate the drive variable."""

        drives = get_drives()

        return self.check_var in drives

def parse_config(config_file: str) -> dict:
    """
    The is the function to parse the toml config file.

    Parameters:
        config_file (string): Contains the path to the configuration file.
    """

    with open(config_file, "rb") as file:
        config_parsed = tomllib.load(file)

    required_varibles: list = ["drive"]

    for i in range(len(required_varibles)):
        if not required_varibles[i] in config_parsed.keys():
            user_input: str = input(f"{Colours.red}[ERROR]{Colours.endc} The specified configuration file {config_file} is missing the required key {required_varibles[i]}. Would you like to specify it now? (Y/n) ")
            match user_input.lower():
                case "" | "y" | "yes":
                    append_new_key: bool = True
                case "n" | "no":
                    append_new_key: bool = False
                case other:
                    die("Invalid input - exiting!")
            if append_new_key:
                user_input: str = input(f"Please enter the value you would like to use for {required_varibles[i]}: ")
                var_checker = VarChecker(required_varibles[i], user_input)

                if not var_checker.checker():
                    die(f"{user_input} is an invalid entry for the key {required_varibles[i]} - exiting!")

                append_data: str = f"{required_varibles[i]} = \"{user_input}\"\n"

                with open(config_file, "a") as file:
                    file.write(append_data)

                config_parsed[required_varibles[i]] = user_input
                print(config_parsed)
            else:
                die(f"Missing required variable {required_varibles[i]} - exiting!")

    return config_parsed

def get_drives() -> list:
    """The function to get all possible drives for installation."""

    all_drives_array = next(os.walk('/sys/block'))[1]

    supported_types: list = ["sd", "nvme", "mmcblk"]

    drives: list = [i for i in all_drives_array if any(i.startswith(drivetype) for drivetype in supported_types)]

    return drives

def download_image(drive: str, stage4_url: str) -> None:
    """
    The function to mount all required filesystems for the install.

    Parameters:
        drive      (string): Contains the drive to install too.
        stage4_url (string): Contains the link to the latest stage4 rootfs.
    """

    if os.path.isfile("/mnt/xenia/roots/root.img"):
        os.remove("/mnt/xenia/roots/root.img")

    print(f"{Colours.yellow}[LOG]{Colours.endc} Downloading root image - this will take a while.")
    urllib.request.urlretrieve(stage4_url, "/mnt/xenia/roots/root.img")
    print(f"{Colours.green}[LOG]{Colours.endc} Root image succesfully downloaded!")

    subprocess.run(["mount", "-o", "ro,loop", "-t", "squashfs", "/mnt/roots/root.img", "/mnt/root"])

    if "nvme" in drive:
        subprocess.run(["mount", f"/dev/{drive}p1", "/mnt/root/boot/efi"])
    else:
        subprocess.run(["mount", f"/dev/{drive}1", "/mnt/root/boot/efi"])

    subprocess.run(["mount", "-t", "proc", "/proc", "/mnt/root/proc"])
    subprocess.run(["mount", "--rbind", "/dev", "/mnt/root/dev"])
    subprocess.run(["mount", "--bind", "/run", "/mnt/root/run"])
    subprocess.run(["mount", "--make-slave", "/mnt/root/run"])
    subprocess.run(["mount", "--move", "/mnt/home", "/mnt/root/home"])

def partition_drives(drive: str) -> None:
    """
    The function to partition and format the drives.

    Parameters:
        drive (string): Contains the drive to install to.
    """

    try:
        os.system(f"sgdisk -o -n 1::+500M -t 1:EF00 -c 1:\"boot\" -n 2::: -t 2:8300 -c 2:\"root\" -p {drive}")

        if "nvme" in drive:
            os.system(f"mkfs.vfat -F 32 -n EFI {drive}p1")
            os.system(f"mkfs.btrfs -L ROOTS {drive}p2")
        else:
            os.system(f"mkfs.vfat -F 32 -n EFI {drive}1")
            os.system(f"mkfs.btrfs -L ROOTS {drive}2")

        if not os.path.exists(r'/mnt/xenia'):
            os.makedirs(r'/mnt/xenia')

        os.system("mount -L ROOTS /mnt/xenia")
        os.system("btrfs subvolume create /mnt/xenia/home")
        os.system("btrfs subvolume create /mnt/xenia/overlay")
        os.system("btrfs subvolume create /mnt/gentoo/overlay/etc")
        os.system("btrfs subvolume create /mnt/gentoo/overlay/var")
        os.system("btrfs subvolume create /mnt/gentoo/overlay/usr")
    except:
        die("Could not format partitions - exiting!")

def chroot() -> None:
    """The function to run the required chroot commands."""

    subprocess.run(["chroot", "/mnt/root", "grub-install", "--modules=lvm", "--target=\"x86_64-efi\"", "--efi-directory=\"/boot/efi\"", "--boot-directory=\"/boot/efi\""])
    subprocess.run(["chroot", "/mnt/root", "grub-mkconfig", "-o", "/boot/efi/grub/grub.cfg"])
    subprocess.run(["chroot", "/mnt/root", "chown", "xenia:xenia", "/home/xenia"])

def interactive() -> None:
    """The function to interactivly ask the user for configuration settings."""

    

def main():
    if not os.path.isdir("/sys/firmware/efi"): # Checking that host system supports UEFI.
        die("This installer does not currently support BIOS systems. Please (if possible) enable UEFI.")

    config_file = parse_args()

    if len(config_file) == 0:
        # die("Installer does not currently support interactive mode!")
        config_file = "auto-generated-configuration.toml"

    config_parsed = parse_config(str(config_file[0]))

    install_drive = config_parsed['drive']

    mount(install_drive, "https://repo.xenialinux.com/releases/current/root.img")

if __name__ == '__main__':
    main()
