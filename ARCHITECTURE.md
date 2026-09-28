# Architecture

Codehull is one api, one operator cli, a repository truth kernel and two
transport adaptors, all landing on the same keel operations. Keel owns metadata
and references; the git store owns objects. Every plane below is a way of
speaking a protocol Codehull did not design onto rows Codehull does.

## Workspace

- `crates/api` — the server and the keel caller (bin `api`). Routes, seam,
  warden, the ground plane, and the model declarations.
- `crates/cli` — the client and the operator (bin `codehull`). Owns `act`.
- `crates/repo` — the repository truth kernel: ordinary git plumbing behind a
  private crate, reached only through the api's ground routes.
- `crates/ssh` — the second transport adaptor, with its own framing, credential
  translation and naming.
- `apps/web` — a placeholder `index.html` holding the deferred web plane's
  seat.
- `charts/codehull` — the helm delivery.

## Boot

Boot is two ceremonies. `bootstrap` mints the sudo token into its declared
artifact and sows the grants; `serve` refuses to start when those grants are
absent. `seat` is the receiver over a repository root — read the cascade, open
the store, run one ceremony — and `berth` the receiver over a raised core,
where `seed` sows gate and codehull grants and `rig` verifies them.

An `artifact` is a declared destination for a durable possession, created once
and never overwritten; absent one, bootstrap keeps the token at `.local/sudo`.
The kubernetes destination `seal` has the binary write and read its own Secret
through the service account, so no operator ever handles the token.
Configuration is the Plumb cascade over `codehull.toml` and the `API_` prefix:
keel itself reads no files and no environment.

```sh
cargo run -p api -- bootstrap .
cargo run -p api -- serve .
cargo run --locked -p codehull -- act
```

`act` runs the staged scenarios against a real process.

## The seam

Codehull issues no identity. An operator exists only as a `Bearer` token some
issuer signed, verified against the JWKS at `API_OIDC_ISSUER`. `warden` reads
the issuer's discovery and keys once at boot, then checks algorithm, issuer,
audience, expiry and kind. `anchor` is the row a verified subject hangs on:
`iss` and `sub`, no login, no name, no suspension flag. It is born through
gate's identity birth so it carries its own self grant — a plain put would make
a row its own operator cannot touch. Rendering a person is the client's round
trip to the issuer, never a field Codehull mirrors.

Reaching a repository is Keel's decision, never a second one taken here. Every
entry point declares the reach it needs — reading answers whoever the
repository is visible to, writing also refuses an archived one — so a team
membership or a grant carries to git the way it carries everywhere else.

## The reference plane

A reference is a `Ref` row and the on-disk store is its projection. A move is
`end` plus `put` in one batch: Keel's own liveness check is the compare and
swap, and ending releases the name because live uniqueness is a partial index.
Retirement is the `end` alone, which is what git's all-zero object id means.

- `point` writes many updates in one `update-ref --stdin -z`; `project` and
  `retire` are its one-element cases, so there is a single path onto the store
  rather than two that can drift.
- `align` writes only the difference, comparing projection against rows before
  spawning anything. Re-projecting unconditionally was correct and unusably
  slow: three git processes per reference on every clone, fetch and push, which
  cost a 453-reference repository 4.6 seconds before a single byte of pack.
- `seats` reads a repository's reference rows once per request instead of once
  per name. Staleness is not a risk, because the authority is Keel's `end`
  refusing a row someone else moved, not the value this snapshot read.
- `absent` asks git once, with `cat-file --batch-check`, which pushed objects
  are missing, and reads the same peel the projection will demand later — so a
  reference is refused before Keel is written rather than after.
- `mark` is the on-disk reference plane and the namespace law; it left the
  kernel's main file when that file crossed its limit, split by subject rather
  than by size.

## Namespace law

`admitted` is the law in one predicate: anything below `refs` that is not in
the reserved set. The reserved set is what Codehull itself owns, and today it
is empty, so every namespace travels — heads, tags, notes alike. Two earlier
rulings narrowed this, first to heads and then to heads and tags, and both
wrote current usage down as if it were the model. Refusing a name for being
unmodelled is only honest when the thing really is outside the model. A tag
moves and retires by the rules a branch does; making tags immutable would be
release governance, and that belongs to Plumb, not to a transport.

Reference names are still refused when they carry a character that would break
the query they are interpolated into. That is a guard against injection, not a
statement about the model, and the two must not be confused when either is
revised.

## Transport

`haul` is git's own smart HTTP over a ground seat: `info/refs` and
`git-upload-pack` under the same `Bearer` every other route takes. Those paths
are git's specification, not a forge's API. `take` is the receive adaptor: the
api reads the command section itself, decides against Keel, hands only the
packfile to `index-pack`, routes every move back through `point`, and writes
its own report-status. **`git-receive-pack` runs on neither transport**, so the
kernel stays the only writer of references.

`line` is pkt-line, the four-hex framing and the advertisement and
report-status shapes built from it. `order` is one `(old, new, name)` command
inside a request, `wanted` is whether the request carries objects at all — a
delete-only push sends no pack, and asking `index-pack` to read one hangs the
session — and `feed` runs git with raw bytes both ways, because trimmed text
would corrupt the stream. `thaw` inflates a body that arrives gzipped: git
compresses the upload-pack request once the want list is big enough, and a real
repository crosses that line long before the acts did. A body inflating past
8 MB is refused rather than held.

