# Regicide Rust Overlay Knowledge Base

**Scope**: `overlays/regicide-rust/`

## OVERVIEW
Gentoo Portage overlay that packages RegicideOS tools and Rust toolchain preferences for the immutable OS image.

## STRUCTURE
```
overlays/regicide-rust/
├── metadata/layout.conf        # Overlay name, masters, EAPI, priority
├── profiles/                     # Overlay profiles and package masks
├── dev-lang/rust/                # Rust toolchain ebuilds
├── dev-rust/                     # Rust library ebuilds
├── regicide-tools/
│   ├── btrmind/                  # BtrMind AI agent ebuild
│   └── regicide-installer/       # OS installer ebuild
├── sys-apps/rust-utils/          # Coreutils-replacement utilities
├── sci-libs/candle-rs/           # ML inference library
├── Dockerfile.test               # Gentoo stage3 test container
├── test-overlay.sh               # Local overlay integrity tests
└── test-in-docker.sh             # Containerized overlay tests
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add a RegicideOS tool package | `regicide-tools/<pkg>/` | Ebuild + metadata |
| Update Rust toolchain | `dev-lang/rust/` | Bumped alongside `profiles/` |
| Change overlay precedence | `metadata/layout.conf` | `priority = 10`; gentoo master |
| Add USE flags for AI tools | `regicide-tools/btrmind/` | `btrmind`, `systemd` |
| Test overlay locally | `test-overlay.sh` | `repoman scan`, cargo check for Rust packages |
| Test in Gentoo container | `test-in-docker.sh` | Uses `Dockerfile.test` |

## CONVENTIONS
- Repo name: `regicide-rust`; master: `gentoo`.
- `thin-manifests = true`, `sign-manifests = false`, `use-manifests = strict`.
- EAPIs supported: `7 8`.
- Priority 10 (lower than GURU 20, higher than gentoo 50 in typical configs).

## ANTI-PATTERNS
- **Do not commit generated manifests without `ebuild ... manifest`**: strict manifest mode requires correct checksums.
- **Do not break the `gentoo` master dependency**: the overlay inherits from the main tree.
- **Do not add ebuilds that cannot `repoman scan` cleanly**: QA gate for overlay integrity.
- **Do not use live/master ebuilds for release images**: pin versions for reproducible OS builds.

## COMMANDS
```bash
# Local overlay QA
./test-overlay.sh

# Test in container
docker build -f Dockerfile.test -t regicide-overlay-test .
./test-in-docker.sh

# Refresh manifest for a package
cd regicide-tools/btrmind
ebuild btrmind-*.ebuild manifest

# Check overlay integrity
repoman scan
```
