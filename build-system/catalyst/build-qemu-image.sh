#!/bin/bash
# RegicideOS QEMU Disk Image Builder
# Creates a bootable QCOW2 disk image from a Catalyst stage4 tarball

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
TARBALL=""
OUTPUT="${SCRIPT_DIR}/output/regicide-qemu.qcow2"
DISK_SIZE="20G"
ENCRYPT=false
PASSPHRASE_FILE=""
DIRECT_DEVICE=""
NO_CONVERT=false

usage() {
    cat << EOF
Usage: $0 [OPTIONS] <stage4-archive> [output-qcow2] [disk-size]

  stage4-archive       Path to the stage4 rootfs archive. Accepted formats:
                         .tar.xz tarball (legacy) or SquashFS image (.img,
                         .squashfs). The archive is extracted onto ROOTS.
  output-qcow2         Path for the output .qcow2 file (optional)
  disk-size            Disk size for the image, e.g. 20G (optional, default: 20G)

Options:
  --encrypt            Encrypt the ROOTS partition with LUKS2
  --passphrase-file    Path to a file containing the LUKS passphrase
                       (required with --encrypt; use - for stdin)
  --direct-device      Use an existing raw block device instead of creating a
                       temporary raw file and loop device (e.g. /dev/vda).
  --no-convert         Do not convert the raw image to QCOW2 at the end.
                       Useful when a wrapper will convert the raw disk.

Examples:
  sudo $0 /var/tmp/catalyst/builds/default/stage4-amd64-systemd-cosmic-*.tar.xz
  sudo $0 --encrypt --passphrase-file /run/luks-passphrase /path/to/stage4.tar.xz ./my-image.qcow2 30G
  sudo $0 --direct-device /dev/vda --no-convert /path/to/stage4.img /tmp/image.qcow2 20G
EOF
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --encrypt)
            ENCRYPT=true
            shift
            ;;
        --passphrase-file)
            PASSPHRASE_FILE="${2:-}"
            shift 2
            ;;
        --direct-device)
            DIRECT_DEVICE="${2:-}"
            shift 2
            ;;
        --no-convert)
            NO_CONVERT=true
            shift
            ;;
        -h|--help)
            usage
            ;;
        -*)
            echo "Error: unknown option: $1"
            usage
            ;;
        *)
            if [[ -z "${TARBALL}" ]]; then
                TARBALL="$1"
            elif [[ "${OUTPUT}" == "${SCRIPT_DIR}/output/regicide-qemu.qcow2" ]]; then
                OUTPUT="$1"
            else
                DISK_SIZE="$1"
            fi
            shift
            ;;
    esac
done

if [[ -z "${TARBALL}" ]]; then
    echo "Error: stage4 archive path is required."
    usage
fi

if [[ ! -f "${TARBALL}" ]]; then
    echo "Error: stage4 archive not found: ${TARBALL}"
    exit 1
fi

if [[ "${ENCRYPT}" == true ]]; then
    if [[ -z "${PASSPHRASE_FILE}" ]]; then
        echo "Error: --passphrase-file is required when --encrypt is used."
        usage
    fi
    if [[ "${PASSPHRASE_FILE}" != "-" && ! -f "${PASSPHRASE_FILE}" ]]; then
        echo "Error: passphrase file not found: ${PASSPHRASE_FILE}"
        exit 1
    fi
fi

# ---------------------------------------------------------------------------
# Root check
# ---------------------------------------------------------------------------
if [[ $EUID -ne 0 ]]; then
    echo "This script must be run as root (partitioning and block devices require root)"
    exit 1
fi

# ---------------------------------------------------------------------------
# Dependency checks
# ---------------------------------------------------------------------------
if [[ -n "${DIRECT_DEVICE}" ]]; then
    REQUIRED_CMDS=(parted mkfs.vfat mkfs.btrfs btrfs tar)
else
    REQUIRED_CMDS=(parted mkfs.vfat mkfs.btrfs btrfs qemu-img tar losetup partprobe)
fi
if [[ "${ENCRYPT}" == true ]]; then
    REQUIRED_CMDS+=(cryptsetup)
fi
for cmd in "${REQUIRED_CMDS[@]}"; do
    if ! command -v "${cmd}" &> /dev/null; then
        echo "Error: required command '${cmd}' not found."
        exit 1
    fi
done

# ---------------------------------------------------------------------------
# Paths and temp setup
# ---------------------------------------------------------------------------
if [[ -n "${DIRECT_DEVICE}" ]]; then
    RAW_IMG="${DIRECT_DEVICE}"
else
    RAW_IMG="$(mktemp --suffix=.raw)"
fi
MOUNT_DIR="$(mktemp -d)"
OVERLAY_TMP="$(mktemp -d)"

# Resolve absolute paths
TARBALL="$(realpath -e "${TARBALL}")"
OUTPUT="$(realpath -m "${OUTPUT}")"
OUTPUT_DIR="$(dirname "${OUTPUT}")"
mkdir -p "${OUTPUT_DIR}"

LOOP_DEV=""

