# BtrMind Agent Knowledge Base

**Scope**: `ai-agents/btrmind/`

## OVERVIEW
AI-powered BTRFS storage monitoring daemon using a small reinforcement-learning loop (Candle DQN) to choose cleanup actions.

## STRUCTURE
```
ai-agents/btrmind/
├── src/
│   ├── main.rs      # CLI, agent loop, commands
│   ├── config.rs    # TOML configuration
│   ├── btrfs.rs     # Filesystem metrics collection
│   ├── learning.rs  # DQN / experience replay
│   └── actions.rs   # Cleanup action executor
├── config/btrmind.toml          # Default runtime config
├── systemd/btrmind.service      # Service unit
├── install.sh                   # Systemd install helper
└── test_btrmind.sh              # Build + smoke tests
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add a CLI command | `src/main.rs` | Extend `Commands` enum and match arm |
| Change learning behavior | `src/learning.rs` | DQN, reward, exploration rate |
| Add a cleanup action | `src/actions.rs` | Add to `Action` enum and executor |
| Change thresholds/defaults | `config/btrmind.toml` | Mirrored in `system-integration/btrmind/config/` |
| Systemd hardening | `systemd/btrmind.service` | Memory cap 512M, CPU 50%, `ProtectSystem=strict` |

## CONVENTIONS
- Crate `btrmind`; binary `btrmind`.
- Async Tokio runtime.
- Config path default: `/etc/btrmind/config.toml`.
- Tracing for logs; `anyhow` for errors.
- Model path default: `/var/lib/btrmind/model.safetensors`.

## ANTI-PATTERNS
- **Do not run destructive actions outside dry-run mode**: `btrmind --dry-run` must be used when testing.
- **Do not leave TODOs without tracking**: existing `TODO` markers in `main.rs` and `btrfs.rs` need issues or resolution.
- **Avoid `#[allow(dead_code)]` without justification**: three current suppressions should be wired, removed, or documented.
- **Do not hardcode credentials or host paths in error output**: use the parent sanitizer patterns.

## COMMANDS
```bash
cargo build --release              # build binary
cargo test                         # unit tests
btrmind --dry-run analyze          # safe analysis smoke test
btrmind --dry-run cleanup          # safe cleanup smoke test
sudo ./install.sh                  # install systemd service
```
