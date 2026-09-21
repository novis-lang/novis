# Handoff

## State

Goal `core-db-rows-and-1-more` — the generated dossier goal over `Core\Db\Rows` and `Core\Db\Schema` —
has 7 of its 11 items landed. All six `Core\Db\Rows` members carry their feature proofs, and so does
`Core\Db\Schema::fromArray`; `python tools/dossier.py --id '<feature>'` prints `complete.` for every one
of them. Nothing is blocked. What is left is four `Core\Db\Schema` members: `toArray`, `planAgainst`,
`applySafe` and `applyIncludingRisky`.

`fromArray`'s attack found a real crash and it is fixed here: `node_of` and `array_node` recursed once
per level of a nested array with no bound, so an array a program wrapped twenty thousand times ended
the process with `has overflowed its stack` and no diagnostic. `DEPTH_LIMIT` at
`crates/nvs-stdlib/src/db/schema.rs:34` is the bound, sixteen arrays against a form that nests six, and
both sides of it are pinned by a `.nvst`. Other members that turn a Novis array into a Rust tree have
**not** been probed for the same thing — the `## Backlog` carries it.

Three things the next session should not re-derive. `crates/nvs-stdlib/src/db/schema.rs`'s `mod tests`
now holds `released`, `smallest()` (the one-table array form) and, from before, `said(&Fault)` for a
refusal's sentence and `connection(block)` for a receiver that reaches no driver — and a test in that
module can call `node_of` directly rather than through `nvs_runtime::call`, which is how a refusal's
message is read without installing an exception class table. `Core\Db\Schema`'s proofs need **no**
`[[app]]` entry unless the program opens a database, because reading and writing the array form asks
for no capability; the one program that applies its schema holds `connect` and `schema` for `notes`.
And an exception's message is `$e->message`, not `getMessage()`.

## Next group

**One file set for all three:** `crates/nvs-stdlib/src/db/schema.rs` (the members and its `mod tests`),
`crates/nvs-stdlib/src/db/registry.rs` (the reference cards), `nvs.toml` where a program opens a
database, and each feature's own proof trees. One slice is one feature with all of
`rule:testing/feature-proofs`.

- [ ] **`Core\Db\Schema::toArray`** — owes examples, hostile, perf, tests. The member is
      `crates/nvs-stdlib/src/db/schema.rs:221` and the card `crates/nvs-stdlib/src/db/registry.rs:1618`;
      the claim worth pinning is that the slot is handed on rather than rebuilt, so two calls answer the
      same array, and that every spelling of one schema answers the same one.
- [ ] **`Core\Db\Schema::planAgainst`** — owes examples, hostile, perf, tests. The member is
      `crates/nvs-stdlib/src/db/schema.rs:696` and the card
      `crates/nvs-stdlib/src/db/registry.rs:1627`; it needs `db.connect` for `notes` and nothing is
      changed by computing a plan, which is the claim an attack should try to break.
- [ ] **`Core\Db\Schema::applySafe`** — owes examples, hostile, perf, tests. The member is
      `crates/nvs-stdlib/src/db/schema.rs:710` and the card
      `crates/nvs-stdlib/src/db/registry.rs:1636`; it needs `db.schema` as well, and the refusal naming
      `applyIncludingRisky` is already pinned one `mod tests` down.

## Backlog

- Other `Core` members that turn a Novis array into a Rust tree are unprobed for the unbounded
  recursion `DEPTH_LIMIT` fixed here — `crates/nvs-stdlib/src/db/schema.rs`'s module doc is where the
  bound is explained.
- `Core\Db\Schema::fromArray` spends 183 allocations and 10 KB reading a one-table schema
  (`docs/perf/members.ndjson`); it runs once at start-up, so this is recorded rather than owed.
- `Core\Db\Schema::applyIncludingRisky` is the fourth and last item of this goal.
