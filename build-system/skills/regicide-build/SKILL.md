# RegicideOS Build Skill

## When to use

Use this skill for any RegicideOS build task: running the Dagger pipeline, checking build status, reading build logs, listing artifacts, or launching the resulting VM image.

## Build pipeline

The canonical build command streams plain text progress (much easier to read for agents than the Dagger TUI):

```bash
DAGGER_PROGRESS=plain dagger run python build-system/dagger_pipeline.py --plain
```

Outputs (on success):
- `regicide-cosmic.img` — live SquashFS image
- `build-system/catalyst/output/stage4-amd64-systemd-cosmic.tar.xz` — stage4 rootfs tarball
- `build-system/catalyst/output/build-status.jsonl` — per-stage progress log

## Observing progress

The build is long-running. Prefer the MCP server over polling the Dagger spinner:

```bash
python build-system/mcp-server.py
```

Use the MCP resources to query state:
- `regicide://build/status` — latest stage and completed stages
- `regicide://build/log` — full `build-status.jsonl`
- `regicide://build/artifacts` — available output files

If the MCP server is unavailable, read `build-system/catalyst/output/build-status.jsonl` directly. Each line is JSON with `time`, `stage`, `event`, and `detail` fields.

## Build artifacts

After a successful build, create a bootable QCOW2:

```bash
# Unencrypted
sudo ./build-system/catalyst/build-vm-image.sh \
    --squashfs regicide-cosmic.img \
    build-system/catalyst/output/stage4-amd64-systemd-cosmic.tar.xz \
    build-system/catalyst/output/regicide-cosmic.qcow2 20G

# Encrypted
printf 'regicide-secure-test' > /tmp/regicide-passphrase.txt
sudo ./build-system/catalyst/build-vm-image.sh \
    --encrypt --passphrase-file /tmp/regicide-passphrase.txt \
    --squashfs regicide-cosmic.img \
    build-system/catalyst/output/stage4-amd64-systemd-cosmic.tar.xz \
    build-system/catalyst/output/regicide-cosmic-enc.qcow2 20G
```

## Launching the VM with a GUI

```bash
cp /usr/share/OVMF/OVMF_VARS.fd /tmp/ovmf-gui.fd
qemu-system-x86_64 \
    -enable-kvm -m 8G -smp 4 -cpu host -machine type=q35,accel=kvm \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd \
    -drive if=pflash,format=raw,file=/tmp/ovmf-gui.fd \
    -vga qxl -spice port=5920,disable-ticketing=on \
    -serial file:/tmp/regicide-gui-serial.log \
    -hda build-system/catalyst/output/regicide-cosmic.qcow2
remote-viewer spice://localhost:5920
```

## Troubleshooting

- If a stage fails, grep the Dagger plain output or `build-status.jsonl` for `"event":"error"`.
- If the COSMIC stage fails quickly, check that `cosmic-overlay` is registered in the rootfs repos.conf and that `ACCEPT_KEYWORDS="~amd64"` is set.
- If `/tmp` fills, move temporary files to `/var/tmp`.
