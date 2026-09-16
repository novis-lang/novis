# Handoff

## State

Goal `m8-stdlib-depth`. **Stage 13 is closed.** `Core\Process::spawn` answers `Core\Process\Handle`,
every read and write goes off the core through `nvs_host::blocking::run`, a child is killed and reaped
when its task's `Ctx` drops, and the member now files `rule:observability/spawn-is-its-own-event`'s
event: a fourth `SpawnForm`, opened where the child starts and closed by the handle's `wait`. All three
of the stage's acceptance checks are on disk — five Rust tests in `crates/nvs-stdlib/src/process.rs`,
the concurrent-spawn scheduler guard in `benches/abi-probe/tests/perf_guards.rs`, and both `.nvst`
cases, the reject one written this session.

Two observability rules were amended in that slice to admit a child process as a fourth spawn form. The
five event kinds and the four things that become a span are unchanged, and `nvs_spawn_duration_seconds`
keeps its three isolate kinds — `rule:observability/spawn-is-its-own-event` says why.

## Next group

**Stage 14: the three `-p nvs-stdlib` tests of M8's verification remainder** — one crate, one test run,
and each item is a test written over code already on disk:

- [ ] **The four intrinsics' prepared artifact is byte-identical to the runtime-built one** — goal
      prose stage 14 item 1, over `docs/decisions/0057.md` § 4. The format half exists at
      `crates/nvs-stdlib/src/format.rs:835`; the pattern, URI and date-format halves do not. The test
      the check names is `every_intrinsic_literal_prepares_the_artifact_the_runtime_builds`.
- [ ] **A value written through the local tier on one core is absent on another** — goal prose stage 14
      item 5, over `docs/decisions/0059.md:163-164`. The tier is `crates/nvs-stdlib/src/cache.rs:1647`
      and the existing case runs on one core; the test is
      `a_local_tier_value_written_on_one_core_is_absent_on_another`.
- [ ] **No protocol class exposes a raw-value accessor** — goal prose stage 14 item 6, over
      `docs/decisions/0060.md:165-166`, asserted off the registry beside
      `crates/nvs-stdlib/src/registry.rs:5346`. The test is
      `no_protocol_class_exposes_a_raw_value_accessor`.

## Backlog

- Stage 14's other three items: the folded-intrinsic coverage test at
  `crates/nvs-codegen/tests/probes.rs:140` and the regex tier record at
  `crates/nvs-types/tests/intrinsics.rs:206`, plus the cache bullet struck with
  `crates/nvs-cli/src/cache.rs:2505` — goal prose stage 14.
- Stage 15, the rulebook — `docs/agent/loop-goal.toml:11155`.
- `ProcessOptions` is shipped on neither member — `rule:core-classes/process-options`.
- A spawn's event is left unjoined when the program never waits, which is also what a cancelled child
  reads as; nothing distinguishes the two — `rule:observability/spawn-is-its-own-event`.
