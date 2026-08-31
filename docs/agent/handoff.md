# Handoff

## State

**ADR 0020 § 2 is closed end to end.** `Core\Fatal::onUncaughtThrow` registers into a
second `Ctx` slot beside `limit_handler` (`crates/nvs-runtime/src/ctx.rs:425`), and
`Ctx::run_uncaught_handler` hands it the **real** `Throwable` — `Thrown::as_value` is a
borrow, not a copy, and `on_uncaught_throw_receives_the_real_throwable`
(`crates/nvs-stdlib/src/fatal.rs`) compares payload bits rather than class and message,
so a ladder that rebuilt the exception fails there while still reading right. Both roots
fire it before the floor builds its record: `crates/nvs-cli/src/main.rs:907` and
`crates/nvs-host/src/isolate.rs:457`. No reserve stands beside the slot, per § 2; the
three decisions this needed — running does not suppress tiers 3 and 4 (tier 1 already
behaves that way), the registration leaves the slot on the way in, and the handler's own
throw is cleared so the request still reports what reached the root — are in
`run_uncaught_handler`'s doc comment, which is their home.

**Still owed on stage 7**: `the_engine_floor_rotates_and_rate_limits_itself` is the one
acceptance test of `[7 fatal and log]` with nothing behind it — ADR 0106 § 10, and it is
the next group below. Nothing reads `[log] target` at run time yet, which is what a
rotation has to attach to. **Still owed on `Core\Cli`**: `arguments`, `write` and
`displayWidth` — `cli.rs`'s gaps 1 and 2, untouched again.

**The orientation pack printed no section of ADR 0020**, though the goal's whole stage 7
is that ADR: `[context] adrs` needs `0020:2` and `0020:3` at least — this session sliced
both by hand, plus § 5. It still does not print `docs/spec/01-core-library.md` either.

## Next group

**ADR 0106 § 10's two bounds on the floor's sink, over one file set:
`crates/nvs-runtime/src/floor.rs`, `crates/nvs-runtime/src/ctx.rs` and
`crates/nvs-stdlib/src/log.rs`'s test module.**

- [ ] **The rate limit with a coalescing counter** — ADR 0106 § 10, second bullet.
      Repeated identical records inside a window become one record carrying a count, and
      the count is a field on ADR 0092's record rather than a second shape. It belongs on
      the sink both callers share, not on either caller: `crates/nvs-runtime/src/floor.rs:148`
      is `report`, and `crates/nvs-runtime/src/ctx.rs:3268` is `write_diagnostic` beneath
      it. Decide which of the two holds the window — the `Ctx` is per request and the
      floor is not, so a counter on the request cannot coalesce across requests.
- [ ] **Rotation and a retention bound on a file target** — ADR 0106 § 10, first bullet,
      for the diagnostic log and the access log alike. This is the half with no reader:
      `crates/nvs-runtime/src/floor.rs:38` says `[log] format` is not read at run time and
      the same is true of `[log] target`, so the slice starts by deciding whether the
      target reaches `Ctx::write_diagnostic` at all today.
- [ ] **`the_engine_floor_rotates_and_rate_limits_itself`** — the acceptance check's third
      test, beside its sibling at `crates/nvs-stdlib/src/log.rs:333`. A buffered `Ctx` and
      a repeated record is the rate-limit half; the rotation half needs a temporary
      directory, so it may want `crates/nvs-stdlib/tests/` instead — either satisfies
      `cargo test -p nvs-stdlib`.

## Backlog

- `Core\Cli::arguments`, `::write`, `::displayWidth` — `crates/nvs-stdlib/src/cli.rs` gaps 1 and 2.
- `crates/nvs-runtime/src/deferred.rs:138` is a third root reporting an uncaught throw and
  does **not** fire tier 2; decide whether a deferred closure's throw is § 2's "request
  root" — `crate::deferred`'s module doc is where that answer belongs.
- `[context] adrs` in `docs/agent/loop-goal.toml` names no ADR 0020 section, and
  `[context]` names `docs/spec/01-core-library.md` nowhere.
- ADR 0020 § 4's `[log] target` reader — the gap `crates/nvs-runtime/src/floor.rs:38`
  records, and what rotation attaches to.
