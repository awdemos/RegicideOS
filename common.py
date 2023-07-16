import subprocess, os, sys, requests
from dataclasses import dataclass

import drive

PRETEND = True


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


def execute(command_string: str, override: bool = False) -> str:
    if not PRETEND or override:
        command = subprocess.Popen(command_string.strip(" "), stdout=subprocess.PIPE, shell=True)
        out, _ = command.communicate()
        return out

    print(f"[COMMAND]\n{command_string}")


def get_drives(value: str = "") -> list:
    """The function to get all possible drives for installation."""

    all_drives_array = [f"/dev/{item}" for item in next(os.walk('/sys/block'))[1]]

    return all_drives_array


def get_fs(value: str = "") -> list:
    return list(drive.LAYOUTS.keys())


def check_url(value: str) -> list:
    try:
        response = requests.head(value)

        if response.status_code == 200 and value.split("/")[-1] == "root.img":
            return value
    except:
        pass
    
    return ["https://example.com/"]
