#!/bin/bash
# RegicideOS VM-based disk image builder
#
# Builds a bootable QCOW2 disk image from a Catalyst stage4 tarball by booting
# the stage4 rootfs inside a KVM VM and running build-qemu-image.sh against a
# virtio block device.  This avoids loop devices, which are unavailable in the
# host build environment.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
TARBALL=""
OUTPUT="${SCRIPT_DIR}/output/regicide-qemu.qcow2"
DISK_SIZE="30G"
ENCRYPT=false
PASSPHRASE_FILE=""
SQUASHFS=""

usage() {
    cat << EOF
Usage: $0 [OPTIONS] <stage4-archive> [output-qcow2] [disk-size]

  stage4-archive       Path to the stage4 rootfs archive. Accepted formats:
                       .tar.xz tarball or SquashFS image (.img, .squashfs).
  output-qcow2         Path for the output .qcow2 file (optional)
  disk-size            Disk size for the image, e.g. 30G (optional, default: 30G)

Options:
  --encrypt            Encrypt the ROOTS partition with LUKS2
  --passphrase-file    Path to a file containing the LUKS passphrase
                       (required with --encrypt; use - for stdin)
  --squashfs           Path to the stage4 SquashFS image used to extract the
                       kernel and initramfs (optional; defaults to the
                       regicide-cosmic.img sibling of the output)

Examples:
  $0 /path/to/stage4.tar.xz ./regicide-cosmic.qcow2 20G
  $0 --encrypt --passphrase-file /run/luks-pass /path/to/stage4.tar.xz ./image.qcow2
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
        --squashfs)
            SQUASHFS="${2:-}"
            shift 2
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
if [[ "${ENCRYPT}" == true && -z "${PASSPHRASE_FILE}" ]]; then
    echo "Error: --passphrase-file is required when --encrypt is used."
    usage
fi
if [[ "${ENCRYPT}" == true && "${PASSPHRASE_FILE}" != "-" && ! -f "${PASSPHRASE_FILE}" ]]; then
    echo "Error: passphrase file not found: ${PASSPHRASE_FILE}"
    exit 1
fi

TARBALL="$(realpath -e "${TARBALL}")"
OUTPUT="$(realpath -m "${OUTPUT}")"
OUTPUT_DIR="$(dirname "${OUTPUT}")"
mkdir -p "${OUTPUT_DIR}"

if [[ -z "${SQUASHFS}" ]]; then
    for candidate in \
        "${OUTPUT_DIR}/regicide-cosmic.img" \
        "${SCRIPT_DIR}/output/regicide-cosmic.img" \
        "$(dirname "${TARBALL}")/regicide-cosmic.img" \
        "${SCRIPT_DIR}/../regicide-cosmic.img"; do
        if [[ -f "${candidate}" ]]; then
            SQUASHFS="${candidate}"
            break
        fi
    done
fi
if [[ ! -f "${SQUASHFS}" ]]; then
    echo "Error: SquashFS image not found: ${SQUASHFS}"
    echo "       Build the SquashFS first, or pass --squashfs."
    exit 1
fi
SQUASHFS="$(realpath -e "${SQUASHFS}")"

# ---------------------------------------------------------------------------
# Dependency checks
# ---------------------------------------------------------------------------
REQUIRED_CMDS=(qemu-img qemu-system-x86_64 unsquashfs mksquashfs tar cpio zstd cryptsetup)
for cmd in "${REQUIRED_CMDS[@]}"; do
    if ! command -v "${cmd}" &> /dev/null; then
        echo "Error: required command '${cmd}' not found."
        exit 1
    fi
done

if [[ ! -e /dev/kvm ]]; then
    echo "Error: /dev/kvm is required for the VM-based builder."
    exit 1
fi

