#!/usr/bin/env nu
use std/assert

def --wrapped invoke [script: path, ...args: string] {
    do { ^nu $script ...$args } | complete
}

def main [] {
    let script = ($env.FILE_PWD | path join worktree.nu)
    let fixture = (^mktemp -d | str trim)
    try {
        cd $fixture
        ^git init -q -b main
        '.worktrees/' | save .gitignore
        ^git add .gitignore
        ^git -c user.name=fixture -c user.email=fixture@example.invalid commit -qm fixture
        ^git remote add origin $fixture
        assert equal (invoke $script check).exit_code 0
        assert equal (invoke $script new 'chore/74-check' '../escape' --dry-run).exit_code 1
        assert equal (invoke $script new 'bad-branch' 'valid' --dry-run).exit_code 1
        assert equal (invoke $script new 'chore/74-check' 'valid' --dry-run).exit_code 0
        ^git config status.showUntrackedFiles no
        'dirty' | save dirty.txt
        assert equal (invoke $script check).exit_code 1
        assert equal (invoke $script new 'chore/74-check' 'valid' --dry-run).exit_code 1
        rm dirty.txt
        ^git switch -qc feature
        assert equal (invoke $script check).exit_code 1
        ^git switch -q main
        assert equal (invoke $script new 'chore/74-check' 'valid').exit_code 0
        assert equal (invoke $script new 'chore/74-other' 'valid' --dry-run).exit_code 1
        let upstream = (do { ^git rev-parse --verify 'chore/74-check@{upstream}' } | complete)
        assert ($upstream.exit_code != 0)
        cd ($fixture | path join .worktrees valid)
        assert equal (invoke $script check).exit_code 0
        cd $fixture
        ^git worktree remove ($fixture | path join .worktrees valid)
        '' | save --force .gitignore
        ^git add .gitignore
        ^git -c user.name=fixture -c user.email=fixture@example.invalid commit -qm unignore
        assert equal (invoke $script new 'chore/74-other' 'other' --dry-run).exit_code 1
        print 'PASS worktree clean-main, input, ignore, collision and nested-checkout journeys'
    } catch {|err|
        cd $env.FILE_PWD
        rm -rf $fixture
        error make {msg: $err.msg}
    }
    cd $env.FILE_PWD
    rm -rf $fixture
}
