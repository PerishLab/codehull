# Agents

This repository is a keel **caller**. It declares models, relations, and
seed grants; it never reaches into engine territory (reign, lifecycle,
authority, events, cache are keel's). The laws of the engine live in
`keel:docs/*`; this repo obeys them from the outside.

- Negentropy laws apply (single word, block/path <= 4, no comments);
  vocabulary deltas in `docs/vocabulary.md`.
- Dependency direction: codehull -> keel-gate/keel-relay -> keel. Never a
  workspace sibling of keel; distribution follows keel's channel.
- Model changes are law-shaped: extend the slice in `docs/model.md`,
  then code.
- Stage law: `docs/spec.md` (F0 foundations → F7 equivalence audit).
- Never commit on `main`; branch, then `runseal :guard` and `runseal :land`.
- Engine gaps become keel issues and registry releases, never local
  workarounds.
