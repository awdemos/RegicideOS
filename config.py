import common

from drive import LAYOUTS


def fix_config(config_file: dict, interactive: bool, key: str, valid: str, default: str):
    if not interactive:
        common.die(f"The value specified for '{key}' in the configuration file is not valid or missing. Valid options are: {valid}.")
    
    new_value = input(f"Enter value for '{key}'. Valid options are {valid}.\n[{default}]: ")
    config_file[key] = new_value if new_value != "" else default

    parse_config(config_file, interactive=True)

    return config_file


def parse_config(config_file: dict, interactive: bool = False) -> dict or str:
    """
    The is the function to parse the toml config file.

    Parameters:
        config_file (string): Contains the path to the configuration file.
    """

    VALIDITY: dict = {"drive": [common.get_drives, "execute", ""], 
                      "root_url": [common.check_url, "check", "https://repo.xenialinux.com/releases/current/root.img", "a URL that points to a root.img"], 
                      "filesystem": [common.get_fs, "execute", "btrfs"]}

    for key in VALIDITY:
        default = VALIDITY[key][2]

        match VALIDITY[key][1]:
            case "execute":
                valid = VALIDITY[key][0]()

                if key not in config_file or config_file[key] not in valid:
                    config_file = fix_config(config_file, interactive, key, valid, default)

            case "check":
                if key not in config_file or config_file[key] != VALIDITY[key][0](value=config_file[key]):
                    config_file = fix_config(config_file, interactive, key, VALIDITY[key][3], default)


    return config_file