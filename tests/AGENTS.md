# Test Suite Knowledge Base

**Scope**: `tests/`

## OVERVIEW
Cross-component Python pytest suites for the installer, BtrMind agent, and ISO build pipeline. Tests are split by component and risk level.

## STRUCTURE
```
tests/
├── installer/
│   ├── unit/                 # Config, UEFI, disk, filesystem logic
│   ├── integration/          # Workflow / error-handling mocks
│   ├── safety/               # Destructive-operation guards
│   └── test_rust_cli.py      # Rust CLI smoke tests
├── btrmind/
│   ├── unit/                 # Core agent logic
│   ├── integration/          # Integration behavior
│   └── safety/               # Safety boundaries
├── iso/
│   ├── unit/                 # ISO build/config validation
│   ├── integration/          # ISO integration
│   ├── safety/               # ISO safety
│   └── validation/           # Validation gates
├── run-installer-tests.sh    # Runner for installer suite
├── run-iso-tests.sh          # Runner for ISO suite
└── test-btrmind-integration.sh # Root/BTRFS integration runner
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add installer unit test | `tests/installer/unit/` | Mirrors `installer/src/lib.rs` public API |
| Add destructive-op safety check | `tests/installer/safety/` | Mock all disk writes; never touch real block devices |
| Add ISO build test | `tests/iso/unit/` | Test spec/config parsing, not actual image builds |
| Add BtrMind safety check | `tests/btrmind/safety/` | Focus on dry-run and action allowlisting |
| Run all installer tests | `tests/run-installer-tests.sh` | Wraps `pytest tests/installer/` |

## CONVENTIONS
- Python pytest for all suites.
- Tests are organized by component, then by level: `unit/`, `integration/`, `safety/`, `validation/`.
- Destructive operations are mocked with `unittest.mock`, fake disk images, and temporary files.
- Safety tests have highest priority and must pass before production use.

## ANTI-PATTERNS
- **Never run tests against real block devices**: all disk/partition tests must use mocks or loop-file fakes.
- **Never disable a failing safety test**: fix the guard or the test; do not skip.
- **Do not add tests that require root unless they are in `*integration*` runners**: keep unit tests hermetic.
- **Do not assert on exact binary paths that vary by host**: validate behavior, not environment specifics.

## COMMANDS
```bash
python -m pytest tests/installer/       # installer suite
python -m pytest tests/btrmind/         # btrmind suite
python -m pytest tests/iso/               # ISO suite
./tests/run-installer-tests.sh            # runner wrapper
./tests/run-iso-tests.sh                  # runner wrapper
./tests/test-btrmind-integration.sh       # root/BTRFS integration
```