# ---------------------------------------------------------------------------
# Temp workspace
# ---------------------------------------------------------------------------
# Use /var/tmp for the work directory so large intermediate files (tarball
# staging, initramfs unpacking, raw disks) do not exhaust a tmpfs-backed /tmp.
WORK_DIR="$(TMPDIR=/var/tmp mktemp -d)"
trap 'rm -rf "${WORK_DIR}"' EXIT

TARGET_RAW="${WORK_DIR}/target.raw"
KERNEL="${WORK_DIR}/kernel"
INITRD="${WORK_DIR}/initrd"

# If the stage4 archive is a SquashFS, the in-VM unsquashfs may be too old to
# handle zstd compression.  Repack it on the host into a tar.zst archive that
# the in-VM tar can decompress, so we don't depend on the stage4's unsquashfs.
if [[ "${TARBALL}" =~ \.(img|squashfs)$ ]]; then
    echo "Stage4 archive is a SquashFS; repacking as tar.zst for the builder VM..."
    STAGE4_EXTRACT_DIR="${WORK_DIR}/stage4-extract"
    mkdir -p "${STAGE4_EXTRACT_DIR}"
    TARBALL_REPACKED="${WORK_DIR}/stage4-repacked.tar.zst"
    unsquashfs -no-xattrs -f -d "${STAGE4_EXTRACT_DIR}" "${TARBALL}" >/dev/null
    tar -C "${STAGE4_EXTRACT_DIR}" --owner=root --group=root -caf "${TARBALL_REPACKED}" .
    TARBALL="${TARBALL_REPACKED}"
    echo "Repacked stage4 archive: ${TARBALL}"
fi

# ---------------------------------------------------------------------------
# Create target disk
# ---------------------------------------------------------------------------
echo "Creating target raw disk (${DISK_SIZE})..."
qemu-img create -f raw "${TARGET_RAW}" "${DISK_SIZE}" > /dev/null

# ---------------------------------------------------------------------------
# Create data disk with stage4 archive, passphrase and in-VM script
# ---------------------------------------------------------------------------
echo "Packing builder data squashfs..."
DATA_SQUASHFS="${WORK_DIR}/data.squashfs"
DATA_STAGING="${WORK_DIR}/data-staging"
mkdir -p "${DATA_STAGING}"

cp "${TARBALL}" "${DATA_STAGING}/$(basename "${TARBALL}")"
cp "${SCRIPT_DIR}/vm-builder.sh" "${DATA_STAGING}/vm-builder.sh"
chmod 0755 "${DATA_STAGING}/vm-builder.sh"
cp "${SCRIPT_DIR}/build-qemu-image.sh" "${DATA_STAGING}/build-qemu-image.sh"
chmod 0755 "${DATA_STAGING}/build-qemu-image.sh"
printf '%s\n' "${DISK_SIZE}" > "${DATA_STAGING}/disk-size"

if [[ "${ENCRYPT}" == true ]]; then
    # Strip a single trailing newline from the passphrase file.  cryptsetup
    # treats the key-file bytes literally, so a trailing newline becomes part
    # of the LUKS key and interactive unlockers (GRUB cryptodisk, passphrase
    # prompts) that do not include the newline will fail to open the device.
    if [[ "$(tail -c 1 "${PASSPHRASE_FILE}" | wc -l)" -eq 1 ]]; then
        head -c -1 "${PASSPHRASE_FILE}" > "${DATA_STAGING}/luks-passphrase"
    else
        cp "${PASSPHRASE_FILE}" "${DATA_STAGING}/luks-passphrase"
    fi
    chmod 0600 "${DATA_STAGING}/luks-passphrase"
fi

echo "Packing data disk squashfs..."
mksquashfs "${DATA_STAGING}" "${DATA_SQUASHFS}" -comp zstd -Xcompression-level 15 -noappend > /dev/null

echo "Extracting kernel/initramfs from SquashFS..."
unsquashfs -no-xattrs -f -d "${WORK_DIR}/sq" "${SQUASHFS}" boot 2>/dev/null

