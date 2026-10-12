# Shared agent skills

The maintainer supplied `github-style` and authorized its use and installation. The shared instruction text is in [`.agents/skills/github-style/SKILL.md`](../.agents/skills/github-style/SKILL.md). It is a Markdown delivery playbook with no executable hooks, dependencies or credentials. LiDB's MIT repository terms apply to this maintainer-contributed copy; no outside package was installed.

The installed source SHA-256 is `46e57b8977c82a6e675ba3ee34deea84c7f7243e7a582388aaba80ceeda29e4b`. The instruction text was retained from the attachment read at task startup; the temporary attachment path is not a persistent source. Keep changes reviewable and update this recorded digest when deliberately editing the canonical skill.

## Repository discovery

- Codex and Antigravity discover the shared lowercase `.agents/skills/github-style/SKILL.md`; root [AGENTS.md](../AGENTS.md) also explicitly directs repository agents to read it.
- Claude has a [native skill entry point](../.claude/skills/github-style/SKILL.md) referencing the same shared source and [CLAUDE.md](../CLAUDE.md).
- Antigravity's [always-on rule](../.agents/rules/continue-project.md) loads the [persistent task queue](../.AGENTS/ANTIGRAVITY.md). Rules apply within an opened project/session; files cannot launch an agent or authorize tools.

The existing uppercase `.AGENTS/` directory remains canonical project policy. Lowercase `.agents/` contains native discovery adapters and this cross-agent skill. Do not add a second skill copy whose path differs only by directory case, especially on Windows.

## User-scope installation

From a checkout containing these changes:

```bash
python3 scripts/install_github_style.py --agent all
# Choose one client when appropriate:
python3 scripts/install_github_style.py --agent codex
python3 scripts/install_github_style.py --agent claude
python3 scripts/install_github_style.py --agent antigravity
```

Windows uses `python` in place of `python3` when required. The standard-library installer copies only the reviewed Markdown file and prints its digest and destinations. Re-running it is safe when the destination matches; it refuses different existing content and final-file symlinks. A conflict requires reviewing the existing skill instead of silently overwriting it.

| Client | User-level destination |
| --- | --- |
| Codex | `~/.agents/skills/github-style/` and configured `CODEX_HOME/skills/github-style/` (default `~/.codex/skills/`) for compatibility |
| Claude Code | `~/.claude/skills/github-style/` |
| Antigravity IDE / 2.0 | `~/.gemini/config/skills/github-style/` |
| Antigravity CLI | `~/.gemini/antigravity-cli/skills/github-style/` |

Start a new local session after installation to reload skills. Repository copies travel through clone/fetch; user-level copies are local to that user and machine. Other devices or newly provisioned cloud environments must fetch the repository and install there as needed. This task does not modify an account-wide ChatGPT plugin catalog or a remote Antigravity installation.

For removal, delete only the installed `github-style` directory after confirming it contains this copy and no user changes. Repository agents can still use the tracked shared skill through their entry points.

## Discovery references

The adapter paths and Antigravity `trigger: always_on` use the documented conventions in [Codex skills](https://developers.openai.com/codex/skills/), [Antigravity skills](https://www.antigravity.google/docs/skills?tab=ide) and [Antigravity rules](https://www.antigravity.google/docs/rules/). Read version-specific client documentation if a future release changes discovery. Native clients were not launched from this environment; file/metadata validation and installation are separate from an actual client-session test.
