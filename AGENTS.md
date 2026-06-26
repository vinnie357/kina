# Agent Instructions

This project uses [bees](https://github.com/ctxshift/bees) for issue tracking. Issues live in `.bees/` (JSONL synced via git, SQLite db local-only).

## Quick Reference

```bash
bees ready            # Find available work (no blockers)
bees show <id>        # View issue details
bees update <id> --status in_progress  # Claim work
bees close <id>       # Complete work
bees comment add <id> "text"  # Add a comment
bees sync             # Export database to JSONL
```

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bees sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
