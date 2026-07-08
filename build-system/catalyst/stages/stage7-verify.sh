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

SUDOERS_DROPIN="${ROOTS_DIR}/etc/sudoers.d/10-regicide-wheel"
if [[ -f "${SUDOERS_DROPIN}" ]]; then
    dropin_mode="$(stat -c '%a' "${SUDOERS_DROPIN}" 2>/dev/null || true)"
    if [[ "${dropin_mode}" != "440" ]]; then
        fail "/etc/sudoers.d/10-regicide-wheel permissions incorrect (mode ${dropin_mode}, expected 440)"
    elif [[ "$(id -u)" -eq 0 && "$(stat -c '%U:%G' "${SUDOERS_DROPIN}" 2>/dev/null || true)" != "root:root" ]]; then
        fail "/etc/sudoers.d/10-regicide-wheel permissions incorrect (expected 440:root:root)"
    else
        pass "/etc/sudoers.d/10-regicide-wheel permissions 440:root:root"
    fi
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

nm_wants="${ROOTS_DIR}/etc/systemd/system/multi-user.target.wants/NetworkManager.service"
if [[ -L "${nm_wants}" ]]; then
    nm_target="$(readlink "${nm_wants}" 2>/dev/null || true)"
    if [[ "${nm_target}" == /usr/lib/systemd/system/NetworkManager.service ]]; then
        pass "NetworkManager wants symlink is absolute"
    else
        fail "NetworkManager wants symlink points to ${nm_target}, expected /usr/lib/systemd/system/NetworkManager.service"
    fi
else
    fail "NetworkManager not enabled in multi-user.target.wants"
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
if grep -q 'ssh-keygen -A' "${ROOTS_DIR}/usr/lib/systemd/system/sshd.service" 2>/dev/null || \
   [[ -x "${ROOTS_DIR}/usr/lib/regicide/regicide-ssh-keygen" ]]; then
    pass "sshd host-key regeneration configured on first boot"
else
    fail "sshd host-key regeneration not configured"
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

# 16. Default user can edit key system files (immutable overlay usability).
for path in /etc/hosts /etc/fstab /etc/portage/make.conf; do
    full_path="${ROOTS_DIR}${path}"
    if [[ -f "${full_path}" ]]; then
        owner="$(stat -c '%u:%g' "${full_path}" 2>/dev/null || true)"
        mode="$(stat -c '%a' "${full_path}" 2>/dev/null || true)"
        if [[ "${owner}" == "1000:1000" ]]; then
            pass "${path} owned by regicide:regicide"
        else
            fail "${path} owner ${owner}, expected 1000:1000"
        fi
        if [[ "${mode}" == "664" ]]; then
            pass "${path} mode 0664"
        else
            fail "${path} mode ${mode}, expected 664"
        fi
    else
        fail "${path} missing"
    fi
done

# 17. Root README.md is world-readable.
readme_path="${ROOTS_DIR}/README.md"
if [[ -f "${readme_path}" ]]; then
    readme_mode="$(stat -c '%a' "${readme_path}" 2>/dev/null || true)"
    if [[ "${readme_mode}" == "644" ]]; then
        pass "/README.md is world-readable"
    else
        fail "/README.md mode ${readme_mode}, expected 644"
    fi
else
    # README.md is not present in all builds; do not fail if missing.
    pass "/README.md not present, skipping"
fi

# 18. SSH does not hang on DNS unavailability.
if [[ -f "${ROOTS_DIR}/etc/ssh/sshd_config" ]] && grep -q '^UseDNS no' "${ROOTS_DIR}/etc/ssh/sshd_config"; then
    pass "sshd UseDNS disabled"
else
    fail "sshd UseDNS no missing"
fi

# 19. NSS allows DNS fallback when systemd-resolved is inactive.
if [[ -f "${ROOTS_DIR}/etc/nsswitch.conf" ]]; then
    if grep -q '^hosts:.*resolve \[!UNAVAIL=return\]' "${ROOTS_DIR}/etc/nsswitch.conf"; then
        fail "nsswitch.conf hosts line blocks DNS fallback"
    else
        pass "nsswitch.conf allows DNS fallback"
    fi
else
    fail "/etc/nsswitch.conf missing"
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
