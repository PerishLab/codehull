# Hosting law

Codehull first hosts every capability it needs, and only then is it evaluated
how Plumb should integrate the estate's demands. That order is the law, not a
preference: a consumer's current API is not modelling pressure on the thing it
consumes. Deriving this surface from what Plumb happens to say today would be
the same act as deriving it from Forgejo's payload shapes, with the vendor
swapped.

A stage is done when its act is green under the repository guard; no calendar.

## What moved, and why

`docs/spec.md` settled a boundary with git mechanics — objects, pack,
diff/merge, CI execution — outside it, and closed at F7 declaring the
keel-ization capability-complete for that boundary. It was right for its
premise: Forgejo was to be kept, so the mechanics could stay there.

Forgejo is now being retired from the estate, and the standing estate rule is
that a forge-shaped need is a missing Codehull capability. Hosting bytes, refs,
packages and runs is the most forge-shaped need there is, so the old boundary
now excludes precisely what this repository has to grow.

The repository truth kernel already landed under the old boundary
(`crates/repo`, restored by `d7324df` after `590091c` deleted it) without
either document being corrected. This law records that rather than leaving two
live documents disagreeing on main.

## Stages

Ordered by how many parties each needs, not by dependency. A stage needing one
party can be finished; a stage needing five can only be advanced.

| Stage | Delivers | Parties |
|-------|----------|---------|
| H1 | change plane: git transport, `refs/tags/**`, real merge, commit status | codehull |
| H2 | registry plane: cargo sparse index and OCI distribution | codehull |
| H3 | execution plane: runner registration, queue, dispatch, logs, workflow and action resolution, artifacts | hardrig, ironbed, plumb, actions, codehull |

H2 precedes H3 because Codehull's own build pulls crates from the estate
registry and its own `ship` pushes images to it. With H2 after H3, hosting
cannot be true of Codehull itself for the whole of H3.

npm is out of these stages: Codehull publishes no npm package, so it is another
product's channel and belongs to whoever needs it.

## Acceptance

Acceptance is the capability, proven by an act against a running process,
**with Plumb absent**. A stage that can only be demonstrated by Plumb landing
through Codehull has proven integration, not capability, and integration is
the step after this law closes.

No stage ends with a switchover. Codehull becomes able to host; it does not
begin hosting. Switching the estate over is one later act, taken once, after
the evaluation this law defers.

Every capability arrives with its act. The acts are the only currency here: a
plane with no act is not delivered, whatever the guard says about the rest.

## Protocols, not vendor shapes

H2 implements cargo's sparse index and OCI distribution, both public
specifications. Writing to a specification is not API mimicry; writing to the
observed responses of `git.perish.top` is. The same test applies wherever a
protocol has to be spoken: implement the document, never the incumbent.

## Adaptors

A protocol Codehull did not design is spoken at exactly one declared seat, and
that seat is an adaptor. `seam` was the first — a foreign token format in,
an anchor row carrying `iss` and `sub` out — and every hosting stage adds
another: `haul` and `take` for git, then the registry protocols, then whatever
H3 needs to talk to a runner.

The must-not above forbids mimicry. This is the matching permission, and it
has an address: foreign shape may exist here and nowhere else. A prohibition
with no designated place for the thing it prohibits is not obeyed, only
hidden, and hidden foreign shape is how a model rots.

Five rules hold at every such seat.

**Negotiation is declaration.** Where a protocol negotiates, the capabilities
advertised are a machine-readable statement of what Codehull models. Not
advertising `delete-refs` would say deletion is not in this model — a stronger
and cleaner thing than refusing each attempt.

**An advertisement states the model, never the backlog.** Which is why
`delete-refs` *is* advertised: retirement is modelled. Declaring absence for
something merely unbuilt launders a gap into a design, and this rule exists
because that laundering was drafted once here before it was caught.

**An adaptor may translate, and may refuse. It must not decide.** Decisions
live in rows and grants; the adaptor reads them. A rule that can only be
written inside an adaptor is a missing unit, reported as one.

**Delete the adaptor and the model must still make sense.** If it does not,
pressure has already flowed backwards from the protocol into the model, which
is the failure this seat exists to prevent.

**What generalises is the discipline, not the machinery.** Git streams and
negotiates, a registry index is static files, a runner queue is long-lived;
an abstraction over all three would be too thin to carry anything and each
implementation would route around it. Three honest adaptors beat one that
fits none.

## What may enter the object store

**Bytes no reference has accepted never enter the store.** A push is indexed
into a pen — an object directory inside the repository that git reads only when
it is told to — and the pen becomes part of the store only once a reference has
passed every check. A refused push leaves the store exactly as it was, so the
one debt on this path that grew with time no longer grows.

The pen migrates **before** any reference is written. That order is the whole
reason the pen holds: a crash can then leave objects nobody points at, which is
waste, but never a reference pointing at objects that are not there, which is a
broken repository.

This is a rule of the write path, not of one seat: both adaptors index into a
pen the kernel hands out, and neither may write objects anywhere else. What the
pen does not answer is the accepted-then-abandoned object — a reference took
those bytes and later moved on. Collecting those needs a trigger on a schedule,
a schedule is a runner, and H3 owns runners. Naming the two separately is the
point: one is a hole and is now closed, the other is housekeeping and is owed
elsewhere.

## Identity

codehull issues no identity. `Actor` is an anchor carrying `iss` and `sub`
and nothing else; login, name, and suspension belong to the issuer. Gate's
stock doors are not mounted, so an operator exists only as a `Bearer` token
verified against `API_OIDC_ISSUER`. Rendering a person is the client's
round trip to that issuer, never a field codehull mirrors.

## Must not

- Gitea/Forgejo API mimicry (paths, payload shapes).
- Business code beyond models, grants, seeds, package wiring, and the hosting
  planes this law names. Each plane earns its code by appearing in the table
  above; nothing earns it by being convenient.
- Bytes in keel or metadata truth in MinIO (each plane owns its half).
- A stage advanced past a red act.
- Engine workarounds living here instead of keel issues.
- Forgejo as an implementation, fixture, adapter, fallback or docking target.

The second clause is the one amended out of `docs/spec.md`. Under the closed
boundary it read as models, grants, seeds and package wiring alone, which the
hosting planes necessarily exceed. It is narrowed rather than dropped: the
table above is the whole list of what may exceed it.
