#!/bin/bash
# Stage 7: verify that the built stage4 tarball/SquashFS satisfies the
# RegicideOS architecture and user-model requirements.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage7-verify"

TARBALL="${OUTPUT_DIR}/stage4-amd64-systemd-cosmic.tar.xz"
SQUASHFS="${OUTPUT_DIR}/regicide-cosmic.img"
VERIFY_SCRATCH_DIR="${REGICIDE_VERIFY_DIR:-/var/tmp}"
mkdir -p "${VERIFY_SCRATCH_DIR}"
ROOTS_DIR="$(mktemp -d -p "${VERIFY_SCRATCH_DIR}" -t regicide-verify-XXXXXX)"

trap 'chmod -R +w "${ROOTS_DIR}" 2>/dev/null || true; rm -rf "${ROOTS_DIR}"' EXIT

log_status "start" "verifying stage4 tarball and SquashFS"
echo "Stage 7: verifying built artifacts..."

if [[ ! -f "${TARBALL}" ]]; then
    echo "ERROR: stage4 tarball missing: ${TARBALL}"
    exit 1
fi
if [[ ! -f "${SQUASHFS}" ]]; then
    echo "ERROR: live SquashFS missing: ${SQUASHFS}"
    exit 1
fi

echo "Extracting tarball for verification..."
tar -C "${ROOTS_DIR}" -xpJf "${TARBALL}" --overwrite --exclude='./var/cache/distfiles/*' --exclude='./var/cache/binpkgs/*' --exclude='./var/tmp/*' --exclude='./tmp/*' .

ERRORS=0

fail() {
    echo "  FAIL: $1"
    ERRORS=$((ERRORS + 1))
}

pass() {
    echo "  PASS: $1"
}

# 1. Default user exists and home directory is correct.
if grep -q '^regicide:' "${ROOTS_DIR}/etc/passwd"; then
    pass "user regicide exists in /etc/passwd"
else
    fail "user regicide missing from /etc/passwd"
fi

if [[ -d "${ROOTS_DIR}/home/regicide" ]]; then
    pass "/home/regicide exists"
else
    fail "/home/regicide missing"
fi

if [[ "$(stat -c '%u:%g' "${ROOTS_DIR}/home/regicide" 2>/dev/null)" == "1000:1000" ]]; then
    pass "/home/regicide owned by regicide:regicide"
else
    fail "/home/regicide not owned by 1000:1000"
fi

# 2. Root password is unset.
ROOT_SHADOW="$(awk -F: '/^root:/ {print $2}' "${ROOTS_DIR}/etc/shadow")"
if [[ -z "${ROOT_SHADOW}" ]]; then
    pass "root password is unset"
else
    fail "root password is set in /etc/shadow"
fi

# 3. Wheel sudo is enabled.
if [[ -f "${ROOTS_DIR}/etc/sudoers.d/10-regicide-wheel" ]]; then
    pass "regicide wheel sudo drop-in exists"
else
    fail "missing /etc/sudoers.d/10-regicide-wheel"
fi

# 4. COSMIC binaries are present.
for bin in cosmic-greeter cosmic-session cosmic-comp cosmic-settings cosmic-app-library cosmic-launcher cosmic-panel cosmic-notifications cosmic-osd cosmic-workspaces cosmic-files cosmic-term cosmic-edit cosmic-store; do
    if [[ -x "${ROOTS_DIR}/usr/bin/${bin}" || -x "${ROOTS_DIR}/usr/local/bin/${bin}" ]]; then
        pass "binary ${bin} present"
    else
        fail "binary ${bin} missing"
    fi
done

# 5. Critical services are enabled.
for svc in cosmic-greeter NetworkManager bluetooth pipewire sshd; do
    svc_file=""
    case "${svc}" in
        cosmic-greeter)
            svc_file="${ROOTS_DIR}/etc/systemd/system/display-manager.service"
            ;;
        bluetooth)
            svc_file="${ROOTS_DIR}/etc/systemd/system/bluetooth.target.wants/${svc}.service"
            ;;
        pipewire)
            svc_file="${ROOTS_DIR}/etc/systemd/user/sockets.target.wants/${svc}.socket"
            ;;
        sshd)
            svc_file="${ROOTS_DIR}/etc/systemd/system/sockets.target.wants/sshd.socket"
            ;;
        *)
            svc_file="${ROOTS_DIR}/etc/systemd/system/multi-user.target.wants/${svc}.service"
            ;;
    esac
    if [[ -L "${svc_file}" ]]; then
        pass "service ${svc} enabled"
    else
        fail "service ${svc} not enabled"
    fi
done

if [[ -L "${ROOTS_DIR}/etc/systemd/system/display-manager.service" ]]; then
    dm_target="$(readlink "${ROOTS_DIR}/etc/systemd/system/display-manager.service" 2>/dev/null || true)"
    if [[ "${dm_target}" == *cosmic-greeter* ]]; then
        pass "display-manager links to cosmic-greeter"
    else
        fail "display-manager links to ${dm_target}, expected cosmic-greeter"
    fi
else
    fail "display-manager.service is not a symlink"
fi

if [[ -f "${ROOTS_DIR}/usr/lib/systemd/system/NetworkManager.service" ]]; then
    pass "NetworkManager unit loadable"
else
    fail "NetworkManager unit not loadable"
fi

# 6. Flatpak and Distrobox are present.
for bin in flatpak distrobox podman; do
    if [[ -x "${ROOTS_DIR}/usr/bin/${bin}" ]]; then
        pass "binary ${bin} present"
    else
        fail "binary ${bin} missing"
    fi
done

# 7. Btrfs tools and kernel are present.
if [[ -x "${ROOTS_DIR}/usr/bin/btrfs" ]]; then
    pass "btrfs binary present"
