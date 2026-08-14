# Stage law, closed

**This program is retired. The live stage law is `docs/host.md`.** F0 through
F7 delivered what they promised and the table below is retained as lineage,
not as a contract. Nothing here governs a new stage.

It was the delivery contract for the keel-ized Forgejo: a stage was done when
its act went green under the repository guard, and the boundary it settled was
**state, authority, events, credentials, and blobs in; git mechanics (objects,
pack, diff/merge, CI execution) out.** That boundary was drawn while Forgejo
was to be kept. Forgejo is now being retired, so a forge-shaped need is a
missing Codehull capability rather than a line to stay behind, and the
successor law admits exactly what this one excluded. The reasoning and the
stages that replace these live in `docs/host.md`.

The Identity and Must-not clauses that used to close this file were live law,
not history. They moved to `docs/host.md` unchanged rather than being copied,
so the two files cannot drift.

## Delivery grammar

Each stage delivers laws (keel-side, when the engine must grow), surface
(models, grants, packages, routes), and an **act** — an L2 scenario over
the running binary. Engine gaps found here become keel issues, keel PRs,
`codehull ship` releases, then version bumps here: the touchstone consumes keel
only through the perish registry.

## Stages

| Stage | Delivers | Act proves |
|-------|----------|------------|
| F0 | wrapperless profile + CI + this law | guard green on main |
| conform | store portability: `API_STORE_URL=... codehull act` runs every act against real Postgres (docker-compose pg:5434) | full stack green on pg and sqlite |
| F1 | org governance: group operators (keel C-M1), org+owners batch (keel C-9 → #3), team CRUD, member/repo grants | a team grant admits a member to a private repo; leaving revokes |
| F2 | issue suite completed: Reaction (keel U4 → #2), issue search (keel text pred → #4), sort by comments (keel agg order → #5) | full issue lifecycle incl. reactions, search, busiest-first |
| F3 | pull & review: Review threads (Note), approvals, merge state — DONE | review round-trip with line notes; merge flips state under authority |
| F4 | watch & notify: watches (Actor.watches), notifications as materialized queries (feed = issues in watched repos, id cursor), relay webhooks | watcher sees public-repo issue feed via C-16; webhooks live via relay |
| F5 | blob plane: `keel-blob` (Asset metadata + presigned MinIO bytes), `API_BLOB_ENDPOINT=... codehull act` | presigned upload, 302 gated download, stranger 404; keel never touches bytes |
| F6 | admin & lifecycle: repo closure delete via caller-space **batch** (no engine seat) — DONE; suspension via gate `bar` hook (DONE); org-label exactly-one charted as two-unit modeling | bare delete 409s under K3, `/close` batch-ends the subtree then deletes |
| F7 | equivalence audit (`docs/audit.md`): every in-boundary plane — incl. Actions metadata (runner/run/secret/variable/key) — green on sqlite + postgres + MinIO; residue is Actions **execution** + git mechanics (app territory) or charted scope modeling | DONE |

## Identity

Moved to `docs/host.md`. Still live law.

## Must not

Moved to `docs/host.md`. Still live law, with one clause amended there.
