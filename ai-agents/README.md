# AI Agents

This directory contains early-stage AI service experiments for RegicideOS.

## Status

- **btrmind/** — Working prototype. A BTRFS monitoring agent with a small DQN
  reinforcement-learning loop. It builds and its tests pass, but treat it as
  experimental until it is integrated into the OS image.

- **portcl/** — Placeholder only. The README and specs describe a Portage
  continual-learning agent, but the implementation is mostly stubs. Do not expect
  it to optimize Portage builds today. It will be rebuilt or removed once the
  base distribution and package workflow are stable.

## Why keep them?

These agents represent the long-term direction for RegicideOS (autonomous
system maintenance), but they are not part of the current install image and are
not maintained on the same cadence as the build system. They are kept here for
reference and future iteration, not as production features.
