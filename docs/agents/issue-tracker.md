# Issue tracker: GitHub

Use `gh` with explicit `--repo vinnie357/kina` for issue and pull-request operations.
GitHub holds durable plans, decisions, specifications, acceptance criteria and completion evidence.
Use `--body-file` for multiline text. Preserve imported bees IDs as provenance, not GitHub issue numbers.

## Operations

- List work: `gh issue list --repo vinnie357/kina`.
- Read work: `gh issue view NUMBER --repo vinnie357/kina --comments`.
- Claim work: `gh issue edit NUMBER --repo vinnie357/kina --add-assignee @me`.
- Record evidence before closing an issue. Open pull requests do not prove completion.
- Use native sub-issues and blocking dependencies when available.
- Use `gh api` with numeric database IDs for relationship endpoints; issue numbers are different identifiers.

## Wayfinding operations

A map uses `wayfinder:map`; children use `wayfinder:research`, `wayfinder:grilling`, `wayfinder:prototype` or `wayfinder:task`.
Wayfinder issues carry no triage labels.
The frontier contains open, unassigned children with no open blockers.
Claim before working. Record resolutions in comments, then close and link the result from the map.
The operator resolves product decisions. Research findings are evidence, not approval to build.

## Local lead tracking

Bees is optional and ephemeral. Give each GitHub parent issue its own local tracker under `.worktrees/issue-N/`.
Initialize bees in that directory; run local tracking commands there explicitly.
Local worklist IDs never appear in commits, pull requests or public reports.
Publish durable decisions and evidence to GitHub before removing a completed local tracker.
Do not sync local worklists into git or use the primary checkout's historical bees database.

## History

The immutable bees export and migration index live in `docs/planning/archive/bees/`.
Git history retains the old tracker configuration and previous exports.

## Pull requests as a request surface

PRs as a request surface: no.