# ---------------------------------------------------------------------------
# Cleanup trap
# ---------------------------------------------------------------------------
cleanup() {
    echo ""
    echo "Cleaning up..."

    # Unmount everything (best effort)
    if mountpoint -q "${MOUNT_DIR}/boot/efi" 2>/dev/null; then
        umount "${MOUNT_DIR}/boot/efi" 2>/dev/null || true
    fi
    if mountpoint -q "${MOUNT_DIR}/run" 2>/dev/null; then
        umount "${MOUNT_DIR}/run" 2>/dev/null || true
    fi
    if mountpoint -q "${MOUNT_DIR}/sys" 2>/dev/null; then
        umount "${MOUNT_DIR}/sys" 2>/dev/null || true
    fi
    if mountpoint -q "${MOUNT_DIR}/proc" 2>/dev/null; then
        umount "${MOUNT_DIR}/proc" 2>/dev/null || true
    fi
    if mountpoint -q "${MOUNT_DIR}/dev" 2>/dev/null; then
        umount "${MOUNT_DIR}/dev" 2>/dev/null || true
    fi
    if mountpoint -q "${MOUNT_DIR}" 2>/dev/null; then
        umount "${MOUNT_DIR}" 2>/dev/null || true
    fi
    if mountpoint -q "${OVERLAY_TMP}" 2>/dev/null; then
        umount "${OVERLAY_TMP}" 2>/dev/null || true
    fi

    # Close LUKS container if still open
    if [[ "${ENCRYPT}" == true ]]; then
        cryptsetup close regicideos 2>/dev/null || true
    fi

    if [[ -n "${LOOP_DEV}" ]]; then
        losetup -d "${LOOP_DEV}" 2>/dev/null || true
    fi

    # Safety: do not remove a passed-in block device.
    if [[ -z "${DIRECT_DEVICE}" ]]; then
        rm -f "${RAW_IMG}" 2>/dev/null || true
    fi
    rm -rf "${MOUNT_DIR}" 2>/dev/null || true
    rm -rf "${OVERLAY_TMP}" 2>/dev/null || true
}
trap cleanup EXIT

if [[ -z "${DIRECT_DEVICE}" ]]; then
    echo "Creating raw disk image (${DISK_SIZE})..."
    qemu-img create -f raw "${RAW_IMG}" "${DISK_SIZE}"
fi

# ---------------------------------------------------------------------------
# Partition with parted
# ---------------------------------------------------------------------------
echo "Partitioning disk image..."

# Size partitions proportionally so ROOTS has enough space for the growing
# stage4 rootfs (including flatpak runtimes) while still leaving room for
# overlay and home directories.
PARTITION_TARGET="${DIRECT_DEVICE:-${RAW_IMG}}"

# Probe the disk size without requiring a partition table.  parted print fails
# on an unlabelled direct block device, so use blockdev for devices and stat
# for plain image files.
if [[ -b "${PARTITION_TARGET}" ]]; then
    DISK_BYTES=$(blockdev --getsize64 "${PARTITION_TARGET}")
else
    DISK_BYTES=$(stat -c '%s' "${PARTITION_TARGET}")
fi
DISK_MIB=$((DISK_BYTES / 1024 / 1024))
EFI_END_MIB=513
# Reserve 1 MiB at the end of the disk for the GPT backup header.
GPT_RESERVED_MIB=1
REMAIN_MIB=$((DISK_MIB - EFI_END_MIB - GPT_RESERVED_MIB))
# Give OVERLAY more room than before: container builds (distrobox/podman) need
# space for overlay layers and package installation in the writable /etc and
# /var subvolumes.  ROOTS still needs the bulk of the disk for the stage4 rootfs.
ROOTS_END_MIB=$((EFI_END_MIB + REMAIN_MIB * 65 / 100))
OVERLAY_END_MIB=$((ROOTS_END_MIB + REMAIN_MIB * 25 / 100))
HOME_END_MIB=$((DISK_MIB - GPT_RESERVED_MIB))

echo "Disk size: ${DISK_MIB} MiB; partition layout: EFI 1-${EFI_END_MIB} MiB, ROOTS ${EFI_END_MIB}-${ROOTS_END_MIB} MiB, OVERLAY ${ROOTS_END_MIB}-${OVERLAY_END_MIB} MiB, HOME ${OVERLAY_END_MIB}-${HOME_END_MIB} MiB"

parted -s "${PARTITION_TARGET}" mklabel gpt
parted -s "${PARTITION_TARGET}" mkpart EFI     fat32 1MiB            "${EFI_END_MIB}MiB"
parted -s "${PARTITION_TARGET}" mkpart ROOTS   btrfs "${EFI_END_MIB}MiB"   "${ROOTS_END_MIB}MiB"
parted -s "${PARTITION_TARGET}" mkpart OVERLAY btrfs "${ROOTS_END_MIB}MiB" "${OVERLAY_END_MIB}MiB"
parted -s "${PARTITION_TARGET}" mkpart HOME    btrfs "${OVERLAY_END_MIB}MiB" "${HOME_END_MIB}MiB"
parted -s "${PARTITION_TARGET}" set 1 esp on

if [[ -n "${DIRECT_DEVICE}" ]]; then
    # Partitions on a bare block device use either a "p<N>" suffix (loop) or
    # a plain "<N>" suffix (virtio/SCSI/SATA).  Determine which convention the
    # device uses before constructing partition paths.
    if [[ "${DIRECT_DEVICE}" =~ /dev/loop[0-9]+$ || "${DIRECT_DEVICE}" =~ /dev/nbd[0-9]+$ ]]; then
        PART_SUFFIX="p"
    else
        PART_SUFFIX=""
    fi
    EFI_PART="${DIRECT_DEVICE}${PART_SUFFIX}1"
    ROOTS_PART="${DIRECT_DEVICE}${PART_SUFFIX}2"
    OVERLAY_PART="${DIRECT_DEVICE}${PART_SUFFIX}3"
    HOME_PART="${DIRECT_DEVICE}${PART_SUFFIX}4"
else
    # ---------------------------------------------------------------------------
    # Attach loop device and wait for partitions
    # ---------------------------------------------------------------------------
    echo "Attaching loop device..."
    LOOP_DEV=$(losetup -f --show -P "${RAW_IMG}")

    # Wait for kernel to create partition devices
    for _ in {1..10}; do
        if [[ -e "${LOOP_DEV}p1" && -e "${LOOP_DEV}p2" && -e "${LOOP_DEV}p3" && -e "${LOOP_DEV}p4" ]]; then
            break
        fi
        sleep 0.5
    done

    if [[ ! -e "${LOOP_DEV}p1" ]]; then
        echo "Error: loop partitions did not appear."
        exit 1
    fi

    EFI_PART="${LOOP_DEV}p1"
    ROOTS_PART="${LOOP_DEV}p2"
    OVERLAY_PART="${LOOP_DEV}p3"
    HOME_PART="${LOOP_DEV}p4"