else
    fail "btrfs binary missing"
fi
if [[ -f "${ROOTS_DIR}/boot/vmlinuz" ]]; then
    pass "boot kernel present"
else
    fail "boot kernel missing"
fi
if [[ -f "${ROOTS_DIR}/boot/initramfs.img" ]]; then
    pass "initramfs present"
else
    fail "initramfs missing"
fi

# 8. No host-specific SSH keys baked into the image; first-boot generation enabled.
BAKED_KEY=0
for key in ssh_host_rsa_key ssh_host_ecdsa_key ssh_host_ed25519_key; do
    key_path="${ROOTS_DIR}/etc/ssh/${key}"
    if [[ -f "${key_path}" ]]; then
        BAKED_KEY=1
        fail "pre-baked SSH private key ${key} must not be in the image"
    fi
done
if [[ ${BAKED_KEY} -eq 0 ]]; then
    pass "no pre-baked SSH host keys in the image"
fi
if grep -q 'ssh-keygen -A' "${ROOTS_DIR}/usr/lib/systemd/system/sshd.service" 2>/dev/null; then
    pass "sshd.service regenerates host keys on first start"
else
    fail "sshd.service does not regenerate host keys"
fi

# 9. Setuid bits on critical binaries.
for path in /usr/bin/sudo /usr/bin/su /usr/bin/passwd /usr/bin/chfn /usr/bin/chsh /usr/bin/newgrp /usr/bin/mount /usr/bin/umount /usr/libexec/dbus-daemon-launch-helper /usr/lib/polkit-1/polkit-agent-helper-1; do
    full_path="${ROOTS_DIR}${path}"
    if [[ -u "${full_path}" ]]; then
        pass "${path} has setuid"
    elif [[ -f "${full_path}" ]]; then
        fail "${path} missing setuid"
    fi
done

# 10. Sudoers drop-in and directory permissions.
if [[ -d "${ROOTS_DIR}/etc/sudoers.d" ]]; then
    if [[ "$(stat -c '%a' "${ROOTS_DIR}/etc/sudoers.d" 2>/dev/null)" == "750" ]]; then
        pass "/etc/sudoers.d mode 750"
    else
        fail "/etc/sudoers.d mode not 750"
    fi
else
    fail "/etc/sudoers.d missing"
fi

if [[ -f "${ROOTS_DIR}/etc/sudoers.d/10-regicide-wheel" ]]; then
    if [[ "$(stat -c '%a:%U:%G' "${ROOTS_DIR}/etc/sudoers.d/10-regicide-wheel" 2>/dev/null)" == "440:root:root" ]]; then
        pass "/etc/sudoers.d/10-regicide-wheel permissions 440:root:root"
    else
        fail "/etc/sudoers.d/10-regicide-wheel permissions incorrect"
    fi
fi

# 11. SSH config drop-ins readable by root only.
if [[ -d "${ROOTS_DIR}/etc/ssh/sshd_config.d" ]]; then
    find "${ROOTS_DIR}/etc/ssh/sshd_config.d" -maxdepth 1 -type f | while read -r conf; do
        mode="$(stat -c '%a' "${conf}" 2>/dev/null || true)"
        if [[ "${mode}" == "600" || "${mode}" == "644" ]]; then
            pass "sshd_config.d/$(basename "${conf}") mode ${mode}"
        else
            fail "sshd_config.d/$(basename "${conf}") mode ${mode} not 600/644"
        fi
    done
fi

# 12. SBOM exists.
if [[ -f "${OUTPUT_DIR}/sbom.spdx.json" ]] || [[ -f "${OUTPUT_DIR}/stage4-sbom.json" ]]; then
    pass "SBOM present"
else
    fail "SBOM missing"
fi

# 13. Recovery directory populated.
if [[ -d "${ROOTS_DIR}/.recovery" && -f "${ROOTS_DIR}/.recovery/etc/passwd" ]]; then
    pass "recovery directory populated"
else
    fail "recovery directory missing or incomplete"
fi

# 14. Flathub remote configured.
if [[ -f "${ROOTS_DIR}/var/lib/flatpak/repo/config" ]] && grep -q "flathub" "${ROOTS_DIR}/var/lib/flatpak/repo/config" 2>/dev/null; then
    pass "flathub remote configured"
else
    fail "flathub remote not configured"
fi

# 15. Time sync enabled.
if [[ -L "${ROOTS_DIR}/etc/systemd/system/sysinit.target.wants/systemd-timesyncd.service" ]]; then
    pass "systemd-timesyncd enabled"
else
    fail "systemd-timesyncd not enabled"
fi

# Boot loader configuration is generated during build-vm-image.sh, not inside
# the stage4 tarball.  Verify that the required GRUB modules and kernel exist.
if [[ -f "${ROOTS_DIR}/boot/vmlinuz" && -f "${ROOTS_DIR}/boot/initramfs.img" ]] || \
   [[ -f "${ROOTS_DIR}/boot/grub/grub.cfg" ]] || \
   [[ -f "${ROOTS_DIR}/boot/EFI/grub/grub.cfg" ]] || \
   [[ -n "$(find "${ROOTS_DIR}/boot/loader/entries" -maxdepth 1 -name '*.conf' -print -quit 2>/dev/null)" ]]; then
    pass "boot loader input files present"
else
    fail "boot loader input files missing"
fi

echo ""
if [[ ${ERRORS} -eq 0 ]]; then
    echo "Stage 7 verification passed."
    log_status "complete" "stage7 verification passed"
    exit 0
else
    echo "Stage 7 verification failed with ${ERRORS} error(s)."
    log_status "error" "stage7 verification failed"
    exit 1
fi
