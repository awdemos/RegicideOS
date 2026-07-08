#!/bin/bash
# Stage 8: post-install VM smoke test.
# Boots a RegicideOS QCOW2, waits for the serial login prompt to confirm the
# system has started, then runs runtime checks over SSH for clean output.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage8-vm-test"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CATALYST_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUTPUT_DIR="${OUTPUT_DIR:-${CATALYST_DIR}/output}"

DEFAULT_QCOW2="${OUTPUT_DIR}/regicide-cosmic.qcow2"
QCOW2="${1:-${DEFAULT_QCOW2}}"
QCOW2="$(realpath -e "${QCOW2}" 2>/dev/null || true)"

# Pick a free local SSH forwarding port.  A fixed port (2222) collides when
# another RegicideOS VM is already running on the host.
find_free_port() {
    local base="${1:-2222}"
    local port
    for port in $(seq "${base}" 2999); do
        if ! (command -v ss >/dev/null 2>&1 && ss -Htn "sport = :${port}" | grep -q .) && \
           ! (command -v netstat >/dev/null 2>&1 && netstat -atn 2>/dev/null | grep -q ":${port} ") && \
           ! (timeout 1 bash -c "exec 3<>/dev/tcp/127.0.0.1/${port}" 2>/dev/null); then
            echo "${port}"
            return 0
        fi
    done
    echo "ERROR: no free TCP port found in range ${base}-2999" >&2
    return 1
}
SSH_PORT="${REGICIDE_VM_SSH_PORT:-$(find_free_port 2222)}"
VM_MEMORY="${REGICIDE_VM_MEMORY:-4096}"
VM_SMP="${REGICIDE_VM_SMP:-4}"
TIMEOUT_SEC="${REGICIDE_VM_TIMEOUT:-300}"
DIAG_DIR="${OUTPUT_DIR}/vm-test-diagnostics"
VM_DISPLAY="${REGICIDE_VM_DISPLAY:-none}"
SSH_USER="regicide"
SSH_PASS="regicide"

log_status() {
    local event="${1:-info}"
    local detail="${2:-}"
    local status_dir="${OUTPUT_DIR}"
    local status_file="${status_dir}/build-status.jsonl"
    mkdir -p "${status_dir}"
    printf '{"time":"%s","stage":"%s","event":"%s","detail":"%s"}\n' \
        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        "${STAGE_NAME}" \
        "${event}" \
        "${detail}" >> "${status_file}"
}

if [[ -z "${QCOW2}" || ! -f "${QCOW2}" ]]; then
    echo "ERROR: QCOW2 image not found: ${1:-${DEFAULT_QCOW2}}"
    exit 1
fi

if [[ ! -e /dev/kvm ]]; then
    echo "Warning: /dev/kvm not available; VM will be very slow."
fi

mkdir -p "${DIAG_DIR}"

# Use /var/tmp for sockets/OVMF copy so a small tmpfs /tmp is not exhausted.
WORK_DIR="$(TMPDIR=/var/tmp mktemp -d -t regicide-vm-test-XXXXXX)"
trap 'rm -rf "${WORK_DIR}"' EXIT

OVMF_CODE=""
OVMF_VARS_SRC=""
for path in \
    /usr/share/OVMF/OVMF_CODE.fd \
    /usr/share/edk2/ovmf/OVMF_CODE.fd \
    /usr/share/qemu/OVMF_CODE.fd \
    /usr/share/ovmf/x64/OVMF_CODE.fd
do
    if [[ -f "${path}" ]]; then
        OVMF_CODE="${path}"
        break
    fi
done
for path in \
    /usr/share/OVMF/OVMF_VARS.fd \
    /usr/share/edk2/ovmf/OVMF_VARS.fd \
    /usr/share/qemu/OVMF_VARS.fd \
    /usr/share/ovmf/x64/OVMF_VARS.fd
do
    if [[ -f "${path}" ]]; then
        OVMF_VARS_SRC="${path}"
        break
    fi
done

if [[ -z "${OVMF_CODE}" ]]; then
    echo "ERROR: OVMF firmware not found."
    exit 1
fi

OVMF_VARS="${WORK_DIR}/ovmf-vars.fd"
cp "${OVMF_VARS_SRC:-${OVMF_CODE}}" "${OVMF_VARS}" 2>/dev/null || true

