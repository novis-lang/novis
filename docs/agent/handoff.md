# Handoff

## State

**ADR 0084 § 1's `Core\Queue::push` is live**, in a new `crates/nvs-stdlib/src/queue.rs` with
`Core\Queue` and the opaque `Core\Queue\Id` in `CLASSES`. It resolves `[queue]` off the boot
snapshot, reaches that connection, and writes one row. Six of § 1's eight options are declared
and honoured; `limits` and `grants` are not, and that module's known gaps say why (a *shape*
parameter, the same blocker `Core\Db::open` waits on).

**§ 3's transactional enqueue holds by construction rather than by machinery.**
`crates/nvs-stdlib/src/db.rs`'s `open_named` is `Core\Db::connect`'s body from the memo down,
extracted so `push` reaches the queue's connection through the *same* memo, pool key and reset —
which means an enqueue inside a `transaction()` on that name is already inside it. The
capability check stayed in `connect`: it is about a name the program wrote, and a queue's name
is the operator's. Both readings are in `open_named`'s doc comment and `queue.rs`'s module doc.

**The jobs table's schema is `queue.rs`'s `JOBS_TABLE`/`INSERT` until `nvs queue migrate` exists**,
and § 2 keeps the DDL out of the request either way. Every instant in it is a `bigint` of epoch
milliseconds, because § 2 wants all five backends and five timestamp dialects is not a cost a
runtime-owned table should carry.

**The acceptance line for `examples/queue.nvs` is still open, and the next blocker in it is not a
missing member.** Line 60 writes `Queue::stats("default")->claimed` — a *property* on a `Core`
instance, which `CoreTy::Instance`'s own rule says a program cannot reach (that is why
`Core\Db\Write` answers `lastId()` and not `->lastId`). So `stats` either answers a shape or the
example changes; decide it before writing the member, not after. Line 63 needs
`Core\Queue\State` as a real `CoreEnum` as well.

**`orient.py`'s pack was short in three places.** `[context] modules` still names no
`nvs-config/src/*` and now owes `nvs-stdlib/src/queue.rs`; `[context] adrs` printed ADR 0084 §§ 1
and 3 but this session also needed §§ 2 and 4, and the next one needs § 6.

## Next group

**§ 1's remaining three members, over the file set this session opened. It is
`crates/nvs-stdlib/src/queue.rs`, `crates/nvs-stdlib/src/registry.rs` and — for the `CoreEnum`
shape only, not to edit — `crates/nvs-stdlib/src/db.rs`.**

- [ ] **`Core\Queue\State` is a `CoreEnum`, and `Core\Queue::status` answers one** — ADR 0084 §§ 1
      and 6. The enum shape is `crates/nvs-stdlib/src/registry.rs:1802` and the two worked
      examples are `crates/nvs-stdlib/src/db.rs:714` and `crates/nvs-stdlib/src/db.rs:896`; the
      class's rows go beside `push` at `crates/nvs-stdlib/src/queue.rs:140`, the enum joins
      `CLASSES` beside `crates/nvs-stdlib/src/registry.rs:1438`, and the symbol joins
      `crates/nvs-stdlib/src/queue.rs:589`. `Pending` must stay ordinal 0 — `queue.rs`'s
      `PENDING` const is written as the number and says it owes this assertion.
- [ ] **`Core\Queue::cancel` moves a pending row out of the queue** — ADR 0084 §§ 1 and 6. Same
      three edits at `crates/nvs-stdlib/src/queue.rs:140`, `crates/nvs-stdlib/src/queue.rs:589`
      and the `ID` slots at `crates/nvs-stdlib/src/queue.rs:284`, which is where the row id and
      its queue come from. A claimed job is not cancellable and that is § 4's visibility timeout,
      not a race to lose.
- [ ] **`Core\Queue::stats` — decide the return shape first** — ADR 0084 §§ 1 and 6. The example
      at `examples/queue.nvs:60` reads `->claimed`, `->attempts` and `->deadLettered` as
      properties, and `crates/nvs-stdlib/src/registry.rs:1027`'s `CoreClass` has no property
      surface at all. Either the member answers a shape or the example takes `()`; whichever it
      is, record it in `queue.rs`'s module doc and add the `CAPABILITIES` row for the whole
      four-member class at `crates/nvs-stdlib/src/registry.rs:1488` at the same time.

## Backlog

- The `[queue] backoff` default has no config field, so `push` stores `null` — `docs/adr/0084`
  § 1 leaves it to the worker.
- `$args` is `CoreTy::Mixed` and cannot refuse a `secret`, which § 1 asks for —
  `crates/nvs-stdlib/src/queue.rs` known gap 2.
- `key`'s dedupe is race-free only under the partial unique index `nvs queue migrate` owes —
  same file, known gap 3.
- Stage 6's `mariadb: n/a` / `mssql: n/a` are did-not-run — `docs/agent/loop-goal.toml`.
- § 7's backoff is blocked on `nvs-runtime`'s known gap 3 — `crates/nvs-stdlib/src/db.rs:149`.
- Stage 5's `args = ["test", "-p", "nvs-db"]` cannot see the `nvs-stdlib` tests — the user's call.
