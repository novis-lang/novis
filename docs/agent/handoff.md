# Handoff

## State

**Stage 0b is closed.** Items 28, 29 and 30 are on disk and all three of its acceptance checks pass
locally. The plan's `Open now` is the one home for what landed; `MethodSig::param_names` is a `Vec`
with no producer writing `None`, and `E0485` is retired in `crates/nvs-diagnostics/src/lib.rs:1019`.
The goal's `[context]` no longer carries the stage's four modules or `0063 §1`, and
`docs/agent/goals/2-concurrency.toml` is byte-identical to `docs/agent/loop-goal.toml` again.

**One hole in ADR 0063 R2 is left open and it is the parser's, not the registry's.** Seven rows name
a parameter `fn` — `Core\Arr::map` and its siblings — and `Core\Arr::map(fn: $f, a: $a)` does not
parse: `crates/nvs-syntax/src/parser/expr.rs:904` admits a name only for `TokenKind::Ident`, so `fn`
is read as the closure keyword and the call reports `E0101` plus two cascade mismatches. The spec is
authoritative for the name and a rename is a breaking change under R2, so the fix is the parser.
That is slice 1 below.

**The driver's failing acceptance check is Stage 7's, and its `args` names a crate that cannot host
it.** `crates/nvs-test` is the `.nvst` case runner (`case.rs`/`expect.rs`/`run.rs`) and does not
depend on `nvs-host`; the `#[Test]` runner is `crates/nvs-cli/src/runner.rs`, which does. The
playbook trap *Tooling > a loop-goal.toml check can name a test in a crate that cannot host it* is
the procedure, including the `cp` back over the goal file.

**Orientation gap:** `[context] modules` has no pattern for `crates/nvs-syntax/src/parser/expr.rs`
or `crates/nvs-cli/src/runner.rs`, which slices 1 and 3 need.

## Next group

**Slices 1 and 2 share `crates/nvs-syntax/src/parser/expr.rs` and its tests; slice 3 opens Stage 7's
own file set and is here for its anchors.**

- [ ] **A keyword-spelled parameter name is callable.** `crates/nvs-syntax/src/parser/expr.rs:904`
      tests `TokenKind::Ident` before a `:`; a keyword token in that position is a name too, because
      the `:` is the whole disambiguation and no expression starts with a keyword followed by one.
      This is the same one-token contextual rule `spawn` and `await` already take
      (`crates/nvs-syntax/src/token.rs`). ADR 0063 R2 is the rule it closes; a `nvs-syntax` test
      beside `parse_arg` is the guard.
- [ ] **The case.** `tests/conformance/core/` — `Core\Arr::map(fn: …, a: …)` written out of order,
      beside the `a-core-member-is-called-by-the-names-the-spec-writes.nvst` this session added, and
      one `reject` line if a keyword name that reaches no parameter should still be `E0486`.
- [ ] **Stage 7 item 25 — one isolate per test.** `crates/nvs-cli/src/runner.rs:48` and `:299` are
      the two comments saying isolate-per-test is M5's and unbuilt; `:190` builds one `Ctx` for the
      whole run and `:436` is the per-test entry. `crates/nvs-host/src/isolate.rs:90` is `Isolate`.
      ADR 0079 §§ 2 and 16. Move the four test names off `-p nvs-test` first.

## Backlog

- Stage 7 item 26 — `#[Test(at:, seed:)]`; `crates/nvs-types/src/testing.rs:554` already reads § 9's
  rows by parameter name (docs/agent/loop-goal.md item 26, ADR 0079 § 12).
- Stage 4 item 15 — `Core\Task::afterResponse` and the `[deferred]` block (loop-goal.md item 15).
- Stage 6 item 22 — `Core\Script::args()` and `Core\Script::valueOrThrow` (plan `Open now`).
- Stage 8 item 27 — `benches/isolation.rs` and its guard (docs/plan/m5.md § *Verify*).
- `Core\Secret::reveal()` is still unregistered, so ADR 0033's escape hatch is open at both ends.
- § 4's prose writes `Date::at(int $y, uint $m, uint $d)` where `DateTime::at`'s table row writes
  `year`/`month`/`day`; the guard reads rows, so nothing compares the two (docs/spec/01-core-library.md).