fi

# ---------------------------------------------------------------------------
# Format partitions
# ---------------------------------------------------------------------------
echo "Formatting partitions..."

mkfs.vfat -F 32 -n EFI "${EFI_PART}"
mkfs.btrfs -L OVERLAY "${OVERLAY_PART}"
mkfs.btrfs -L HOME "${HOME_PART}"

# ---------------------------------------------------------------------------
# Optional LUKS encryption for ROOTS
# ---------------------------------------------------------------------------
ROOTS_TARGET="${ROOTS_PART}"
LUKS_UUID=""
if [[ "${ENCRYPT}" == true ]]; then
    echo "Setting up LUKS encryption on ROOTS partition..."
    if [[ "${PASSPHRASE_FILE}" == "-" ]]; then
        cryptsetup luksFormat --batch-mode --type luks2 --label regicideos --key-file - "${ROOTS_PART}"
        cryptsetup open --type luks2 --key-file - "${ROOTS_PART}" regicideos
    else
        cryptsetup luksFormat --batch-mode --type luks2 --label regicideos --key-file "${PASSPHRASE_FILE}" "${ROOTS_PART}"
        cryptsetup open --type luks2 --key-file "${PASSPHRASE_FILE}" "${ROOTS_PART}" regicideos
    fi
    ROOTS_TARGET="/dev/mapper/regicideos"
    LUKS_UUID=$(cryptsetup luksUUID "${ROOTS_PART}")
    echo "LUKS container opened: ${ROOTS_TARGET} (UUID: ${LUKS_UUID})"
fi

# Disable block-group-tree so GRUB's btrfs module can read ROOTS.
echo "Formatting ROOTS with GRUB-compatible Btrfs features..."
mkfs.btrfs -L ROOTS -O ^block-group-tree "${ROOTS_TARGET}"
btrfs inspect-internal dump-super "${ROOTS_TARGET}" 2>/dev/null | grep -E '^Features' || true

# ---------------------------------------------------------------------------
# Create BTRFS subvolumes on OVERLAY and HOME
# ---------------------------------------------------------------------------
echo "Creating overlay subvolumes..."

mount "${OVERLAY_PART}" "${OVERLAY_TMP}"

btrfs subvolume create "${OVERLAY_TMP}/etc"
btrfs subvolume create "${OVERLAY_TMP}/var"
btrfs subvolume create "${OVERLAY_TMP}/usr"

# Create overlay work directories
mkdir -p "${OVERLAY_TMP}/etcw"
mkdir -p "${OVERLAY_TMP}/varw"
mkdir -p "${OVERLAY_TMP}/usrw"

umount "${OVERLAY_TMP}"

# /home is mounted from a separate partition, so create its subvolume there.
echo "Creating /home subvolume..."
mount "${HOME_PART}" "${OVERLAY_TMP}"
btrfs subvolume create "${OVERLAY_TMP}/home"
umount "${OVERLAY_TMP}"

# ---------------------------------------------------------------------------
# Extract stage4 archive to ROOTS
# ---------------------------------------------------------------------------
echo "Extracting stage4 archive to ROOTS partition..."

mount "${ROOTS_TARGET}" "${MOUNT_DIR}"

if [[ "${TARBALL}" =~ \.(img|squashfs)$ ]]; then
    if ! command -v unsquashfs &> /dev/null; then
        echo "Error: unsquashfs is required to extract a SquashFS stage4 archive."
        exit 1
    fi
    unsquashfs -no-xattrs -f -d "${MOUNT_DIR}" "${TARBALL}"
    # SquashFS preserves the build host UID/GID.  Force critical paths to
    # root so systemd services that validate ownership can start.
    chown -R root:root "${MOUNT_DIR}/etc" "${MOUNT_DIR}/var" "${MOUNT_DIR}/usr/lib/systemd" 2>/dev/null || true
elif [[ "${TARBALL}" == *.tar.zst || "${TARBALL}" == *.tzst ]]; then
    # --owner/--group ensure Catalyst build-user ownership does not leak in.
    if tar -C "${MOUNT_DIR}" --owner=root --group=root -xp --zstd -f "${TARBALL}" 2>/dev/null; then
        :
    elif command -v zstdcat &> /dev/null; then
        zstdcat "${TARBALL}" | tar -C "${MOUNT_DIR}" --owner=root --group=root -xpf -
    else
        echo "Error: cannot decompress .tar.zst archive (tar lacks --zstd and zstdcat missing)."
        exit 1
    fi
else
    tar -C "${MOUNT_DIR}" --owner=root --group=root -xpJf "${TARBALL}"
fi

# Podman rootless containers require newuidmap/newgidmap to have setuid or
# file capabilities.  Gentoo's shadow package installs them without either in
# some configurations, so ensure they are setuid here.  This is needed for
# distrobox and other rootless container workflows to work out-of-the-box.
if [[ -f "${MOUNT_DIR}/usr/bin/newuidmap" && -f "${MOUNT_DIR}/usr/bin/newgidmap" ]]; then
    chmod u+s "${MOUNT_DIR}/usr/bin/newuidmap" "${MOUNT_DIR}/usr/bin/newgidmap"
fi

