# Design

Codehull began as an absorption touchstone: keel's laws were pressure-tested
against a full forge data model on paper, and this repository turned that audit
into a running system built purely as a keel caller. That thesis is settled.
What the repository is now deciding is a different question — what a forge must
host, and what it must refuse to become while hosting it.

## Why the boundary moved

The first program settled a boundary with git mechanics outside it: objects,
pack, diff and merge, CI execution were app territory, and state, authority,
events, credentials and blobs were modelled. It closed complete against that
boundary, and it was right for its premise — Forgejo was to be kept, so the
mechanics could stay there.

Forgejo is now being retired, and the standing estate rule is that a
forge-shaped need is a missing Codehull capability. Hosting bytes, references,
packages and runs is the most forge-shaped need there is, so the old boundary
excluded precisely what this repository has to grow. The repository truth
kernel had already landed under the old boundary without either document being
corrected; the hosting law records that rather than leaving two live documents
disagreeing on main.

## Hosting before integration

The capability surface is derived from what this estate actually does, and
never from what its consumers currently say. Letting Plumb's present API decide
Codehull's model is the same act as letting a vendor's payload shapes decide
it — the must-not names the vendor case, and the consumer case only escapes
notice because it wears a friendlier name.

The first version of this task had the order backwards, and its open question
offered two answers: grow a Codehull driver inside Plumb, or grow inside
Codehull the vocabulary Plumb already speaks. Both were wrong, and for the same
reason. **A consumer's current API is not modelling pressure on the thing it
consumes.** Missing capability is repaid on its own terms; whether the pieces
fit is an evaluation held afterwards, and holding it first turns a capability
surface into an endorsement of an existing integration.

## Protocols, not vendor shapes

Every hosting stage speaks a protocol somebody else designed. Writing to a
published specification is not mimicry; writing to the observed responses of a
running incumbent is. Git's smart HTTP paths, cargo's sparse index and OCI
distribution are documents. Implement the document, never the incumbent.

That distinction is what lets a stage advance without colliding with the
must-not, and it is why the two are stated together rather than as separate
rules.

## A whole git, not this estate's subset

Where Codehull hosts git, it hosts the whole of it. The estate happening to use
only branches is a fact about the estate, not about the model, and a model that
encodes current usage will refuse the first thing that changes. Two rulings
narrowed the namespace before this one — heads alone, then heads and tags — and
both had to be reversed. The cost of the narrow model was not theoretical: the
estate's exact releases trigger on tags, so the kernel structurally could not
publish the estate it was being built for.

Refusing a name for being unmodelled is only honest when the thing really is
outside the model. Everything else is a backlog wearing a design's clothes.

## Scale of the closure

The closure this repository owes is *everything this estate needs*, which is
not the same as everything a forge can do and not the same as the estate's
current usage either. Two rules divide those:

- **Shape is decided by completeness.** Once something is done, it is not done
  as a crippled subset. Notes travel because refusing them would be a lie about
  the model, not because anything here uses notes.
- **The list and the timing are decided by need.** What gets built, and when,
  comes from what this estate actually does.

Under that scale H1 is closed: transport over http and ssh, every namespace
below `refs`, objects quarantined until a reference accepts them, real merge in
three shapes, verdicts on a commit, and a proposal that merges through its own
door behind the checks its branch demands. H2 registry and H3 execution remain,
and what Codehull records about them it already records well — the gap is
doing, not recording.

## Not owed here

Named with reasons, so the next reader does not rebuild them by accident.

- **Caller-space authentication** — two-factor, passkeys, external login. The
  gate is the seam: Codehull issues no identity and an operator exists only as
  a verified `Bearer`, so these are middleware a caller adds.
- **Charted scope modeling** — organization-scoped labels, runners and secrets
  are the same two-unit pattern already shipped, added when a live need
  appears. Keel has typed relations and no polymorphism, so one table binding
  any rule to any object is not expressible without an untyped escape.
- **Minor units on existing primitives** — stopwatches, pinned issues, issue
  templates, wiki metadata, avatars. Each is fields on a relation, added on
  demand, and listing them as debt would overstate what is missing.
- **Branch deletion** — the kernel has no such face on purpose, and the estate
  keeps its branches after merge. The two agree.
- **Rebase merges, auto-merge, merge queues, deleting a branch after merge** —
  the first two need a scheduled trigger, and a schedule is a runner; the last
  two are client policy. Use them and they arrive; until then they are not
  owed.
- **Protocol v2, atomic push, push options** — client-side negotiation and
  options nothing in this estate sends.
- **Derived state that can go stale** — whether a proposal merges cleanly is
  computed when asked, never stored.
- **A dry-run mode for the gate** — a rule not installed does not stop
  anything, and one installed stops it for real. Dry runs exist for estates
  where policy is handed down by somebody else.

## What the gate cannot do

Until a runner exists, verdicts can only be written by whoever holds the
repository, so the gate on the proposal door does not defend against that
holder. This is acceptable in a single-owner estate of twenty repositories,
and it is written down because the one thing a gate must never do is claim a
defence it does not have. The defences that do work are elsewhere: scoped
credentials under keel, credential isolation under Runseal, boundary proofs
under Concord, and the guard Plumb runs before a landing.

For the same reason a verdict carries no producer field. Restricting judgement
to a particular writer defends nothing that a narrow token does not already
defend, and a wider token's holder cannot be stopped by asking who they are.

## Where the debts live

The debts this repository still carries are recorded in the Concord task that
owns each of them, not here. A file that lists what is done drifts against a
repository that already answers that question; a file that lists what is not
done competes with the task record that has to hold it anyway. The unclosed
parts of H1 — conflict presentation, cross-repository proposals, draft
proposals, the unbounded git processes, the http body still buffered whole, and
the objects a reference accepted and later abandoned — are named there, with
the measurements that make them decisions rather than opinions.
