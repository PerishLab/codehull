# Forgejo capability ledger

Honest, itemized. ✅ modeled + act-green · 🟡 modeled, not act-proven ·
⬜ in-boundary, NOT done · ⛔ outside the stages `docs/host.md` names.

The boundary moved when `docs/spec.md` closed: git mechanics and CI execution
are in-boundary now, staged as H1, H2 and H3. Rows carrying a stage tag are
owed, not excluded.

## Identity & access
- ✅ Users (Actor kind=user)
- ✅ Organizations (Actor kind=org)
- ✅ Teams + membership + permissions (Team crew, act 1)
- ✅ Access tokens / PAT (gate Token, act 3)
- ✅ Sessions (gate Session, act 3)
- ✅ Suspension (act 7)
- ✅ Emails (Email unit, act 10)
- 🟡 Avatars (blob Asset)
- ✅ User SSH / GPG account keys (UserKey, act 10)
- 🟡 2FA / passkeys (gate auth extension — caller space)
- 🟡 OAuth2 / login sources (auth middleware — caller space)

## Repositories
- ✅ Create / visibility public·private (act 2,3)
- ✅ Delete (closure delete batch, act 6)
- ✅ Topics (Topic m2m)
- ✅ Collaborators (= `@grant` rows, act 1 pattern)
- ✅ Webhooks (Hook via keel-relay)
- ✅ Deploy keys (Key, act 8)
- ✅ Fork (fork ref, act 10)
- ✅ Archive (archived field, act 10)
- 🟡 Transfer ownership (set owner — subtree follows, keel act)
- 🟡 Default branch (trunk field)
- ✅ Branch protection — `Shield` records a branch's approval policy and the
  proposal door reads it: a merge is refused until the branch's demands are
  judged green and its approvals are counted (act 14). `force` is still
  recorded and unread; the push path does not consult it
- ✅ Mirrors (Mirror one2one Repo, act 10)
- ✅ Bare seat and bundle ingress (`crates/repo`, act 13)
- ✅ References as Keel rows: compare-and-swap by liveness, projection to the
  git store, reconciled before advertisement (`Ref`, act 13). Reconciliation
  writes only what differs: it used to re-project every reference on every
  advertisement, which cost three `git` processes per reference per fetch
- ✅ Reference retirement, releasing the name for reuse (act 13)
- ✅ Transport: clone and fetch, git smart HTTP over the seat, including the
  compressed request a real client sends once its want list grows (act 14)
- ✅ Transport: push, spoken by the api rather than by `git-receive-pack`;
  deletes arrive as retirements (act 14)
- ✅ Transport: fetch over SSH, public-key authenticated against `actor:key`
  rows, `upload-pack` piped onto the channel (act 14)
- ✅ Transport: push over SSH, spoken by the adaptor; the pack streams straight
  into `index-pack` and every reference move routes back through `point` (act 14)
- ✅ Objects quarantined in a pen on both transports: a refused push admits
  nothing, an accepted one migrates before any reference is written (act 14)
- ⬜ Collecting objects a reference accepted and later abandoned — needs a
  trigger on a schedule, and a schedule is a runner (H3)
- ✅ Verdicts on a commit, one live per context, combined by rollup (act 14)

## Issues
- ✅ Open / close (business `closed`, act 2)
- ✅ Per-repo issue number (serial index, keel act)
- ✅ Comments (Comment)
- ✅ Reactions (composite unique, act 2)
- ✅ Labels — repo scope (act 3)
- ✅ Search (`like`, act 3)
- ✅ Milestones (Milestone, act 10)
- ✅ Assignees (m2m, act 10)
- ✅ Dependencies / blocks (self-ref m2m, act 10)
- ✅ Org-scoped labels (OrgLabel rooted at org, scoped-unique, act 11)
- ✅ Timeline / activity feed (= `@pulse` stream, audit)
- ⬜ Time tracking (Stopwatch) — trivial unit, on demand
- ⬜ Pinned issues, issue templates — trivial, on demand

## Pull requests
- ✅ Create from issue (Pull one2one Issue, act 3)
- ✅ Reviews + approval state (Review, act 3)
- ✅ Line notes (Note, act 3)
- ✅ Actual merge: forward, join and squash, each proven by cloning the result
  back and counting parents (`Weld`, act 14)
- ✅ Merging a proposal through its own door: base and head come from the `Pull`
  row, and the row carries the commit the merge produced (act 14)
- ✅ Opening a proposal without owning the repository: `/propose` creates the
  row above the operator, so no one is minted coverage over it; the author may
  edit it while it carries no merge and can never write the merge itself
  (`Pull.author`, seeded predicate, act 3)