KERNEL_SRC=$(find "${WORK_DIR}/sq/boot" -maxdepth 1 \( -name 'kernel-*' -o -name 'vmlinuz-*' \) -type f | sort | head -n1 || true)
INITRD_SRC=$(find "${WORK_DIR}/sq/boot" -maxdepth 1 \( -name 'initramfs-*.img' -o -name 'initrd-*.img' \) -type f | sort | head -n1 || true)

# Extract the modules tree so we can inject the squashfs module (and any
# future required modules) into the unified initramfs.
echo "Extracting kernel modules from SquashFS..."
unsquashfs -no-xattrs -f -d "${WORK_DIR}/sq" "${SQUASHFS}" usr/lib/modules 2>/dev/null || true
unsquashfs -no-xattrs -f -d "${WORK_DIR}/sq" "${SQUASHFS}" lib/modules 2>/dev/null || true

# Extract a minimal runtime for the custom init: chroot plus the dynamic
# linker and libc.  This lets the initramfs pivot into the stage4 rootfs even
# when the base dracut initramfs does not include chroot.
echo "Extracting chroot runtime from SquashFS..."
unsquashfs -no-xattrs -f -d "${WORK_DIR}/sq" "${SQUASHFS}" usr/bin/chroot 2>/dev/null || true
unsquashfs -no-xattrs -f -d "${WORK_DIR}/sq" "${SQUASHFS}" usr/lib64/ld-linux-x86-64.so.2 2>/dev/null || true
unsquashfs -no-xattrs -f -d "${WORK_DIR}/sq" "${SQUASHFS}" usr/lib64/libc.so.6 2>/dev/null || true
unsquashfs -no-xattrs -f -d "${WORK_DIR}/sq" "${SQUASHFS}" lib64/ld-linux-x86-64.so.2 2>/dev/null || true
unsquashfs -no-xattrs -f -d "${WORK_DIR}/sq" "${SQUASHFS}" lib64/libc.so.6 2>/dev/null || true

if [[ -z "${KERNEL_SRC}" ]]; then
    echo "Error: no kernel found in SquashFS /boot"
    exit 1
fi
if [[ -z "${INITRD_SRC}" ]]; then
    echo "Error: no initramfs found in SquashFS /boot"
    exit 1
fi

cp "${KERNEL_SRC}" "${KERNEL}"
cp "${INITRD_SRC}" "${INITRD}"

# Determine the kernel version string so we can lay out modules in the
# initramfs exactly where this kernel expects them.
KERNEL_VERSION=""
if [[ "${KERNEL_SRC}" =~ kernel-([0-9]+\.[0-9]+\.[0-9]+[^/]*)$ ]]; then
    KERNEL_VERSION="${BASH_REMATCH[1]}"
fi
if [[ -z "${KERNEL_VERSION}" && "${INITRD_SRC}" =~ initramfs-([0-9]+\.[0-9]+\.[0-9]+[^/]*)\.img$ ]]; then
    KERNEL_VERSION="${BASH_REMATCH[1]}"
fi
if [[ -z "${KERNEL_VERSION}" ]]; then
    echo "Error: unable to determine kernel version from ${KERNEL_SRC} / ${INITRD_SRC}"
    exit 1
fi
echo "Selected kernel version: ${KERNEL_VERSION}"

echo "Building custom initramfs overlay..."
OVERLAY_DIR="${WORK_DIR}/overlay"
mkdir -p "${OVERLAY_DIR}"

cat > "${OVERLAY_DIR}/init" <<'INITEOF'
#!/bin/sh
# Minimal initramfs init: bring up basic virtual filesystems, mount the
# stage4 rootfs and the data disk, then switch into the rootfs to run the
# image builder.

mount -t devtmpfs devtmpfs /dev
mount -t proc proc /proc
mount -t sysfs sysfs /sys

# Ensure /dev/fd exists so process substitution works inside the chroot.
ln -sf /proc/self/fd /dev/fd 2>/dev/null || true

