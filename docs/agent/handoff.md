# Handoff

## State

**ADR 0020 § 1's tier 1 exists as far as registration goes.** `Core\Fatal::onLimit(callable): void`
is a registry row in the new `crates/nvs-stdlib/src/fatal.rs`, and the closure it takes is held on
`Ctx` beside the memory ceiling (`Ctx::set_limit_handler`/`limit_handler`/`has_limit_handler`,
`crates/nvs-runtime/src/ctx.rs`), owned: a second registration releases the first and dropping the
context releases the last. Three conformance cases pin registration, that nothing runs the handler
eagerly, and that a thousand registrations under an 8M ceiling do not leak the closures they
replaced. **Nothing fires it yet** — a breach still goes straight to the `FATAL` print.

**The driver's failing check passes, and so does the rest of that check.** It named
`a_root_is_named_by_config_else_found_else_defaulted`; five more names in the same `[[check]]` block
were paraphrases of tests spelled differently, so all six now carry the names
`loop-goal.toml` asks for. Two of the renames were not free and gained the assertion their new name
claims: `optional_covers_absence_and_not_unreadability` now has a present-but-unreadable include (the
`Fake` grew an `unreadable` builder for it), and `a_group_writable_file_or_directory_refuses_the_boot`
now has the directory half through a `[[include]] dir`.

Still true and still unfixed: `orient.py`'s `[context] modules` names `crates/nvs-host/src/budget.rs`,
which never existed — the accounting is `crates/nvs-runtime/src/budget.rs`. The pack prints its own
warning about it every session.

## Next group

**The rest of ADR 0020 § 1's ladder** — the handler is held and nothing reaches it. File set:
`crates/nvs-runtime/src/ctx.rs` (`memory_breach` at :944 and the new handler slot above it at :932),
`crates/nvs-stdlib/src/fatal.rs`, `crates/nvs-config/src/directive.rs` and
`crates/nvs-cli/src/main.rs:828`, which is where a `FATAL` is printed today.

- [ ] **The breach calls the handler before the ladder prints** — ADR 0020 § 1
      (`docs/adr/0020-error-escalation-ladder.md:66`). The `Fault` is built where the breach is
      detected; `Ctx::has_limit_handler` is the first question and `nvs_runtime::call_closure` is
      how the call is made (the playbook's *A `-p nvs-stdlib` test can hand a `Core` member a real
      `callable`* bullet has the ownership rule the callee owes). **Zero retries**: a handler that
      throws or breaches again is abandoned where it stands and never called twice, which means the
      slot is cleared *before* the call and not after.
- [ ] **The reserved slice is what the handler runs under** — ADR 0020 § 1's
      `[limits] fatal_reserve_memory` / `fatal_reserve_time`, both `System` class: a row each in
      `crates/nvs-config/src/directive.rs`, a field on `tree::Limits`, and the ceiling raised by
      exactly that much for the duration of the call in `ctx.rs`. A handler that cannot allocate at
      all is the failure mode this closes.
- [ ] **`LimitReport` is the argument the handler is handed** — § 1 spells the parameter
      `closure(LimitReport): void`. A `Core` class with slots naming which limit was reached;
      `crates/nvs-stdlib/src/fatal.rs`'s module doc records why the registry row says `callable`
      regardless (ADR 0031 § 4).
- [ ] **The CPU-time limit reaches the same ladder** — ADR 0020 § 1. `cpu_time` sits unread beside
      `memory` in the `[limits]` block.

## Backlog

- Item 18's `Core\Secret::reveal()` is not in the registry — `docs/agent/loop-goal.toml` item 18.
- `Live::admit`'s same-class check is asked of the answer, not the argument —
  `crates/nvs-runtime/src/graph.rs` § *Known gaps*.
- Item 22's `Core\Script` members are unwritten — `crates/nvs-stdlib/src/script.rs`.
- Wall time, `max_script_depth` and call-stack depth are three more § 1 limits with no ladder —
  `docs/adr/0020-error-escalation-ladder.md:66`.
- `orient.py`'s `[context] modules` names a `crates/nvs-host/src/budget.rs` that never existed.