SERIAL_SOCK="${WORK_DIR}/serial.sock"
MONITOR_SOCK="${WORK_DIR}/monitor.sock"
PIDFILE="${WORK_DIR}/qemu.pid"

KVM_FLAGS=()
if [[ -e /dev/kvm ]] && [[ -r /dev/kvm ]]; then
    KVM_FLAGS=(-enable-kvm -cpu host)
fi

log_status "start" "booting ${QCOW2}"
echo "Stage 8: post-install VM test"
echo "  Image:  ${QCOW2}"
echo "  SSH:    localhost:${SSH_PORT} -> :22"
echo "  Display: ${VM_DISPLAY}"
echo "  Memory: ${VM_MEMORY} MB"
echo "  CPUs:   ${VM_SMP}"
echo "  Timeout: ${TIMEOUT_SEC}s"

# Default to headless; use REGICIDE_VM_DISPLAY=vnc or sdl for manual observation.
DISPLAY_ARGS=(-display none -vga none)
case "${VM_DISPLAY}" in
    vnc) DISPLAY_ARGS=(-display vnc=:0 -vga virtio) ;;
    sdl) DISPLAY_ARGS=(-display "sdl,gl=on" -vga virtio) ;;
esac

echo "Starting QEMU..."
qemu-system-x86_64 \
    "${KVM_FLAGS[@]}" \
    -m "${VM_MEMORY}" \
    -smp "${VM_SMP}" \
    -drive "file=${QCOW2},format=qcow2,if=virtio" \
    -netdev "user,id=net0,hostfwd=tcp::${SSH_PORT}-:22" \
    -device virtio-net-pci,netdev=net0 \
    "${DISPLAY_ARGS[@]}" \
    -machine type=q35 \
    -drive "if=pflash,format=raw,readonly=on,file=${OVMF_CODE}" \
    -drive "if=pflash,format=raw,file=${OVMF_VARS}" \
    -serial "unix:${SERIAL_SOCK},server,nowait" \
    -monitor "unix:${MONITOR_SOCK},server,nowait" \
    -daemonize \
    -pidfile "${PIDFILE}"

# Wait for QEMU to create the serial socket.
for _ in $(seq 1 30); do
    if [[ -S "${SERIAL_SOCK}" ]]; then
        break
    fi
    sleep 0.5
done
if [[ ! -S "${SERIAL_SOCK}" ]]; then
    echo "ERROR: QEMU serial socket did not appear."
    exit 1
fi

QEMU_PID="$(cat "${PIDFILE}" 2>/dev/null || true)"
if [[ -z "${QEMU_PID}" ]]; then
    echo "ERROR: QEMU pidfile empty."
    exit 1
fi

cleanup_qemu() {
    if [[ -n "${QEMU_PID}" ]] && kill -0 "${QEMU_PID}" 2>/dev/null; then
        echo "Stopping QEMU (pid ${QEMU_PID})..."
        kill "${QEMU_PID}" 2>/dev/null || true
        sleep 2
        kill -9 "${QEMU_PID}" 2>/dev/null || true
    fi
}
trap 'cleanup_qemu; rm -rf "${WORK_DIR}"' EXIT

# Wait for the login prompt on the serial console; this confirms the OS has
# booted far enough for SSH to be usable without polling the TCP port.
echo "Waiting for VM to reach login prompt (up to ${TIMEOUT_SEC}s)..."
python3 - "${SERIAL_SOCK}" "${TIMEOUT_SEC}" <<'PYEOF'
import socket, sys, time
sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
deadline = time.time() + 30
while True:
    try:
        sock.connect(sys.argv[1])
        break
    except (FileNotFoundError, ConnectionRefusedError):
        if time.time() > deadline:
            print("!ERR: serial socket unavailable", flush=True)
            sys.exit(1)
        time.sleep(0.2)
sock.setblocking(False)
buf = b""
login_deadline = time.time() + int(sys.argv[2])
while time.time() < login_deadline:
    try:
        data = sock.recv(4096)
        if data:
            buf += data
            if len(buf) > 8192:
                buf = buf[-8192:]
            text = buf.decode(errors="replace")
            if "login:" in text or "Password:" in text:
                print("LOGIN_PROMPT_OK", flush=True)
                sys.exit(0)
    except BlockingIOError:
        pass
    time.sleep(0.2)
print("!ERR: login prompt not seen within timeout", flush=True)
sys.exit(1)
PYEOF

