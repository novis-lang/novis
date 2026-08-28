# Handoff

## State

**M4's Stage 7: ADR 0079 §§ 1, 4, 5, 20 and 22's human half are closed, and
the runner runs.** `nvs test <program.nvs>` compiles the program, constructs
each `#[Test]` class and calls its methods one fresh instance at a time, and
reports § 22's human format with the verdict read off § 5's ledger.

- **`nvs_cli::runner` is the runner and its module doc is the home** of what it
  owes and of the two decisions it took: the entry file's own top-level
  statements do not run, and class order is the roster's sorted order rather
  than § 20's declaration order (`ExprTypeTable::tests` records no sequence).
- **`nvs_runtime::construct_and_call` is construct-call-release** and
  `nvs_codegen::Unit::call_on_new_instance` the half above it, which is where
  the descriptor's liveness is provable — `nvs-cli` forbids `unsafe`, so
  neither could have been written there.
- **Nothing reads `Ctx::take_assertions` but the runner**, and that is now a
  reader rather than a gap: `nvs_stdlib::test`'s known gap 2 is closed.
- § 4's roster is still the three equality members. `assertTrue`/`assertNull`/
  `assertCount`/`assertThrows` are owed (`nvs_stdlib::test`'s known gap 3), and
  § 20's `assertDoesNotThrow` joins them — the empty-ledger failure names the
  rule rather than that member because it does not resolve yet.
- **Stage 8's named case is blocked on the `.nvst` format, not on the runner.**
  `nvs_test::run` spawns `nvs run case.nvs` unconditionally, so no case can
  reach `nvs test <program>` at all. That is the next group's first slice.
- The conformance corpus is at **730**.

## Next group

**Letting a `.nvst` case run through the runner, then writing the one Stage 8
owes.** The file set is `crates/nvs-test/` plus one new case:

- [ ] **A `.nvst` case can say it is run through `nvs test`** — a section
      beside `--ARGS--`, or a `--FILE--` variant; `crates/nvs-test/src/run.rs:225`
      is the one `spawn(&opts.nvs, &["run", name])`, `crates/nvs-test/src/case.rs:130`
      the section roster and `crates/nvs-test/src/lib.rs:23` the table that
      documents it. The runner's report carries a per-test duration, so the
      case wants `--EXPECTF--`'s `%f`, not `--EXPECT--`.
- [ ] **`a-test-attribute-builds-a-table-the-runner-reports.nvst`** (ADR 0079
      §§ 1, 20) — the last named case `python tools/loop.py --list` reports
      missing: declaration order within a class, a skip with its reason, a
      failure, and a caught failure that still fails.
- [ ] **ADR 0079 § *Verification*** — the section M4's acceptance names, over
      the four cases that now pin §§ 1, 4, 5 and 20, plus what only a Rust test
      can assert (`crates/nvs-cli/src/runner.rs:141`).

## Backlog

- § 4's `assertTrue`/`assertNull`/`assertCount`/`assertThrows` and § 20's
  `assertDoesNotThrow` — `nvs_stdlib::test`'s known gap 3.
- § 22's `--format=junit` and `--format=json`, and § 20's `retries:`/`FLAKY` —
  `nvs_cli::runner`'s module doc.
- An expression-bodied `fn (): void => <a void call>` does not lower —
  playbook, *Writing Novis itself*; the block-bodied form is the way round.
- § 1's last compile error, a `#[Test]` parameter no `#[Fixture]` supplies —
  waits on §§ 8-9, stated at `nvs_types::testing::check_method_shape`.
- ADR 0079 § 20's declaration order across classes wants a sequence on
  `ExprTypeTable::tests`; the runner sorts instead.
