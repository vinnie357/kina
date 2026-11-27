#!/usr/bin/env nu

def main [plan: path] {
    if not ($plan | path exists) {
        error make {msg: "Plan file does not exist."}
    }
    with-env {REX_SESSION: null REX_BLOCK: null} {
        ^altana council --council kina-plan --read-only --require-all --json --file $plan
    }
}