# Wait for sshd to be ready on the forwarded port.  A TCP open is not enough;
# QEMU's user networking accepts the connection before the guest sshd is ready,
# so wait for the SSH protocol banner.
echo "Waiting for sshd banner on localhost:${SSH_PORT}..."
ready=false
for _ in $(seq 1 120); do
    # Read one line rather than a fixed byte count; OpenSSH banners are
    # shorter than 32 bytes ("SSH-2.0-...\\r\\n"), so head -c 32 can block
    # waiting for more data and cause the timeout to fire even when sshd is
    # already listening.
    if timeout 2 bash -c "exec 3<>/dev/tcp/localhost/${SSH_PORT}; head -n 1 <&3 | grep -q SSH" 2>/dev/null; then
        ready=true
        break
    fi
    sleep 1
done
if [[ "${ready}" != true ]]; then
    echo "ERROR: sshd did not become reachable on port ${SSH_PORT}"
    exit 1
fi

# Use sshpass for password-based SSH; StrictHostKeyChecking=no because this is
# a freshly built VM with new host keys generated on first boot.
SSH_HOST="localhost"
SSH_OPTS=(-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR -o ConnectTimeout=5 -p "${SSH_PORT}")
run_ssh() {
    sshpass -p "${SSH_PASS}" ssh "${SSH_OPTS[@]}" "${SSH_USER}@${SSH_HOST}" "$@"
}

mkdir -p "${DIAG_DIR}"

checks=(
    "whoami:whoami:regicide"
    "uid:id -u:1000"
    "groups:id:wheel"
    "sudo:sudo -n whoami:root"
    "kernel:uname -r:"
    # COSMIC on Gentoo exposes the greeter via cosmic-greeter-daemon.service.
    # The generic display-manager alias and the per-user cosmic-comp/session
    # processes are not reliable in a headless VM smoke test.
    "cosmic-greeter:systemctl is-active cosmic-greeter-daemon.service:active"
    "sshd-socket:systemctl is-active sshd.socket:active"
    "failed-units:systemctl --failed --no-pager:0 loaded units listed"
    # Avoid colons and single quotes in the command; colons split the label/cmd/expect
    # fields and single quotes are mangled by sshpass/ssh argument parsing.
    "network-interface:ip -o link show up | grep -v lo | grep state | grep UP | head -1 && echo up:up"
    "loopback-up:ip link show lo:<LOOPBACK,UP,LOWER_UP>"
    "resolv-conf:cat /etc/resolv.conf:nameserver"
    # sshd.socket is already checked, so just verify some TCP socket is listening.
    "ssh-listen:ss -tlnp | grep -q LISTEN && echo listening:listening"
    "podman-smoke:export HOME=/home/regicide XDG_RUNTIME_DIR=/run/user/1000; timeout 120 podman run --rm docker.io/library/alpine echo podman-smoke-ok:podman-smoke-ok"
    # Distrobox create+remove is enough in constrained test images; entering the
    # box installs packages and can fill the small OVERLAY partition.  We use a
    # fully-qualified image name because Gentoo's podman has no default short-name
    # registry config, so an unqualified "alpine" would hang waiting for input.
    "distrobox-smoke:export HOME=/home/regicide XDG_RUNTIME_DIR=/run/user/1000; rm -rf /home/regicide/.local/share/containers /var/tmp/regicide-distrobox-* /run/user/1000/libpod /run/user/1000/containers 2>/dev/null || true; distrobox rm regicide-smoke-alpine --force >/dev/null 2>&1 || true; timeout 120 podman pull docker.io/library/alpine >/dev/null 2>&1; pull_rc=\$?; timeout 300 distrobox create --image docker.io/library/alpine --name regicide-smoke-alpine --yes >/dev/null 2>&1; create_rc=\$?; timeout 60 distrobox rm regicide-smoke-alpine --force >/dev/null 2>&1; rm_rc=\$?; echo pull_rc=\${pull_rc} create_rc=\${create_rc} rm_rc=\${rm_rc}; test \${pull_rc} -eq 0 && test \${create_rc} -eq 0 && test \${rm_rc} -eq 0 && echo distrobox-smoke-ok:distrobox-smoke-ok"
    "cosmic-processes:pgrep -f -c cosmic-greeter-daemon:1"
    "sshd-keygen:systemctl is-active sshd-keygen@ed25519.service || systemctl is-active sshd-keygen@rsa.service || ls /etc/ssh/ssh_host_ed25519_key /etc/ssh/ssh_host_rsa_key 2>/dev/null:ssh_host"
    "btrfs:command -v btrfs:btrfs"
    "flatpak:command -v flatpak:flatpak"
    "distrobox:command -v distrobox:distrobox"
    "sudoers-dropin:sudo cat /etc/sudoers.d/10-regicide-wheel:%wheel"
    # Avoid awk -F: being split by sshpass; just verify root has a passwd entry.
    "root-password:getent passwd root | grep -q root && echo root-ok:root-ok"
    # Btrfs layout checks adapted from RegicideOSArch.  Gentoo/COSMIC uses real
    # Btrfs subvolumes for /, /etc, /var and /home rather than cross-partition
    # overlays, so only the immutable-lowerdir tests are carried over.
    "root-fs-btrfs:findmnt -n -o FSTYPE / | grep -q btrfs && echo btrfs:btrfs"
    "root-subvolid-5:findmnt -n -o OPTIONS / | grep -q subvolid=5 && echo subvolid5:subvolid5"
    "home-fs-btrfs:findmnt -n -o FSTYPE /home | grep -q btrfs && echo btrfs:btrfs"
    "home-subvol-home:findmnt -n -o OPTIONS /home | grep -q subvol=/home && echo home:home"
    "overlay-fs-btrfs:findmnt -n -o FSTYPE /overlay | grep -q btrfs && echo btrfs:btrfs"
    "etc-fs-btrfs:findmnt -n -o FSTYPE /etc | grep -q btrfs && echo btrfs:btrfs"
    "var-fs-btrfs:findmnt -n -o FSTYPE /var | grep -q btrfs && echo btrfs:btrfs"
    "efi-partition-vfat:test -d /sys/firmware/efi && lsblk -f | grep -qi vfat.*efi && echo efi:efi"
    "overlay-etc-subvol:sudo -n btrfs subvolume list /overlay | rev | cut -d\  -f1 | rev | grep -q ^etc$ && echo etc:etc"
    "overlay-var-subvol:sudo -n btrfs subvolume list /overlay | rev | cut -d\  -f1 | rev | grep -q ^var$ && echo var:var"
    "overlay-workdirs:ls -d /overlay/etcw /overlay/varw >/dev/null 2>&1 && echo workdirs:workdirs"
    "usr-bin-readonly:bash -c 'if touch /usr/bin/.smoke-test 2>/dev/null; then rm -f /usr/bin/.smoke-test; echo writable; else echo readonly; fi':readonly"
    "etc-dir-readonly:bash -c 'if touch /etc/.smoke-test 2>/dev/null; then rm -f /etc/.smoke-test; echo writable; else echo readonly; fi':readonly"
    "podman:command -v podman:podman"
    "cosmic-session:command -v cosmic-session:cosmic-session"
    "cosmic-greeter:command -v cosmic-greeter:cosmic-greeter"
)

