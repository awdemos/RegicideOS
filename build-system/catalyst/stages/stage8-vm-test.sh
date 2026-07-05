#!/bin/bash
# Stage 8: post-install VM smoke test.
# Boots a RegicideOS QCOW2, logs in over the serial console, and verifies
# core runtime behavior derived from the v12 live-image findings.
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage8-vm-test"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CATALYST_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUTPUT_DIR="${OUTPUT_DIR:-${CATALYST_DIR}/output}"

DEFAULT_QCOW2="${OUTPUT_DIR}/regicide-cosmic.qcow2"
QCOW2="${1:-${DEFAULT_QCOW2}}"
QCOW2="$(realpath -e "${QCOW2}" 2>/dev/null || true)"
SSH_PORT="${REGICIDE_VM_SSH_PORT:-2222}"
VM_MEMORY="${REGICIDE_VM_MEMORY:-4096}"
VM_SMP="${REGICIDE_VM_SMP:-4}"
TIMEOUT_SEC="${REGICIDE_VM_TIMEOUT:-300}"
DIAG_DIR="${OUTPUT_DIR}/vm-test-diagnostics"
VM_DISPLAY="${REGICIDE_VM_DISPLAY:-none}"

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

KVM_FLAGS=""
if [[ -e /dev/kvm ]] && [[ -r /dev/kvm ]]; then
    KVM_FLAGS="-enable-kvm -cpu host"
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
DISPLAY_ARGS="-display none -vga none"
case "${VM_DISPLAY}" in
    vnc) DISPLAY_ARGS="-display vnc=:0 -vga virtio" ;;
    sdl) DISPLAY_ARGS="-display sdl,gl=on -vga virtio" ;;
esac

echo "Starting QEMU..."
qemu-system-x86_64 \
    ${KVM_FLAGS} \
    -m "${VM_MEMORY}" \
    -smp "${VM_SMP}" \
    -drive file="${QCOW2}",format=qcow2,if=virtio \
    -netdev user,id=net0,hostfwd=tcp::${SSH_PORT}-:22 \
    -device virtio-net-pci,netdev=net0 \
    ${DISPLAY_ARGS} \
    -machine type=q35 \
    -drive if=pflash,format=raw,readonly=on,file="${OVMF_CODE}" \
    -drive if=pflash,format=raw,file="${OVMF_VARS}" \
    -serial unix:"${SERIAL_SOCK}",server,nowait \
    -monitor unix:"${MONITOR_SOCK}",server,nowait \
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

# Embedded Python serial driver: connects to the QEMU serial socket, bridges
# to a PTY, and lets us send commands / read output interactively.
PYTHON_SCRIPT="${WORK_DIR}/vm_serial_driver.py"
cat > "${PYTHON_SCRIPT}" <<'PYEOF'
import os
import pty
import re
import select
import socket
import sys
import termios
import time
import tty

SERIAL_SOCK = sys.argv[1]
TIMEOUT = int(sys.argv[2])
DIAG_DIR = sys.argv[3]

def main():
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    deadline = time.time() + 30
    while True:
        try:
            s.connect(SERIAL_SOCK)
            break
        except (FileNotFoundError, ConnectionRefusedError):
            if time.time() > deadline:
                print("!ERR: serial socket unavailable", flush=True)
                sys.exit(1)
            time.sleep(0.2)
    s.setblocking(False)

    master, slave = pty.openpty()
    old = termios.tcgetattr(master)
    tty.setraw(master)
    try:
        _drive(s, master, slave, TIMEOUT, DIAG_DIR)
    finally:
        termios.tcsetattr(master, termios.TCSADRAIN, old)

def _read_until(s, master, deadline, needles, max_bytes=8192):
    """Read from the serial socket until one of needles appears or deadline hits."""
    buffer = b""
    while time.time() < deadline:
        r, _, _ = select.select([s], [], [], 0.2)
        if s in r:
            try:
                data = s.recv(4096)
            except BlockingIOError:
                data = b""
            if data:
                os.write(master, data)
                buffer += data
                if len(buffer) > max_bytes:
                    buffer = buffer[-max_bytes:]
                for needle in needles:
                    if needle.encode() in buffer:
                        return buffer.decode(errors="replace"), needle
        if time.time() > deadline:
            break
    return buffer.decode(errors="replace"), None

def _send(s, text):
    s.sendall(text.encode())

