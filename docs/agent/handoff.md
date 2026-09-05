# Handoff

## State

**ADR 0116 § 2's sweep reads array elements, and stage 4's three checks are green.** The tally and
the mark walk share one reader, `member_targets` in `crates/nvs-runtime/src/object.rs`, which counts
an object a field slot names *and* an object held by an element of an array that member solely owns
— a reference count of one on the array being what makes the element countable, since the reference
the walk just found is that one owner. The descent carries into a nested array on the same test, and
a uniquely owned array cannot contain itself, so the arrays reached from one object are a tree and
the worklist drains without its own cycle check. `sweep`'s doc comment is that rule's home.

**A shared array is still an external hold**, which is the goal's § *Standing decisions* item 7
direction and the only one the widening is allowed to move anything: an array above a count of one
has an owner this walk cannot name, so its elements are not tallied and the objects behind them are
left exactly where they were. A `debug_assertions` assertion pins it — a tally may never exceed the
reference count it is compared against, because a tally that overshot would free something somebody
still holds. `crates/nvs-runtime/src/lib.rs`' known gap 7 now names the shared array as what is left
rather than arrays as a whole.

`examples/cycles.nvs` builds both shapes a thousand times over and prints nothing new, because the
fixture's stdout is frozen by the stage 1 floor check; the WSL `valgrind --leak-check=full` leg is
the whole check on them and it is green. Nothing is blocked on a decision.

## Next group

**Stage 5 — `Core\Db` answers spec § 18's tail.** Five of that stage's six named tests do not exist
yet; `close_releases_the_connection_and_a_later_member_refuses` already does, in
`crates/nvs-stdlib/src/db/open.rs`. One file set: `crates/nvs-stdlib/src/db/stream.rs`,
`crates/nvs-stdlib/src/db/pool.rs`, `crates/nvs-stdlib/src/db/open.rs`,
`crates/nvs-stdlib/src/db/execute.rs`, `crates/nvs-config/src/db.rs`.

- [ ] **ADR 0067 § 4's `stream` is pinned as the member that does not buffer**, with
      `stream_answers_rows_without_holding_the_result_set` and
      `a_second_statement_on_a_streaming_connection_is_a_logic_error` — the second is the sharper
      of the pair, since a driver that had buffered would have finished its statement and answered
      the second query instead of refusing it — `crates/nvs-stdlib/src/db/stream.rs:1`,
      `crates/nvs-stdlib/src/db/execute.rs:1287`.
- [ ] **Item 10's pool answer lands**, per the goal's § *Standing decisions*: an `open`'s bounds come
      from the `[db.<name>.pool]` table of the block whose settings hash the connection was opened
      under, `PoolBounds::DEFAULT` when there is none, and `pool = false` is writable **unscoped** so
      it reaches a program's own connection too —
      `an_open_reads_its_pool_bounds_from_the_blocks_pool_table` and
      `an_unscoped_pool_false_reaches_a_program_opened_connection` —
      `crates/nvs-stdlib/src/db/pool.rs:289`, `crates/nvs-config/src/db.rs:180`.
- [ ] **ADR 0067 § 13's amendment is folded into the ADR body**, not left as an overlay, once the
      slice above lands — `python tools/adr.py --fold` writes both halves and the body edit is the
      half no tool does — `docs/adr/0067-core-db.md:1`, `crates/nvs-stdlib/src/db/pool.rs:289`.
- [ ] **A statement timeout reaches the socket**, with
      `a_statement_timeout_reaches_the_socket_and_throws_on_expiry` —
      `crates/nvs-stdlib/src/db/execute.rs:1287`, `crates/nvs-config/src/db.rs:180`.

## Backlog

- Goal 27's stage 3 item 4 is **already answered by the tree**: `router.rs`'s gap 3 is about
  `Core\Request::method`'s verb parse, not about `Core\Router::match` being absent, so the goal file
  is the stale half — `docs/agent/goals/27-gap-owners.md:70`, `crates/nvs-stdlib/src/router.rs:60`.
- **ADR 0102 § 1 gains the sentence saying where a capture is decoded**, once a goal's standing
  decisions admit that ADR — `docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md:96`,
  `crates/nvs-stdlib/src/router.rs:1007`.
- The eight spec §§ 16-17 classes still have no owner on the chain — `docs/agent/carried-gaps.md`.