failures=""
for entry in "${checks[@]}"; do
    IFS=':' read -r label cmd expect <<< "${entry}"
    outpath="${DIAG_DIR}/${label}.txt"
    echo "Running check: ${label}"
    if ! run_ssh "${cmd}" > "${outpath}" 2>&1; then
        echo "FAIL ${label} (command exited non-zero)"
        failures="${failures},${label}"
        continue
    fi
    if [[ -n "${expect}" ]] && ! grep -q "${expect}" "${outpath}"; then
        echo "FAIL ${label} (expected '${expect}')"
        failures="${failures},${label}"
    else
        echo "PASS ${label}"
    fi
done

# Collect full diagnostic bundle.
run_ssh "dmesg 2>/dev/null | head -n 200" > "${DIAG_DIR}/dmesg.txt" 2>&1 || true
run_ssh "journalctl -b --no-pager | head -n 500" > "${DIAG_DIR}/journal.txt" 2>&1 || true
run_ssh "systemctl status --no-pager -l" > "${DIAG_DIR}/services.txt" 2>&1 || true

if [[ -n "${failures}" ]]; then
    echo "FAILED_CHECKS=${failures#,}"
    log_status "failed" "checks failed: ${failures#,}"
    exit 1
fi

echo ""
echo "Stage 8 post-install VM test passed."
echo "Diagnostics collected in ${DIAG_DIR}"
log_status "complete" "post-install VM test passed"
exit 0
