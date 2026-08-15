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
- `act` — the staged scenario runner over the api binary (`codehull act`).
  Acts 1 to 13 were one per stage of the closed program; the hosting stages
  take as many acts as their capabilities need.
- `ship` — the operator release that publishes both images and the chart at one version.
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
- `point` — the reference plane: a `Ref` row is the authority, the on-disk
  reference store is its projection. A move is `end` plus `put` in one batch,
  so Keel's liveness check is the compare and swap and no engine change was
  needed. Retiring is the `end` alone.
- `align` — reconcile the projection from the rows before serving an
  advertisement, so a crash between the two writes leaves the projection
  behind rather than ahead.
- `take` — the receive adaptor: the api speaks git's update protocol itself,
  reads the commands, lets `index-pack` write only the objects, and routes
  every reference move back through `point`. `git-receive-pack` never runs, so
  the kernel stays the only writer of references.
- `line` — pkt-line: the four-hex length framing git uses on the wire, and the
  advertisement and report-status shapes built from it.
- `verdict` — an assertion about one commit under one context. One live row
  per `(repo, commit, context)`; posting again ends the old row and puts a new
  one, the same replace `point` uses. Combined is a rollup — any failure wins,
  then any pending, otherwise success, and a commit nobody judged reads null.
- `verdict` is not `Run`. A run is one execution; a verdict is a claim about a
  commit. A commit can carry several verdicts under different contexts, and it
  can carry one with no run behind it at all.
- `weld` — the merge plane. `forward` moves the base when it is an ancestor,
  `join` writes a two-parent commit, `squash` writes a one-parent commit. The
  tree comes from `merge-tree --write-tree` and the commit from `commit-tree`,
  so the merge is git's own; only the choice of shape is Codehull's.
- `Weld` — one row per merge shape a repository permits. A repository with no
  rows permits nothing, on purpose: the permitted set is data, and a default
  living in code would be the plane deciding for itself.
- `admitted` — the namespace law in one predicate: anything below `refs` that
  is not in the reserved set, shared by the kernel and the adaptor so the two
  cannot drift. `RESERVED` is empty and gains an entry only when Codehull
  actually owns a namespace, never to stand in for something unbuilt.
- `order` — one `(old, new, name)` command inside an update request. A new
  object of all zeros is a deletion, and translates to retirement.
- `hall` — what `crates/ssh` asks of its host: admit a public key to an actor,
  and resolve a path to a repository seat. The crate speaks ssh and knows
  nothing about Keel; everything it decides, it asks for.
- `wanted` — whether an update request carries objects at all. A delete-only
  push sends no pack, so asking `index-pack` to read one hangs up the session.
- `port` — the api side of that trait. It resolves a key through a sudo read,
  because the lookup runs before an operator exists, and it resolves
  `owner/name` through ordinary operator-visible queries.
- `haul` — the transport plane: git's own smart HTTP over a ground seat,
  `info/refs` and `git-upload-pack` under the same `Bearer` every other route
  takes. The paths are git's specification, not a forge's API.
- `feed` — run git with bytes on stdin and raw bytes back. The protocol paths
  need it because trimmed text would corrupt the stream.
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
- `pen` — the object quarantine: `index-pack` writes into a pen inside the
  repository instead of into the object store, and the pen migrates in only
  when a reference has accepted its bytes. A refused push leaves the store as
  it was. Both transports index into a pen the kernel hands out; the ssh
  adaptor is told where its pen is, never where the store is.
- `keep` / `wipe` — the two ends a pen can have. `keep` moves the pack files
  in, indexes last so no reader sees an index before its pack, and then
  retires the pen; `wipe` retires it with the bytes still inside.
- `pull` — the merge door named after a proposal: base and head come from the
  `Pull` row, the merge is the same `weld` the repository door runs, and the
  row records the commit it produced. A proposal whose head lives in another
  repository is refused with a reason; that fetch does not exist yet.
- `weld` (the `Pull` field) — the commit this proposal's merge produced, empty
  until it is merged. It replaced a boolean, which could say a merge happened
  but never which commit it was. Open and closed stay on the `Issue`; who
  merged and when stay in the audit stream.
- `expect` — the head a merge is willing to merge. It is the caller's
  compare and swap: the seat refuses when the head moved under the expectation,
  the same refusal a reference move gives.
- `propose` — the proposal ceremony: the caller must be the issue's author, and
  the row is written above the operator so keel mints nobody coverage over it.
  Authority then comes only from the seeded predicate, which is the point: the
  row a person creates is a row they own outright, and a proposal must not be.
- A `Pull` is rooted at its `Repo`, not at its `Issue`. Rooting it at the issue
  handed the issue's author the whole subtree, so the seeded predicate could
  never bind — the proof of this is that act 3 caught it. The issue stays as an
  ordinary one-to-one relation: a proposal is described by an issue and belongs
  to a repository.
- `demand` — one context a branch requires before a proposal may merge into it.
  One row per context, hung on the branch's `Shield`; a shield with no demands
  requires nothing. The empty set means the empty set — the same reading `Weld`
  takes from the other side, where no rows permit nothing.
- `cleared` — the gate on the proposal door: every demand of the target branch
  must read `success` on the head commit, and the branch's approval count must
  be met. **Absence is not a pass**: a context nobody judged refuses, because a
  gate that treats silence as consent is not a gate. A refusal is 412 with the
  context and what it read, which is a different answer from 409, the one a
  reference that moved under the expectation gives.
- `align` writes only the difference. The projection is compared against the
  rows before anything is spawned, so a repository whose references have not
  moved costs nothing to advertise. Re-projecting unconditionally was correct
  and unusably slow: three `git` processes per reference, on every clone, fetch
  and push. A 453-reference repository — the estate has one — spent 4.6 seconds
  in process spawns before a single byte of pack.
- `thaw` — inflate a request body that arrives under `Content-Encoding: gzip`.
  Git compresses the upload-pack request once the want list is big enough, and
  a real repository crosses that line long before the acts did: a 453-reference
  mirror answered 422 to every clone while a hand-built request of the same
  wants answered 200. What the acts had proven was the small case, and the
  small case is the one nobody has. A body that inflates past 8 MB is refused
  rather than held — the largest real one measured is 4 KB.

