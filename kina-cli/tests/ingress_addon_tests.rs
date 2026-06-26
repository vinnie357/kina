//! Pure-function tests for ingress addon detection helpers.
//! No process spawns, no network, no kubectl — pure string logic.
use kina_cli::core::verify::pods_all_ready;

#[test]
fn pods_all_ready_none_on_empty() {
    assert_eq!(pods_all_ready(""), None);
}

#[test]
fn pods_all_ready_counts_ready_over_total() {
    // cols: NAME READY STATUS RESTARTS AGE
    let out = "traefik-abc   1/1   Running   0   2m\n\
               traefik-def   0/1   Pending   0   2m\n";
    assert_eq!(pods_all_ready(out), Some((1, 2)));
}

#[test]
fn pods_all_ready_all_up() {
    let out = "traefik-abc   1/1   Running   0   2m\n\
               traefik-def   1/1   Running   0   2m\n";
    assert_eq!(pods_all_ready(out), Some((2, 2)));
}
