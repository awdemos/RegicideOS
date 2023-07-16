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
    common.execute(f"umount -q {drive}")
    vgs = common.execute("vgs | awk '{ print $1 }' | grep -vw VG")

    if vgs != None:
        vgs = [line.strip().decode('UTF-8') for line in vgs.splitlines()]
        for vg in vgs:
            common.execute(f"vgchange -an {vg}")
    
    command: str = f"cat <<EOF | sfdisk -q --wipe always --force {drive}\nlabel: gpt"
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
    name: str = "/dev/" + common.execute(f"sudo lsblk -o NAME --list | grep -m 1 '{drive.split('/')[-1]}.'", override=True).strip().decode('UTF-8')
    name = name.replace("-", "/")
    number = int(name[-1:])

    for i, partition in enumerate(layout):
        name = name[:-1] + str(number)
        number += 1
        
        match partition["format"]:
            case "vfat":
                if "label" in partition:
                    common.execute(f"mkfs.vfat -q -F 32 -n {partition['label']} {name}")
                else:
                    common.execute(f"mkfs.vfat -q -F 32 {name}")

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
                    common.execute(f"mkdir /mnt/temp")
                    common.execute(f"mount {name} /mnt/temp")

                    for subvolume in partition["subvolumes"]:
                        common.execute(f"btrfs subvolume create /mnt/temp{subvolume}")
                    
                    common.execute(f"umount {name}")
            
            case "lvm":
                common.execute(f"yes | pvcreate -ff -q {name}")
                common.execute(f"vgcreate -ff -q {partition['name']} {name}")

                for i, lv in enumerate(partition["lvs"]):
                    if lv["size"] == True:
                        common.execute(f"lvcreate -q -l 100%FREE -n lv{i} {partition['name']}")
                    elif lv["size"][-1] == "%":
                        common.execute(f"lvcreate -q -l {lv['size']}FREE -n lv{i} {partition['name']}")
                    else:
                        common.execute(f"lvcreate -q -L {lv['size']} -n lv{i} {partition['name']}")

                format_drive(f"/dev/mapper/{partition['name']}-", partition["lvs"])
