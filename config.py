import common
import tomllib
import urllib.request
import subprocess

# The following stuff is verbatim copied from foxupdate. I'll find a better way later.

ARCH_TABLE = {
    "x86_64": "amd64"
}

arch = ARCH_TABLE[subprocess.run(["uname", "-m"], shell=True, capture_output=True, text=True).stdout.strip()]

def get_flavours(ret):
    manifest = tomllib.load(urllib.request.urlopen(f"{ret['repository']}Manifest.toml"))
    return [flavour for flavour in manifest.keys() if arch in manifest[flavour]["arch"]]


def get_releases(ret):
    manifest = tomllib.load(urllib.request.urlopen(f"{ret['repository']}Manifest.toml"))
    return [release for release in manifest[ret["flavour"]]["versions"] if arch in manifest[ret["flavour"]]["versions"][release]["arch"]]

    
def get_url(config_parsed):
    manifest = tomllib.load(
        urllib.request.urlopen(f"{config_parsed['repository']}Manifest.toml")
    )

    filename = manifest[config_parsed["flavour"]]["versions"][
        config_parsed["release_branch"]
    ]["filename"]

    return f"{config_parsed['repository']}{arch}/{config_parsed['release_branch']}/{filename}"


# End of foxupdate code


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
        "repository": {
            "func": common.check_url,
            "mode": "check",
            "default": "https://repo.xenialinux.com/releases/",
            "valid_text": "a URL that points to a Xenia repository (remember the trailing slash)",
        },
        "flavour": {
            "func": get_flavours,
            "mode": "execute",
            "default": "gnome-systemd",
            "return": True,
        },
        "release_branch": {
            "func": get_releases,
            "mode": "execute",
            "default": "main",
            "return": True,
        },
        "filesystem": {"func": common.get_fs, "mode": "execute", "default": "btrfs"},
        "username": {
            "func": common.check_username,
            "mode": "check",
            "default": "",
            "valid_text": "a username to use for the main account (leave empty for none)",
        },
        "applications": {
            "func": common.get_package_sets,
            "mode": "execute",
            "default": "recommended",
        },
    }

    for key in VALIDITY:
        default = VALIDITY[key]["default"]

        match VALIDITY[key]["mode"]:
            case "execute":
                if "return" in VALIDITY[key].keys():
                    if VALIDITY[key]["return"] == True:
                        valid = VALIDITY[key]["func"](ret=config_file) 
                    else:
                        valid = VALIDITY[key]["func"]()
                else:
                    valid = VALIDITY[key]["func"]()

                if key not in config_file or config_file[key] not in valid:
                    config_file = fix_config(
                        config_file, interactive, key, valid, default
                    )

            case "check":
                do_return = False
                if "return" in VALIDITY[key].keys():
                    do_return = VALIDITY[key]["return"]

                if do_return: # yes, i know this is horrific. I'm sorry.
                    if key not in config_file or not VALIDITY[key]["func"](
                        value=config_file[key], ret=config_file
                    ):
                        config_file = fix_config(
                            config_file,
                            interactive,
                            key,
                            VALIDITY[key]["valid_text"],
                            default,
                        )
                else:
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
