# Handoff

## State

**`Core\Db\DbError` carries ADR 0067 § 8's `kind`.** It is a row in
`crates/nvs-hir/src/errors.rs:141`'s `OWN_PROPERTIES` with `KIND_SLOT` beside `REASON_SLOT`, typed in
`crates/nvs-types/src/error_lib.rs:117` as the registered enum `Core\Db\ErrorKind`, and written by a
synthesized constructor of its own (`crates/nvs-ir/src/lower/exception.rs:713`).

**`kind` is a registered enum, not a string** — `crates/nvs-stdlib/src/db.rs:748`'s `ERROR_KIND`, the
eleven cases of `nvs_db::DbErrorKind` in § 8's own order, following `Core\Db\Isolation` and
`Core\Db\ColumnType` in the same class family. Its doc comment owns why (a closed set the compiler
can check a `match` against; the wire half stays authoritative). The name is spelled in three crates
and `the_kind_property_names_a_registered_enum` is what holds them together, including the `Other`
ordinal `nvs-ir` restates because it depends on neither of the other two.

**A hand-constructed `DbError` reads `kind` as `Other`** — § 8 defines that case as what a driver's
code table does not name, and a program's own `new` has no code table behind it.

**The server's real kind is still dropped, and that is now the whole of gap 4's first half.**
`statement_failure` (`crates/nvs-stdlib/src/db.rs:2624`) holds it — `nvs_db::ServerError::of` reads it
out of the driver error — but `Fault::thrown_as` carries a class and a message and nothing else.
`Fault::ThrownWithIssues` is the one existing shape that writes an extra slot on a thrown object
(`ParseError::$issues`); generalising it is the next group's first slice. The retry loop still
branches on the *commit's* refusal for the same reason (gap 9's second half, untouched).

**No `.nvst` case pins `kind` on a live refusal**: that needs the `[db.main]` fixture and a
multi-file case. Nothing new is on disk under `tests/conformance/`.

The driver's acceptance line still names `examples/queue.nvs` — Stage 8's unlanded `Core\Queue`
(ADR 0084), not a regression; the check's own `[[check]]` block at `docs/agent/loop-goal.toml:2927`
is a program leg for work no session has started. Stage 5's `args = ["test", "-p", "nvs-db"]`
(`docs/agent/loop-goal.toml:2830`) still cannot see the two `nvs-stdlib` tests, and is still the
user's call. The CA is still not in git; `nvs_host::tls`'s module doc owns why.

**`orient.py` printed ADR 0067 §§ 1, 9 and 13 — not § 8, which specified every slice of this
group.** Two handoffs have now asked for it. Add `0067 § 7` and `0067 § 8` to `[context] adrs` in
`docs/agent/loop-goal.toml`; § 8 was sliced by hand again.

## Next group

**Let a native throw carry a slot, so the server's own `kind` reaches the `catch` and the retry loop
can read the closure's conflict. The file set is `crates/nvs-runtime/src/abi.rs`,
`crates/nvs-runtime/src/ctx.rs`, `crates/nvs-stdlib/src/db.rs` and `crates/nvs-stdlib/src/task.rs`.**

- [ ] **A `Fault` can carry one extra slot value, not just an issue list** — `Fault::ThrownWithIssues`
      at `crates/nvs-runtime/src/abi.rs:96` is the shape and its arm at
      `crates/nvs-runtime/src/abi.rs:415` the materialisation; `crates/nvs-runtime/src/ctx.rs:3376` is
      the destination that writes the slot. Decide there whether it generalises in place (a slot index
      plus a `Value`) or gains a sibling variant — the first keeps one path, the second keeps
      `ParseError`'s name on its own. `crates/nvs-stdlib/src/task.rs:358` matches the variant and must
      stay exhaustive. ADR 0067 § 8, ADR 0071 § 5.
- [ ] **`statement_failure` fills the slot** — `crates/nvs-stdlib/src/db.rs:2624`'s `Other` arm already
      has `nvs_db::ServerError::of(refused)` in reach; map `nvs_db::DbErrorKind` to
      `crates/nvs-stdlib/src/db.rs:748`'s `ERROR_KIND` ordinal and hand it to the new `Fault`. Closes
      gap 4's first half at `crates/nvs-stdlib/src/db.rs:85`. ADR 0067 § 8.
- [ ] **The retry loop reads the closure's conflict** — `crates/nvs-stdlib/src/db.rs:3552` branches on
      the commit's refusal because a fault out of the closure carried no kind; once it does, branch on
      it. `nvs_db::DbErrorKind::is_retryable` is the rule and stays the driver's. Closes gap 9's second
      half. ADR 0067 § 7.
- [ ] **A `.nvst` case pins a real refusal's `kind`** — multi-file, with a `--FILE nvs.toml--`
      granting `db.connect` over `[db.main]`;
      `tests/conformance/db/an-enqueue-commits-with-the-write-that-made-it.nvst:1` is the nearest
      shape, and `crates/nvs-stdlib/src/db.rs:2624` is the arm it has to observe. Run it with
      `target/debug/nvs.exe test`, never `try.py`.

## Backlog

- § 18's `sqlState`, `driverCode`, `constraint` and `sql` — gap 4, `crates/nvs-stdlib/src/db.rs:85`.
- `stream`/`streamAs` are owed whole — gap 5, same module doc.
- Stage 5's `-p nvs-db` args cannot see the two `nvs-stdlib` tests — `docs/agent/loop-goal.toml:2830`.
- `Core\Queue` (Stage 8, ADR 0084) is what the acceptance line has named for two sessions.
- `[context] adrs` is missing `0067 § 7` and `0067 § 8` — `docs/agent/loop-goal.toml`.
- `cargo install cargo-insta` would end the by-hand snapshot accept — `docs/agent/playbook.md`.
