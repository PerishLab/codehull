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
- `crates/ssh` is its own adaptor, not the http one with a different pipe. It
  keeps its own framing, its own credential translation and its own naming,
  and it lands on the same Keel operations everything else lands on. Fetch is
  piped straight to `upload-pack`, because a piped half gets whole-git for
  free; anything that writes is intercepted, so `receive-pack` never runs on
  either transport. Over ssh the pack streams straight into `index-pack`
  rather than being buffered, which is what the http half should grow to.
- **Objects a reference has not accepted never enter the object store.** A push
  is indexed into a pen inside the repository, and the pen migrates in only
  after every check has passed, before any reference is written. Both adaptors
  index into a pen the kernel hands out; neither writes objects anywhere else.
  What this does not answer is the object a reference took and later abandoned:
  collecting those needs a scheduled trigger, and a trigger is a runner.
- A public key identifies by its material, not by its line. The comment is
  decoration and is ignored on lookup. The lookup itself is the one read that
  runs above the operator, because it runs before an operator exists — the
  same shape as verifying a token in `seam`.
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
- A merge has two doors and one implementation: name two references, or name a
  proposal and let its row supply them. Both may carry the head they expect and
  both refuse when it moved, because a merge is a write and every other write
  here states what it believed. A proposal records the commit its merge
  produced, never a flag saying it happened — a flag cannot be checked against
  the repository, and a commit can.
- **A proposal is not owned by whoever wrote it.** `Pull` is rooted at `Repo`,
  the `/propose` ceremony writes the row above the operator so keel mints no
  creator coverage, and the author's power arrives as one seeded predicate:
  edit your own proposal while it carries no merge. Keel checks a `set`
  predicate before and after, so that one clause is what makes the merge record
  unwritable by the author — no field-level authority is needed and none exists.
  Whoever holds the repository can still write it; that is the same principal
  the merge door already requires, and no model can defend against the holder
  of a wider token.
- Reference names are still refused when they carry a character that would
  break the query they are interpolated into. That is a guard against
  injection, not a statement about the model, and the two must not be confused
  when either one is next revised.
- The ground plane mounts only where a repository path is configured, and with
  none the api serves metadata and hosts no git at all. Losing that plane is a
  regression, not a simplification — it was deleted once and went unnoticed
  for eleven days.
