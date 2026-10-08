# Branch and release-tag protection

**Important:** Configuration files alone do **not** activate rules on GitHub. GitHub's repository administration permission is required to create/update rulesets. The currently available GitHub App connection cannot modify these admin settings. A repository owner must apply them through GitHub UI or a separately authenticated administrative CLI.

## Desired protection of `main`

The [ruleset payload](../.github/rulesets/main.json) requires:

- Pull requests to `main`, with squash/rebase only; no direct main pushes.
- Successful unique status check named **`CI Gate`** with branches up to date.
- Resolved PR review conversations.
- Linear history; force-push and deletion prohibited; no privileged bypass list.
- **Zero mandatory approvals at first**, since this is initially a single-maintainer repository. Change to one or more independent approvals and required CODEOWNERS reviews as soon as regular independent reviewers exist. Do not misrepresent automated self-review as independent approval.

The [release tag ruleset](../.github/rulesets/release-tags.json) prohibits deletion or force-moving `v*` tags, but permits first-time tag creation by the restricted release workflow.

## One-time activation

Option A (recommended): Repository → **Settings → Rules → Rulesets → New ruleset → Import a ruleset**, import each JSON file, inspect the settings and select Active. Ensure the required status context exists as `CI Gate` (run CI on the PR first). Check that GitHub permits settings on the repository plan.

Option B, from a trusted local workstation with an admin-authorized `gh` session:

```bash
bash scripts/admin/apply_rulesets.sh --dry-run
# Review printed payloads and ensure the CI Gate check has appeared.
bash scripts/admin/apply_rulesets.sh --apply
```

The script is dry-run by default, uses the official GitHub API, never stores credentials and fails on missing admin permissions. It creates rules by name or updates the matching ruleset. Run only with an account that has *Administration: write* for this repo. **Do not** put an admin PAT in GitHub Actions secrets simply to deploy repository protections.

## Validation after activation

1. Check [repository rulesets](https://github.com/hami9/LiDB/settings/rules).
2. Open a test PR; verify `CI Gate` is required and cannot merge before checks pass.
3. Confirm direct push/force-push to main is prohibited; do not attempt a destructive real force-push.
4. Confirm GitHub Releases can create a **new** `v*` tag when the application is buildable.
5. Re-check settings after edits to actions, CodeQL or merge configuration.

## Governance caveats

Branch rules may require compatible plan/visibility. An admin can still change repository settings. Add CODEOWNERS to request specialist eyes without making a solo contributor's repository permanently unmergeable. When a reliable second reviewer joins, raise `required_approving_review_count` to 1 and enable CODEOWNERS enforcement.
