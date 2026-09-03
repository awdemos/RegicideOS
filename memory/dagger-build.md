# Dagger build — working invocation

**Verified working:** 2026-08-26

Rootless Podman is not supported by the Dagger engine (the engine must create
the `dagger0` bridge, which requires root). The pipeline fails fast with an
explicit error from `_check_container_runtime()` in
`build-system/dagger_pipeline.py` when it detects rootless Podman.

What works:

```bash
sudo systemctl enable --now podman.socket
DOCKER_HOST=unix:///run/podman/podman.sock sudo -E \
    ~/.local/bin/dagger run .venv/bin/python build-system/dagger_pipeline.py --plain
```

Notes:

- Use the full path `~/.local/bin/dagger` — `sudo` resets `PATH`, so a bare
  `dagger` is not found when it is only installed for the user.
- Run the pipeline with `.venv/bin/python`, not bare `python` — under `sudo`
  the system interpreter is used, which lacks the `dagger-io` package
  (`ModuleNotFoundError: No module named 'dagger'`). The repo `.venv` has it.
- The `regicide-binpkgs-v5` binpkg cache lives in the Dagger engine's
  `/var/lib/dagger` volume. If the engine container must be recreated, reuse
  its existing volume or the whole Gentoo binpkg cache is lost.
- If the engine container wedges (podman reports it "Up" but crun says it is
  not running, e.g. after suspend/resume), remove and recreate the container
  with the same `/var/lib/dagger` volume; a plain `podman restart` can fail
  with stale netavark/nftables state.
