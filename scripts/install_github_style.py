#!/usr/bin/env python3
"""Install the reviewed repository skill for future sessions of local agents."""
import argparse
import hashlib
import os
from pathlib import Path
import sys

SOURCE = Path(__file__).resolve().parents[1] / '.agents/skills/github-style/SKILL.md'


def install(source: Path, directory: Path) -> str:
    """Copy one skill without overwriting a different file or following a symlink."""
    data = source.read_bytes()
    directory.mkdir(parents=True, exist_ok=True)
    target = directory / 'SKILL.md'
    if target.is_symlink():
        raise ValueError(f'Refusing symlink: {target}')
    if target.exists():
        if target.read_bytes() != data:
            raise ValueError(f'Existing skill differs; review it before replacing: {target}')
        return f'Already installed: {target}'
    with target.open('xb') as output:
        output.write(data)
    return f'Installed: {target}'


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--agent', choices=['codex', 'claude', 'antigravity', 'all'], default='all')
    args = parser.parse_args()
    home = Path.home()
    codex_root = Path(os.environ.get('CODEX_HOME') or home / '.codex')
    locations = {
        'codex': [home / '.agents/skills', codex_root / 'skills'],
        'claude': [home / '.claude/skills'],
        'antigravity': [home / '.gemini/config/skills', home / '.gemini/antigravity-cli/skills'],
    }
    agents = locations if args.agent == 'all' else [args.agent]
    try:
        print(f'Source SHA-256: {hashlib.sha256(SOURCE.read_bytes()).hexdigest()}')
        seen = set()
        for agent in agents:
            for root in locations[agent]:
                directory = root / 'github-style'
                resolved = directory.resolve()
                if resolved not in seen:
                    print(install(SOURCE, directory))
                    seen.add(resolved)
    except (OSError, ValueError) as error:
        print(f'Installation stopped: {error}', file=sys.stderr)
        return 1
    print('Start a new local agent session to reload skills. Other machines need their own installation.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
