# Commit Squashing Plan

## Summary
- **Total commits**: 310
- **Fix commits**: 149 (48%)
- **Merge commits**: 16
- **Goal**: Squash fix-commits into their parent feature commits where logical

## Identified Squash Groups

### Group 1: PortCL Compiler Fixes (7 commits)
```
257e10f chore: fix clippy errors in portcl tests
42fdf86 fix: resolve unused import and var in portcl safety
1b9010d fix: resolve unused var warnings in portcl service
7b3abf6 fix: resolve unused var/field warnings in portcl rl_engine
c867d78 fix: resolve unused var warnings in portcl monitor
af9dbb5 chore: fix unused imports with cargo fix
75f3b39 refactor: reduce clippy suppressions from 12 to 9
```
**Target squash**: Combine into single "chore: fix PortCL compiler warnings and clippy errors"

### Group 2: Installer Quality Improvements (4 commits)
```
17b2bd5 feat: improve code quality score +17.5 pts (71.3 → 88.8)
af9dbb5 chore: fix unused imports with cargo fix
7a0aa58 refactor: improve installer code quality and clean project
f63fe53 fix: resolve 13 correctness bugs in installer
```
**Target squash**: Combine into single "refactor: improve installer code quality and fix correctness bugs"

### Group 3: Bind Mount & RO Filesystem Fixes (6 commits)
```
449e497 fix: create mountpoint directories before bind mounts
0891bb7 fix: complete bind mount bypass to resolve exit code 32
33512a1 feat: replace squashfs with native ROOTS mount and overlay template
31f5c8b fix: resolve RO filesystem and bind mount timing issues
93726c3 fix: move GRUB config creation to post-install after overlay mount
e5a9b85 fix: add complete bind mounts and resolve RO filesystem issues
```
**Target squash**: Combine into single "fix: resolve bind mount and RO filesystem issues for overlay root"

### Group 4: GRUB & LUKS Boot Fixes (3 commits)
```
675fe30 fix: embed GRUB crypto modules to resolve LUKS boot failures
aaadfdc feat: enhance GRUB implementation with System.map detection and improved configuration
```
**Target squash**: Combine into single "feat: enhance GRUB with LUKS crypto modules and System.map detection"

### Group 5: Recent RegicideOS Fixes (by me — keep separate or squash)
These are already clean, atomic commits:
```
243618f feat: add COSMIC stage4 Catalyst spec and practical build pipeline
11d4583 fix: use correct repository URL (repo.xenialinux.org)
850458f fix: handle unreachable repository gracefully in interactive mode
40373c8 chore: add macOS resource forks to .gitignore and clean repo
ebe70c6 fix: installer repo check and add dependency install script
23c6953 fix: remove macOS binary and update Handbook instructions
```
**Recommendation**: Keep as-is — each is a distinct logical change.

### Group 6: BtrMind Fix (1 commit)
```
345d532 fix: resolve 5 compiler warnings in btrmind
```
**Recommendation**: Keep as-is or squash into nearest btrmind feature commit.

## Remote Branches to Consider
- `origin/004-portcl-test-suite`
- `origin/feature/mle-fragmentation-estimation`
- `origin/feature/portcl-implementation`

These branches may have commits that should be merged/squashed before cleaning main history.

## Execution Strategy

Since this is a public repo with existing clones, rewriting history requires coordination:

1. **Option A: Soft squash** (recommended)
   - Use `git merge --squash` for future feature branches
   - Keep main history as-is
   - Clean going forward

2. **Option B: Hard rebase** (destructive)
   - `git rebase -i HEAD~N` to squash commits
   - Requires force push
   - Breaks all existing clones

3. **Option C: Archive + fresh start**
   - Create archive branch of current history
   - Start clean main with squashed history
   - Preserves full history in archive branch

**Recommendation**: Option C — archive current main as `archive/pre-squash`, then create clean history.
