# Handoff

## State

**ADR 0084 § 1's roster is three of four.** `Core\Queue::push`, `::status` and `::cancel` are live
in `crates/nvs-stdlib/src/queue.rs`, with `Core\Queue\Id` and the new `Core\Queue\State` in the
registry's `CLASSES` and `ENUMS`. Only `stats` is owed, and that module's known gaps say what each
remaining thing waits on.

**`Core\Queue\State` is `Pending`/`Claimed`/`Succeeded`/`Dead`/`Cancelled`, and the enum *is* the
column.** § 2's jobs table stores the ordinal, so a case's number is part of the schema `nvs queue
migrate` will create and renumbering one is a migration. The SQL literals that no `const` can reach
— `INSERT`'s `state = 0`, `STATUS`'s `select 3`, `CANCEL`'s `set state = 4` — are held to the enum
by `queue_statements_agree_with_the_state_enum` in `queue.rs`'s own test module.

**Two design calls landed and are recorded in ADR 0084 § 1's body, not only in code.** `cancel`
answers a `bool` although § 1 annotated no return, because § 4 makes losing the race the ordinary
outcome and a `void` spelling would make "cancelled" and "too late" identical at the call site; and
it writes a state rather than deleting the row, so a caller that cancels and then asks `status`
gets `Cancelled` instead of a refusal. `Cancelled` is the case ADR 0084 did not originally name and
§ 1 now does.

**`status` reads both of § 2's tables in one statement**, because § 6 *moves* an exhausted job to
the dead-letter table rather than deleting it. `DEAD_TABLE` therefore decides two of that table's
columns — `id` and `queue`, carried over so an old receipt still names the job — and deliberately
none of the rest, which belongs to the worker that writes one.

**The acceptance line for `examples/queue.nvs` is still open on `stats`, and the shape decision is
still the blocker.** Line 60 writes `Queue::stats("default")->claimed` — a *property* on a `Core`
instance, which `CoreTy::Instance`'s own rule says a program cannot reach. The cheap answer is
`Core\Db\Write::lastId()`'s shape: a `Core\Queue\Stats` class whose counters are instance members,
and `examples/queue.nvs` lines 59-60 and 79 change to `->claimed()` / `->attempts()`. The other
answer needs the shape-return the registry cannot spell (gap 1's blocker).

**`orient.py`'s pack was short in the same three places the last session named**, all still unfixed:
`[context] modules` names no `nvs-config/src/*` and owes `nvs-stdlib/src/queue.rs`; `[context] adrs`
printed §§ 1, 6, 9 and 13 but this session also needed §§ 2 and 4 of ADR 0084.

## Next group

**`stats`, over the file set this session had open. It is `crates/nvs-stdlib/src/queue.rs`,
`crates/nvs-stdlib/src/registry.rs`, `examples/queue.nvs` and — for the instance-member shape only,
not to edit — `crates/nvs-stdlib/src/db.rs`.**

- [ ] **`Core\Queue\Stats` is a class whose counters are instance members** — ADR 0084 §§ 1 and 6.
      The worked shape is `Core\Db\Write`, whose class is at `crates/nvs-stdlib/src/db.rs:4767`
      and whose instance rows sit beside it; the new class goes next to `ID` at
      `crates/nvs-stdlib/src/queue.rs:434` and joins `CLASSES` beside
      `crates/nvs-stdlib/src/registry.rs:1443`. Four counters at least — `pending`, `claimed`,
      `attempts` and the dead-letter depth § 6 names.
- [ ] **`Core\Queue::stats` answers one, over one grouped statement** — ADR 0084 §§ 1 and 6. The
      row goes beside `cancel` at `crates/nvs-stdlib/src/queue.rs:243`, the statement beside
      `CANCEL` at `crates/nvs-stdlib/src/queue.rs:141`, and the symbol joins
      `crates/nvs-stdlib/src/queue.rs:735`. It counts by `state` in `nvs_jobs` and by row in
      `nvs_dead_jobs`, so it is the third statement that owes
      `queue_statements_agree_with_the_state_enum` an entry.
- [ ] **`examples/queue.nvs` calls the members and not properties** — ADR 0084 § 1.
      `examples/queue.nvs:59`, `examples/queue.nvs:60` and `examples/queue.nvs:79` write
      `->claimed` and `->attempts`, which `CoreTy::Instance`'s rule says a program cannot reach;
      they become calls. This is what closes the driver's failing acceptance line for that
      example.

## Backlog

- `limits` and `grants` on `push` wait on a shape parameter — `crates/nvs-stdlib/src/queue.rs` gap 1.
- `$args` does not refuse a `secret` — `crates/nvs-stdlib/src/queue.rs` gap 2.
- `key`'s dedupe is racy without its partial unique index — `crates/nvs-stdlib/src/queue.rs` gap 3.
- The four non-PostgreSQL drivers have no statement path — `crates/nvs-stdlib/src/db.rs` gap 2.
- `nvs queue migrate` does not exist; the schema lives in `queue.rs`'s consts — ADR 0084 § 2.
- `[context]` in `docs/agent/loop-goal.toml` owes `nvs-stdlib/src/queue.rs` and ADR 0084 §§ 2, 4.
