# Handoff

## State

**Goal `core-csrf-and-3-more` is met.** `Core\Csrf`, `Core\Csv`, `Core\Db` and `Core\Db\Column`
each verify with nothing owed and every example and attack green, and the floor check `dossier:
the chain already names every emitted goal` is green again.

**That floor check was a regression, not open work.** A by-hand commit added
`docs/reference/tools/15-install.md`, so the derived roster grew five features no goal claimed.
`python tools/dossier.py --emit-goals` appended goal `tools-install` and renamed the three
`position: last` goals behind it; the chain is `1..176`, all walkable.

**`Core\Db\Connection::query` carries every feature proof**, measured at 17.64 µs over 92
allocations. Seven of the class's eleven members still owe theirs. The goal files for
`core-db-connection-and-2-more` were regenerated before `query` landed, so they list it as owed;
`python tools/dossier.py --id` is the live answer.

**One priority-1 finding is recorded rather than fixed.** A buffered `query` is not held to the
request's memory ceiling: a `select` of ten gigabytes ran under a `[limits] memory` of 256M with no
`FATAL`, while the same bytes accumulated from Novis stop at the ceiling. `crates/nvs-stdlib/src/db/mod.rs`
gap 1 owns it, owner M10, and the attack is marked `known-gap` for it.

## Next group

**Stage: one slice is one feature with all its proofs** — one file set:
`crates/nvs-stdlib/src/db/open.rs`, `crates/nvs-stdlib/src/db/execute.rs`,
`docs/examples/core/Db-Connection/`, `tests/hostile/core/Db-Connection/`,
`benches/members/core/Db-Connection/` and the root `nvs.toml`.

- [ ] **`Core\Db\Connection::isOpen`** — owes about, three examples, an attack, a bench and two
      tests, per `rule:testing/feature-proofs`. The member is
      `crates/nvs-stdlib/src/db/open.rs:1272`, and `close`'s own programs are the receivers its
      examples reuse.
- [ ] **`Core\Db\Connection::executeMany`** — same file set. The member is
      `crates/nvs-stdlib/src/db/execute.rs:1375` and its registry row is
      `crates/nvs-stdlib/src/db/registry.rs:320`.
- [ ] **`Core\Db\Connection::serverVersion`** — same file set, and its registry row is
      `crates/nvs-stdlib/src/db/registry.rs:422`. It answers between two rows of a live walk, which
      is the boundary its attack is written around.

## Backlog

- The buffered-read ceiling gap above needs a milestone that scopes it; M10 carries the runtime's
  own safepoint gaps, which is why it took the tag — `crates/nvs-stdlib/src/db/mod.rs` gap 1.
- `docs/agent/goals/dossier/105-core-db-connection-and-2-more.*` are one member stale; a later
  `--emit-goals` refreshes them.
- `Core\Db\Connection::stream`, `streamAs`, `queryAs` and `transaction` are the four members after
  the group above — `docs/agent/goals/dossier/105-core-db-connection-and-2-more.md`.