modprobe dm_mod || insmod /lib/modules/*/kernel/drivers/md/dm-mod.ko
modprobe dm_crypt || insmod /lib/modules/*/kernel/drivers/md/dm-crypt.ko
modprobe squashfs || insmod /lib/modules/*/kernel/fs/squashfs/squashfs.ko
modprobe overlay || insmod /lib/modules/*/kernel/fs/overlayfs/overlay.ko
modprobe fat || insmod /lib/modules/*/kernel/fs/fat/fat.ko
modprobe vfat || insmod /lib/modules/*/kernel/fs/fat/vfat.ko
modprobe nls_cp437 || insmod /lib/modules/*/kernel/fs/nls/nls_cp437.ko
modprobe nls_ascii || insmod /lib/modules/*/kernel/fs/nls/nls_ascii.ko

mkdir -p /sysroot /data

i=0
while [ "$i" -lt 30 ]; do
    if [ -b /dev/vdb ] && [ -b /dev/vdc ]; then
        break
    fi
    sleep 1
    i=$((i + 1))
done

if [ ! -b /dev/vdb ]; then
    echo "Error: rootfs disk /dev/vdb not found"
    poweroff -f
fi
if [ ! -b /dev/vdc ]; then
    echo "Error: data disk /dev/vdc not found"
    poweroff -f
fi

mount -t squashfs /dev/vdc /data || { echo "Failed to mount /dev/vdc"; poweroff -f; }

# The stage4 rootfs on /dev/vdb is a read-only SquashFS, but the builder
# needs a writable rootfs (e.g. for /var/tmp, /run, and bind mount points).
# Stack it under a tmpfs-backed overlay so /sysroot becomes writable without
# altering the base image.  upperdir and workdir must be on the same mount,
# so they are created under a single tmpfs parent after it is mounted.
mkdir -p /lower /overlay-upper /sysroot
mount /dev/vdb /lower || { echo "Failed to mount /dev/vdb"; poweroff -f; }
mount -t tmpfs -o size=1G tmpfs /overlay-upper || { echo "Failed to mount tmpfs for overlay"; poweroff -f; }
mkdir -p /overlay-upper/upper /overlay-upper/work
mount -t overlay overlay -o lowerdir=/lower,upperdir=/overlay-upper/upper,workdir=/overlay-upper/work /sysroot || { echo "Failed to mount overlay rootfs"; poweroff -f; }

# The builder script must run inside the stage4 rootfs, but /data (mounted
# from the data disk squashfs) lives outside the rootfs.  Bind mount the
# essential virtual filesystems and /data into the rootfs so chroot can
# see the tarball, passphrase, and builder script.
mkdir -p /sysroot/dev /sysroot/proc /sysroot/sys /sysroot/data
mount --bind /dev /sysroot/dev
mount --bind /proc /sysroot/proc
mount --bind /sys /sysroot/sys
mount --bind /data /sysroot/data

/usr/bin/chroot /sysroot /bin/bash /data/vm-builder.sh || true
poweroff -f
INITEOF
chmod +x "${OVERLAY_DIR}/init"

# Stage the minimal chroot runtime inside the overlay so it becomes part of
# the unified initramfs.  Use the same /usr paths as the stage4 rootfs so the
# embedded ELF interpreter path resolves, without converting /bin or /lib64
# symlinks (common in dracut initramfs) into real directories.
if [[ -f "${WORK_DIR}/sq/usr/bin/chroot" ]]; then
    mkdir -p "${OVERLAY_DIR}/usr/bin"
    cp "${WORK_DIR}/sq/usr/bin/chroot" "${OVERLAY_DIR}/usr/bin/chroot"
fi
if [[ -f "${WORK_DIR}/sq/usr/lib64/ld-linux-x86-64.so.2" ]]; then
    mkdir -p "${OVERLAY_DIR}/usr/lib64"
    cp "${WORK_DIR}/sq/usr/lib64/ld-linux-x86-64.so.2" "${OVERLAY_DIR}/usr/lib64/ld-linux-x86-64.so.2"