# ---------------------------------------------------------------------------
# Seed the /etc and /var subvolumes with the stage4 contents.
# /var cannot be an overlay because systemd-logind and other services rename
# directories under /var, and overlayfs returns EXDEV for directory renames
# even with redirect_dir=on.
# /etc cannot be an overlay either: overlayfs lowerdir and upperdir must live
# on the same filesystem, but ROOTS and OVERLAY are separate Btrfs partitions.
# Cross-partition overlay writes fail with EXDEV ("Invalid cross-device link"),
# breaking systemd-tmpfiles, ssh-keygen, and other early-boot setup.
# Mount both as real Btrfs subvolumes instead.
# ---------------------------------------------------------------------------
echo "Seeding /etc subvolume..."
mount "${OVERLAY_PART}" "${OVERLAY_TMP}"
cp -aT "${MOUNT_DIR}/etc" "${OVERLAY_TMP}/etc"
# The stage4 tarball sometimes leaves /etc as 0700. Many systemd services run
# as non-root users and must traverse /etc to read their configuration, so
# ensure the directory is world-executable (and root-owned).
chmod 0755 "${OVERLAY_TMP}/etc"
# Portage configuration must be readable by regular users so `emerge --info`,
# `emerge -pv <pkg>`, etc. work for the default desktop user. Catalyst creates
# /etc/portage as 0700, so fix it in the seeded subvolume.
chmod 0755 "${OVERLAY_TMP}/etc/portage"
chmod -R go+rX "${OVERLAY_TMP}/etc/portage"
echo "Seeding /var subvolume..."
cp -aT "${MOUNT_DIR}/var" "${OVERLAY_TMP}/var"
umount "${OVERLAY_TMP}"

# Ensure mountpoint directories exist
mkdir -p "${MOUNT_DIR}/overlay"
mkdir -p "${MOUNT_DIR}/home"
mkdir -p "${MOUNT_DIR}/boot/efi"

# Older stage4 tarballs enabled both sshd.socket and sshd.service, which
# causes both units to bind port 22 and incoming connections to be reset.
# Prefer socket activation; remove the standalone service symlink if present.
if [[ -L "${MOUNT_DIR}/etc/systemd/system/sockets.target.wants/sshd.socket" && -L "${MOUNT_DIR}/etc/systemd/system/multi-user.target.wants/sshd.service" ]]; then
    rm -f "${MOUNT_DIR}/etc/systemd/system/multi-user.target.wants/sshd.service"
fi

# Dracut's switch-root executes /sbin/init.  With Gentoo's merged-/usr layout
# (/sbin -> usr/bin and /usr/sbin -> bin -> usr/bin) we must have an init binary
# in /usr/bin.  Create the canonical systemd symlink if the stage4 tarball did
# not already provide it.
if [[ -x "${MOUNT_DIR}/usr/lib/systemd/systemd" && ! -e "${MOUNT_DIR}/usr/bin/init" ]]; then
    ln -sf /usr/lib/systemd/systemd "${MOUNT_DIR}/usr/bin/init"
fi

# Set the system hostname; the stage4 tarball leaves it as "gentoo".
printf 'regicideos\n' > "${MOUNT_DIR}/etc/hostname"

# Ensure writable state directories exist before the /var overlay is mounted.
# systemd-logind, timesyncd and journal-catalog-update need these at startup.
mkdir -p "${MOUNT_DIR}/var/lib/systemd/catalog"
mkdir -p "${MOUNT_DIR}/var/lib/systemd/timesync"
mkdir -p "${MOUNT_DIR}/var/log/journal"
mkdir -p "${MOUNT_DIR}/var/tmp"

# Seed the /home subvolume with the default user's home directory.
# /home is mounted from a separate partition at runtime, so the stage4
# /home/regicide contents must be present in the HOME subvolume before first
# boot or PAM will report "No directory, logging in with HOME=/".
echo "Seeding /home subvolume..."
mount -o subvol=home "${HOME_PART}" "${OVERLAY_TMP}"
if [[ -d "${MOUNT_DIR}/home/regicide" ]]; then
    cp -aT "${MOUNT_DIR}/home/regicide" "${OVERLAY_TMP}/regicide"
    chown -R 1000:1000 "${OVERLAY_TMP}/regicide" 2>/dev/null || true
fi
umount "${OVERLAY_TMP}"

# ---------------------------------------------------------------------------
# Create /etc/fstab with overlay mounts
# ---------------------------------------------------------------------------
echo "Creating /etc/fstab..."

ROOTS_FSTAB_SPEC="LABEL=ROOTS"
if [[ "${ENCRYPT}" == true ]]; then
    ROOTS_FSTAB_SPEC="/dev/mapper/regicideos"
fi

cat > "${MOUNT_DIR}/etc/fstab" << EOF
# RegicideOS QEMU image fstab
# Generated by build-qemu-image.sh

# Base system (read-write for QEMU VM; overlay mounts provide writable layers)
${ROOTS_FSTAB_SPEC}   /       btrfs   defaults,noatime           0 0

# Writable overlay partition
LABEL=OVERLAY /overlay btrfs   defaults,noatime           0 0

# User data
LABEL=HOME    /home   btrfs   subvol=home,defaults,noatime           0 0

# Mutable system directories.
# /usr is intentionally omitted: dracut treats a separate /usr mount
# specially and breaks merged-/usr switch-root.
# /etc and /var are mounted as real Btrfs subvolumes on OVERLAY. Using an
# overlay here fails because lowerdir (/etc on ROOTS) and upperdir
# (/overlay/etc on OVERLAY) live on separate partitions, which causes EXDEV
# ("Invalid cross-device link") errors during early-boot file creation.
LABEL=OVERLAY /etc    btrfs   subvol=etc,defaults,noatime,x-systemd.requires-mounts-for=/overlay 0 0
LABEL=OVERLAY /var    btrfs   subvol=var,defaults,noatime,x-systemd.requires-mounts-for=/overlay 0 0
LABEL=HOME    /home   btrfs   subvol=home,defaults,noatime 0 0
EOF

if [[ "${ENCRYPT}" == true ]]; then
    cat >> "${MOUNT_DIR}/etc/fstab" << EOF

# Encrypted ROOTS backing device (informational)
# UUID=${LUKS_UUID} /dev/mapper/regicideos luks defaults 0 0
EOF
fi

