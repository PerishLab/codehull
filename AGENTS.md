# Agents

Codehull is a keel **caller** and, in the long run, the estate's replacement
for GitHub. It declares models, relations and seed grants, and it hosts git
itself; it never reaches into engine territory, where reign, lifecycle,
authority, events and cache are keel's. The engine's laws live in
`keel:AGENTS.md` and are obeyed from the outside.

## Position

Codehull is not required to be usable on day 0. It joins the wharf system first
— built, guarded and delivered through Plumb and wharf on GitHub like every
other product — and strengthens from there, one hosting plane at a time. A
consumer's current API, Plumb's included, is not modelling pressure on a
plane: each plane is derived from the protocol it speaks and from what this
estate actually does.

The planes are ordered by how many parties each needs rather than by
dependency. A plane needing one party can be finished; a plane needing five can
only be advanced.

| Stage | Delivers | Parties |
|-------|----------|---------|
| H1 | change plane: transport, real merge, verdicts, the gate | codehull |
| H2 | registry plane: cargo sparse index and OCI distribution | codehull |
| H3 | execution plane: runner registration, queue, dispatch, logs, workflow and action resolution, artifacts | hardrig, ironbed, plumb, actions, codehull |

npm sits outside these stages: Codehull publishes no npm package, so that
channel belongs to whoever needs it.

## Acceptance

Acceptance is the capability, proven by an act against a running process,
**with Plumb absent**. A stage that can only be demonstrated by Plumb landing
through Codehull has proven integration, not capability, and integration is the
step after this law closes. Every capability arrives with its act: a plane with
no act is not delivered, whatever the guard says about the rest.

**No stage ends with a switchover.** Codehull becomes able to host; it does not
begin hosting. Switching the estate over is one later act, taken once, and that
act waits on somewhere to put it. Until Hardrig's absorption of infra lets the
infra repository be removed, Codehull is proven locally — an act against a
running process, on a workstation. No plane turning green moves this.

## Laws

- Ectropy owns syntax law: single word, block at most four, path at most
  three, no comments. Plumb owns repository shape and the canonical
  `ectropy.toml`.
- **The whole prose surface of this repository is `AGENTS.md`,
  `ARCHITECTURE.md` and `DESIGN.md`.** Anything else belongs in the code, or in
  the Concord task that owns the work. A fourth file is not a small exception;
  it is the shape this surface was closed to prevent.
- **Model changes are law-shaped: record the change in the Concord task that
  owns it, then write the code.** This law once named a row in a hand-kept
  capability ledger. The ledger is gone because a matrix of what is done drifts
  against a repository that already answers that question, while what is *not*
  done is exactly what no file in here can hold.
- Dependency direction: codehull -> keel-gate / keel-relay -> keel. Never a
  workspace sibling of keel; distribution follows keel's channel.
- Engine gaps become keel issues and registry releases, never local
  workarounds.
- Never commit on `main`. Branch, let the guard prove the commit, then land
  it through Concord or `plumb land`.
- One Concord member, one branch, one cut. A second cut on a member branch
  reuses a projection and stalls.

## Must not

- Gitea or Forgejo API mimicry, in paths or in payload shapes. Forgejo is not
  an implementation, a fixture, an adapter, a fallback or a docking target.
- Business code beyond models, grants, seeds, package wiring and the hosting
  planes the stage table names. A plane earns its code by appearing in that
  table; nothing earns it by being convenient.
- Bytes in keel, or metadata truth in MinIO. Each plane owns its half.
- A stage advanced past a red act.
- Engine workarounds living here instead of keel issues.

## Territory

`crates/api` serves, `crates/cli` operates, `crates/repo` holds repository
truth, `crates/ssh` is the second transport adaptor, `apps/web` is the web face,
`deploy` holds the api image recipe and `charts/codehull` is the delivery. What each owns, and the mechanisms they
land on, is `ARCHITECTURE.md`; why the boundary sits where it does is
`DESIGN.md`.

`apps/web` is a placeholder `index.html` only: the web plane is deferred, not
deleted, and the pnpm workspace files hold its seat. When it returns, its
components live under `apps/web/src/lib/components`, stay style-free, and take
reusable visual behavior from the Design runtime; every version is pinned in
the `pnpm-workspace.yaml` catalog and dependencies reference `catalog:` only.

Plumb's pre-commit guard proves every commit against its exact staged tree, and
`plumb guard .` shows what it runs. The deno act scripts under
`crates/cli/scripts` sit outside biome; `deno fmt` and `deno check`, `helm
lint` and the acts are run by hand when a change touches them.
