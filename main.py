#!/usr/bin/env python3
import argparse
import os
import tomllib

import common
import config
import drive
import system


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

    common.info(f"Entering interactive mode. Default values are shown wrapped in square brackets like {common.Colours.blue}[this]{common.Colours.endc}. Press enter to accept the default.\n" if interactive else "Checking config")

    config_parsed = config.parse_config(config_file if config_file != "" else {}, interactive=interactive)

    common.info(f"Done checking config")

    if interactive:
        common.warn(f"Drive partitioning is about to start. After this process, drive {config_parsed['drive']} will be erased. Press enter to continue.")
        input("")

    common.info(f"Partitioning drive {config_parsed['drive']}")
    drive.partition_drive(config_parsed['drive'], drive.LAYOUTS[config_parsed['filesystem']])

    common.info(f"Formatting drive {config_parsed['drive']}")
    drive.format_drive(config_parsed['drive'], drive.LAYOUTS[config_parsed['filesystem']])

    common.info("Starting installation")
    system.mount_roots()

    common.info("Downloading root image")
    system.download_root(config_parsed["root_url"])
    system.mount()

    common.info("Installing bootloader")
    system.install_bootloader()

    common.info("Starting post-installation tasks")
    system.post_install()


if __name__ == '__main__':
    main()