# ---------------------------------------------------------------------------
# ---------------------------------------------------------------------------
# Configure LUKS boot support inside the chroot when encrypting
# ---------------------------------------------------------------------------
if [[ "${ENCRYPT}" == true ]]; then
    echo "Configuring LUKS boot support..."
    mkdir -p "${MOUNT_DIR}/etc/dracut.conf.d"
    cat > "${MOUNT_DIR}/etc/dracut.conf.d/99-regicide-luks.conf" << 'EOF'
add_dracutmodules+=" regicide-crypt "
omit_dracutmodules+=" zfs "
force_drivers+=" dm_mod dm_crypt overlay "
EOF
    # Embed the LUKS passphrase as a key file inside the initramfs.  The
    # initramfs is stored on the encrypted ROOTS partition, so the key file
    # is protected at rest; GRUB must unlock the partition first to load it.
    # This avoids a second interactive prompt in the systemd initramfs, where
    # console access competes with systemd status output.
    cat > "${MOUNT_DIR}/etc/crypttab" << EOF
regicideos UUID=${LUKS_UUID} /etc/luks-keyfile luks
EOF
    cp "${PASSPHRASE_FILE}" "${MOUNT_DIR}/etc/luks-keyfile"
    chmod 0400 "${MOUNT_DIR}/etc/luks-keyfile"

    # Install a custom dracut module that unlocks root LUKS from the initqueue.
    # We cannot rely on systemd-cryptsetup because the stage4 image does not
    # ship the systemd-cryptsetup binary; the legacy crypt module also does not
    # prompt reliably on a serial console, so we perform the unlock explicitly.
    mkdir -p "${MOUNT_DIR}/usr/lib/dracut/modules.d/99regicide-crypt"
    cat > "${MOUNT_DIR}/usr/lib/dracut/modules.d/99regicide-crypt/module-setup.sh" << 'MODULEEOF'
#!/bin/bash
# RegicideOS custom LUKS root unlock module

check() {
    [[ -f /etc/crypttab ]] && return 0
    return 1
}

depends() {
    echo initqueue
    return 0
}

install() {
    # Run in initqueue: the systemd initramfs waits for /dev/mapper/regicideos
    # during initqueue, so unlocking must happen there, not pre-mount.
    inst_hook initqueue 99 "$moddir/cryptroot-unlock.sh"
    inst_simple /etc/crypttab
    inst_simple /etc/luks-keyfile
    inst_multiple cryptsetup stty
    inst_libdir_file "libcryptsetup.so*"
    inst_libdir_file "libdevmapper.so*"
    inst_libdir_file "libblkid.so*"
    inst_libdir_file "libuuid.so*"
    inst_libdir_file "libcrypto.so*"
    inst_libdir_file "libgcrypt.so*"
    inst_libdir_file "libargon2.so*"
    inst_libdir_file "libjson-c.so*"
}
MODULEEOF
    chmod 0755 "${MOUNT_DIR}/usr/lib/dracut/modules.d/99regicide-crypt/module-setup.sh"

    cat > "${MOUNT_DIR}/usr/lib/dracut/modules.d/99regicide-crypt/cryptroot-unlock.sh" << 'HOOKEOF'
#!/bin/sh
# Unlock root LUKS device from the initramfs serial console.
#
# This hook is SOURCED by dracut-initqueue, so it must use "return", not
# "exit".  Using exit would terminate the whole initqueue runner.

# Ensure dracut helpers (info/warn/getarg) are available.
type getarg >/dev/null 2>&1 || . /lib/dracut-lib.sh

# Log to kernel messages so debug output survives on the serial console even
# when "quiet" is set.  Prefix with <4> (WARNING) because "quiet" lowers the
# console loglevel so that unprefixed /dev/kmsg writes are not shown.
_kmsg() {
    printf '<4>regicide-crypt: %s\n' "$*" > /dev/kmsg 2>/dev/null || true
}

_kmsg "hook starting"

LUKS_UUID=$(getarg rd.luks.uuid=)
if [ -z "${LUKS_UUID}" ] && [ -f /etc/crypttab ]; then
    # Fallback to the UUID recorded in the embedded crypttab.
    LUKS_UUID=$(awk '$1 == "regicideos" { print $2 }' /etc/crypttab)
    LUKS_UUID="${LUKS_UUID#UUID=}"
fi
if [ -z "${LUKS_UUID}" ]; then
    warn "regicide-crypt: no rd.luks.uuid on cmdline and no crypttab"
    return 0
fi

DEVICE="/dev/disk/by-uuid/${LUKS_UUID}"

modprobe dm_mod 2>/dev/null || true
modprobe dm_crypt 2>/dev/null || true

_kmsg "waiting for ${DEVICE}"
if [ ! -e "${DEVICE}" ]; then
    _i=0
    while [ "${_i}" -lt 30 ]; do
        [ -e "${DEVICE}" ] && break
        sleep 1
        _i=$((_i + 1))
    done
fi

if [ ! -e "${DEVICE}" ]; then
    warn "regicide-crypt: device not found: ${DEVICE}"
    return 1
fi

if [ -e /dev/mapper/regicideos ]; then
    info "regicide-crypt: /dev/mapper/regicideos already exists"
    return 0
fi

# Prevent repeated prompts during the initqueue loop on failure.
if [ -f /run/regicide-crypt-failed ]; then
    warn "regicide-crypt: already failed, skipping"
    return 1
fi

# If a key file is embedded in the initramfs, unlock automatically.
if [ -f /etc/luks-keyfile ]; then
    _kmsg "unlocking with embedded key file"
    if cryptsetup open --type luks2 --key-file /etc/luks-keyfile "${DEVICE}" regicideos; then
        _kmsg "root LUKS unlocked with key file"
        return 0
    fi
    warn "regicide-crypt: key file unlock failed, falling back to prompt"
fi