fi
if [[ -f "${WORK_DIR}/sq/usr/lib64/libc.so.6" ]]; then
    mkdir -p "${OVERLAY_DIR}/usr/lib64"
    cp "${WORK_DIR}/sq/usr/lib64/libc.so.6" "${OVERLAY_DIR}/usr/lib64/libc.so.6"
fi

OVERLAY_CPIO="${WORK_DIR}/overlay.cpio"
(
    cd "${OVERLAY_DIR}"
    find . -mindepth 1 -print0 | cpio --null -o -H newc --owner=root:root > "${OVERLAY_CPIO}"
)

CUSTOM_INITRD="${WORK_DIR}/custom-initrd"
MAIN_INITRD="${WORK_DIR}/main-initrd"
MAIN_INITRD_CPIO="${WORK_DIR}/main-initrd.cpio"
INITRD_STAGING="${WORK_DIR}/initrd-staging"

cp "${INITRD}" "${WORK_DIR}/initrd.orig"
/usr/lib/dracut/skipcpio "${WORK_DIR}/initrd.orig" > "${MAIN_INITRD}"
if zstd -d "${MAIN_INITRD}" -o "${MAIN_INITRD_CPIO}" 2>/dev/null; then
    :
elif gzip -d -c "${MAIN_INITRD}" > "${MAIN_INITRD_CPIO}" 2>/dev/null; then
    :
elif xz -d -c "${MAIN_INITRD}" > "${MAIN_INITRD_CPIO}" 2>/dev/null; then
    :
else
    echo "Error: unable to decompress main initramfs."
    exit 1
fi

# Merge the stage4 initramfs and the overlay into one CPIO archive so that
# /init and all modules coexist in the same archive.
mkdir -p "${INITRD_STAGING}"
( cd "${INITRD_STAGING}"; cpio -id < "${MAIN_INITRD_CPIO}" 2>/dev/null )
( cd "${INITRD_STAGING}"; cpio -idu < "${OVERLAY_CPIO}" 2>/dev/null )

# Inject required filesystem modules from the stage4 rootfs into the initramfs.
# The kernel we are booting may not carry these modules in its own initramfs,
# so making them self-contained removes the dependency on the host module layout.
INJECTED_MODULES=(
    "kernel/fs/squashfs/squashfs.ko"
    "kernel/fs/overlayfs/overlay.ko"
    "kernel/drivers/md/dm-mod.ko"
    "kernel/drivers/md/dm-crypt.ko"
    "kernel/fs/fat/fat.ko"
    "kernel/fs/fat/vfat.ko"
    "kernel/fs/nls/nls_cp437.ko"
    "kernel/fs/nls/nls_ascii.ko"
)
for rel_path in "${INJECTED_MODULES[@]}"; do
    module_name="$(basename "${rel_path}" .ko)"
    src="${WORK_DIR}/sq/usr/lib/modules/${KERNEL_VERSION}/${rel_path}"
    if [[ ! -f "${src}" ]]; then
        src="${WORK_DIR}/sq/lib/modules/${KERNEL_VERSION}/${rel_path}"
    fi
    dst="${INITRD_STAGING}/lib/modules/${KERNEL_VERSION}/${rel_path}"
    if [[ -f "${src}" ]]; then
        echo "Injecting ${module_name}.ko (${KERNEL_VERSION}) into initramfs..."
        mkdir -p "$(dirname "${dst}")"
        cp "${src}" "${dst}"
    else
        echo "Warning: ${module_name}.ko not found for ${KERNEL_VERSION}; relying on host module loading"
    fi
done

# Regenerate modules.dep so modprobe can locate the injected modules.
if command -v depmod &> /dev/null; then
    depmod -a -b "${INITRD_STAGING}" "${KERNEL_VERSION}" 2>/dev/null || true