- ✅ A merge may carry the head it expects, and refuses when the head moved
  under it — the same compare and swap a reference move takes (act 14)
- ⬜ Draft PRs — `Pull` carries no draft field. This row read ✅ against
  `Release.draft`, which is a different unit answering a different question
- ⬜ Merging a proposal whose head lives in another repository — the `source`
  relation exists, the fetch across repositories does not, and the door refuses
  with a reason rather than merging the wrong branch
- ✅ Required checks — one `Demand` row per context a branch requires, hung on
  its `Shield`. A branch with no demands requires nothing, an unjudged context
  is not a pass, and a failing or pending one refuses with which context and
  what it read (act 14)
- ⬜ Conflict presentation — a conflicting merge refuses; nothing reports where

## Actions (CI)
- ✅ Runners (act 8)
- ✅ Runs (act 8)
- ✅ Secrets — ciphertext field, scoped-unique (act 8)
- ✅ Variables (act 8)
- ✅ Artifacts (= blob Asset)
- ✅ Org runners & secrets (OrgRunner/OrgSecret, team-subtree gated, act 11)
- ⬜ Workflow execution: registration, queue, dispatch, logs — H3
- ⬜ Workflow and action resolution, including across repositories — H3

## Planning & delivery
- ✅ Projects / kanban columns (Project, Column with card sort, act 9)
- ✅ Releases + release notes (Release, scoped-unique tag, act 9)
- ✅ Release assets (= blob Asset, act 5)
- ✅ Package registry metadata (Package, act 9; bytes = blob)
- ⬜ Cargo sparse index, served and published — H2
- ⬜ OCI distribution, served and published — H2
- 🟡 Wiki metadata (trivial unit; content lives in git)
- ✅ Tags, plain and annotated, over both transports (act 14)
- ✅ Every namespace below `refs`, notes included; the reserved set is empty
  because Codehull owns no namespace of its own yet (act 14)

## Notifications & social
- ✅ Watch (act 4)
- ✅ Follow (follows, keel act)
- ✅ Notification feed (materialized query, act 4)
- ✅ Webhooks, coverage-bound (keel-relay)
- ✅ Stars (stars m2m has-count, act 10)

## Admin & cross-cutting
- ⬆️ User suspend — moved upstream with identity. codehull carries no
  suspension flag; a barred person stops getting tokens, and a token this
  issuer did not sign resolves nothing (act 7)
- ✅ Org management (act 1)
- ✅ Audit log (`@pulse` stream + `@grant` enumeration)
- ✅ Fine-grained authorization (`@grant`, root chain, groups, pred-subtree)
- ✅ Object storage (keel-blob presigned; bytes never touch keel)
- ✅ **Store portability** — sqlite AND real Postgres, byte-equal

## Verdict

Every metadata plane is ✅ — modeled and act-green on **sqlite and real
Postgres**, blobs on real MinIO. 30 business units + 7 keel-provided. That was
the whole of the boundary `docs/spec.md` settled, and against that boundary it
was complete.

**It is no longer the whole of the ledger.** `docs/host.md` moved the boundary,
and what used to sit behind ⛔ is now owed:

- ✅ **H1 change plane**: transport over http and ssh, every namespace below
  `refs`, objects quarantined until a reference accepts them, real merge in
  three shapes, verdicts on a commit, a proposal that merges through its own
  door behind the checks its branch demands.
- ⬜ **H2 registry plane**: cargo sparse index and OCI distribution.
- ⬜ **H3 execution plane**: runner registration, queue, dispatch, logs,
  workflow and action resolution, artifacts.

H1 closed against the scale its stage was given: everything this estate needs
except a registry and a runner. It did not close against every row above —
conflict presentation, cross-repository proposals and draft proposals are
still ⬜, and the debts are named where they live: http push buffers its whole
body, `upload-pack`, `index-pack`, `merge-tree` and the ssh listener all run
unbounded, and objects a reference accepted and later abandoned still have no
collector. None of those are capabilities H1 owed; all of them are owed to
somebody.

What Codehull records about H2 and H3 it already records well; what it does
about them is close to nothing. That gap is doing, not recording.

Still not owed here:

- **Caller-space auth integrations**: 2FA / passkeys — gate is the seam; these
  are middleware the caller adds, not keel. OAuth2 / external login is no
  longer pending: codehull issues no identity at all, and an operator exists
  only as a verified `Bearer` token (`crates/api/src/{warden,seam}.rs`).
- **Charted scope modeling** (no gap, no new primitive): org-scoped labels /
  runners / secrets are the two-unit pattern, added when a live need appears.
- **Minor**: Action feed = the `@pulse` stream (audit already ✅); Stopwatch,
  pinned issues, issue templates are trivial units addable on demand.
