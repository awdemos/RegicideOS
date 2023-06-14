#!/usr/bin/env python3
import os
import subprocess
import tomllib
import argparse
from dataclasses import dataclass

@dataclass
class Colours:
    """This is a class to hold the ascii escape sequences for printing colours."""

    red: str = "\033[31m"
    endc: str = "\033[m"
    green: str = "\033[32m"
    yellow: str = "\033[33m"
    blue: str = "\033[34m"

def die(message):
    """This is a function to exit the program with a die message."""

    print(f"{Colours.red}[ERROR]{Colours.endc} {message}")
    exit(1)

def parse_args():
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
        config_file: string = args.config_name
        interactive: bool = True
    else:
        interactive: bool = False
    #print(args.config_name[0])

def var_checker(config_type, config_value) -> bool:
    return True

def parse_config(config_file) -> dict:
    """
    The is the function to parse the toml config file.

    Parameters:
        config_file (string): Contains the path to the configuration file.
    """

    with open(config_file, "rb") as file:
        config_parsed = tomllib.load(file)

    required_varibles = ["drive"]

    for i in range(len(required_varibles)):
        if not required_varibles[i] in config_parsed.keys():
            user_input = input(f"{Colours.red}[ERROR]{Colours.endc} The specified configuration file {config_file} is missing the required key {required_varibles[i]}. Would you like to specify it now? (Y/n) ")
            match user_input.lower():
                case "" | "y" | "yes":
                    append_new_key = True
                case "n" | "no":
                    append_new_key = False
                case other:
                    die("Invalid input - exiting!")
            if append_new_key:
                user_input = input(f"Please enter the value you would like to use for {required_varibles[i]}: ")
                if not var_checker(required_varibles[i], user_input):
                    die(f"{user_input} is an invalid entry for the key {required_varibles[i]} - exiting!")
                append_data = f"{required_varibles[i]} = {user_input}\n"
                file = open(config_file, "a")
                file.write(append_data)
                file.close()
                config_parsed[required_varibles[i]] = user_input
                print(config_parsed)
            else:
                die(f"Missing required variable {required_varibles[i]} - exiting!")

    exit(1) # Temporary - for development/debugging
    return config_parsed

class DriveHandler():
    """This is a class to handle drives."""

    def __init__(self):
        """
        The constructors for the Drive_Handler class.

        Parameters:
            drives       (array): Contains all suitable drives for installation.
            is_removable (array): Contains removable setting of drive (correspondes to drives)
        """

        self.drives = []
        self.is_removable = []
    def get_drives(self):
        """The function to get all possible drives for installation."""

        all_drives_array = next(os.walk('/sys/block'))[1]

        supported_types = ["sd", "nvme", "mmcblk"]

        for i in range(len(all_drives_array)):
            for j in range(len(supported_types)):
                if all_drives_array[i].startswith(supported_types[j]):
                    self.drives.append(all_drives_array[i])

        for i in range(len(self.drives)):
            file = open(f"/sys/block/{self.drives[i]}/removable","r")
            file_contents: str = file.readline()
            stripped_file = ''.join(file_contents.split())
            self.is_removable.append(stripped_file)
            file.close()

def mount(drive, stage4_url):
    """
    The function to mount all required filesystems for the install.

    Parameters:
        drive      (string): Contains the drive to install too.
        stage4_url (string): Contains the link to the latest stage4 rootfs.
    """

    if not os.path.exists(r'/mnt/xenia'):
        os.makedirs(r'/mnt/xenia')
    folder_paths = [r'/mnt/xenia', r'/mnt/xenia/roots', r'/mnt/xenia/overlay', r'/mnt/xenia/root', r'/mnt/xenia/home']
    for i in range (len(folder_paths)):
        if not os.path.exists(folder_paths[i]):
            os.makedirs(folder_paths[i])
    subprocess.run(["mount", "--label", "ROOTS", "/mnt/roots"])
    subprocess.run(["mount", "--label", "OVERLAY", "/mnt/overlay"])
    subprocess.run(["mount", "--label", "HOME", "/mnt/home"])

    folder_paths = [r'/mnt/overlay/var',r'/mnt/overlay/varw',r'/mnt/overlay/etc',r'/mnt/overlay/etcw']
    for i in range (len(folder_paths)):
        if not os.path.exists(folder_paths[i]):
            os.makedirs(folder_paths[i])
    subprocess.run(["rm", "-f", "/mnt/xenia/roots/root.img"])
    # TODO: use requests instead of shelling out
    subprocess.run(["wget", "-O", "/mnt/xenia/roots/root.img", stage4_url])
    
def main():
    """The main function."""

    parse_args()
    parse_config("config.toml")

    drive_handler = DriveHandler()
    drive_handler.get_drives()
    
    print(drive_handler.drives)
    print(drive_handler.is_removable)
    mount(drive_handler.drives, "https://repo.xenialinux.com/releases/current/root.img")

if __name__ == '__main__':
    main()
