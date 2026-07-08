# Catalyst Build System Knowledge Base

**Scope**: `build-system/catalyst/`

## OVERVIEW
Gentoo Catalyst stage4 builder for RegicideOS: produces a stage4 tarball and a live SquashFS image, plus bootable QCOW2 VM images.

## STRUCTURE
```
build-system/catalyst/
├── build.sh                     # Root-required Catalyst wrapper
├── stage4-systemd-cosmic.spec   # Stage4 package/profile spec
├── stage4-systemd-cosmic.sh     # Post-build chroot configuration
├── build-vm-image.sh            # Bootable QCOW2 builder (KVM-based)
├── build-qemu-image.sh          # In-VM disk image installer
├── vm-builder.sh                # Script run inside the builder VM
├── run-qemu.sh                  # Launch a built QCOW2
├── stages/                      # Dagger cacheable stage scripts
│   ├── common.sh
│   ├── stage1-setup.sh
│   ├── stage2-sync.sh
│   ├── stage3-base.sh
│   ├── stage4-cosmic.sh
│   ├── stage5-regicide.sh
│   └── stage6-finalize.sh
├── overlay/                     # Base Portage overlay config
└── cosmic-overlay/              # Vendored COSMIC desktop overlay
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add/remove stage4 packages | `stage4-systemd-cosmic.spec` | Profile `default/linux/amd64/23.0/desktop/systemd` |
| Change post-build config | `stage4-systemd-cosmic.sh` | Root password `regicide`, service enables, dracut |
| Build the OS image | `build.sh` | Requires root + `dev-util/catalyst` on a Gentoo host |
| Build a QCOW2 VM image | `build-vm-image.sh` | Boots stage4 in KVM to avoid loop-device limits |
| Encrypted ROOTS image | `build-qemu-image.sh` | LUKS2 + custom `99regicide-crypt` dracut module |
| CI/CD cacheable stages | `stages/*.sh` | Consumed by `../dagger_pipeline.py` |
| COSMIC overlay QA | `cosmic-overlay/.github/workflows/qa-check.yml` | Runs `scripts/simple-qa-check.py` |

## CONVENTIONS
- Build artifacts go to `output/`: `stage4-amd64-systemd-cosmic.tar.xz`, `regicide-cosmic.img`, `*.qcow2`.
- Dagger plain-text logs: `DAGGER_PROGRESS=plain` or `--plain`.
- LUKS passphrase file must have no trailing newline; use `printf`.
- SPICE display during VM build/observation uses port `5920`.

## ANTI-PATTERNS
- **Do not run `build.sh` as non-root**: Catalyst requires root and writes to `/var/tmp/catalyst`.
- **Do not use `echo` to write LUKS passphrase files**: `printf` avoids a trailing newline.
- **Do not rely on the aspirational files `../dagger.py` or `../regicide_image_builder.py`**: they are deprecated.
- **Do not ignore the default user and root password policy**: the default user is `regicide` with password `regicide`; root password is intentionally unset. Treat this as a release/security concern when changing post-build scripts.
- **Do not let `/tmp` fill up**: GTK/SPICE tools crash with misleading quota errors when `/tmp` is a small tmpfs.

## COMMANDS
```bash
# Build stage4 + SquashFS (root, Gentoo host)
sudo ./build.sh

# Bootable unencrypted QCOW2
sudo ./build-vm-image.sh \
  --squashfs output/regicide-cosmic.img \
  output/stage4-amd64-systemd-cosmic.tar.xz \
  output/regicide-cosmic.qcow2 20G

# Boot built image
./run-qemu.sh output/regicide-cosmic.qcow2

# Dagger CI/CD (any Docker/Podman host)
DAGGER_PROGRESS=plain dagger run python ../dagger_pipeline.py --plain
```
