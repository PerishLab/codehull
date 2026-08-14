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
