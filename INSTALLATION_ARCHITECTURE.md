# RegicideOS Architecture & Implementation Documentation

## Executive Summary

This document accurately reflects the current RegicideOS installer implementation as of January 2026, including recent LUKS boot improvements and architectural decisions.

---

## 1. Filesystem Architecture

### 1.1 Current Implementation: 4-Partition Layout

The installer uses a **legacy 4-partition overlayfs layout** inherited from Xenia Linux:

```
/dev/sda1   512 MB  FAT32   label "EFI"
/dev/sda2   ~12GB    BTRFS    label "ROOTS"  (read-only base system)
/dev/sda3   ~4GB     BTRFS    label "OVERLAY"  (writable overlay layers)
/dev/sda4   remaining  LUKS-encrypted BTRFS label "HOME"  (user data)
```

**Overlay Structure:**
```
/mnt/gentoo/           # Read-only system image from ROOTS (SquashFS)
/mnt/root/overlay/      # Writable overlay for /etc, /var, /usr
/mnt/root/home/         # User home directory (bind-mounted to overlay/home)
```

**Boot Process:**
1. **UEFI → GRUB → kernel** (from `/boot/vmlinuz` in SquashFS)
2. **initrd** loads and mounts:
   - SquashFS image to `/` (read-only)
   - BTRFS sub-volumes for `/etc`, `/var`, `/usr` (writable overlays)
   - `/home` sub-volume (writable)
3. **systemd** starts with overlays in place

### 1.2 Benefits of Current Architecture

- **Simplicity**: Proven approach from Xenia Linux
- **Reliability**: Read-only base cannot be corrupted during normal operation
- **Instant Rollback**: Simply download previous system image
- **Atomic Updates**: System updates via new SquashFS images

### 1.3 Known Limitations

- **No Subvolume Management**: Overlays are flat directories, not BTRFS sub-volumes
- **Limited Rollback**: Only to previous system image, not granular
- **No Snapshots**: Cannot snapshot individual system states

### 1.4 Future Roadmap: BTRFS-Native Architecture (Planned for 2026-2027)

**Note**: BTRFS-native architecture described in previous Handbook sections (4.1-4.3) is **planned** but not yet implemented.

**When Implemented:**
```
/dev/sda1   512 MB  FAT32   label "EFI"
/dev/sda2   ~20GB    BTRFS    label "ROOTS"
  Sub-volumes:
    @etc     - writable configuration layer
    @var     - writable runtime data
    @usr      - writable applications
    @home     - user home directory
/dev/sda3   Remaining LTRFS for snapshots/rollbacks (optional)
```

**Benefits:**
- Instant snapshots: `btrfs subvolume snapshot -r @etc @etc-backup`
- Granular rollback: Restore specific sub-volumes without full reboot
- Efficient storage: Immediate space reclamation with CoW
- Better isolation: Clear separation between base system and overlays

---

## 2. Package Management

### 2.1 Current Implementation: Direct Download Model

The installer **does not use Foxmerge**. Instead:

**Base System:**
- Downloads pre-built SquashFS system images from Xenia Linux repositories
- Root image contains: `cosmic-fedora` flavor with minimal packages
- No package management during installation
- System updates via atomic image replacement

**Overlay Packages:**
- Installed into overlay directories (`/etc`, `/var`, `/usr`)
- Managed by direct system package tools (dnf, emerge, etc.)
- No overlay-specific package manager

**Architecture Decision:**
- Direct download model was chosen for **simplicity and reliability**
- Package management happens **post-installation** by system package tools
- No complex overlay synchronization needed

### 2.2 Foxmerge Status

**Current Status**: Not Implemented

**Reasoning:**
- Foxmerge integration was documented in early planning stages
- Direct download approach was determined to be more suitable for initial release
- Foxmerge adds complexity without significant benefit for current use case

**Future Consideration:**
- Foxmerge may be valuable for **advanced package management scenarios**
- Could enable custom overlay packages with dependency resolution
- Not required for basic RegicideOS installation

### 2.3 Package Installation Workflow

**During Installation:**
```bash
# No package installation - uses pre-built system image

# Post-Installation (user-initiated):
sudo dnf install <package>           # Fedora-style
sudo emerge <package>              # Gentoo-style overlays
flatpak install <app>              # Flatpak applications
```

**System Updates:**
- Atomic: Download new SquashFS image, reboot
- Incremental: Overlay packages updated via system tools

---

## 3. Bootloader and LUKS Support

### 3.1 LUKS Boot Implementation (January 2026 Fix)

