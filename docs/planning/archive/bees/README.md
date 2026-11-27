# Bees archive

GitHub Issues became the system of record on 2026-11-27 under [the wayfinder map](https://github.com/vinnie357/kina/issues/73).

`issues-2026-11-27.jsonl` preserves the exact pre-migration export: 55 records, including comments and dependencies.
SHA-256: `e46ef07b72c1c4b4b01be32e5fa748ddad4ed61a346439e323d2019ab2c94833`.
It includes the two previously uncommitted records. Do not modify this historical snapshot.
`main-export-a9b3612.jsonl` also preserves the primary branch export; several historical records differ from the Kubero checkout.
The 33 closed records remain archived; the 22 unresolved records were migrated for triage.
No unresolved record was proven complete by the source audit at `a9b3612`.

`migration-map.json` maps legacy IDs to GitHub issues. Identical numbers do not mean identical issues.
Kubero remains deferred, with `feature/kubero-example` and PR #66 preserved.
Kagent and image-loading fixes have open PRs; they remain unresolved until validated and merged.

## Dependency handling

Unresolved blockers become native GitHub dependencies. Closed blockers remain documented in the original export.
The registry-to-demo-image edge is preserved initially; research found it may be a related-work link instead of a true blocker.
Broad proposals remain triage work, not approved implementation.

| Legacy ID | GitHub | Disposition |
| --- | --- | --- |
| kina-54 | [#77](https://github.com/vinnie357/kina/issues/77) | deferred |
| kina-53 | [#78](https://github.com/vinnie357/kina/issues/78) | deferred |
| kina-52 | [#79](https://github.com/vinnie357/kina/issues/79) | needs-triage |
| kina-51 | [#80](https://github.com/vinnie357/kina/issues/80) | needs-triage |
| kina-49 | [#81](https://github.com/vinnie357/kina/issues/81) | needs-triage |
| kina-50 | [#82](https://github.com/vinnie357/kina/issues/82) | needs-triage |
| kina-48 | [#83](https://github.com/vinnie357/kina/issues/83) | needs-triage |
| kina-46 | [#84](https://github.com/vinnie357/kina/issues/84) | needs-triage |
| kina-26 | [#85](https://github.com/vinnie357/kina/issues/85) | needs-triage |
| kina-25 | [#86](https://github.com/vinnie357/kina/issues/86) | needs-triage |
| kina-24 | [#87](https://github.com/vinnie357/kina/issues/87) | needs-triage |
| kina-23 | [#88](https://github.com/vinnie357/kina/issues/88) | needs-triage |
| kina-22 | [#89](https://github.com/vinnie357/kina/issues/89) | needs-triage |
| kina-21 | [#90](https://github.com/vinnie357/kina/issues/90) | needs-triage |
| kina-20 | [#91](https://github.com/vinnie357/kina/issues/91) | needs-triage |
| kina-17 | [#92](https://github.com/vinnie357/kina/issues/92) | needs-triage |
| kina-55 | [#93](https://github.com/vinnie357/kina/issues/93) | needs-triage |
| kina-43 | [#94](https://github.com/vinnie357/kina/issues/94) | needs-triage |
| kina-42 | [#95](https://github.com/vinnie357/kina/issues/95) | needs-triage |
| kina-40 | [#96](https://github.com/vinnie357/kina/issues/96) | needs-triage |
| kina-30 | [#97](https://github.com/vinnie357/kina/issues/97) | needs-triage |
| kina-31 | [#98](https://github.com/vinnie357/kina/issues/98) | needs-triage |

## Audit findings

- Host mounts are a concrete subset of advanced configuration, not completed by schema fields alone.
- Remote cargo tests are disabled; existing unit tests do not establish the comprehensive testing proposal.
- Networking overlaps the container update in #72 without duplicating its scope.
- CLI verbosity shipped; completions and progress indicators still need scope decisions.
- The kubectl error issue points to `cli/cluster.rs`, not its historical `core/cluster.rs` reference.
- The native registry addon and experimental Gateway API L4 support remain unimplemented at the audited revision.
