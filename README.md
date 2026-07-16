# forgejo

Forgejo on keel: the absorption touchstone.

keel's laws were pressure-tested against Forgejo's full data model on paper
(`keel:.task` dream-code exercise, F/G/C/T/H seat ledgers). This repository
turns that audit into a running system: a Forgejo-shaped application built
purely as a keel **caller** — models, relations, and seed grants only. The
engine owns lifecycle, uniqueness, authority, events, and cache.

Ground truth exists: the lab itself runs on real Forgejo. Every behavior
here is diffable against it.

## Slice one

Actor, Email, Team, Topic, Repo (fork, topics), Label, Milestone, Issue
(per-repo serial index, assignees, labels), Comment, Pull (one2one Issue),
Review — plus `keel-gate` (register/login/token/session) and `keel-relay`
(webhooks) via their declaration macros.

Known holds (filed as issues): self-referential many2many (follows,
issue blocks), Reaction (needs composite unique), org-scoped labels,
org+owners batch atomicity, text search, sort-by-aggregate, repo closure
delete.

## Run

```sh
cargo run -- .
# sudo token prints once on first boot; store is .local/forgejo.sqlite
curl -s -X POST 127.0.0.1:3400/register -H 'content-type: application/json' -d '{"login":"ada"}'
```

Identity middleware for development: `x-login: <login>` resolves an
operator directly; real credentials go through `/login` (session cookie)
or `authorization: token <t>`.
