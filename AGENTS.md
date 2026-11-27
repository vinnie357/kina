# Agent Instructions

Load the `core` skill before work.

GitHub Issues is the system of record for plans, decisions, acceptance criteria and completion evidence.
Read `docs/agents/issue-tracker.md` before creating, migrating or updating issues.
Bees is optional, ephemeral lead tracking. Keep its files uncommitted.

Keep the primary checkout on clean `main`; advance it with `git pull --ff-only`.
Read `docs/agents/worktrees.md` before creating a branch, worktree or local worklist, or cleaning up work.
Read `docs/agents/council.md` before research delegation or plan review.

Before ending a work session:

1. Record remaining work and evidence in GitHub.
2. Run relevant mise gates; run `mise run ci` when code changes.
3. Scan outgoing changes for secrets and attribution.
4. Commit and push the feature branch; verify its remote SHA.
5. Keep unrelated branches, worktrees and host resources intact.
6. Report unfinished checks and the next decision.

Do not implement runtime upgrades or library extraction before resolving the product decision in GitHub issue #76.