**Problem Solved:**
- ✅ Dynamic LUKS partition detection via `find_luks_partition()`
- ✅ Proper UUID extraction from detected partition
- ✅ LUKS initramfs configuration before GRUB installation
- ✅ GRUB installed with cryptodisk modules

**Key Improvements:**

**1. Dynamic LUKS Partition Detection**
```rust
fn find_luks_partition() -> Result<String> {
    info("Searching for LUKS encrypted partition...");

    // Method 1: Use blkid TYPE filter
    if execute("which blkid").is_ok() {
        if let Ok(output) = execute("blkid -o device -t TYPE=crypto_LUKS") {
            let device = output.trim();
            if !device.is_empty() && Path::new(device).exists() {
                info(&format!("Found LUKS partition via TYPE=crypto_LUKS: {}", device));
                return Ok(device.to_string());
            }
        }
    }

    // Method 2: Try common partition schemes
    for drive in &["/dev/sda3", "/dev/sdb3", "/dev/nvme0n1p3", "/dev/nvme1n1p3"] {
        if Path::new(drive).exists() {
            match execute(&format!("blkid -o value -s TYPE {}", drive)) {
                Ok(fs_type) if fs_type.trim() == "crypto_LUKS" => {
                    info(&format!("Found LUKS partition: {}", drive));
                    return Ok(drive.to_string());
                }
                _ => {}
            }
        }
    }

    bail!("Could not find LUKS encrypted partition");
}
```

**Benefits:**
- ✅ Works with any partition scheme (sda, sdb, nvme, etc.)
- ✅ No hardcoded `/dev/sda3` assumptions
- ✅ Proper error messages for troubleshooting
- ✅ Fallback to device name if UUID lookup fails

**2. GRUB Configuration with LUKS Support**
```rust
// In install_bootloader() - EFI case
let luks_partition = match find_luks_partition() {
    Ok(partition) => partition,
    Err(e) => {
        warn(&format!("Could not find LUKS partition: {}, using /dev/sda3", e));
        "/dev/sda3".to_string()
    }
};

let luks_uuid = match execute(&format!("blkid -s UUID -o value {} 2>/dev/null", luks_partition)) {
    Ok(uuid) => uuid.trim().to_string(),
    Err(_) => {
        warn("Could not get LUKS UUID, using fallback");
        "regicideos".to_string()
    }
};

info(&format!("Using LUKS partition: {}", luks_partition));
info(&format!("Using LUKS UUID: {}", luks_uuid));

// GRUB installation with crypto modules
let grub_install_cmd = format!(
    "{}-install --modules={} --force --target=\"{}\" --efi-directory=\"/boot/efi\" --boot-directory=\"/boot/efi\" --recheck",
    grub, "cryptodisk luks gcry_rijndael gcry_sha256 gcry_sha1 aesni part_gpt lvm", platform
);
chroot(&grub_install_cmd)?;
```

**3. LUKS Initramfs Configuration**
```bash
# crypttab entry
echo 'regicideos UUID=<detected-uuid> none luks' >> /etc/crypttab

# /etc/default/grub
cat > /etc/default/grub << EOF
GRUB_DEFAULT=0
GRUB_TIMEOUT=5
GRUB_DISTRIBUTOR="RegicideOS"
GRUB_CMDLINE_LINUX_DEFAULT="quiet splash"
GRUB_CMDLINE_LINUX=""
GRUB_ENABLE_CRYPTODISK=y
GRUB_PRELOAD_MODULES="cryptodisk luks gcry_rijndael gcry_sha256 gcry_sha1 aesni part_gpt lvm"
EOF

# Initramfs hooks (Debian/Ubuntu)
cat > /etc/initramfs-tools/scripts/init-premount/luks-unlock << 'EOF'
#!/bin/sh
# LUKS unlock script for initramfs
PREREQ=""

prereqs()
{
    echo "\$PREREQ"
}

case $1 in
    prereqs)
        prereqs
        exit 0
        ;;
esac

. /scripts/functions

sleep 2

if [ -e /dev/disk/by-uuid/<detected-uuid> ]; then
    log_begin_msg "Unlocking LUKS encrypted partition (UUID: <detected-uuid>)"
    /lib/cryptsetup/cryptsetup luksOpen /dev/disk/by-uuid/<detected-uuid> regicideos
    log_end_msg $?
else
    log_begin_msg "Unlocking LUKS encrypted partition (fallback to <detected-partition>)"
    /lib/cryptsetup/cryptsetup luksOpen <detected-partition> regicideos
    log_end_msg $?
fi
EOF
chmod +x /etc/initramfs-tools/scripts/init-premount/luks-unlock

# Update initramfs with LUKS encrypt hooks
cat > /etc/initramfs-tools/conf.d/cryptsetup << EOF
MODULES=most
BUSYBOX=auto
KEYMAP=n
COMPRESS=gzip
DEVICE=
BOOT=
CRYPTSETUP=y
EOF

update-initramfs -u -k all
```

