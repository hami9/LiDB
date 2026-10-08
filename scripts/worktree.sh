#!/usr/bin/env bash
set -euo pipefail

# Thin cross-platform wrapper: all safety checks live in worktrees.py.
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if command -v python3 >/dev/null 2>&1; then
  exec python3 "$SCRIPT_DIR/worktrees.py" "$@"
fi
if command -v python >/dev/null 2>&1; then
  exec python "$SCRIPT_DIR/worktrees.py" "$@"
fi
echo "Python 3 is required." >&2
exit 2