ask_passphrase() {
    _prompt="$1"
    _passphrase=""

    # Primary: stty + read from /dev/console.  This is the only reliable
    # method on a serial console in the initramfs.
    if command -v stty >/dev/null 2>&1; then
        stty -echo < /dev/console
        printf '%s' "${_prompt}" > /dev/console
        read -r _passphrase < /dev/console
        stty echo < /dev/console
        printf '\n' > /dev/console
        printf '%s' "${_passphrase}"
        return 0
    fi

    # Last resort: read without echo suppression.
    printf '%s' "${_prompt}" > /dev/console
    read -r _passphrase < /dev/console
    printf '%s' "${_passphrase}"
    return 0
}

TRIES=0
MAX_TRIES=3
while [ "${TRIES}" -lt "${MAX_TRIES}" ]; do
    PASSPHRASE=$(ask_passphrase "Enter LUKS passphrase for RegicideOS root: ")
    if [ -z "${PASSPHRASE}" ]; then
        warn "regicide-crypt: empty passphrase"
        TRIES=$((TRIES + 1))
        continue
    fi
    if printf '%s' "${PASSPHRASE}" | cryptsetup open --type luks2 --key-file - "${DEVICE}" regicideos; then
        _kmsg "root LUKS unlocked"
        return 0
    fi
    warn "regicide-crypt: incorrect passphrase"
    TRIES=$((TRIES + 1))
done

warn "regicide-crypt: failed to unlock root LUKS"
: > /run/regicide-crypt-failed
return 1
HOOKEOF
    chmod 0755 "${MOUNT_DIR}/usr/lib/dracut/modules.d/99regicide-crypt/cryptroot-unlock.sh"
fi

# ---------------------------------------------------------------------------
# Mount EFI partition and special filesystems for chroot
# ---------------------------------------------------------------------------
echo "Preparing chroot environment..."

mount "${EFI_PART}" "${MOUNT_DIR}/boot/efi"
mount --bind /dev   "${MOUNT_DIR}/dev"
mount --bind /proc  "${MOUNT_DIR}/proc"
mount --bind /sys   "${MOUNT_DIR}/sys"
mount --bind /run   "${MOUNT_DIR}/run"

# ---------------------------------------------------------------------------
# Install GRUB for UEFI
# ---------------------------------------------------------------------------
echo "Installing GRUB bootloader..."

CHROOT_SCRIPT="${MOUNT_DIR}/run/regicide-grub-install.sh"
cat > "${CHROOT_SCRIPT}" << 'CHROOTEOF'
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# The initramfs must not treat /usr as a separate filesystem.  With a merged-/usr
# layout, dracut's own usrmount module can leave the initramfs's /usr mounted
# after switch-root and hide the real root's /usr from systemd.  Disable
# dracut's usrmount module; the real root will apply its own fstab after
# switch-root.  Also ensure the overlay driver is present for the /etc and
# /var overlays applied later by systemd.
mkdir -p /etc/dracut.conf.d
echo 'omit_dracutmodules+=" usrmount "' > /etc/dracut.conf.d/99-regicide-no-usrmount.conf
echo 'force_drivers+=" overlay "' > /etc/dracut.conf.d/99-regicide-overlay.conf

KVER="$(ls /lib/modules/ | head -n1)"
if [[ -z "${KVER}" ]]; then
    echo "Error: no kernel modules directory found"
    exit 1
fi

dracut -v --force --no-hostonly --kver "${KVER}" 2>&1 | tee /boot/dracut-v83.log

# Copy the generated initramfs to the ESP so it can be inspected offline
# without decrypting the ROOTS partition.
cp "/boot/initramfs-${KVER}.img" /boot/efi/initramfs-debug.img

# GRUB's config references the canonical /boot/vmlinuz and /boot/initramfs.img
# names, so make sure those point at the kernel/initramfs dracut just built.
if [[ -f "/boot/vmlinuz-${KVER}" ]]; then
    cp -f "/boot/vmlinuz-${KVER}" /boot/vmlinuz
fi
cp -f "/boot/initramfs-${KVER}.img" /boot/initramfs.img

GRUB_MODULES="cryptodisk luks luks2 gcry_rijndael gcry_sha256 gcry_sha1 part_gpt btrfs"

grub-install \
    --modules="${GRUB_MODULES}" \
    --force \
    --target="x86_64-efi" \
    --efi-directory="/boot/efi" \
    --boot-directory="/boot/efi" \
    --removable \
    --recheck \
    --no-nvram
CHROOTEOF

chmod +x "${CHROOT_SCRIPT}"
chroot "${MOUNT_DIR}" /bin/bash /run/regicide-grub-install.sh
rm -f "${CHROOT_SCRIPT}"

# ---------------------------------------------------------------------------
# Create GRUB configuration
# ---------------------------------------------------------------------------
echo "Creating GRUB configuration..."

# Ensure GRUB config directory exists
mkdir -p "${MOUNT_DIR}/boot/efi/grub"
mkdir -p "${MOUNT_DIR}/boot/efi/EFI/fedora"

# Verify kernel and initramfs exist in the chroot
if [[ ! -f "${MOUNT_DIR}/boot/vmlinuz" ]]; then
    echo "Warning: /boot/vmlinuz not found in stage4. Searching for alternatives..."
    # Try to find and copy the kernel if it exists under a different name
    KERNEL_SRC=$(find "${MOUNT_DIR}/boot" -maxdepth 1 \( -name 'vmlinuz-*' -o -name 'kernel-*' \) -type f | head -n1 || true)
    if [[ -n "${KERNEL_SRC}" ]]; then
        cp "${KERNEL_SRC}" "${MOUNT_DIR}/boot/vmlinuz"
    else
        echo "Error: no kernel found in /boot"
        exit 1
    fi
fi

if [[ ! -f "${MOUNT_DIR}/boot/initramfs.img" ]]; then
    echo "Warning: /boot/initramfs.img not found in stage4. Searching for alternatives..."
    INITRD_SRC=$(find "${MOUNT_DIR}/boot" -maxdepth 1 \( -name 'initramfs-*' -o -name 'initrd-*' \) -type f | head -n1 || true)
    if [[ -n "${INITRD_SRC}" ]]; then
        cp "${INITRD_SRC}" "${MOUNT_DIR}/boot/initramfs.img"
    else
        echo "Error: no initramfs found in /boot"
        exit 1
    fi
