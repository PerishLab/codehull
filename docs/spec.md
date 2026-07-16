# Stage law

Delivery contract for the keel-ized Forgejo. A stage is done when its act
is green under `runseal :guard`; no calendar. The boundary is settled:
**state, authority, events, credentials, and blobs are in; git mechanics
(objects, pack, diff/merge, CI execution) are out.** Acceptance is
capability equivalence on a keel-native surface — never Gitea API
mimicry. The completeness ledger is keel's Forgejo dream-code model
(~40 units); the opening issues (#2–#7) burn down en route.

## Delivery grammar

Each stage delivers laws (keel-side, when the engine must grow), surface
(models, grants, packages, routes), and an **act** — an L2 scenario over
the running binary. Engine gaps found here become keel issues, keel PRs,
`:ship` releases, then version bumps here: the touchstone consumes keel
only through the perish registry.

## Stages

| Stage | Delivers | Act proves |
|-------|----------|------------|
| F0 | runseal + CI + this law | guard green on main |
| conform | store portability: `KEEL_PG=... runseal :act` runs every act against real Postgres (docker-compose pg:5434) | full stack green on pg and sqlite |
| F1 | org governance: group operators (keel C-M1), org+owners batch (keel C-9 → #3), team CRUD, member/repo grants | a team grant admits a member to a private repo; leaving revokes |
| F2 | issue suite completed: Reaction (keel U4 → #2), issue search (keel text pred → #4), sort by comments (keel agg order → #5) | full issue lifecycle incl. reactions, search, busiest-first |
| F3 | pull & review: Review threads (Note), approvals, merge state — DONE | review round-trip with line notes; merge flips state under authority |
| F4 | watch & notify: watches (Actor.watches), notifications as materialized queries (feed = issues in watched repos, id cursor), relay webhooks | watcher sees public-repo issue feed via C-16; webhooks live via relay |
| F5 | blob plane: `keel-blob` (Asset metadata + presigned MinIO bytes), `KEEL_S3=... runseal :act` | presigned upload, 302 gated download, stranger 404; keel never touches bytes |
| F6 | admin & lifecycle: repo closure delete via caller-space **batch** (no engine seat) — DONE; suspension via gate `bar` hook (DONE); org-label exactly-one charted as two-unit modeling | bare delete 409s under K3, `/close` batch-ends the subtree then deletes |
| F7 | equivalence audit: ledger sweep against the dream-code model; gap list either closed or chartered | the capability diff against real Forgejo reads empty in-boundary |

## Must not

- Gitea/Forgejo API mimicry (paths, payload shapes).
- Business code beyond models, grants, seeds, and package wiring.
- Bytes in keel or metadata truth in MinIO (each plane owns its half).
- A stage advanced past a red act.
- Engine workarounds living here instead of keel issues.
