# Vocabulary

- `codehull` — this product; the absorption touchstone. Also the cli
  binary name.
- `api` — the server crate; the keel caller.
- `cli` — the client crate; gh for codehull.
- `web` / `components` — the pnpm workspace: the vite app and the
  component library (sole style territory).
- `charts` — the helm delivery.
- `rig` / `post` — boot ceremony: idempotent service operators + package
  rises.
- `seed` — the eight-row grant policy sown once at first boot.
- `halt` — boot failure exit.
- `act` — the staged scenario runner over the api binary (`:act`,
  one act per spec stage).
- `join` / `want` / `visible` / `grant` — act helpers: register a user,
  assert a 201 write, probe repo visibility, seed a grant.
- `found` — atomic org creation route: org + owners team + membership + grant in one batch.
- `seam` — the identity boundary: a `Bearer` token this issuer signed resolves an
  operator, and nothing else does. Gate keeps `token` and `session`; the two
  schemes are disjoint, so neither needs to outrank the other.
- `warden` — the verifier: reads the issuer's discovery and JWKS once at boot,
  then checks ES256, issuer, audience, expiry, and `kind = "access"`.
- `anchor` — the row a verified subject hangs on: `iss` plus `sub`, no user
  semantics, born through gate's identity birth so it carries its own self
  grant. A plain put would make a row its own operator cannot touch.
- `hail` — find-or-make a service seat under the `codehull:svc` issuer.
- `hail` — idempotent service-operator lookup-or-create at boot.
- `shape` — build the plugged graph (units + gate + relay) once.
- `raise` / `serve` — store-generic boot: identify, cache mode, share;
  seed, rise packages, wall, listen. Works over Sqlite or Postgres.
- `fresh` — dev-only PG schema reset (`API_FRESH`), applied before bootstrap.
- `stock` — plug the Asset unit into the graph (keel-blob macro).
- `hoard` — build the Vault from the `blob` config section, or None (blob disabled).
- `bucket` — act helper: ensure the MinIO bucket exists via mc.
- Act 3 covers issue search (`like`) and PR review round-trip (Pull one2one Issue, Review, Note, gated merge).
- Act 4 watch/notify: `Actor.watches`, feed = materialized query over watched repos (pull model); pred-subtree (C-16) surfaces public-repo issues to watchers.
- `close` — repo closure delete: a caller-space batch ends the repo's
  issues/labels/milestones then the repo, all-or-nothing. #7 (F9a) resolved
  without an engine primitive — batch + forward queries suffice.
- Route (alias) — axum Path renamed to avoid the std::path::Path clash.
- Act 7 suspension: Actor.barred + gate.bar hook; a suspended token stops resolving to an operator (private repo 200→404).
- `seat` — the receiver over a repository root: reads the cascade, opens the
  store, and runs one of the two boot ceremonies.
- `berth` — the receiver over a raised core: `seed` sows gate and codehull
  grants at bootstrap, `rig` verifies them at serve and refuses without them.
- `artifact` — a declared destination for a durable possession
  (`--artifact sudo=file:PATH` or `sudo=kubernetes:SECRET`); created once,
  never overwritten. Absent, bootstrap keeps it at `.local/sudo`.
- `seal` — the kubernetes destination: the binary writes and reads its own
  Secret through the service account, so no operator handles the token.
- `place` — read one artifact destination into the seat it names.
- `custody` — hold an existing sudo artifact, or mint one into a vacant estate.
- `born` — this boot bootstraps rather than binds: a memory store, or a
  postgres store after `fresh`.
