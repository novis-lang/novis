# Handoff

## State

**Goal `sqlite-queue`, stage 2 — the keystone — is landed except its ADR.** Both of stage 2's
`[[check]]` blocks now name tests that exist and pass.

- `SqliteConn::begin_immediate` is at `crates/nvs-db/src/sqlite.rs:763`: its own entry point, not a
  sixth `Isolation` case, refusing any depth above zero and moving the depth only after a command
  SQLite accepted. Four cases hold it, all over two connections.
- `queue::CLAIM_SQLITE` is at `crates/nvs-stdlib/src/queue.rs:713`, with no locking clause and the
  sentence saying why one would be a syntax error rather than an improvement.
- `crates/nvs-stdlib/tests/queue_sqlite.rs` executes it: three cases, two connections over one
  in-memory database, no container and no matrix gate. It is the first queue suite that runs on the
  default `python tools/verify.py` legs.
- **Nothing past the claim exists.** `Queued` at `crates/nvs-stdlib/src/queue.rs:2224` still has two
  arms, so `Core\Queue` refuses a SQLite block at `no_dialect` and `nvs serve` starts no worker for
  one. That is stage 3's and stage 4's whole subject.
- Nothing is blocked, and no design call is waiting on the user.

## Next group

**Stage 2's ADR, then stage 3's seam** — one file set: `crates/nvs-stdlib/src/queue.rs`,
`crates/nvs-cli/src/worker.rs`, and the rulebook pair the ADR edits.

- [ ] **The ADR** — `docs/rules/concurrency/claiming-is-one-statement.md:1` and that rule's entry in
      `docs/rules/concurrency.json:1`. This goal's one slot: the claim, the primitive, and where the
      primitive sits. A record touching no rule is not a decision (`conventions.md` § *A decision
      record*), and the fragment already names the immediate transaction — so what its
      `changes.modifies` buys is the sentence the fragment does not have yet: on the one backend
      with a single writer the exclusion *is* the immediate transaction, which is why that dialect's
      claim carries no locking clause and cannot be given one. Re-read `docs/decisions/` for the
      free number before creating the file, and finish with `python tools/rules.py --render`.
- [ ] **The dispatch seam** — `crates/nvs-stdlib/src/queue.rs:2224` (`Queued`, two arms) and
      `queue_connection` at `crates/nvs-stdlib/src/queue.rs:2192`, which sends SQLite to
      `no_dialect` at `crates/nvs-stdlib/src/queue.rs:2253`. Every statement stages 3 and 4 owe
      needs a third arm here first, and `no_dialect`'s SQLite sentence — "one text away" — is what
      stops being true the day it lands.
- [ ] **`INSERT_SQLITE`**, beside `CLAIM_SQLITE` at `crates/nvs-stdlib/src/queue.rs:713`.
      `INSERT_MYSQL` is the `Split` shape and `rule:core-classes/queue-storage-is-a-table`'s
      `dedupe_pending` is the column it maintains; `tests/queue_sqlite.rs`'s own `push` fixture is
      the hand-written insert a real one replaces.
- [ ] **The worker's claim path** — `crates/nvs-cli/src/worker.rs:456` (`claimed_in_two`) runs the
      pair for `Framed` inside a `Framed` transaction. SQLite needs the same two statements inside
      `begin_immediate`, and the columns are already at one set of ordinals for all three dialects.

## Backlog

- Stage 3's remaining statements — dedupe, the dead-letter move, the retry ladder, the roster —
  `docs/agent/loop-goal.md` § *Stage 3*.
- Stage 4's six members on SQLite, and stage 5's diagnostics — same file, § *Stage 4* and § *Stage 5*.
- `[queue] workers` owes a sentence where an operator reads it, saying where the number stops buying
  throughput against a single-writer database — the goal's § *Standing decisions*.
- `crates/nvs-stdlib/tests/queue.rs:209` refuses SQLite in `open` and stays that way on purpose:
  `tests/queue_sqlite.rs` is the SQLite home, because its gate is the opposite one.