Both transports stream, in both directions. `sift` reads the command section off
the request until its flush, hands the rest to `index-pack` as it arrives, and
never holds the pack; `draw` runs `upload-pack` and pours its output into the
response as it is written. The memory a transfer costs is the depth of one
channel, not the size of the repository — which also means the status line goes
out before the outcome is known, the same bargain git's own protocol makes. The http half used to take a buffered body, which capped every push at
the framework's two megabytes — small enough that no repository this estate
keeps could have been pushed over http at all. `hall` is what the ssh crate
asks of its host — admit a public key to an actor, resolve a path to a
repository seat — and `port` is the api side of it. A public key identifies by
its material, never by its comment. That lookup is the one read running above
the operator, because it runs before an operator exists, the same shape as
verifying a token in the seam.

The ground plane mounts only where a repository path is configured; with none,
the api serves metadata and hosts no git at all. Losing that plane is a
regression, not a simplification — it was deleted once and went unnoticed for
eleven days.

## The pen

**Objects no reference has accepted never enter the store.** `index-pack`
writes into a `pen` inside the repository, handed out by the kernel, and the
whole request is checked against Keel first. Only when some reference actually
accepts the bytes does the pen migrate; if none does, it is wiped. `keep` moves
the packs in and indexes last, so no reader sees an index before its pack;
`wipe` retires the pen with the bytes still inside.

**The pen migrates before any reference is written.** That order is the whole
reason it holds: a crash can leave objects nobody points at, which is waste,
but never a reference pointing at objects that are not there, which is a broken
repository. Both adaptors index into a pen; neither writes objects anywhere
else. The ssh adaptor is told where its pen is, never where the store is.

## Merge

`weld` is the merge plane and its shapes are data — one `Weld` row per shape a
repository permits, and a repository with no rows permits none. There is no
default in code, because a default in code is this plane deciding a policy
question for itself. `forward` moves the base when it is an ancestor, `join`
writes a two-parent commit, `squash` a one-parent commit; the tree comes from
`merge-tree --write-tree` and the commit from `commit-tree`, so the merge is
git's own and only the choice of shape is Codehull's.

A merge has two doors and one implementation: name two references, or name a
proposal and let its row supply them. Both may carry the head they `expect` and
both refuse when it moved, because a merge is a write and every other write
here states what it believed.

A `Pull` is rooted at its `Repo`, not at its `Issue` — rooting it at the issue
handed the issue's author the whole subtree, and act 3 caught it. `propose`
writes the row above the operator, so keel mints nobody coverage over it, and
the author's power arrives as one seeded predicate: edit your own proposal
while it carries no merge. Keel checks a `set` predicate before and after the
change, so that single clause is the entire mechanism making the merge record
unwritable by its author. The row records the commit its merge produced, never
a flag saying it happened: a flag cannot be checked against the repository, and
a commit can. Open and closed stay on the `Issue`; who merged and when stay in
the audit stream.

## Verdicts and the gate

A `verdict` is an assertion about a commit, not a record of an execution. `Run`
stays what it was: a commit may carry verdicts under several contexts, and one
with no run behind it. One live row per repository, commit and context; posting
again ends the old row and puts a new one, the same replace `point` uses.
Combined is a rollup — any failure wins, then any pending, otherwise success —
and a commit nobody judged reads null. Landing decisions read that rollup, so
the rule is law rather than an implementation detail.

`Demand` is one context a branch requires, hung on the branch's `Shield`; a
shield with no demands requires nothing. `cleared` is the gate on the proposal
door: every demand of the target branch must read success on the head commit
and the approval count must be met, or the merge refuses with 412 naming the
context and what it read — a different answer from the 409 a moved head gives.
**Absence is not a pass**, because a gate that treats silence as consent is not
a gate. Combining is conjunction: no priority, no override, no bypass list. Who
may act at all is a grant question, not a column on a policy row.

The gate is on the proposal door only. Naming two references is a
repository-level action and stays ungated. Until a runner exists, verdicts can
only be written by whoever holds the repository, so this gate does not defend
against them and must not be described as though it does.

## Adaptors

A protocol Codehull did not design is spoken at exactly one declared seat, and
that seat is an adaptor: `seam` for a foreign token, `haul` and `take` for git,
`crates/ssh` for its own transport, and whatever the registry and runner
protocols need next. The must-not against mimicry has this as its matching
permission, and it has an address: foreign shape may exist at these seats and
nowhere else. A prohibition with no designated place for the thing it prohibits
is not obeyed, only hidden.

Five rules hold at every seat.

**Negotiation is declaration.** Where a protocol negotiates, the capabilities
advertised are a machine-readable statement of what Codehull models.

**An advertisement states the model, never the backlog.** Which is why
`delete-refs` is advertised: retirement is modelled. Declaring absence for
something merely unbuilt launders a gap into a design.

**An adaptor may translate, and may refuse. It must not decide.** Decisions
live in rows and grants; the adaptor reads them. A rule that can only be
written inside an adaptor is a missing unit, reported as one.

**Delete the adaptor and the model must still make sense.** If it does not,
pressure has already flowed backwards from the protocol into the model.

**What generalises is the discipline, not the machinery.** Git streams and
negotiates, a registry index is static files, a runner queue is long-lived; an
abstraction over all three would be too thin to carry anything, and three
honest adaptors beat one that fits none.
