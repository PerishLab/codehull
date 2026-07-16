# Vocabulary

- `forgejo` — this caller; the absorption touchstone.
- `stamp` — dev middleware: `x-login` header to operator (toy, replaced by
  gate credentials in anger).
- `rig` / `post` — boot ceremony: idempotent service operators + package
  rises.
- `seed` — the eight-row grant policy sown once at first boot.
- `halt` — boot failure exit.
- `act` — the staged scenario runner over the forgejo binary (`:act`,
  one act per spec stage).
- `join` / `want` / `visible` / `grant` — act helpers: register a user,
  assert a 201 write, probe repo visibility, seed a grant.
- `kind` — Actor field: user / org / svc (Forgejo's actor unification).
- `found` — atomic org creation route: org + owners team + membership + grant in one batch.
- `hail` — idempotent service-operator lookup-or-create at boot.
- `shape` — build the plugged graph (units + gate + relay) once.
- `raise` / `serve` — store-generic boot: identify, cache mode, share;
  seed, rise packages, wall, listen. Works over Sqlite or Postgres.
- `fresh` — dev-only PG schema reset (`KEEL_FRESH`), off the async runtime.
- `stock` — plug the Asset unit into the graph (keel-blob macro).
- `hoard` — build the Vault from `KEEL_S3` env, or None (blob disabled).
- `bucket` — act helper: ensure the MinIO bucket exists via mc.
- Act 3 covers issue search (`like`) and PR review round-trip (Pull one2one Issue, Review, Note, gated merge).
- Act 4 watch/notify: `Actor.watches`, feed = materialized query over watched repos (pull model); pred-subtree (C-16) surfaces public-repo issues to watchers.
- `close` — repo closure delete: a caller-space batch ends the repo's
  issues/labels/milestones then the repo, all-or-nothing. #7 (F9a) resolved
  without an engine primitive — batch + forward queries suffice.
- Route (alias) — axum Path renamed to avoid the std::path::Path clash.
