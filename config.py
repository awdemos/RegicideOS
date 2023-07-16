import common

from drive import LAYOUTS


def fix_config(config_file: dict, interactive: bool, key: str, valid: str, default: str):
    if not interactive:
        common.die(f"The value specified for '{key}' in the configuration file is not valid or missing. Valid options are: {valid}.")
    
    new_value = input(f"Enter value for '{key}'. Valid options are {valid}.\n{common.Colours.blue}[{default}]{common.Colours.endc}: ")
    config_file[key] = new_value if new_value != "" else default

    parse_config(config_file, interactive=True)

    return config_file


def parse_config(config_file: dict, interactive: bool = False) -> dict or str:
    """
    Parses config file and presents an interactive mode if empty config is specified.
    """

    # This code is a bit of a mess, but here is a bit of an explanation
    # VALIDITY is a dictionary with each config item, and then a list. 
    # - item 0 is the function to check validity
    # - item 1 is the validity mode
    # - item 2 is the default value 
    # - item 3 is the valid options in case of the validity mode being 'check'.
    # 
    # VALIDTY MODES:
    # In the 'execute' mode, this function will execute that function and use the list it returns as a list of valid options.
    # In the 'check' mode, this function will execute the function with the key the user gives it (through config or interactive).
    # If the function returns False, the input is deemed to be invalid. If the function returns True, the input is valid.

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
                if key not in config_file or not VALIDITY[key][0](value=config_file[key]):
                    config_file = fix_config(config_file, interactive, key, VALIDITY[key][3], default)


    return config_file