import common


def fix_config(
    config_file: dict, interactive: bool, key: str, valid: str, default: str
):
    if not interactive:
        common.die(
            f"The value specified for '{key}' in the configuration file is not valid or missing. Valid options are: {valid}."
        )

    new_value = input(
        f"Enter value for '{key}'. Valid options are {valid}.\n{common.Colours.blue}[{default}]{common.Colours.endc}: "
    )
    config_file[key] = new_value if new_value != "" else default

    parse_config(config_file, interactive=True)

    return config_file


def parse_config(config_file: dict, interactive: bool = False) -> dict or str:
    """
    Parses config file and presents an interactive mode if empty config is specified.
    """

    # This code is a bit of a mess, but here is a bit of an explanation
    # VALIDITY is a dictionary with each config item. In the dictionary for each config item:
    # - func is the function to check validity
    # - mode is the validity mode
    # - default is the default value
    # - valid_text is the valid options in case of the validity mode being 'check'.
    #
    # VALIDTY MODES:
    # In the 'execute' mode, this function will execute that function and use the list it returns as a list of valid options.
    # In the 'check' mode, this function will execute the function with the value the user gives it (through config or interactive).
    # If the function returns False, the input is deemed to be invalid. If the function returns True, the input is valid.

    VALIDITY: dict = {
        "drive": {"func": common.get_drives, "mode": "execute", "default": ""},
        "root_url": {
            "func": common.check_url,
            "mode": "check",
            "default": "https://repo.xenialinux.com/releases/current/root.img",
            "valid_text": "a URL that points to a root.img",
        },
        "filesystem": {"func": common.get_fs, "mode": "execute", "default": "btrfs"},
    }

    for key in VALIDITY:
        default = VALIDITY[key]["default"]

        match VALIDITY[key]["mode"]:
            case "execute":
                valid = VALIDITY[key]["func"]()

                if key not in config_file or config_file[key] not in valid:
                    config_file = fix_config(
                        config_file, interactive, key, valid, default
                    )

            case "check":
                if key not in config_file or not VALIDITY[key]["func"](
                    value=config_file[key]
                ):
                    config_file = fix_config(
                        config_file,
                        interactive,
                        key,
                        VALIDITY[key]["valid_text"],
                        default,
                    )

    return config_file
