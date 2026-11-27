# Worktrees

The primary checkout stays on clean `main`. Advance it only with `git pull --ff-only`.
Each GitHub issue gets one repository-owned worktree under `<primary>/.worktrees/<slug>` and one writer.
Use a conventional branch name such as `chore/74-process`.

## Create and inspect

Run `mise run worktree:list` to inspect existing checkouts.
Run `mise run worktree:new -- chore/74-process 74-process` from a repository checkout.
The command checks primary cleanliness, the ignore rule, the branch name and destination before fetching and creating.
Run `mise run worktree:check` to check primary cleanliness.

Nested worktrees inherit parent mise configuration. Isolate ports, clusters and local configuration when work can collide.
Keep build outputs separate. Do not share dependencies or build folders by symlink.
Attach an existing checkout with Codex `attach_worktree` when useful.
Codex managed creation and archive/restore are not used for committed repository work.

## Local worklists

For optional bees tracking, create `.worktrees/issue-N/` and initialize bees there.
Run bees from that directory. Share it only among the parent issue's leads.
Transfer durable evidence to GitHub before deleting the completed worklist.

## Feature batches

Create item worktrees from the feature branch when parallel writers are needed.
Keep one writer on the feature branch. Record each item's base revision.
Review and validate each item before fast-forwarding it into the feature branch.
Open one PR after local gates pass. Preserve Kubero's branch and open PR #66 while deferred.

## Finish

First push with `git push -u origin HEAD`; verify the remote branch SHA. Record evidence in GitHub.
After verified merge, remove only the completed worktree with `git worktree remove PATH`.
Inspect untracked files if removal refuses. Do not prune unrelated worktrees or clear shared stashes.
Fetch and fast-forward primary main. Confirm the other worktrees still exist.