def _drive(s, master, slave, timeout, diag_dir):
    boot_deadline = time.time() + timeout
    login_deadline = time.time() + 120

    print("SLAVE_TTY=" + os.ttyname(slave), flush=True)

    # Wait for the login prompt (systemd has reached getty).
    _, matched = _read_until(s, master, login_deadline, ["login:", "regicideos login:", "Password:", "~$", "# "])
    if not matched:
        print("!ERR: login prompt not seen within 120s", flush=True)
        sys.exit(1)
    print("LOGIN_PROMPT_OK", flush=True)

    # Login as regicide / regicide.
    _send(s, "regicide\n")
    time.sleep(1)
    output, matched = _read_until(s, master, time.time() + 30, ["Password:", "login:", "$ "])
    if "Password:" in output:
        _send(s, "regicide\n")
        output, matched = _read_until(s, master, time.time() + 30, ["$", "Login incorrect", "login:"])
    if "Login incorrect" in output or matched == "login:":
        print("!ERR: login failed", flush=True)
        sys.exit(1)
    print("LOGIN_OK", flush=True)

    os.makedirs(diag_dir, exist_ok=True)

    # Commands to run inside the VM.  Each entry is (label, command, expect_substring_or_none, collect_file).
    checks = [
        ("whoami", "whoami", "regicide", "whoami.txt"),
        ("uid", "id -u", "1000", "uid.txt"),
        ("groups", "id", "wheel", "groups.txt"),
        ("sudo", "sudo -n whoami", "root", "sudo.txt"),
        ("kernel", "uname -r", None, "kernel.txt"),
        ("cosmic-greeter", "systemctl is-active cosmic-greeter", "active", "cosmic-greeter.txt"),
        ("display-manager", "systemctl is-active display-manager", "active", "display-manager.txt"),
        ("sshd-socket", "systemctl is-active sshd.socket", "active", "sshd-socket.txt"),
        ("failed-units", "systemctl --failed --no-pager", "0 loaded units listed", "failed-units.txt"),
        ("network-interface", "ip link show", "enp0s2", "network-interface.txt"),
        ("loopback-up", "ip link show lo", "<LOOPBACK,UP,LOWER_UP>", "loopback.txt"),
        ("resolv-conf", "cat /etc/resolv.conf", "nameserver", "resolv-conf.txt"),
        ("ssh-listen", "ss -tlnp | grep -E '\\*:22|0.0.0.0:22'", "22", "ssh-listen.txt"),
        ("cosmic-processes", "pgrep -a cosmic-greeter; pgrep -a cosmic-comp; pgrep -a cosmic-session", "cosmic", "cosmic-processes.txt"),
        ("sshd-keygen", "systemctl is-active sshd-keygen@ed25519.service || systemctl is-active sshd-keygen@rsa.service || ls /etc/ssh/ssh_host_ed25519_key /etc/ssh/ssh_host_rsa_key 2>/dev/null", "ssh_host", "sshd-keygen.txt"),
        ("btrfs", "command -v btrfs", "btrfs", "btrfs.txt"),
        ("flatpak", "command -v flatpak", "flatpak", "flatpak.txt"),
        ("distrobox", "command -v distrobox", "distrobox", "distrobox.txt"),
        ("sudoers-dropin", "sudo cat /etc/sudoers.d/10-regicide-wheel", "%wheel", "sudoers-dropin.txt"),
        ("root-password", "sudo awk -F: '/^root:/ {print \$2}' /etc/shadow", "", "root-password.txt"),
    ]

    results = []
    for label, cmd, expect, outfile in checks:
        _send(s, cmd + "\n")
        output, _ = _read_until(s, master, time.time() + 20, ["$ "])
        path = os.path.join(diag_dir, outfile)
        with open(path, "w") as f:
            f.write(output)
        ok = True
        detail = ""
        if expect is not None:
            if expect not in output:
                ok = False
                detail = f"expected '{expect}'"
        status = "PASS" if ok else "FAIL"
        print(f"{status} {label}" + (f" ({detail})" if detail else ""), flush=True)
        results.append((label, status, detail))

    # Collect full diagnostic bundle.
    _send(s, "dmesg 2>/dev/null | head -n 200 > /tmp/dmesg.txt || true\n")
    _read_until(s, master, time.time() + 10, ["$ "])
    _send(s, "journalctl -b --no-pager | head -n 500 > /tmp/journal.txt || true\n")
    _read_until(s, master, time.time() + 10, ["$ "])
    _send(s, "systemctl status --no-pager -l > /tmp/services.txt || true\n")
    _read_until(s, master, time.time() + 10, ["$ "])
    _send(s, "cat /tmp/dmesg.txt\n")
    dmesg_output, _ = _read_until(s, master, time.time() + 20, ["$ "])
    with open(os.path.join(diag_dir, "dmesg.txt"), "w") as f:
        f.write(dmesg_output)
    _send(s, "cat /tmp/journal.txt\n")
    journal_output, _ = _read_until(s, master, time.time() + 20, ["$ "])
    with open(os.path.join(diag_dir, "journal.txt"), "w") as f:
        f.write(journal_output)
    _send(s, "cat /tmp/services.txt\n")
    services_output, _ = _read_until(s, master, time.time() + 20, ["$ "])
    with open(os.path.join(diag_dir, "services.txt"), "w") as f:
        f.write(services_output)

    failures = [label for label, status, _ in results if status == "FAIL"]
    if failures:
        print("FAILED_CHECKS=" + ",".join(failures), flush=True)
        sys.exit(1)
    print("ALL_CHECKS_PASS", flush=True)

if __name__ == "__main__":
    main()
PYEOF

echo "Waiting for VM to reach login prompt (up to ${TIMEOUT_SEC}s)..."
if ! python3 "${PYTHON_SCRIPT}" "${SERIAL_SOCK}" "${TIMEOUT_SEC}" "${DIAG_DIR}"; then
    echo "ERROR: VM runtime checks failed.  Diagnostics in ${DIAG_DIR}"
    exit 1
fi

echo ""
echo "Stage 8 post-install VM test passed."
echo "Diagnostics collected in ${DIAG_DIR}"
log_status "complete" "post-install VM test passed"
exit 0
