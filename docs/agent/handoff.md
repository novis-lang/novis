# Handoff

## State

**Goal `finish-response` — stage 2 is landed and green.** The exit queue now drains on the served
path: `nvs-host`'s `finish` reaches `nvs_stdlib::script::run_exit_hooks` through an inverted seam,
and a request's registrations are no longer released unrun at teardown.

**How the ending reaches the classifier, settled.** `Program` answers a `Value` and no status
(`crates/nvs-runtime/src/script.rs:116`), and both production programs dropped the status on the
floor. Widening `Program` was rejected — ~50 sites across four crates — for one recorded word:
`Ctx::ending` / `Ctx::set_ending` (`crates/nvs-runtime/src/ctx/wiring.rs:278`), written by
`crates/nvs-cli/src/script.rs:738` and `crates/nvs-stdlib/src/script.rs:800`. The reasoning is in
the `ending` field's own doc (`crates/nvs-runtime/src/ctx/mod.rs:584`); no rule changed, because
`rule:observability/three-endings-fire-the-exit-queue` already asserted what the served path did not
do. Recording the *status* rather than an `exited` flag is what keeps a `FATAL` out of the queue:
`pending` is set for a throw and a `FATAL` alike and `Pending`'s variants are private.

**The seam is `Session::write_back`'s shape.** `Ctx::exit_drain` is
`Option<fn(&mut Ctx, Result<(), i32>, Option<&Thrown>)>`, filled by `Core\Script::onExit` on every
registration (`crates/nvs-stdlib/src/script.rs:917`) and `None` for a script that registers nothing.
`nvs_stdlib::script::run_exit_hooks` already had exactly that signature, so no stdlib door changed
shape. The call site is `crates/nvs-host/src/isolate.rs:927-948` — after the ladder, before
`end_session`, before the buffer is taken, before the deferred work both callers run.

**Nothing is blocked.** Stages 3–5 are untouched; no code for the `finish` member exists yet, which
is what the acceptance check on `examples/finish.nvs` is still reporting.

**A `[context]` gap.** `rules` did not print
`rule:observability/a-fatal-and-a-cancellation-run-no-exit-hook`, which this stage is decided
against as much as the three-endings table it does print — add it.

## Next group

**Stage 3: the keystone — an unwind every `finally` sees and no `catch` admits** — one file set:
`crates/nvs-codegen/src/emit.rs`, `crates/nvs-ir/src/ir.rs` and `crates/nvs-runtime/src/ctx/error.rs`.
Tag it stage 3 so `[context.stage.3]`'s overlay applies.

- [ ] **Confirm the pending slot can carry a value outside the `Throwable` tree**, which is what the
      whole marker-object line rests on: read `Pending` and its promotion at
      `crates/nvs-runtime/src/ctx/error.rs:132-148`, against
      `rule:observability/three-endings-fire-the-exit-queue`'s siblings in the same chapter.
- [ ] **Read the landing block's one question and what `onward` releases**, which fixes where the
      marker has to differ: `crates/nvs-codegen/src/emit.rs:3415-3425` and
      `crates/nvs-ir/src/ir.rs:2585-2592`. A fifth status is the trap the goal names — it is `exit`'s
      mechanics and skips every `finally`.
- [ ] **Settle where the marker class lives and how `InstanceOf` misses it**, at
      `crates/nvs-codegen/src/emit.rs:3308`'s fallible-call-site test, against
      `rule:errors/escalation-ladder`. If the slot cannot carry it, take § *Standing decisions*'
      fallback rather than a fifth status.

## Backlog

- Stage 4's member and its four endings — `docs/agent/loop-goal.md` § *Stage 4*.
- Stage 5's fragments, which `docs/decisions/0178.md`'s empty `changes:` lists are waiting on.
- `examples/finish.nvs` — the acceptance fixture, which stage 4 is what writes.
- `finish` classifies a `FATAL` reaching it as an uncaught throw and climbs the ladder
  (`crates/nvs-host/src/isolate.rs:899`); out of this goal's scope, now visible because
  `Ctx::ending()` can tell them apart. Its home is `crates/nvs-host/src/isolate.rs`'s module doc.
