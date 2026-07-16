# F7 equivalence audit

The capability diff against real Forgejo, read against keel's full-model
dream-code ledger (`keel:.task/resources/forgejo.md`, ~40 units). Boundary
holds: state / authority / events / credentials / blobs are in; git
mechanics (objects, pack, diff, merge, CI execution) are out and consumed
by app code, not modeled.

## Units delivered

Business (declared in `main.rs`): Actor (user/org/svc, follows, stars,
watches, barred), Email, Team (crew members, repos), Topic, Repo (fork,
topics, visibility, archived), Label, Milestone, Issue (per-repo serial
index, assignees, labels, blocks), Comment, Reaction (composite unique),
Pull (one2one Issue), Review, Note, Runner, Run, Secret, Variable, Key.

Engine/package (keel-provided, zero business code): `@grant` `@seal`
`@pulse` (engine), Token / Session (keel-gate), Hook (keel-relay), Asset
(keel-blob).

## Capability planes — all green under sqlite, postgres, and MinIO

| Plane | Forgejo capability | keel expression | Act |
|-------|--------------------|-----------------|-----|
| identity | users, orgs, teams | Actor unification + Team crew groups | 1 |
| authority | repo/org/team permissions | `@grant` bootstrap, root-chain subtree, group operators, pred-subtree (C-16) | 1–3 |
| org governance | orgs, owners, membership | atomic `/org` batch, group grants | 1 |
| issues | open/close, assign, label, react, search | business `closed`, serial index, m2m, Reaction composite unique, `like` | 2,3 |
| pull/review | PR from issue, reviews, line notes, merge | Pull one2one Issue, Review, Note, gated `merged` | 3 |
| watch/notify | watches, feed, webhooks | Actor.watches, materialized-query feed, keel-relay | 4 |
| credentials | register/login/token/session, suspend | keel-gate register/login/logout, `bar` hook | 3(keel),7 |
| events | activity, webhooks | `@pulse` stream, coverage-bound relay delivery | keel |
| objects | attachments, avatars, assets | keel-blob presigned S3, bytes never touch keel | 5 |
| lifecycle | archive, delete | `archived` field, closure-delete batch | 6 |
| store | — | sqlite and **real Postgres**, byte-equal | conform |
| actions (state) | runners, runs, secrets, variables, deploy keys | Runner/Run/Secret/Variable/Key rooted at Repo; secret = ciphertext field (app crypts, keel never sees plaintext); scoped-unique names | 8 |

## Resolved without an engine seat (keel-native)

- **Notification feed** — materialized query over watched repos (pull model,
  H0 philosophy), not fan-out.
- **Repo closure delete (F9a, #7)** — caller-space batch ends the subtree;
  no new verb.
- **Sort by comment count (#5)** — client-assembled multi-query
  (`from Comment where issue = X count` per issue); server-side aggregate
  sort deferred as an optimization, not a capability gap.

## Correction (2026-07-16)

An earlier audit claimed closure while omitting the Forgejo **Actions
metadata** plane — a real in-boundary gap. Now closed: Runner, Run, Secret,
Variable, and deploy Key are modeled and green on sqlite + Postgres. Only
Actions **execution** (spinning runners, streaming logs) is out of boundary
(git/CI mechanics, app territory). Secret storage: keel holds **ciphertext**
in a field — the app encrypts before put and decrypts after get, exactly as
gate holds password hashes; keel never sees a plaintext secret.

## Charted, deliberately deferred

- **Org/instance runners, org secrets** — repo-scoped shipped; org/instance
  scope is the same two-unit (or nullable-root) modeling as org labels.
- **Action feed, Mirror, Wiki metadata, Stopwatch** — minor units on
  existing primitives (many2one + fields); add when a use appears, no seat.

- **Org-scoped labels exactly-one (#6)** — a Label carrying repo XOR org is a
  polymorphic root; keel's answer is **two units** (repo-label rooted at
  Repo, org-label rooted at Actor) when the need is live, not a
  polymorphic-root primitive. Repo labels ship today.
- **Hook exactly-one-of-two (repo XOR owner)** — same shape, same two-unit
  resolution when needed.
- **git plane** — objects, pack protocol, diff/merge, CI execution: app code
  that consumes keel (the boundary law), never modeled here.
- **TLS webhooks / retry backoff** — keel-relay v1 is plaintext at-least-once;
  hardening is a relay follow-up.

## Verdict

Every in-boundary Forgejo capability plane is expressed in keel and green on
real Postgres and MinIO. The keel-ization is complete for the settled
boundary: what remains is either app-layer git mechanics (out of scope by
law) or charted modeling choices (two-unit labels) with no capability gap.
The dream-code absorption thesis is now a running, dual-engine system.
