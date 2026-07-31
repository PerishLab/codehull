# Forgejo capability ledger

Honest, itemized. ✅ modeled + act-green · 🟡 modeled, not act-proven ·
⬜ in-boundary, NOT done · ⛔ out of boundary (execution / git mechanics,
app code that consumes keel).

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
- ✅ Branch protection (Shield, act 9)
- ✅ Mirrors (Mirror one2one Repo, act 10)

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
- ✅ Merge state, gated by authority (merged field, act 3)
- ✅ Draft PRs (Pull.merged / Release.draft fields, act 3,9)
- ⛔ Diff / conflict / actual merge (git mechanics)

## Actions (CI)
- ✅ Runners (act 8)
- ✅ Runs (act 8)
- ✅ Secrets — ciphertext field, scoped-unique (act 8)
- ✅ Variables (act 8)
- ✅ Artifacts (= blob Asset)
- ✅ Org runners & secrets (OrgRunner/OrgSecret, team-subtree gated, act 11)
- ⛔ Workflow execution (spin runners, stream logs)

## Planning & delivery
- ✅ Projects / kanban columns (Project, Column with card sort, act 9)
- ✅ Releases + release notes (Release, scoped-unique tag, act 9)
- ✅ Release assets (= blob Asset, act 5)
- ✅ Package registry metadata (Package, act 9; bytes = blob)
- 🟡 Wiki metadata (trivial unit; content ⛔ in git)
- ⛔ Tags (git)

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

Every in-boundary Forgejo capability plane is now ✅ — modeled and act-green
on **sqlite and real Postgres**, blobs on real MinIO. 30 business units +
7 keel-provided. Remaining items are all explicitly ⛔ or caller-space:

- ⛔ **Execution / git mechanics**: workflow runs, merge/diff, tags, wiki &
  package *content* — app code that consumes keel, never modeled (boundary law).
- **Caller-space auth integrations**: 2FA / passkeys — gate is the seam; these
  are middleware the caller adds, not keel. OAuth2 / external login is no
  longer pending: codehull issues no identity at all, and an operator exists
  only as a verified `Bearer` token (`crates/api/src/{warden,seam}.rs`).
- **Charted scope modeling** (no gap, no new primitive): org-scoped labels /
  runners / secrets are the two-unit pattern, added when a live need appears.
- **Minor**: Action feed = the `@pulse` stream (audit already ✅); Stopwatch,
  pinned issues, issue templates are trivial units addable on demand.

The keel-ization is capability-complete for the settled boundary.
