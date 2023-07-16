import common
import tomllib
import os


def parse_config(config_file: str) -> dict:
    """
    The is the function to parse the toml config file.

    Parameters:
        config_file (string): Contains the path to the configuration file.
    """

    VALIDITY: dict = {"drive": common.get_drives}

    if not os.path.isfile(config_file):
        common.die(f"Config file {config_file} does not exist.")

    with open(config_file, "rb") as file:
        config_parsed = tomllib.load(file)

    for key in VALIDITY:
        valid = VALIDITY[key]()
        if key not in config_parsed or config_parsed[key] not in valid:
            common.die(f"The value specified for '{key}' in configuration file {config_file} is not valid or missing. Valid options are: {valid}.")
            exit()

    return config_parsed