fi

if [[ ! -x "${INITRD_STAGING}/init" ]]; then
    echo "Error: unified initramfs is missing an executable /init"
    ls -la "${INITRD_STAGING}" | head -20
    exit 1
fi

(
    cd "${INITRD_STAGING}"
    find . -mindepth 1 -print0 | cpio --null -o -H newc --owner=root:root > "${WORK_DIR}/merged.cpio"
)
zstd -19 -f "${WORK_DIR}/merged.cpio" -o "${CUSTOM_INITRD}"


# ---------------------------------------------------------------------------
# Boot the VM and run the builder
# ---------------------------------------------------------------------------
echo "Booting KVM builder VM..."
qemu-system-x86_64 \
    -enable-kvm \
    -m 8G \
    -smp 4 \
    -cpu host \
    -nographic \
    -no-reboot \
    -kernel "${KERNEL}" \
    -initrd "${CUSTOM_INITRD}" \
    -append "root=/dev/vdb ro console=ttyS0,115200n8 init=/init" \
    -drive "file=${TARGET_RAW},format=raw,if=virtio" \
    -drive "file=${SQUASHFS},format=raw,if=virtio,readonly=on" \
    -drive "file=${DATA_SQUASHFS},format=raw,if=virtio,readonly=on"

echo "Verifying built raw disk..."

if ! parted -s "${TARGET_RAW}" print > /dev/null 2>&1; then
    echo "Error: built raw disk does not have a valid partition table."
    exit 1
fi

# The builder VM powers off regardless of success; verify it actually finished
# by looking for the sentinel file on the ROOTS partition.  A raw file needs a
# loop device before its partitions are addressable.
SENTINEL_MNT="$(TMPDIR=/var/tmp mktemp -d)"
SENTINEL_LOOP=""
SENTINEL_FOUND=false
if SENTINEL_LOOP="$(losetup -f --show -P "${TARGET_RAW}" 2>/dev/null)"; then
    if mount -o ro "${SENTINEL_LOOP}p2" "${SENTINEL_MNT}" 2>/dev/null; then
        if [[ -f "${SENTINEL_MNT}/var/lib/regicide-build-complete" ]]; then
            SENTINEL_FOUND=true
        fi
        umount "${SENTINEL_MNT}" 2>/dev/null || true
    fi
    losetup -d "${SENTINEL_LOOP}" 2>/dev/null || true
fi
rmdir "${SENTINEL_MNT}" 2>/dev/null || true
if [[ "${SENTINEL_FOUND}" != true ]]; then
    echo "Error: builder VM did not complete successfully (sentinel missing on ROOTS)."
    exit 1
fi

if [[ "${ENCRYPT}" == true ]]; then
    ROOTS_OFFSET=$(parted -s "${TARGET_RAW}" unit B print 2>/dev/null | awk '/^ 2 / {gsub(/B$/,""); print $2}')
    if [[ -z "${ROOTS_OFFSET}" ]]; then
        echo "Error: could not determine ROOTS partition offset."
        exit 1
    fi
    LUKS_SAMPLE="${WORK_DIR}/luks-sample.bin"
    dd if="${TARGET_RAW}" of="${LUKS_SAMPLE}" bs=1 count=4096 skip="${ROOTS_OFFSET}" status=none
    if ! file "${LUKS_SAMPLE}" | grep -q 'LUKS'; then
        echo "Error: ROOTS partition does not contain a LUKS header."
        exit 1
    fi
fi

# ---------------------------------------------------------------------------
# Convert the built raw image to QCOW2
# ---------------------------------------------------------------------------
echo "Converting target disk to QCOW2..."
qemu-img convert -f raw -O qcow2 "${TARGET_RAW}" "${OUTPUT}"

echo ""
echo "========================================"
echo "RegicideOS QEMU image build complete!"
echo "========================================"
echo ""
echo "Disk image: ${OUTPUT}"
echo ""
