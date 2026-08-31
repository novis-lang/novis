# Handoff

## State

**ADR 0020 § 6 is asserted rather than claimed.**
`application_code_and_the_engine_floor_produce_schema_identical_records`
(`crates/nvs-stdlib/src/log.rs:333`) puts one promoted `Thrown` through both callers — the floor's
`nvs_runtime::floor::uncaught` and `Core\Log::write` driven through `nvs_runtime::call` — and
compares the two rendered lines **byte for byte**, so a second serialiser growing on either side
fails here while still reading correctly on its own. `log.rs`'s module doc no longer carries the
"the floor is the caller that does not exist yet" gap: `crates/nvs-runtime/src/floor.rs` has been
that caller for some time, and `nvs-cli`, `nvs-host`'s isolate and `nvs-runtime`'s deferred queue
are its three call sites.

**Stage 7's check is now two checks, one per crate that can host its names.**
`the_configured_handler_script_runs_charged_to_the_engines_own_reserve` has always lived in
`crates/nvs-host/tests/limits.rs:525` and could never have run under `cargo test -p nvs-stdlib`, so
that check would have failed forever even once the other four landed — the playbook's *a check can
name a test in a crate that cannot host it* trap, found live. It and its sibling ceiling case moved
to a `-p nvs-host` check; `on_uncaught_throw_receives_the_real_throwable` and
`the_engine_floor_rotates_and_rate_limits_itself` stayed, since `nvs-stdlib` reaches `Ctx` and can
ask both.

**Still owed on stage 7**: `Core\Fatal::onUncaughtThrow` does not exist — ADR 0020 § 2 specifies it,
`Core\Fatal` is `onLimit` alone, and `Ctx` has no `uncaught_handler` beside `limit_handler`. The
floor's own rotation (ADR 0106's amendment to § 4) has no writer yet, since nothing reads
`[log] target` at run time. **Still owed on `Core\Cli`**: `arguments`, `write` and `displayWidth` —
`cli.rs`'s gaps 1 and 2 own them, untouched by this session.

**The orientation pack still does not print `docs/spec/01-core-library.md`**, which is in no
`[context]` field; § 15's `Core\Cli` list is what those three signatures are written against.

## Next group

**§ 2's `onUncaughtThrow`, over one file set: `crates/nvs-stdlib/src/fatal.rs` and
`crates/nvs-runtime/src/ctx.rs`, then the two roots that report an uncaught throw.**

- [ ] **`Ctx`'s second handler slot** — ADR 0020 § 2. A field beside `limit_handler`
      (`crates/nvs-runtime/src/ctx.rs:373`), a `set_uncaught_handler` mirroring
      `crates/nvs-runtime/src/ctx.rs:1429`, and a `run_uncaught_handler` mirroring
      `crates/nvs-runtime/src/ctx.rs:1556` — **without** the reserved slice, since § 2 says
      execution was healthy up to this point and the request's ordinary budget applies. The handler
      is handed the **real** `Thrown` object, not a copied report, which is the one way it differs
      from `onLimit`'s array.
- [ ] **`Core\Fatal::onUncaughtThrow(callable $handler): void`** — ADR 0020 § 2, the five edits: a
      row on `CLASS` (`crates/nvs-stdlib/src/fatal.rs:36`), its card, the helper beside
      `crates/nvs-stdlib/src/fatal.rs:80` (whose retain-then-`set` body is the shape to copy), an
      `address` arm (`crates/nvs-stdlib/src/fatal.rs:73`), and
      `on_uncaught_throw_receives_the_real_throwable` as a unit test — the closure built by hand,
      per `crates/nvs-stdlib/tests/allocation_policy.rs`'s `closure_of`, asserting object
      *identity* rather than a message.
- [ ] **The two roots fire it before the floor reports** — `crates/nvs-cli/src/main.rs:907` and
      `crates/nvs-host/src/isolate.rs:457`, both of which already hold the `Thrown` and call
      `nvs_runtime::floor::uncaught` on it. `crates/nvs-runtime/src/deferred.rs:132` deliberately
      does **not** fire, and says so. Three `.nvst` cases land with this slice, since a case is only
      green once a root runs the handler.

## Backlog

- `the_handler_still_fires_when_the_reporting_request_is_at_its_memory_ceiling` — `nvs-host`'s check
  now, over `crates/nvs-host/tests/limits.rs:266`'s `breached()`.
- `the_engine_floor_rotates_and_rate_limits_itself` — ADR 0106's amendment to ADR 0020 § 4; no
  `[log] target` writer exists yet, so the rotating sink is the slice, not the test.
- `Core\Cli::write`, `arguments` and `displayWidth` — ADR 0086 § 3 and spec § 15, `cli.rs`'s gaps 1
  and 2, at `crates/nvs-stdlib/src/cli.rs:167`.
- `Text + Text` still needs a row in `crates/nvs-types/src/expr/operators.rs`.
- Stage 2 owes `truncate`, `lock` and `Core\IO::stdin`/`stdout`/`stderr` — `io.rs`'s module doc.
- `docs/spec/01-core-library.md` belongs in `[context]`'s spec selector; § 15 is unreachable from
  the pack today.
