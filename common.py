import subprocess
import os
import sys
import requests
import re
import tomllib
from dataclasses import dataclass

import drive

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

    if drive_size != "":
        return int(drive_size)

    return 0


def check_drive_size(value: str = "") -> bool:
    drive_size: int = get_drive_size(value)

    if drive_size > 12884901888:
        return True

    return False


def get_drives(value: str = "") -> list:
    """The function to get all possible drives for installation."""

    all_drives_array = [
        f"/dev/{item}"
        for item in next(os.walk("/sys/block"))[1]
        if check_drive_size(f"/dev/{item}")
    ]

    return all_drives_array


def get_fs(value: str = "") -> list:
    return list(drive.LAYOUTS.keys())


def get_package_sets(value: str = "") -> list:
    with open("system.toml", "rb") as system_conf:
        sets = list(tomllib.load(system_conf)["applications"].keys())

    return sets


def check_url(value: str) -> bool:
    try:
        response = requests.head(value)

        if response.status_code == 200 and ".img" in value.split("/")[-1]:
            return True

        warn("URL entered is not reachable, or does not end in .img. Please try again.")
    except:
        warn("URL entered is not valid - did you forget 'https://'?")

    return False


def check_username(value: str) -> bool:
    if value == "":
        return True

    matches = re.match(r"[a-z_][a-z0-9_]{0,30}", value)

    if matches != None:
        if matches[0] == value:
            return True

    return False
