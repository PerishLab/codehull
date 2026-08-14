# Agents

This repository is a keel **caller**. It declares models, relations, and
seed grants; it never reaches into engine territory (reign, lifecycle,
authority, events, cache are keel's). The laws of the engine live in
`keel:docs/*`; this repo obeys them from the outside.

## Layout

A monorepo of four delivery planes, split by toolchain:

- `crates/api` — the server; the keel caller (bin `api`).
- `crates/cli` — the client; gh for codehull (bin `codehull`).
- `crates/repo` — the repository seat; ordinary git plumbing behind a
  private crate, reached only through the api's ground routes.
- `apps/web` — the pnpm web application (node 24, vite, react,
  typescript, vitest, biome); every version is pinned in the
  `pnpm-workspace.yaml` catalog and dependencies reference `catalog:` only.
- `charts/codehull` — the helm delivery.

Territory: application components live under
`apps/web/src/lib/components`, remain style-free, and consume the Design
  runtime for reusable visual behavior. The Forgejo guard spans all planes:
cargo fmt/clippy/test, biome/tsc/vitest, helm lint, Plumb, Ectropy, and acts.

## Laws

- Ectropy owns syntax laws (single word, block/path <= 4, no comments);
  Plumb owns repository shape and the canonical `ectropy.toml`. Vocabulary
  deltas remain documented in `docs/vocabulary.md`.
- Dependency direction: codehull -> keel-gate/keel-relay -> keel. Never a
  workspace sibling of keel; distribution follows keel's channel.
- Model changes are law-shaped: extend the row in `docs/ledger.md`, then
  code. This law named a `docs/model.md` for a while; no such file was ever
  written, and the ledger is where the model has always been recorded.
- Stage law: `docs/host.md` (H1 change → H2 registry → H3 execution).
  Codehull hosts what it needs before how Plumb integrates the estate is
  evaluated. `docs/spec.md` is the closed program it succeeds, kept as
  lineage; its boundary excluded git mechanics and no longer governs.
- Never commit on `main`; branch, let the repository guard pass, then use `plumb land`.
- Engine gaps become keel issues and registry releases, never local
  workarounds.
- Codehull hosts its own git. **Keel owns metadata and references; the git
  store owns objects.** A reference is a `Ref` row, so moving one is an `end`
  plus a `put` in one batch — Keel's own liveness check is the compare and
  swap, and ending releases the name because live uniqueness is a partial
  index. The on-disk reference store is a projection of those rows, written
  after Keel commits and reconciled from Keel before every advertisement;
  it is never the authority and never the thing that decides.
- **Where Codehull hosts git, it is a whole git, not the subset this estate
  happens to use.** Every namespace below `refs` is accepted; the reserved set
  is what Codehull itself owns, and today that set is empty. Two earlier
  rulings narrowed this — heads alone, then heads and tags — and both wrote
  current usage down as if it were the model. Refusing a name for being
  unmodelled is only honest when the thing really is outside the model.
  A tag moves and retires by the same rules a branch does; making tags
  immutable would be release governance, and that belongs to Plumb, not to a
  transport.
- A verdict is an assertion about a commit, not a record of an execution.
  `Run` stays what it was; a commit may carry verdicts under several contexts
  and may carry one with no run behind it. Landing decisions read the combined
  rollup, so the rollup rule — any failure, then any pending, otherwise
  success — is law rather than an implementation detail.
- Which merge shapes a repository permits is data — one `Weld` row per shape —
  and a repository with no rows permits none. There is no default in code,
  because a default in code is this plane deciding a policy question for
  itself. The merge itself is git's: `merge-tree` writes the tree and
  `commit-tree` writes the commit, and Codehull chooses only the shape.
- Reference names are still refused when they carry a character that would
  break the query they are interpolated into. That is a guard against
  injection, not a statement about the model, and the two must not be confused
  when either one is next revised.
- The ground plane mounts only where a repository path is configured, and with
  none the api serves metadata and hosts no git at all. Losing that plane is a
  regression, not a simplification — it was deleted once and went unnoticed
  for eleven days.
