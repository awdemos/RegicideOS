#!/usr/bin/env python3
import argparse
import os
import sys
import tomllib

import common
import config


def parse_args() -> str:
    """This is a function to handle the parsing of command line args passed to the program."""

    parser = argparse.ArgumentParser(
        prog='Xenia Installer',
        description='Program to install Xenia Linux.'
    )
    parser.add_argument(
        '-c', '--config',
        dest='config_file',
        help='Run the installer automated from a toml config file.',
        default='',
        action="store"
    )

    args = parser.parse_args()

    return args.config_file


def main():
    if not os.path.isdir("/sys/firmware/efi"): # Checking that host system supports UEFI.
        common.die("This installer does not currently support BIOS systems. Please (if possible) enable UEFI.")

    config_file = parse_args()
    interactive = True

    if config_file != "":
        interactive = False

        if not os.path.isfile(config_file):
            common.die(f"Config file {config_file} does not exist.")

        with open(config_file, "rb") as file:
            config_file = tomllib.load(file)

    config_parsed = config.parse_config(config_file if config_file != "" else {}, interactive=interactive)

if __name__ == '__main__':
    main()
