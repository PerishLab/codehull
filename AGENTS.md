# Agents

This repository is a keel **caller**. It declares models, relations, and
seed grants; it never reaches into engine territory (reign, lifecycle,
authority, events, cache are keel's). The laws of the engine live in
`keel:docs/*`; this repo obeys them from the outside.

## Layout

A monorepo of four delivery planes, split by toolchain:

- `crates/api` — the server; the keel caller (bin `api`).
- `crates/cli` — the client; gh for codehull (bin `codehull`).
- `apps/web` — the pnpm web application (node 24, vite, react,
  typescript, vitest, biome); every version is pinned in the
  `pnpm-workspace.yaml` catalog and dependencies reference `catalog:` only.
- `charts/codehull` — the helm delivery.

Territory: application components live under
`apps/web/src/lib/components`, remain style-free, and consume the Design
runtime for reusable visual behavior. `runseal :guard` spans all planes:
cargo fmt/clippy/test, biome/tsc/vitest, helm lint, Plumb, Ectropy, and acts.

## Laws

- Ectropy owns syntax laws (single word, block/path <= 4, no comments);
  Plumb owns repository shape and the canonical `ectropy.toml`. Vocabulary
  deltas remain documented in `docs/vocabulary.md`.
- Dependency direction: codehull -> keel-gate/keel-relay -> keel. Never a
  workspace sibling of keel; distribution follows keel's channel.
- Model changes are law-shaped: extend the slice in `docs/model.md`,
  then code.
- Stage law: `docs/spec.md` (F0 foundations → F7 equivalence audit).
- Never commit on `main`; branch, then `runseal :guard` and `runseal :land`.
- Engine gaps become keel issues and registry releases, never local
  workarounds.