fi

ROOTS_GRUB="root=LABEL=ROOTS"
LUKS_UUID_NOHYPH=""
if [[ "${ENCRYPT}" == true ]]; then
    ROOTS_GRUB="rd.luks.uuid=${LUKS_UUID} root=/dev/mapper/regicideos"
    LUKS_UUID_NOHYPH="${LUKS_UUID//-/}"
fi

# Build a GRUB config that can locate the kernel/initramfs.  For encrypted
# ROOTS the inner Btrfs label is not visible to GRUB until the LUKS container
# is unlocked, so we use cryptomount and then reference the unlocked crypto device.
cat > "${MOUNT_DIR}/boot/efi/grub/grub.cfg" << GRUBEOF
set default="RegicideOS"
set timeout=5
set color_normal=light-gray/black
set color_highlight=green/black

insmod part_gpt
insmod cryptodisk
insmod luks
insmod luks2
insmod gcry_rijndael
insmod gcry_sha256
insmod gcry_sha1
insmod btrfs

GRUBEOF

if [[ "${ENCRYPT}" == true ]]; then
    cat >> "${MOUNT_DIR}/boot/efi/grub/grub.cfg" << GRUBEOF
cryptomount -u ${LUKS_UUID_NOHYPH}
set root=(crypto0)

GRUBEOF
else
    cat >> "${MOUNT_DIR}/boot/efi/grub/grub.cfg" << GRUBEOF
search --no-floppy --label --set=root ROOTS

GRUBEOF
fi

cat >> "${MOUNT_DIR}/boot/efi/grub/grub.cfg" << GRUBEOF
menuentry "RegicideOS" {
    linux /boot/vmlinuz ${ROOTS_GRUB} quiet splash rw console=ttyS0,115200n8
    initrd /boot/initramfs.img
}

menuentry "RegicideOS (Recovery)" {
    linux /boot/vmlinuz ${ROOTS_GRUB} quiet splash rw single console=ttyS0,115200n8
    initrd /boot/initramfs.img
}

menuentry "RegicideOS (Verbose)" {
    linux /boot/vmlinuz ${ROOTS_GRUB} verbose rw console=ttyS0,115200n8
    initrd /boot/initramfs.img
}
GRUBEOF

# Copy to EFI/fedora for compatibility with some firmware
mkdir -p "${MOUNT_DIR}/boot/efi/EFI/fedora"
cp "${MOUNT_DIR}/boot/efi/grub/grub.cfg" "${MOUNT_DIR}/boot/efi/EFI/fedora/grub.cfg"

echo "Verifying EFI System Partition contents..."
EFI_BOOT_FILE="${MOUNT_DIR}/boot/efi/EFI/BOOT/BOOTX64.EFI"
GRUBX64_FILE="${MOUNT_DIR}/boot/efi/EFI/grub/grubx64.efi"
GRUB_CFG="${MOUNT_DIR}/boot/efi/grub/grub.cfg"
if [[ ! -f "${EFI_BOOT_FILE}" && ! -f "${GRUBX64_FILE}" ]]; then
    echo "Error: no GRUB EFI binary found in ESP."
    echo "  Looked for: ${EFI_BOOT_FILE}"
    echo "              ${GRUBX64_FILE}"
    find "${MOUNT_DIR}/boot/efi" -type f 2>/dev/null | head -20 || true
    exit 1
fi
if [[ ! -f "${GRUB_CFG}" ]]; then
    echo "Error: GRUB config missing: ${GRUB_CFG}"
    exit 1
fi
if [[ ! -f "${MOUNT_DIR}/boot/vmlinuz" ]]; then
    echo "Error: kernel missing on ROOTS: ${MOUNT_DIR}/boot/vmlinuz"
    exit 1
fi
if [[ ! -f "${MOUNT_DIR}/boot/initramfs.img" ]]; then
    echo "Error: initramfs missing on ROOTS: ${MOUNT_DIR}/boot/initramfs.img"
    exit 1
fi
echo "ESP verification passed."

echo "EFI partition layout:"
find "${MOUNT_DIR}/boot/efi" -type f -printf "  %p\\n" 2>/dev/null || true

# ---------------------------------------------------------------------------
# Create /etc/default/grub for grub-mkconfig compatibility
# ---------------------------------------------------------------------------
mkdir -p "${MOUNT_DIR}/etc/default"
GRUB_CMDLINE=""
if [[ "${ENCRYPT}" == true ]]; then
    GRUB_CMDLINE="rd.luks.uuid=${LUKS_UUID} root=/dev/mapper/regicideos"
fi

cat > "${MOUNT_DIR}/etc/default/grub" << GRUBDEFAULT
GRUB_DEFAULT=0
GRUB_TIMEOUT=5
GRUB_DISTRIBUTOR="RegicideOS"
GRUB_CMDLINE_LINUX_DEFAULT="quiet splash"
GRUB_CMDLINE_LINUX="${GRUB_CMDLINE}"
GRUB_ENABLE_CRYPTODISK=y
GRUB_PRELOAD_MODULES="cryptodisk luks luks2 gcry_rijndael gcry_sha256 gcry_sha1 part_gpt btrfs"
GRUBDEFAULT

# ---------------------------------------------------------------------------
# Build completion sentinel
# ---------------------------------------------------------------------------
# Write a marker file so VM-based wrappers can distinguish a successful build
# from an early poweroff caused by a builder failure.
echo "Writing build completion sentinel..."
mkdir -p "${MOUNT_DIR}/var/lib"
touch "${MOUNT_DIR}/var/lib/regicide-build-complete"

# ---------------------------------------------------------------------------
# Unmount chroot filesystems
# ---------------------------------------------------------------------------
echo "Unmounting chroot filesystems..."

