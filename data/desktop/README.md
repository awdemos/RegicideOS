# Welcome to RegicideOS

RegicideOS is an AI-native, Rust-first, immutable Linux distribution built on Gentoo with the COSMIC desktop.

## First steps

- **Default user**: `regicide`
- **Default password**: `regicide`
- **Root password**: unset by default; use `sudo` after login

## Opening the handbook

Double-click this file to open it in the default text editor. You can also read it from a terminal:

```bash
cat ~/Desktop/README.md
```

## Desktop basics

- **Applications**: press the Super key or click the launcher to open apps.
- **Terminal**: search for `cosmic-terminal` or any installed terminal emulator.
- **Files**: use COSMIC Files to browse your home directory and external drives.
- **Settings**: open `COSMIC Settings` to configure display, network, users, and more.

## Updating the system

RegicideOS uses atomic updates with automatic rollback support. Run:

```bash
sudo regicide-update
```

The system will download and verify the update, then stage it for the next boot. If something goes wrong, you can roll back to the previous version.

## BTRFS snapshots and rollback

Your system state lives on BTRFS subvolumes. To create a manual snapshot before making changes:

```bash
sudo btrfs subvolume snapshot -r /etc /etc/manual-backup-$(date +%Y%m%d-%H%M%S)
```

To roll back to the previous update if the current one fails:

```bash
sudo regicide-rollback
```

Then reboot.

## AI assistance with BtrMind

BtrMind monitors BTRFS health and can run cleanup actions. Basic commands:

```bash
# Analyze storage
btrmind analyze

# Run automatic cleanup (dry-run first)
btrmind cleanup --dry-run
btrmind cleanup
```

## Network and services

- **Wi-Fi**: configure in COSMIC Settings or use `nmcli`/`nmtui`.
- **Bluetooth**: managed through COSMIC Settings.
- **SSH**: the `sshd.socket` service is enabled. Host keys are generated automatically on first boot.

## Installing software

RegicideOS is based on Gentoo. You can install software with Portage:

```bash
sudo emerge -av <package>
```

For sandboxed applications, Distrobox and Flatpak support are planned.

## Getting help

- Project repository: <https://github.com/RegicideOS/RegicideOS>
- Documentation: see `/usr/share/doc/regicideos/` and the project wiki
- Report issues: <https://github.com/RegicideOS/RegicideOS/issues>

## License

RegicideOS is licensed under the GNU General Public License v3.0.

---

*Converge and conquer.*
