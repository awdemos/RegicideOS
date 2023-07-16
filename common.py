import subprocess, os, sys, requests
from dataclasses import dataclass

import drive

PRETEND = True

SIZE_CLASS = ["K", "M", "G", "T", "P"]

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
        command = subprocess.Popen(command_string.strip(" "), stdout=subprocess.PIPE, shell=True)
        out, _ = command.communicate()
        return out

    print(f"[COMMAND]\n{command_string}")


def get_drive_size(drive: str) -> str:
    return execute(f"lsblk -o SIZE {drive} | grep -v -m 1 SIZE", override=True).strip().decode('UTF-8')


def check_drive_size(value: str = "") -> bool:
    drive_size: str = get_drive_size(value)

    if SIZE_CLASS.index(drive_size[-1:]) > SIZE_CLASS.index("G"):
        return True

    if drive_size[-1:] == "G":
        if float(drive_size[:-1]) >= 12:
            return True
    
    return False


def get_drives(value: str = "") -> list:
    """The function to get all possible drives for installation."""

    all_drives_array = [f"/dev/{item}" for item in next(os.walk('/sys/block'))[1] if check_drive_size(f"/dev/{item}")]

    return all_drives_array


def get_fs(value: str = "") -> list:
    return list(drive.LAYOUTS.keys())


def check_url(value: str) -> bool:
    try:
        response = requests.head(value)

        if response.status_code == 200 and value.split("/")[-1] == "root.img":
            return True
        
        warn("URL entered is not reachable, or does not end in root.img. Please try again.")
    except:
        warn("URL entered is not valid - did you forget 'https://'?")
    
    return False