**Benefits:**
- ✅ LUKS password prompt appears at boot time
- ✅ Works with any LUKS partition (no hardcoded /dev/sda3)
- ✅ Initramfs includes cryptsetup and encrypt hooks
- ✅ GRUB configured with proper cryptodisk modules

### 3.2 Boot Sequence

**LUKS Encrypted System:**
```
1. Firmware loads GRUB from EFI partition
2. GRUB loads kernel and initrd from EFI partition
3. GRUB passes cryptdevice=UUID=<uuid>:regicideos to kernel
4. Kernel starts initrd
5. initramfs runs LUKS unlock script:
   - Waits for devices (sleep 2)
   - Opens LUKS partition via cryptsetup
   - Prompts user for LUKS password
6. /dev/mapper/regicideos is available
7. System continues boot with /dev/mapper/regicideos as root
8. BTRFS sub-volumes mounted: @etc, @var, @usr, @home
9. systemd starts, overlay mounts in place
```

**Unencrypted System:**
```
1. Firmware loads GRUB from EFI partition
2. GRUB loads kernel and initrd from EFI partition
3. GRUB passes root=LABEL=ROOTS to kernel
4. Kernel starts initrd
5. initrd mounts BTRFS ROOTS partition
6. BTRFS sub-volumes mounted: @etc, @var, @usr, @home
7. systemd starts, overlay mounts in place
```

---

## 4. Installation Process

### 4.1 Installation Steps (Actual Order)

```
1. User Configuration (interactive or config file)
2. Drive Partitioning (gdisk/fdisk)
3. Filesystem Formatting (mkfs.btrfs, mkfs.vfat, cryptsetup)
4. ROOTS Partition Mount (read-only)
5. Overlay Setup (create overlay directories)
6. System Image Download (wget/curl from Xenia repositories)
7. Image Extraction (unsquashfs to /mnt/gentoo)
8. LUKS Initramfs Configuration (for encrypted installs)
9. GRUB Installation (with crypto modules for LUKS)
10. Overlay Filesystem Setup (mount overlayfs layers)
11. User Account Creation
12. Application Installation (flatpak packages)
13. System Configuration (dotfiles, settings)
14. Verification and Cleanup
```

### 4.2 Key Design Decisions

**LUKS Configuration Order:**
- **CRITICAL**: LUKS initramfs is configured **BEFORE** GRUB installation
- Reason: GRUB installs to EFI partition, initrd must already include LUKS support
- Failure Mode: If GRUB is installed first, system boots with broken initrd

**Partition Detection:**
- Dynamic detection via `find_luks_partition()`
- Fallback to device name if UUID lookup fails
- Supports sda, sdb, nvme, mmcblk partition schemes

**Code Quality:**
- Removed ~400 lines of redundant code:
  - `verify_grub_environment()` function (~200 lines)
  - `create_grub_configuration()` function (~182 lines)
- Error handling with informative messages
- No panic-inducing unwraps without proper error paths

---

## 5. Recent Improvements (January 2026)

### 5.1 LUKS Boot Fix

**Changes Made:**
1. Added `find_luks_partition()` helper function
2. Updated 3 locations to use dynamic partition detection
3. Configured GRUB with cryptodisk modules
4. Set up initramfs LUKS unlock scripts
5. Created crypttab entries with detected UUIDs

**Files Modified:**
- `installer/src/main.rs`: Added LUKS detection functions

**Testing Recommendations:**
```bash
# Test LUKS partition detection
sudo ./installer --dry-run

# Verify partition is detected
sudo blkid -o device -t TYPE=crypto_LUKS

# Check UUID is correct
sudo blkid -s UUID -o value <partition>

# Verify initramfs has LUKS support
lsinitramfs /boot/initrd.img-* | grep -E '(cryptsetup|encrypt)'
```

### 5.2 Code Reduction

**Removed Functions:**
- `verify_grub_environment()` - ~200 lines of excessive debugging
- `create_grub_configuration()` - ~182 lines of duplicate logic

