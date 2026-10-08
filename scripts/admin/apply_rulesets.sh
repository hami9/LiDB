#!/usr/bin/env bash
set -euo pipefail

# Explicit opt-in administrative operation: no token stored in this repository.
# Usage: bash scripts/admin/apply_rulesets.sh --dry-run
#        bash scripts/admin/apply_rulesets.sh --apply
MODE="${1:---dry-run}"
[[ "$MODE" == "--dry-run" || "$MODE" == "--apply" ]] || {
  echo "Usage: $0 [--dry-run|--apply]" >&2
  exit 2
}

REPOSITORY="hami9/LiDB"
RULESET_DIR=".github/rulesets"
[[ -f "$RULESET_DIR/main.json" && -f "$RULESET_DIR/release-tags.json" ]] || {
  echo "Run from the repository root" >&2; exit 2;
}

if [[ "$MODE" == "--dry-run" ]]; then
  echo "DRY RUN: administrative ruleset changes are NOT applied."
  for file in "$RULESET_DIR"/*.json; do
    echo "Proposed payload: $file"
    python3 -m json.tool "$file" >/dev/null
    cat "$file"
  done
  exit 0
fi

command -v gh >/dev/null || { echo "GitHub CLI (gh) is required" >&2; exit 2; }
command -v python3 >/dev/null || { echo "Python 3 is required" >&2; exit 2; }
gh auth status >/dev/null || { echo "Authenticate gh with repository Administration write access" >&2; exit 2; }

for file in "$RULESET_DIR"/*.json; do
  name="$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["name"])' "$file")"
  id="$(gh api "repos/$REPOSITORY/rulesets" --jq ".[] | select(.name == \"$name\") | .id" | head -n 1)"
  if [[ -z "$id" ]]; then
    echo "Creating ruleset: $name"
    gh api --method POST "repos/$REPOSITORY/rulesets" --input "$file" >/dev/null
  else
    echo "Updating existing ruleset: $name ($id)"
    gh api --method PUT "repos/$REPOSITORY/rulesets/$id" --input "$file" >/dev/null
  fi
done
echo "Ruleset requests applied. Verify enforcement at repository Settings → Rules → Rulesets."
