# codehull

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

## Shape

- `crates/api` — the server; the keel caller.
- `crates/cli` — the client; gh for codehull.
- `apps/web` — the web face; local style-free components stay under
  `src/lib/components` and reusable visual behavior comes from Design.
- `charts/codehull` — the helm delivery.

## Operator CLI

The `codehull` binary owns the product acceptance and release workflows:

```sh
cargo run --locked -p codehull -- act
cargo run --locked -p codehull -- ship
```

`act` runs the staged API scenarios; the stages are `docs/host.md`, and the
acts that predate it answer to the closed program in `docs/spec.md`. `ship`
publishes the API image plus the Helm chart from a clean `main` at one exact
version.

## Run

```sh
cargo run -p api -- bootstrap .
cargo run -p api -- serve .
# the sudo token lands in .local/sudo; store is .local/codehull.sqlite
curl -s -X POST 127.0.0.1:3400/api/register -H 'content-type: application/json' -d '{"login":"ada"}'
```

Boot is two ceremonies since keel 0.10: `bootstrap` mints the sudo token into
its declared artifact and sows the grants; `serve` refuses to start when those
grants are absent. A `memory` store is born in place and prints its token.
Configuration is the plumb cascade over `codehull.toml` and the `API_` env
prefix — keel reads no files and no environment.

codehull issues no identity. An operator exists only as
`authorization: Bearer <jwt>` — a token some issuer signed, verified against
the JWKS at `API_OIDC_ISSUER`. The first time a subject arrives, codehull
gives it an anchor row carrying `iss` and `sub` and nothing else: no login,
no name, no suspension flag. Those are the issuer's to hold, and a client
that wants to render a person asks the issuer, not codehull.
