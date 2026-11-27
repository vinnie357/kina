#!/usr/bin/env nu

def primary [] {
    let common = (^git rev-parse --path-format=absolute --git-common-dir | str trim)
    $common | path dirname
}

def checked-primary [] {
    let root = (primary)
    if ((^git -C $root branch --show-current | str trim) != "main") {
        error make {msg: "Primary checkout must be on main."}
    }
    if ((^git -C $root status --porcelain --untracked-files=normal | str trim) != "") {
        error make {msg: "Primary checkout must be clean. Preserve existing work before proceeding."}
    }
    $root
}

# Inspect repository-owned and existing worktrees.
def "main list" [] {
    ^git worktree list
}

# Fail when the primary checkout is dirty or off main.
def "main check" [] {
    let root = (checked-primary)
    print $"PASS primary main is clean: ($root)"
}

# Create an issue worktree from origin/main after checking the primary checkout.
def "main new" [branch: string, slug: string, --dry-run] {
    if ($slug !~ '^[a-z0-9][a-z0-9-]*$') {
        error make {msg: "Slug must contain lowercase letters, digits and hyphens."}
    }
    if ($branch !~ '^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)/[0-9]+-[a-z0-9-]+$') {
        error make {msg: "Branch must use type/ISSUE-slug."}
    }
    let root = (checked-primary)
    let destination = ($root | path join .worktrees $slug)
    let ignored = (do { ^git -C $root check-ignore -q $destination } | complete)
    if $ignored.exit_code != 0 {
        error make {msg: "Commit .worktrees/ in the primary .gitignore before creating worktrees."}
    }
    if ($destination | path exists) {
        error make {msg: "Destination exists. Inspect and reuse its worktree when appropriate."}
    }
    ^git check-ref-format --branch $branch | ignore
    if $dry_run {
        print $"Would create ($destination) on ($branch) from origin/main after fetching origin."
        return
    }
    ^git -C $root fetch origin
    ^git -C $root worktree add --no-track $destination -b $branch origin/main
}

def main [] {
    help main
}
