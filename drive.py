import os

import common

LAYOUTS = {
    "traditional": [
        {"size": "512M", "label": "EFI", "format": "vfat", "type": "uefi"},
        {"size": "8G", "label": "ROOTS", "format": "ext4", "type": "linux"},
        {"size": "30%", "label": "OVERLAY", "format": "ext4", "type": "linux"},
        {"size": True, "label": "HOME", "format": "ext4", "type": "linux"},
    ],
    "lvm": [
        {"size": "512M", "label": "EFI", "format": "vfat", "type": "uefi"},
        {
            "size": True,
            "format": "lvm",
            "name": "vg0",
            "lvs": [
                {"size": "8G", "label": "ROOTS", "format": "ext4"},
                {"size": "30%", "label": "OVERLAY", "format": "ext4"},
                {"size": True, "label": "HOME", "format": "ext4"},
            ],
            "type": "lvm",
        },
    ],
    "btrfs": [
        {"size": "512M", "label": "EFI", "format": "vfat", "type": "uefi"},
        {
            "size": True,
            "label": "ROOTS",
            "format": "btrfs",
            "subvolumes": [
                "/home",
                "/overlay",
                "/overlay/etc",
                "/overlay/var",
                "/overlay/usr",
            ],
            "type": "linux",
        },
    ],
    "btrfs_encryption_dev": [
        {"size": "512M", "label": "EFI", "format": "vfat", "type": "uefi"},
        {"size": "8G", "label": "ROOTS", "format": "ext4", "type": "linux"},
        {
            "size": True,
            "label": "XENIA",
            "format": "luks",
            "inside": {
                "size": True,
                "format": "btrfs",
                "subvolumes": [
                    "/home",
                    "/overlay",
                    "/overlay/etc",
                    "/overlay/var",
                    "/overlay/usr",
                ],
            },
            "type": "linux",
        },
    ],
}


def human_to_bytes(size) -> int:
    sizes = {
        "B": 1,
        "K": 1024,
        "M": 1024**2,
        "G": 1024**3,
        "T": 1024**4,
        "P": 1024**5,
    }

    return int(size[:-1]) * sizes[size[-1]]


def is_efi() -> bool:
    return os.path.isdir("/sys/firmware/efi")


def partition_drive(drive: str, layout: list) -> bool:
    common.execute(f"umount -ql {drive}?*")
    vgs = common.execute("vgs | awk '{ print $1 }' | grep -vw VG")

    if vgs != None:
        vgs = [line.strip().decode("UTF-8") for line in vgs.splitlines()]
        for vg in vgs:
            common.execute(f"vgchange -an {vg}")

    command: str = f"cat <<EOF | sfdisk -q --wipe always --force {drive}\nlabel: gpt"
    drive_size: int = common.get_drive_size(drive)
    running_drive_size: int = -1048576  # for BIOS systems, -1M so there is space for bios boot

    for partition in layout:
        size: str = ""

        if partition["size"] == True:
            if not is_efi():
                size = f"size={(running_drive_size*1024)}K, " # convert to Kibibites instead of using sectors
        elif partition["size"][-1] == "%":
            partition_size: float = drive_size * (float(partition["size"][:-1]) / 100)
            partition_size = round(partition_size, 0)
            running_drive_size += partition_size
            size = f"size={partition_size}, "
        else:
            running_drive_size += human_to_bytes(partition["size"])
            size = f"size={partition['size']}, "

        command += f"\n{size}type={partition['type']}"

    if not is_efi():
        command += "\ntype=c"

    command += "\nEOF"

    common.execute(command)


def format_drive(drive: str, layout: list) -> None:
    # formats drives i suppose

    # i think this is used for lvm to find the lv* - fuck lvm layout for making me do this.
    # aside: why do we support LVM in the installer? it's not legacy as we changed the naming scheme (wont work on < 0.3).
    #        FUCK LVM, ALL MY HOMIES HATE LVM
    #
    # just think of this as finding the drive we are installing to, same with number as the partition.
    # this is stupid messy and I am SURE there is a better way to do this, but oh well - it works.

    name: str = "/dev/" + common.execute(
        f"lsblk -o NAME --list | grep -m 1 '{drive.split('/')[-1]}.'",
        override=True,
    ).strip().decode("UTF-8")

    noNum = False

    if (
        name == "/dev/"
    ):  # the drive passed in doesnt have partitions/numbers at the end (luks inside partition)
        name = drive
        noNum = True
    else:
        name = name.replace("-", "/")
        number = int(name[-1:])

    for i, partition in enumerate(layout):
        if not noNum:
            name = name[:-1] + str(number)  # enumerates partitions
            number += 1

        match partition["format"]:
            case "vfat":
                if "label" in partition:
                    common.execute(f"mkfs.vfat -F 32 -n {partition['label']} {name}")
                else:
                    common.execute(f"mkfs.vfat -F 32 {name}")

            case "ext4":
                if "label" in partition:
                    common.execute(f"mkfs.ext4 -q -L {partition['label']} {name}")
                else:
                    common.execute(f"mkfs.ext4 -q {name}")

            case "btrfs":
                if "label" in partition:
                    common.execute(f"mkfs.btrfs -q -f -L {partition['label']} {name}")
                else:
                    common.execute(f"mkfs.btrfs -q -f {name}")

                if "subvolumes" in partition:
                    if not os.path.exists("/mnt/temp"):
                        os.mkdir("/mnt/temp")

                    common.execute(f"mount {name} /mnt/temp")

                    for subvolume in partition["subvolumes"]:
                        common.execute(f"btrfs subvolume create /mnt/temp{subvolume}")

                    common.execute(f"umount {name}")

            case "lvm":
                common.execute(f"yes | pvcreate -ff -q {name}")
                common.execute(f"vgcreate -ff -q {partition['name']} {name}")

                for i, lv in enumerate(partition["lvs"]):
                    if lv["size"] == True:
                        common.execute(
                            f"lvcreate -q -l 100%FREE -n lv{i} {partition['name']}"
                        )
                    elif lv["size"][-1] == "%":
                        common.execute(
                            f"lvcreate -q -l {lv['size']}FREE -n lv{i} {partition['name']}"
                        )
                    else:
                        common.execute(
                            f"lvcreate -q -L {lv['size']} -n lv{i} {partition['name']}"
                        )

                format_drive(f"/dev/mapper/{partition['name']}-", partition["lvs"])

            case "luks":
                common.execute(f"cryptsetup -q luksFormat {name}")
                common.execute(
                    f"cryptsetup -q config {name} --label {partition['label']}"
                )
                common.execute(
                    f"cryptsetup luksOpen /dev/disk/by-label/{partition['label']} xenia"
                )

                format_drive(f"/dev/mapper/xenia", [partition["inside"]])
