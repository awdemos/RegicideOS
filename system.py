import common
import os
import urllib
import subprocess
import tomllib


def chroot(command: str) -> None:
    common.execute(f'chroot /mnt/root /bin/bash <<"EOT"\n{command}\nEOT')


def post_install(config: dict) -> None:
    layout_name = config["filesystem"]
    common.info("Mounting overlays & home")

    etc_path = "/mnt/root/overlay/etc"
    var_path = "/mnt/root/overlay/var"
    usr_path = "/mnt/root/overlay/usr"

    match layout_name:
        case "btrfs":
            common.execute("mount -L ROOTS -o subvol=overlay /mnt/root/overlay")
            common.execute("mount -L ROOTS -o subvol=home /mnt/root/home")
        case "btrfs_encryption_dev":
            common.execute(
                "mount /dev/mapper/xenia -o subvol=overlay /mnt/root/overlay"
            )
            common.execute("mount /dev/mapper/xenia -o subvol=home /mnt/root/home")
        case _:
            common.execute("mount -L OVERLAY /mnt/root/overlay")
            common.execute("mount -L HOME /mnt/root/home")

            etc_path = "/mnt/root/overlay"
            var_path = "/mnt/root/overlay"
            usr_path = "/mnt/root/overlay"

    for path in [
        etc_path + "/etc",
        etc_path + "/etcw",
        var_path + "/var",
        var_path + "/varw",
        usr_path + "/usr",
        usr_path + "/usrw",
    ]:
        if not os.path.isdir(path):
            os.mkdir(path)

    common.execute(
        f"mount -t overlay overlay -o lowerdir=/mnt/root/usr,upperdir={usr_path}/usr,workdir={usr_path}/usrw,ro /mnt/root/usr"
    )
    common.execute(
        f"mount -t overlay overlay -o lowerdir=/mnt/root/etc,upperdir={etc_path}/etc,workdir={etc_path}/etcw,rw /mnt/root/etc"
    )
    common.execute(
        f"mount -t overlay overlay -o lowerdir=/mnt/root/var,upperdir={var_path}/var,workdir={var_path}/varw,rw /mnt/root/var"
    )

    if config["username"] != "":
        common.info("Creating user")
        chroot(f"useradd -m {config['username']}")

        valid = False
        while not valid:
            try:
                subprocess.run(
                    f"chroot /mnt/root /bin/bash -c 'passwd {config['username']}'",
                    shell=True,
                    check=True,
                )
            except subprocess.CalledProcessError:
                valid = False
            else:
                valid = True

        chroot(f"usermod -aG wheel {config['username']}")

    with open("system.toml", "rb") as system_conf:
        flatpaks = " ".join(
            tomllib.load(system_conf)["applications"][config["applications"]]
        )

    if len(flatpaks) != 0:
        chroot(f"touch /etc/declare && echo '{flatpaks}' > /etc/declare/flatpak")

        if not os.path.exists("/mnt/root/usr/bin/rc-service"): 
            chroot("systemctl enable declareflatpak")
        else:
            chroot("rc-update add declareflatpak")


def install_bootloader(platform, device="/dev/vda") -> None:
    if "efi" in platform:
        chroot(
            f"""grub-install --modules=lvm --target="{platform}" --efi-directory="/boot/efi" --boot-directory="/boot/efi"
grub-mkconfig -o /boot/efi/grub/grub.cfg"""
        )
    else:
        chroot(
            f"""grub-install --modules=lvm --target="{platform}" --boot-directory="/boot/efi" {device}
grub-mkconfig -o /boot/efi/grub/grub.cfg"""
        )


def download_root(url: str) -> None:
    if os.path.exists("/mnt/gentoo/root.img"):
        os.remove("/mnt/gentoo/root.img")

    urllib.request.urlretrieve(url, "/mnt/gentoo/root.img")


def mount_roots() -> None:
    if not os.path.exists("/mnt/gentoo"):
        os.mkdir("/mnt/gentoo")

    common.info("Mounting roots on /mnt/gentoo")
    common.execute("mount -L ROOTS /mnt/gentoo")


def mount() -> None:
    if not os.path.exists("/mnt/root"):
        os.mkdir("/mnt/root")

    common.info("Mounting root.img on /mnt/root")
    common.execute("mount -o ro,loop -t squashfs /mnt/gentoo/root.img /mnt/root")

    common.info("Mounting ESP on /mnt/root/boot/efi")
    common.execute("mount -L EFI /mnt/root/boot/efi")

    common.info("Mounting special filesystems")
    common.execute("mount -t proc /proc /mnt/root/proc")
    common.execute("mount --rbind /dev /mnt/root/dev")
    common.execute("mount --rbind /sys /mnt/root/sys")
    common.execute("mount --bind /run /mnt/root/run")
    common.execute("mount --make-slave /mnt/root/run")
