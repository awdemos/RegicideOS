import shutil

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
            "type": "lvm"
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
            "type": "linux"
        },
    ],
}

def partition_drive(drive: str, layout: list) -> bool:
    command: str = f"cat <<EOF | sfdisk --wipe always --force {drive}\nlabel: gpt"
    drive_size: str = common.get_drive_size(drive)
    drive_size_class = drive_size[-1:]

    for partition in layout:
        size: str = ""

        if partition["size"] == True:
            size = ""
        elif partition["size"][-1] == "%":
            partition_size: float = float(drive_size[:-1]) * (float(partition["size"][:-1])/100)

            if partition_size < 1:
                partition_size *= 1000
                drive_size_class = common.SIZE_CLASS[common.SIZE_CLASS.index(drive_size_class)-1]

            partition_size = int(round(partition_size, 0))
            partition_size = str(partition_size) + drive_size_class

            size = f"size={partition_size}, "
        else:
            size = f"size={partition['size']}, "

        command += f"\n{size}type={partition['type']}"

    command += "\nEOF"
    
    common.execute(command)

def format_drive(drive: str, layout: list) -> None:
    name: str = common.execute(f"blkid -o device {drive}* | grep -vw -m 1 {drive}", override=True).strip().decode('UTF-8')
    
    for i, partition in enumerate(layout):
        name = name[:-1] + str(i+1)
        match partition["format"]:
            case "vfat":
                if "label" in partition:
                    common.execute(f"mkfs.vfat -F 32 -n {partition['label']} {name}")
                else:
                    common.execute(f"mkfs.vfat -F 32 {name}")

            case "ext4":
                if "label" in partition:
                    common.execute(f"mkfs.ext4 -L {partition['label']} {name}")
                else:
                    common.execute(f"mkfs.ext4 {name}")

            case "btrfs":
                if "label" in partition:
                    common.execute(f"mkfs.btrfs -L {partition['label']} {name}")
                else:
                    common.execute(f"mkfs.btrfs {name}")

                if "subvolumes" in partition:
                    common.execute(f"mkdir /mnt/temp")
                    common.execute(f"mount {name} /mnt/temp")

                    for subvolume in partition["subvolumes"]:
                        common.execute(f"btrfs subvolume create /mnt/temp{subvolume}")
                    
                    common.execute(f"umount {name}")
            
            case "lvm":
                common.execute(f"pvcreate -ff {name}")
                common.execute(f"vgcreate -ff {partition['name']} {name}")

                for i, lv in enumerate(partition["lvs"]):
                    if lv["size"] == True:
                        common.execute(f"lvcreate -l 100%FREE -n lv{i} {partition['name']}")
                    elif lv["size"][-1] == "%":
                        common.execute(f"lvcreate -l {lv['size']}FREE -n lv{i} {partition['name']}")
                    else:
                        common.execute(f"lvcreate -L {lv['size']} -n lv{i} {partition['name']}")

                format_drive(f"/dev/mapper/{partition['name']}-", partition["lvs"])
