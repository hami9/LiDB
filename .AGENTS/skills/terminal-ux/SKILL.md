---
name: lidb-terminal-ux
description: Terminal UI, SSH accessibility, CLI/headless output and diagnostics.
---
# Terminal UX skill

**Use when:** adding TUI panels, shortcuts, rendering, filtering, CLI help or export.

- Keep an accessible keyboard-first interface with monochrome/no-color and narrow-window fallbacks.
- No invented demo metrics in normal mode. Fixture/demo mode must be clearly labeled in every view and export.
- Show freshness, units, data source and support status. Never hide unavailable data behind zeros.
- Separate normal warnings from destructive operations; show exact user approval requirements for active diagnostics.
- Prefer concise explanation + drill-down to evidence. Navigation and help must be discoverable over SSH.
- Test Unicode/wide glyphs, terminal resize, disconnection, high-frequency updates and non-TTY mode.