umount "${MOUNT_DIR}/run"  2>/dev/null || true
umount "${MOUNT_DIR}/sys"  2>/dev/null || true
umount "${MOUNT_DIR}/proc" 2>/dev/null || true
umount "${MOUNT_DIR}/dev"  2>/dev/null || true
umount "${MOUNT_DIR}/boot/efi" 2>/dev/null || true
umount "${MOUNT_DIR}"      2>/dev/null || true

# Close the LUKS container before detaching the loop device
if [[ "${ENCRYPT}" == true ]]; then
    echo "Closing LUKS container..."
    cryptsetup close regicideos 2>/dev/null || true
fi

# ---------------------------------------------------------------------------
# Detach loop device
# ---------------------------------------------------------------------------
echo "Detaching loop device..."
losetup -d "${LOOP_DEV}" 2>/dev/null || true
LOOP_DEV=""

if [[ "${NO_CONVERT}" == false ]]; then
    # ---------------------------------------------------------------------------
    # Convert raw to qcow2
    # ---------------------------------------------------------------------------
    echo "Converting raw image to QCOW2..."
    qemu-img convert -f raw -O qcow2 "${RAW_IMG}" "${OUTPUT}"

    # Remove raw image (cleanup trap will also try, but be explicit)
    rm -f "${RAW_IMG}"

    # ---------------------------------------------------------------------------
    # Create QEMU runner script
    # ---------------------------------------------------------------------------
    RUNNER_PATH="${OUTPUT_DIR}/run-qemu.sh"
    cat > "${RUNNER_PATH}" << QEMUEOF
#!/bin/bash
# RegicideOS QEMU Runner
# Auto-generated by build-qemu-image.sh

set -euo pipefail

IMAGE="$(realpath -m --relative-to="${OUTPUT_DIR}" "${OUTPUT}" 2>/dev/null || basename "${OUTPUT}")"
IMAGE_DIR="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"
IMAGE_PATH="\${IMAGE_DIR}/\${IMAGE}"

if [[ ! -f "\${IMAGE_PATH}" ]]; then
    echo "Error: disk image not found: \${IMAGE_PATH}"
    exit 1
fi

# Pick a free local SSH forwarding port so multiple RegicideOS VMs can run.
find_free_port() {
    local port
    for port in $(seq 2222 2999); do
        if ! (command -v ss >/dev/null 2>&1 && ss -Htn "sport = :${port}" | grep -q .) && \
           ! (command -v netstat >/dev/null 2>&1 && netstat -atn 2>/dev/null | grep -q ":${port} ") && \
           ! (timeout 1 bash -c "exec 3<>/dev/tcp/127.0.0.1/${port}" 2>/dev/null); then
            echo "${port}"
            return 0
        fi
    done
    echo "ERROR: no free TCP port found in range 2222-2999" >&2
    return 1
}
SSH_PORT="\${REGICIDE_VM_SSH_PORT:-\$(find_free_port)}"

echo "Starting RegicideOS QEMU VM..."
echo "  Image: \${IMAGE_PATH}"
echo "  Memory: 4G"
echo "  CPUs: 2"
echo "  SSH: localhost:\${SSH_PORT} -> :22"
echo ""
echo "To connect via SSH:  ssh -p \${SSH_PORT} regicide@localhost"
echo "To stop: Ctrl+A then X (if using -nographic) or close window"
echo ""

# Detect OVMF firmware
OVMF_CODE=""
OVMF_VARS=""
for path in \\
    /usr/share/OVMF/OVMF_CODE.fd \\
    /usr/share/edk2/ovmf/OVMF_CODE.fd \\
    /usr/share/qemu/OVMF_CODE.fd \\
    /usr/share/ovmf/x64/OVMF_CODE.fd
do
    if [[ -f "\${path}" ]]; then
        OVMF_CODE="\${path}"
        break
    fi
done
for path in \\
    /usr/share/OVMF/OVMF_VARS.fd \\
    /usr/share/edk2/ovmf/OVMF_VARS.fd \\
    /usr/share/qemu/OVMF_VARS.fd \\
    /usr/share/ovmf/x64/OVMF_VARS.fd
do
    if [[ -f "\${path}" ]]; then
        OVMF_VARS="\${path}"
        break
    fi
done

if [[ -z "\${OVMF_CODE}" ]]; then
    echo "Error: OVMF firmware not found. Install ovmf or edk2-ovmf."
    exit 1
fi

UEFI_FLAGS="-drive if=pflash,format=raw,readonly=on,file=\${OVMF_CODE}"
if [[ -n "\${OVMF_VARS}" ]]; then
    TMP_VARS=\$(mktemp --suffix=_OVMF_VARS.fd)
    cp "\${OVMF_VARS}" "\${TMP_VARS}"
    UEFI_FLAGS="\${UEFI_FLAGS} -drive if=pflash,format=raw,file=\${TMP_VARS}"
fi

qemu-system-x86_64 \\
    -enable-kvm \\
    -m 4G \\
    -smp 2 \\
    -cpu host \\
    -drive file="\${IMAGE_PATH}",format=qcow2,if=virtio \\
    -netdev "user,id=net0,hostfwd=tcp::\\${SSH_PORT}-:22" \\
    -device virtio-net-pci,netdev=net0 \\
    -vga virtio \\
    -display sdl,gl=on \\
    -machine type=q35,accel=kvm \\
    \${UEFI_FLAGS} \\
    \$@
QEMUEOF

    chmod +x "${RUNNER_PATH}"
fi


# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
echo ""
echo "========================================"
echo "RegicideOS QEMU image build complete!"
echo "========================================"
echo ""
echo "Disk image: ${OUTPUT}"
if [[ -n "${RUNNER_PATH:-}" ]]; then
    echo "Runner:     ${RUNNER_PATH}"
    echo ""
    echo "To start the VM:"
    echo "  ${RUNNER_PATH}"
    echo ""
    echo "To start headless (VNC):"
    echo "  ${RUNNER_PATH} -display vnc=:1"
    echo ""
fi