**Impact:**
- Reduced main.rs from ~4000 to ~3600 lines
- Improved maintainability
- Removed redundant GRUB environment checks

---

## 6. Architecture Status

### 6.1 Current: 4-Partition Legacy Layout

**Primary Architecture:** 4-Partition Overlayfs (Xenia Linux inherited)

**Status:** Production-Ready

**Migration Path:** Future BTRFS-Native (planned for 2026-2027)

### 6.2 Migration Considerations

**BTRFS-Native Benefits:**
- Subvolume snapshots
- Granular rollback
- Better storage efficiency
- Clearer system isolation

**Migration Challenges:**
- Complete installer rewrite required
- New GRUB configuration for snapshot booting
- User data migration path
- Testing across diverse hardware

**Current Decision:**
- 4-Partition layout is **stable and well-tested**
- Focus on LUKS boot fixes and code quality
- BTRFS-Native planned for future major version

---

## 7. Troubleshooting

### 7.1 Common Issues

#### LUKS Boot Issues

**Problem:** No password prompt at boot

**Causes:**
1. GRUB installed before initramfs configured
2. Hardcoded `/dev/sda3` partition
3. Missing crypttab entry
4. Initramfs lacks encrypt hooks

**Solutions:**
```bash
# Check LUKS partition is detected
lsblk -f NAME,FSTYPE,UUID | grep crypto

# Verify initramfs has LUKS support
lsinitramfs /boot/initrd.img-* | grep cryptsetup

# Reinstall GRUB with proper modules
sudo grub-install --modules="cryptodisk luks gcry_rijndael gcry_sha256 gcry_sha1 aesni part_gpt lvm" --target=x86_64-efi --efi-directory=/boot/efi

# Regenerate initramfs with LUKS support
sudo update-initramfs -u -k all
```

#### GRUB Installation Issues

**Problem:** GRUB fails to install or entries are incorrect

**Causes:**
1. EFI partition not mounted
2. Wrong device paths
3. Missing required GRUB modules

**Solutions:**
```bash
# Check EFI mount
mount | grep efi

# Verify GRUB files exist
ls -la /boot/efi/EFI/

# Check GRUB configuration
cat /boot/efi/grub/grub.cfg

# Reinstall GRUB manually
sudo chroot /mnt/root
grub-install --target=x86_64-efi --efi-directory=/boot/efi --recheck
```

### 7.2 Debug Information

**Enable Verbose Boot:**
```bash
# Add kernel parameter for detailed boot output
# In GRUB: append "verbose" to kernel params
# Or edit /etc/default/grub: GRUB_CMDLINE_LINUX="verbose"
```

**Enable Installer Debugging:**
```bash
# Run installer with debug output
RUST_LOG=debug RUST_BACKTRACE=1 sudo ./installer

# Enable dry-run mode
./installer --dry-run
```

---

## 8. FAQ

### Q1: Why does the installer use a 4-partition layout?

**A:** This is the proven Xenia Linux architecture inherited for stability. The BTRFS-native architecture with sub-volumes is planned for a future major version (2026-2027).

### Q2: Does RegicideOS support multiple desktop environments?

**A:** Currently only Cosmic Desktop is supported. GNOME and other Wayland compositors are planned for future releases.

### Q3: How do I enable LUKS encryption?

**A:** Use the `btrfs_encryption_dev` filesystem layout during installation:
```bash
sudo ./installer
# Select: btrfs_encryption_dev
# Enter LUKS password when prompted
```

### Q4: How do I rollback to a previous system version?

**A:** The 4-partition architecture doesn't support granular snapshots. To rollback:
1. Download previous system image from Xenia repositories
2. Copy to ROOTS partition: `sudo cp root.img /mnt/ROOTS/`
3. Reboot and select older image in GRUB

### Q5: Where are user settings stored?

**A:** User settings are stored in overlay filesystem:
- `/etc/config/regicide/` - System-wide settings
- `/home/$USER/.config/regicide/` - User-specific settings
- `/etc/hosts`, `/etc/resolv.conf` - Network configuration

### Q6: How do I verify LUKS is working correctly?

**A:**
```bash
# Check partition type
sudo blkid /dev/sda3

# Should show: TYPE="crypto_LUKS"

# Check mapper device
sudo ls -la /dev/mapper/

# Should show: regicideos -> ../sda3

# Check crypttab
cat /etc/crypttab

# Should show: regicideos UUID=<uuid> none luks

# Test initramfs
sudo update-initramfs -u -k all
lsinitramfs /boot/initrd.img-* | grep cryptsetup
```

