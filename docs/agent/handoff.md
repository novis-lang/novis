# Handoff

## State

**Goal 21, stage 5 is the live stage**, and the driver's acceptance run still stops there: `native
examples/stream.nvs [5 db tail]` is item 8, and its fixture is still session 0003's stand-in. Only
the fixture is left of that item. Nothing regressed; conformance is 1553.

**Item 8's second slice is on disk and works against a real server.** `Core\Db\Connection::stream`
and `Core\Db\Transaction::stream` answer `Core\Db\Stream`
(`crates/nvs-stdlib/src/db/stream.rs:65`) — spec § 18's `Iterable<Db\Row>`, a registered memberless
class that is its own iterator, with `iterate`/`advance`/`current` on `instance`'s dispatch roster
and its element type declared in `registry::ITERABLES`. It holds the connection's key, the block
name and **one** row; the rows themselves stay on the connection as session 0004's `PgCursor`, so
memory is O(1) in the result set. § 10's literal check covers it: two rows joined
`nvs_types::intrinsics`, one per declaring class. `Core\Db\Transaction::stream` is the same member
under the same symbol, per ADR 0043, and carries its own three cases.

**Smoke-tested end to end** against `tests/db/compose.yaml`'s PostgreSQL, which is up: 10,000 rows
walked, the connection free for a `query` afterwards, and a statement issued mid-walk caught as
`LogicError` — the three facts the frozen check wants, less its second line. The scratch program and
its config are `.agent-tmp/stream-smoke.nvs` and `.agent-tmp/nvs.toml`; it is run from inside that
directory, because `nvs run` resolves the config against the *working* directory.

**`stream` lands on PostgreSQL alone**, and that is deliberate rather than unfinished: the other
four drivers have no parked read state, and buffering behind the caller would break both the
member's constant-memory promise and § 4's *uniform* connection-busy rule. They throw a
`RuntimeError` naming `query`. `crates/nvs-stdlib/src/db/mod.rs`'s gap 5 is the record; `streamAs`
is still owed whole.

Nothing is blocked on a decision.

## Next group

**Item 8's last slice, and the twin it shares every file with.** One file set:
`examples/stream.nvs`, `nvs.toml`, `crates/nvs-stdlib/src/db/registry.rs`,
`crates/nvs-stdlib/src/db/stream.rs`, `crates/nvs-stdlib/src/db/row.rs`.

- [ ] **`examples/stream.nvs` becomes the program its own comment describes**, and the acceptance
      check at `docs/agent/loop-goal.toml:4128` goes green — `examples/stream.nvs:1`. It needs an
      `[[app]]` block granting `db.connect = ["main"]` in the root `nvs.toml`, beside the two that
      already grant it — `grep -n 'examples/transaction.nvs' nvs.toml` lands on them.
      Lines 1 and 3 of the frozen `want` are already produced by the smoke program
      above; **line 2, `rows held at once: 1`, has no spelling yet** and deciding it is this
      slice's real work. The fixture's own comment says the count must be "read from the connection
      rather than from the program", and no `Core` member answers that today — `Core\Debug` has
      only `dump`/`render`. The two honest ways out are (a) count what the *program* holds and say
      so in the comment, or (b) amend the check's `want` in
      `docs/agent/goals/<goal>.toml` **and** the live copy, since a `[[check]]`'s wording is this
      goal's own file rather than an ADR. Do not invent a `Core` member for it: that is surface,
      and this goal's § *Standing decisions* does not pre-authorize one.
- [ ] **`streamAs<T>` joins `stream` the way `queryAs<T>` joined `query`** — ADR 0067 § 4 and spec
      § 18's `Queryable` row. The registry rows go beside `stream`'s at
      `crates/nvs-stdlib/src/db/registry.rs:313` and `:576` with a
      `CoreTy::Written("T")` element, the body beside
      `crates/nvs-stdlib/src/db/stream.rs:209`, and the per-row hydration is
      `crates/nvs-stdlib/src/db/row.rs:122`'s `hydrate` — the same call
      `nvs_core_db_rows_iterate` makes, moved to the `advance()` that reads the row. Its class
      needs a fourth slot for the descriptor, on `ROWS_CLASS_SLOT`'s pattern.

## Backlog
- Item 9's `serverVersion` is owed because no driver keeps the string — `crates/nvs-stdlib/src/db/mod.rs` gap 5.
- `stream` on the other four drivers is a `nvs-db` wire question, not a `Core\Db` one — same gap.
- § 4's `{timeout?}` and `stream`'s `{chunk?}` are both unspellable — same file, gap 6.
- Item 10's pool bounds for an `open` — gap 1, and the goal's standing decision names the answer.
