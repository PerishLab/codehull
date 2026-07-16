# Agents

This repository is a keel **caller**. It declares models, relations, and
seed grants; it never reaches into engine territory (reign, lifecycle,
authority, events, cache are keel's). The laws of the engine live in
`keel:docs/*`; this repo obeys them from the outside.

- Negentropy laws apply (single word, block/path <= 4, no comments);
  vocabulary deltas in `docs/vocabulary.md`.
- Dependency direction: forgejo -> keel-gate/keel-relay -> keel. Never a
  workspace sibling of keel; distribution follows keel's channel.
- Model changes are law-shaped: extend the slice in `docs/model.md`,
  then code.
- `cargo build && cargo run -- .` must stay green; scenario gates arrive
  with slice two.