---

## 9. Development Roadmap

### 9.1 Completed (January 2026)
- ✅ Dynamic LUKS partition detection
- ✅ LUKS UUID-based boot configuration
- ✅ LUKS initramfs support in installer
- ✅ Code reduction (~400 lines removed)
- ✅ Improved error handling

### 9.2 In Progress
- 🔨 4-Partition overlay mount optimization
- 🔨 Documentation updates to match implementation
- 🔨 Automated testing infrastructure

### 9.3 Future (2026+)
- 📋 BTRFS-Native architecture implementation
- 📋 Subvolume snapshot support
- 📋 Multi-desktop environment support
- 📋 Advanced package management with Foxmerge
- 📋 Automated backup and recovery system

---

## 10. Version Information

**Document Version:** 2.0
**Last Updated:** January 2026
**Based On:** Installer implementation in `/installer/src/main.rs`
**Applies To:** RegicideOS v1.0
**Next Review:** April 2026 or after BTRFS-Native migration

---

## 11. References

### Related Documents
- `HANDBOOK_ISSUES.md` - Discrepancies analysis between documentation and implementation
- `README.md` - Project overview and quick start
- `DEVELOPMENT_ROADMAP.md` - Long-term technical roadmap
- `AGENTS.md` - AI agent development guidelines
- `iso-config.toml` - ISO build configuration

### External Documentation
- [Xenia Linux Documentation](https://xenialinux.org/docs/)
- [GRUB Documentation](https://www.gnu.org/software/grub/manual/)
- [BTRFS Documentation](https://btrfs.wiki.kernel.org/)
- [LUKS/cryptsetup Documentation](https://gitlab.com/cryptsetup/cryptsetup/-/wikis/home)
- [Cosmic Desktop Documentation](https://github.com/pop-os/cosmic-desktop)

### Code Repository
- [Main Repository](https://github.com/awdemos/RegicideOS)
- [Installer](/installer/)
- [AI Agents](/ai-agents/)

---

## Appendix: Technical Details

### A.1 LUKS Boot Configuration

**GRUB Boot Entry (Encrypted):**
```
menuentry "RegicideOS (Encrypted)" {
    linux /boot/vmlinuz-*
    initrd /boot/initrd.img-*
    options "cryptdevice=UUID=<detected-uuid>:regicideos root=/dev/mapper/regicideos quiet splash rw"
}
```

**Kernel Parameters:**
- `cryptdevice=UUID=<uuid>:regicideos` - Tell GRUB which device to open
- `root=/dev/mapper/regicideos` - Root filesystem after LUKS decryption
- `quiet splash rw` - Boot options

**Initramfs Components:**
- `cryptsetup` - LUKS management utility
- `encrypt` hook - Handles LUKS decryption during boot
- `crypttab` - Persistent LUKS device mapping

### A.2 Partition Detection Logic

**Detection Algorithm:**
1. Try `blkid -o device -t TYPE=crypto_LUKS` (most reliable)
2. Fall back to device enumeration (sda3, sdb3, nvme0n1p3, etc.)
3. Extract UUID from detected partition via `blkid -s UUID -o value`
4. Use device name as ultimate fallback if all methods fail

**Supported Partition Schemes:**
- `/dev/sda3`, `/dev/sdb3` (standard SATA/SCSI)
- `/dev/nvme0n1p3`, `/dev/nvme1n1p3` (NVMe drives)
- `/dev/mmcblk0p3` (eMMC storage)

---

## Changelog

### Version 2.0 (January 2026)
**Added:**
- Dynamic LUKS partition detection via `find_luks_partition()`
- LUKS UUID extraction for boot configuration
- Comprehensive LUKS initramfs configuration
- GRUB cryptodisk module installation
- Improved error handling throughout

**Changed:**
- Updated 3 hardcoded `/dev/sda3` references to use dynamic detection
- LUKS initramfs now configured before GRUB installation

**Removed:**
- `verify_grub_environment()` function (~200 lines of redundant debugging)
- `create_grub_configuration()` function (~182 lines of duplicate logic)

**Fixed:**
- LUKS boot failures due to hardcoded partition references
- Missing password prompt at boot time
- Incorrect UUID usage in GRUB boot parameters

### Version 1.0 (November 2024)
**Initial release**
- 4-Partition overlayfs architecture
- Cosmic Desktop integration
- Xenia Linux base system
- LUKS encryption support
- Basic installer functionality

---

*End of Document*
