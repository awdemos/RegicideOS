# RegicideOS Root Update System Plan

> **Status**: Planning  
> **Priority**: First feature after MVP (bootable COSMIC build + VM image)  
> **Owner**: TBD  

## Goal

Give users a safe, observable way to update the immutable ROOTS base image after installation, without rebuilding their Dagger pipeline or reinstalling from scratch.

## Why this is the first priority after MVP

Once the COSMIC-enabled QCOW2 boots, the very next question users will ask is: *"How do I change my system?*" That includes:

- Installing packages that persist.
- Applying security updates.
- Switching to a newer base image.
- Rolling back if an update breaks something.

RegicideOS currently has no update CLI. Users must manually mount the ROOTS partition and copy a new SquashFS image. That is acceptable for developers, but it is not a real distribution workflow.

## Core design principles

1. **Do not fork Xenia tools** (`foxmerge`, `foxsnapshot`, `foxbox`). They are Python, Xenia-branded, and hardcoded to Xenia paths/package sets. Build a Rust-native tool from scratch.
2. **Support two update models**:
   - **Image replacement** (default): atomically swap the ROOTS SquashFS image.
   - **Live overlay modification** (advanced, discouraged): allow `emerge` on the writable `/usr` overlay with snapshot/rollback.
3. **Let power users customize the Dagger pipeline** to produce their own base images. The update tool must accept any SquashFS built by `dagger run python build-system/dagger_pipeline.py`.
4. **Safety first**: create a btrfs snapshot or back up the previous ROOTS image before replacing it. Offer rollback.
5. **No network repository required for MVP**: the tool operates on local images. Signed remote updates are a later phase.

## User stories

- As a user, I can run `regicide-update --image /path/to/regicide-cosmic.img` so the system replaces the ROOTS image and reboots.
- As a user, I can run `regicide-update --rollback` to boot the previous image after a failed update.
- As a developer, I can add packages to `build-system/catalyst/stages/stage4-cosmic.sh`, rebuild with Dagger, and deploy the new image to my running VM.
- As a user, I can add packages to the live overlay with `regicide-update --live-emerge <atom>` and have the tool snapshot first and roll back on failure.

## Proposed CLI

```text
regicide-update
  status              Show current ROOTS image, overlay state, available snapshots
  apply <image>       Replace ROOTS image with <image>, keep backup
  rollback            Reboot into the previous ROOTS image
  snapshot            Manually snapshot the current overlay state
  live-emerge <atom>  Snapshot, remount /usr rw, emerge atom, verify, or rollback
  verify <image>      Check SquashFS integrity and signature (when signing exists)
```

## Stages

### Stage 1: MVP update tool (1-2 days)

- Rust crate under `regicide-tools/regicide-update/`.
- Read ROOTS partition label or `/etc/regicide/update.conf`.
- Validate that the new image is a valid SquashFS (`file` / `unsquashfs -s`).
- Back up current `root.img` to `root.img.previous`.
- Install new image atomically (copy to temp name, rename).
- Set GRUB to boot newest by timestamp (already the default).
- Optional `--reboot` flag.
- Unit tests for path validation and backup logic.

### Stage 2: Rollback (1 day)

- Track `root.img.previous` and `root.img.rollback`.
- `regicide-update rollback` renames the previous image to `root.img` and touches it to be newest.
- Reboot required; no live rollback because the running root is the SquashFS.

### Stage 3: Live overlay updates (2-3 days)

- Detect whether `/usr` is a mount that can be remounted rw or an overlay/subvolume.
- Use btrfs snapshot of `/overlay/usr` (or the correct subvolume path) before any `emerge`.
- Wrap `emerge -1 <atom>` and capture exit code.
- On failure, delete the modified overlay state and restore the snapshot.
- Record installed atoms in `/etc/portage/sets/regicide` so `emerge` can depclean later.

### Stage 4: Remote / signed updates (future)

- Download from a GitHub release or RegicideOS mirror.
- Verify SHA-256 and GPG signature.
- Delta updates or zsync are future optimizations.

## Integration with the build pipeline

- Dagger pipeline already produces `build-system/catalyst/output/regicide-cosmic.img`.
- The update tool consumes exactly that artifact.
- Advanced users can pass a custom stage4/SquashFS by changing the Dagger pipeline or building their own stage4 tarball.
- No change to the installer required; it already lays down the initial ROOTS image.

## Open questions

1. Should the tool live in the main repo (`regicide-tools/regicide-update`) or a separate repo?
2. What is the exact runtime mount path for the OVERLAY `/usr` layer on a live system? Needs verification after the COSMIC VM boots.
3. Should live-emerge be disabled by default to discourage overlay drift?
4. Do we want A/B image slots (`root-a.img` / `root-b.img`) instead of timestamp-based selection?

## Acceptance criteria

- [ ] `regicide-update apply <image>` replaces ROOTS and leaves a working rollback path.
- [ ] `regicide-update rollback` successfully boots the previous image in a VM test.
- [ ] Handbook section 7 is updated to use the new CLI instead of manual `cp`.
- [ ] No new Python dependencies in the base image.
- [ ] Build passes `cargo test` for the new crate.

## Risks

- **Layout changes**: if the installer changes mount paths, the tool must be updated.
- **GRUB timestamp race**: two images with the same second timestamp could be ambiguous.
- **Overlay corruption**: live `emerge` can leave Portage state inconsistent if interrupted.

## Related docs

- Handbook Section 7 (Post-Installation Updates)
- `build-system/dagger_pipeline.py`
- `installer/src/lib.rs` (filesystem layout)
- `build-system/catalyst/stages/stage6-finalize.sh`
