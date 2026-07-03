<div align="center">

# 🖥️ RegicideOS

### AI-Native · Rust-First · Immutable Linux Distribution

> *Converge and conquer.*

> ⚠️ **Development Status**: The installer works, the Dagger/Catalyst build pipeline produces a bootable stage4 + SquashFS + QCOW2, and the COSMIC desktop boots to a greeter. A bootable ISO is not yet automated. See [STATUS.md](STATUS.md) for the full breakdown.

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Linux](https://img.shields.io/badge/Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black)](https://kernel.org/)
[![Btrfs](https://img.shields.io/badge/Btrfs-8db600?style=for-the-badge&logo=linux&logoColor=white)](https://btrfs.wiki.kernel.org/)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg?style=for-the-badge)](https://www.gnu.org/licenses/gpl-3.0)

**A forward-looking Linux distribution with a clear mission:** every component that can be implemented in Rust will be migrated to Rust, and AI capabilities are integrated at the system level.

RegicideOS is built from a Gentoo stage4 with COSMIC as the default desktop. The root filesystem is a read-only Btrfs volume; stateful data lives on separate subvolumes. Updates are prepared offline, verified, and swapped atomically. The live image is a SquashFS produced directly by the Catalyst/Dagger pipeline, so what boots is byte-for-byte what was validated.

[📥 Install](#installation) · [🏗️ Architecture](#architecture) · [🗺️ Roadmap](#roadmap) · [🤝 Contributing](#contributing)

</div>

---

## 🎯 Why RegicideOS?

> *"Regicide" refers to the "kings" of the current operating system marketplace: unsafe programming languages, bloated legacy stacks, and human-centric system administration.*

**The commits will keep coming until every single Red Hat Enterprise customer cancels their subscription.**

### Core Principles

- 🦀 **System-wide Rust adoption** — Replace C/C++ system components with memory-safe Rust binaries
- 🛡️ **Memory safety by default** — Eliminate entire classes of vulnerabilities through Rust's ownership model
- ⚡ **Zero-cost performance** — Leverage Rust's abstractions without runtime overhead
- 🤖 **AI-native from day one** — RL-driven optimization, predictive maintenance, and intelligent resource allocation
- 🔒 **Immutable foundation** — Read-only Btrfs root with atomic updates and instant rollback
- 🚀 **Aggressive Dagger caching** — Incremental builds reuse cached stages so subsequent builds take 99% less time

---

## 🏗️ Architecture

| Component | Technology | Purpose | Status |
|-----------|------------|---------|--------|
| Kernel | Linux (→ [Asterinas](https://asterinas.github.io/)) | System foundation | ✅ Working |
| Init System | systemd | Service management | ✅ Working |
| Filesystem | Btrfs (read-only) | Immutable system image with overlay writes | ✅ Working |
| Container Runtime | Distrobox | Application isolation and compatibility | 📋 Planned |
| Desktop Environment | [Cosmic Desktop](https://github.com/pop-os/cosmic-epoch) | GPU-accelerated, Wayland-native UI | ✅ Installed |
| Package Management | Portage + custom overlays | Gentoo source-based + curated bundles | ✅ Working |
| AI Agent | BtrMind | BTRFS optimization and cleanup | ✅ Working |
| AI Agent | (future) | Portage optimization | 📋 Planned |

### Directory Layout

```
/
├── boot/efi          # EFI System Partition
├── root/             # Native ROOTS partition (read-only base system)
│   ├── usr/          # System binaries
│   ├── etc/          # Base configuration
│   └── var/          # Variable data templates
├── home/             # User data (separate Btrfs subvolume)
└── overlay/          # Writable overlays
    ├── etc/          # Configuration overlay
    ├── var/          # Variable data overlay
    └── usr/          # User software overlay
```

### Key Design Decisions

- **Read-only root** — System files protected from accidental or malicious modification
- **Atomic updates** — Safe, transactional system updates via Btrfs snapshots
- **Rollback capability** — Boot into any previous system state instantly
- **Distrobox integration** — Seamless containerized application environment with full distro compatibility

---

## 📥 Installation

> **Note**: The installer exists and works for basic UEFI installs. There is currently **no bootable ISO**, but the build system now produces a local SquashFS image and a bootable QCOW2 VM image. You can install to bare metal from the live SquashFS image, or boot the QCOW2 image directly in a VM.

### Requirements

- 64-bit x86 processor
- 12GB disk space minimum (20GB recommended)
- UEFI firmware
- Internet connection
- Existing Linux live environment (e.g., Fedora Workstation) for bare-metal installs

### Build from Source

The recommended way to build RegicideOS is with the Catalyst-based build scripts in `build-system/catalyst/`.

#### Dependencies

You need a Gentoo host or container with Portage, plus Catalyst and the VM builder tools:

```bash
emerge -av dev-util/catalyst app-arch/pixz sys-fs/squashfs-tools
emerge -av app-emulation/qemu sys-fs/cryptsetup
```

#### 1. Build the Catalyst stage4 + SquashFS

**Option A: Catalyst (requires a Gentoo host)**

```bash
cd build-system/catalyst
sudo ./build.sh
```

**Option B: Dagger (works in any container with Docker/Podman)**

```bash
DAGGER_PROGRESS=plain dagger run python build-system/dagger_pipeline.py --plain
```

The Dagger pipeline splits the build into six cacheable stages in `build-system/catalyst/stages/`. The COSMIC stage compiles many Rust packages from source, so the first run can take several hours; subsequent runs reuse the `distfiles` and `binpkgs` cache volumes and can take 99% less time. Use `--plain` (or set `DAGGER_PROGRESS=plain`) to stream plain text logs instead of the interactive TUI.

Both methods produce:
- `build-system/catalyst/output/stage4-amd64-systemd-cosmic.tar.xz`
- `build-system/catalyst/output/regicide-cosmic.img` (live SquashFS image)

#### 2. Build a bootable QCOW2 VM image

```bash
# Unencrypted
sudo ./build-vm-image.sh \
    --squashfs output/regicide-cosmic.img \
    output/stage4-amd64-systemd-cosmic.tar.xz \
    output/regicide-cosmic.qcow2 20G

# Encrypted with LUKS2 (recommended)
printf 'regicide-secure-test' > /tmp/regicide-passphrase.txt
sudo ./build-vm-image.sh \
    --encrypt --passphrase-file /tmp/regicide-passphrase.txt \
    --squashfs output/regicide-cosmic.img \
    output/stage4-amd64-systemd-cosmic.tar.xz \
    output/regicide-cosmic-enc.qcow2 20G
```

> **Current status**: both unencrypted and encrypted images build successfully, boot to a serial-console login prompt, and reach the COSMIC greeter. The default user is `regicide` with password `regicide`; the root password is intentionally unset, so manage root via `sudo passwd root` after login. The LUKS passphrase for the encrypted example is `regicide-secure-test`.

#### 3. Install to bare metal

Boot an existing Linux live environment, clone the repo, build the installer, and point it at the local SquashFS image:

```bash
cd installer
cargo build --release
sudo ./target/release/installer \
    --image ../build-system/catalyst/output/regicide-cosmic.img \
    /dev/sdX
```

### AI Agent Installation Procedure

This section gives deterministic, observable steps an AI agent can follow to build and install RegicideOS. Each step includes a success criterion you can verify automatically. All paths assume the repo was cloned into the current working directory.

> **Agent prerequisites**: a Gentoo host or container with Portage, root access, `/dev/kvm`, and the tools installed with `emerge -av dev-util/catalyst app-arch/pixz sys-fs/squashfs-tools app-emulation/qemu sys-fs/cryptsetup`.

#### A. Build the Catalyst stage4 + SquashFS from source

1. Enter the Catalyst build directory and run the stage4/SquashFS builder.

   ```bash
   cd "$(pwd)/build-system/catalyst"
   sudo ./build.sh
   ```

2. Verify the stage4 tarball exists.

   ```bash
   test -f "$(pwd)/output/stage4-amd64-systemd-cosmic.tar.xz"
   ```

3. Verify the live SquashFS image exists and is non-empty.

   ```bash
   test -s "$(pwd)/output/regicide-cosmic.img"
   ```

4. Success criterion: both commands return exit code `0`.

#### B. Build an encrypted bootable QCOW2 VM image from the stage4/SquashFS

1. Create a passphrase file **without a trailing newline**. The example passphrase below is `regicide-secure-test`.

   ```bash
   printf 'regicide-secure-test' > /tmp/regicide-passphrase.txt
   ```

   > **Important**: a trailing newline becomes part of the LUKS key when `cryptsetup` reads the file, so typing the passphrase interactively later will fail. Use `printf` (not `echo`) to avoid a newline.

2. Build the encrypted QCOW2 image from the stage4 tarball and the SquashFS image produced in step A.

   ```bash
   cd "$(pwd)/build-system/catalyst"
   sudo env REGICIDE_LUKS_PASSPHRASE="$(cat /tmp/regicide-passphrase.txt)" \
       ./build-vm-image.sh \
       --encrypt \
       --passphrase-file /tmp/regicide-passphrase.txt \
       --squashfs "$(pwd)/output/regicide-cosmic.img" \
       "$(pwd)/output/stage4-amd64-systemd-cosmic.tar.xz" \
       "$(pwd)/output/regicide-cosmic.qcow2" 20G
   ```

3. Verify the QCOW2 image exists.

   ```bash
   test -s "$(pwd)/output/regicide-cosmic.qcow2"
   ```

4. Verify the image has a valid partition table.

   ```bash
   sudo parted -s "$(pwd)/output/regicide-cosmic.qcow2" print > /dev/null 2>&1
   ```

5. Success criterion: the build prints `RegicideOS QEMU image build complete!`, exit code is `0`, and the QCOW2 file exists and has a partition table.

> **LUKS passphrase**: if you use the example encrypted build, the passphrase is `regicide-secure-test`.

#### D. Boot the generated QCOW2 image in QEMU

1. Ensure no other QEMU process is holding the image, then copy `OVMF_VARS.fd` so the UEFI firmware can create a boot entry.

   ```bash
   pkill -9 -f 'qemu-system-x86_64' 2>/dev/null || true
   cp /usr/share/OVMF/OVMF_VARS.fd /tmp/ovmf-vars.fd
   ```

2. Boot the image with UEFI and a serial console.

   ```bash
   qemu-system-x86_64 \
       -enable-kvm \
       -m 8G \
       -smp 4 \
       -cpu host \
       -machine type=q35,accel=kvm \
       -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd \
       -drive if=pflash,format=raw,file=/tmp/ovmf-vars.fd \
       -nographic \
       -hda /tmp/regicide-cosmic.qcow2
   ```

3. For encrypted images, enter the LUKS passphrase `regicide-secure-test` when prompted.

4. Success criterion: the VM reaches the COSMIC greeter or serial login prompt. Log in as `regicide` / `regicide` and run `systemctl status systemd-logind` to confirm it is `active (running)`.

#### E. Observe the VM in a GUI window

If you are on a host with a display server, launch the VM with a SPICE console instead of `-nographic`:

```bash
cp /usr/share/OVMF/OVMF_VARS.fd /tmp/ovmf-gui.fd
qemu-system-x86_64 \
    -enable-kvm -m 8G -smp 4 -cpu host -machine type=q35,accel=kvm \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd \
    -drive if=pflash,format=raw,file=/tmp/ovmf-gui.fd \
    -vga qxl -spice port=5920,disable-ticketing=on \
    -serial file:/tmp/regicide-gui-serial.log \
    -hda /tmp/regicide-cosmic.qcow2
```

Then attach a SPICE viewer:

```bash
remote-viewer spice://localhost:5920
```

For encrypted images, enter the LUKS passphrase `regicide-secure-test` in the VM window when prompted.

#### F. Install to bare metal from the live SquashFS image

1. Boot the target machine from an existing Linux live environment, clone the repo, and build the installer.

   ```bash
   cd "$(pwd)/installer"
   cargo build --release
   ```

2. Verify the installer binary exists.

   ```bash
   test -x "$(pwd)/target/release/installer"
   ```

3. Identify the target disk and run the installer against the local SquashFS image. Replace `/dev/sdX` with the actual target device.

   ```bash
   sudo "$(pwd)/target/release/installer" \
       --image "$(pwd)/../build-system/catalyst/output/regicide-cosmic.img" \
       /dev/sdX
   ```

4. Verify the installer exited cleanly.

   ```bash
   echo $?
   ```

5. Success criterion: the installer exits `0` and the target disk has a partition table with a `ROOTS` partition.

   ```bash
   sudo parted -s /dev/sdX print | grep -q ROOTS
   ```

#### Troubleshooting notes for AI-agent builds

- **`/tmp` may be a tmpfs and can fill up**. The Catalyst wrapper stages builds under `/var/tmp/catalyst`, but monitor `/tmp` if you write other files there:

  ```bash
  df -h /tmp
  ```

  If `/tmp` is near capacity, move temporary files to `/var/tmp` instead. GTK applications may crash with "Disk quota exceeded" when `/tmp` is full, even though your home directory has space.

- **`remote-viewer`/GTK apps may fail under quota or in a headless session**. The VM builder exposes a SPICE display on port `5920` by default. If you need a display, attach to it:

  ```bash
  remote-viewer spice://localhost:5920
  ```

  If GTK cannot open a display, boot the generated image manually with `-nographic` using the command in step D above and inspect the serial console output.

- **LUKS passphrase reminder**: the example encrypted build uses passphrase `regicide-secure-test`. Keep this value available for unlocking or decrypting the image later.

### Automated Configuration

```bash
cat > regicide-config.toml << 'EOF'
drive = "/dev/sda"
image_path = "build-system/catalyst/output/regicide-cosmic.img"
filesystem = "btrfs"
username = "your-username"
applications = "minimal"
EOF

sudo ./target/release/installer -c regicide-config.toml
```

---

## 🤖 AI Integration Roadmap

| Phase | Feature | Status |
|-------|---------|--------|
| Now | BtrMind local AI assistant | ✅ Working (17/17 tests pass) |
| 2026 | Package optimization AI | 📋 Planned |
| 2026 | Predictive system maintenance | 📋 Planned |
| 2026 | Intelligent resource allocation | 📋 Planned |
| 2027 | Natural language system control | 📋 Planned |
| 2027 | Asterinas kernel migration | 📋 Planned |

---

## 🗺️ Roadmap

- [x] Core installer with Btrfs validation (basic, needs refactor)
- [x] Read-only root filesystem (conceptual, not bootable)
- [x] Rust rewrite of installer (functional but monolithic)
- [x] BtrMind AI agent (fully working)
- [x] Bootable QCOW2 VM image
- [ ] Bootable ISO / Base system image
- [x] Cosmic Desktop integration
- [ ] Rust replacements of core GNU utilities
- [ ] Memory-safe package manager
- [ ] Advanced AI capabilities (predictive maintenance, NL control)
- [ ] Asterinas kernel integration

---

## 🤝 Contributing

We particularly need help with:

- 🦀 **Rust development** — Rewriting system components in Rust
- 🤖 **AI integration** — Implementing intelligent system features
- 📦 **Overlay creation** — Developing useful package collections
- 📝 **Documentation** — Improving guides and references
- 🧪 **Testing** — Bug reports and verification

**Found a bug?** Please [file an Issue](https://github.com/RegicideOS/RegicideOS/issues) with detailed logs, the command you ran, and your environment so we can reproduce it.

---

## 📜 License

RegicideOS is licensed under the **GNU General Public License v3.0**.

Built on the excellent foundation of Gentoo Linux and the COSMIC Desktop ecosystem.

---

<div align="center">

**© 2026 Andrew White · RegicideOS Project**

</div>
