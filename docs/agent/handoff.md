# Handoff

## State

**ADR 0067 § 6's "the requested type drives the conversion" is now implemented as well as
specified.** `nvs_stdlib::db`'s `requested_bool`/`requested_int`/`requested_uint` are the rule's one
home, and both surfaces § 6 states it for route through them: `Core\Db\Row`'s `->bool()`, `->int()`
and `->uint()` readers, and `queryAs<T>`'s `bool`/`int`/`uint` fields through `converted`. A
`TINYINT(1)` — which § 9 gives the `int` row, MySQL having no boolean column for the map to point at
— reads as `bool` on request, and a stored `7` throws rather than becoming PHP's `true`. Before this
the reader refused the crossing outright and `ROW_BOOL_DOC` said so in as many words, which was a
card stating the opposite of its ADR.

**Stage 4's acceptance check was two checks wearing one name.** Four of its eight tests are § 9 rows
that are *rules* rather than mappings, and none of them is a question `nvs-db` can be asked — see the
playbook bullet. They are now a `-p nvs-stdlib` check of their own, in both `docs/agent/loop-goal.toml`
and `docs/agent/goals/5-database.toml`; the four wire-level ones stay where they were. Note the two
goal files are **not** byte-identical any more — the live one carries ADR 0133's stage 0, which the
goals copy never gained — so an edit has to be applied to both by hand rather than by copying.

**`nvs_stdlib::db`'s known gap 1 is untouched and is the next item**: `open` still files its
connection with no lease, because § 13 keys an `open` pool on a hash of every settings field and
`nvs_runtime::pool::Ticket::for_block` takes a block *name*. Within a request § 2's memo holds.

## Next group

**§ 13's pool for `open`, then the two arms' own conformance depth. File set:
`crates/nvs-runtime/src/pool.rs` with `crates/nvs-stdlib/src/db.rs`.** Unchanged from the last
handoff — this session spent itself on the acceptance check ahead of it, which outranked the group.
The first item is the group's weight; the second and third are over landed work and share the second
file.

- [ ] **A ticket keyed on the settings hash, so `open` pools** (0067 § 13).
      `crates/nvs-runtime/src/pool.rs:1` is the pool and `Ticket::for_block`'s block-name key;
      `crates/nvs-stdlib/src/db.rs:2995` is the merged-slot list `settings_key` hashes, and
      `crates/nvs-stdlib/src/db.rs:64` is known gap 1, which this closes. The key is § 2's — the
      hash, scoped to the configuration generation it was read from, exactly as a named block's is.
- [ ] **`{shared: false}` still draws from and returns to that pool** (0067 § 13).
      `crates/nvs-stdlib/src/db.rs:3055` is where the lease is filed for an unshared `open` and the
      comment there already states the rule; the case is that the memo is bypassed and the pool is
      not.
- [ ] **`open`'s reset is the one a failed reset destroys** (0067 § 13).
      `crates/nvs-runtime/src/pool.rs:1` again — an `open` connection is reset on the same terms a
      `connect` one is, and a reset that fails closes it rather than filing it.

## Backlog

- § 6's readers throw `LogicError`, not § 8's `DbError` — `nvs_stdlib::db` known gap 4 owns why.
- `converted`'s `Float` arm has no `int`-widens crossing; § 6 names none, so this is deliberate and
  is recorded only here in case a later section adds one.
- Stage 5 onward of `docs/agent/loop-goal.toml` is unread by this session.
