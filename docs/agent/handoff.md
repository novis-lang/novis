# Handoff

## State

**Stage 6's isolate can now be *fed*.** `crates/nvs-runtime/src/script.rs` is the seam that turns a
written path into runnable code — a `Resolver` trait in the crate both sides already depend on, a
thread-local, and `resolve(path) -> Result<Program, ResolveError>`. Its module doc owns why it is a
second seam rather than a third `Host` method. `nvs_host::Program` is a re-export of that type now, so
the argument-transfer contract has one home instead of two.

**`crates/nvs-cli/src/script.rs` is the one implementor.** The front end and backend `nvs run` already
carries, behind a cache keyed by the path as written and leaked for the process; its module doc owns the
two decisions — a relative path anchors at the **working directory** (not the entry file), and one unit
per written path, which is where ADR 0006's "shares immutable compiled code" actually lives.
`nvs_runtime::script::install` is called for the whole run in `run_run` (`crates/nvs-cli/src/main.rs:668`).

**Where a transferred argument lands is decided:** `Ctx::set_isolate_argument`
(`crates/nvs-runtime/src/ctx.rs:947`) holds it as the isolate's own ownership root and `Drop for Ctx`
releases it — ADR 0116 § 2's wholesale release reaching the one value `Program`'s contract hands over.

**The route is proved without any language surface.** Five tests in `nvs-cli` and four in
`nvs-runtime`: `examples/isolate/hello.nvs` resolves, runs inside a real `Isolate` and hands back
`child said hello`; `throws.nvs` arrives as `ok = false` carrying `RuntimeError`; one written path
compiles once.

**The acceptance check is three constructs away, not one.** `nvs check examples/isolate.nvs` reports
`E0703` for `spawn script`, and then `E0319`/`E0101` on the next line — **`await` is not a keyword**.
It is nowhere in `nvs-syntax`'s AST, so the example does not parse past it, and no `ScriptResult` class
exists either. The playbook bullet under *Running things* owns how that stayed invisible.

**Orientation gaps, still open from the previous session and unchanged:** `[context] adrs` prints
ADR 0023 § 2 only — ADR 0006's `## Decision`, *What is and is not shared*, *Failure is a value* and
*Output is captured by default* are the sections a Stage 6 slice is written against and none is in the
manifest. `[context] modules` has no pattern for `nvs-types/src/expr/`, `nvs-cli/src/` or
`nvs-runtime/src/{graph,alloc,release}.rs`.

## Next group

**The `await` half of ADR 0006's surface, which is where the acceptance check actually stops.** File
set: `crates/nvs-syntax/src/{lexer,ast.rs,parser/expr.rs}`, then `crates/nvs-types/src/expr/mod.rs`,
against `crates/nvs-ir/src/lower/expr.rs:405` (`ExprKind::Require`, the closest analogue — it also calls
another file's script frame) read-only for the first two.

- [ ] **`await` is a prefix expression, not a constant** — ADR 0006 § *Failure is a value* and
      `docs/spec/00-overview.md:126` (the awaitable handle a subsequent `await` turns into a
      `ScriptResult`). Add the keyword and an `ExprKind::Await`, then a checker arm beside
      `ExprKind::SpawnScript` at `crates/nvs-types/src/expr/mod.rs:742`. Anchors:
      `crates/nvs-syntax/src/ast.rs:935` (`SpawnScript`'s own variant),
      `crates/nvs-syntax/src/parser/expr.rs:2004` (where `spawn script` is parsed).
- [ ] **Every walker that matches `ExprKind` gains the arm** — the five that already carry
      `SpawnScript`: `crates/nvs-hir/src/members.rs:817`, `crates/nvs-hir/src/requires.rs:1177`,
      `crates/nvs-syntax/src/casing.rs:808`, `crates/nvs-ir/src/lower/control.rs:2296`,
      `crates/nvs-types/src/expr/mod.rs:742`. Grep for `SpawnScript` and the roster is exact.
- [ ] **`ScriptResult`'s shape, decided and recorded** — `->ok`, `->output`, `->error` are what
      `examples/isolate.nvs` reads, and `nvs_host::Completion`
      (`crates/nvs-host/src/isolate.rs:90`) is the native half already on disk. A `Core` class row
      (ADR 0051 Tier 0) or a synthesized shape is the call to make; record it where AGENTS.md names.

## Backlog

- Lower `spawn script` and retire `E0703` at `crates/nvs-types/src/expr/mod.rs:749` — needs a seam
  from a runtime helper to `nvs_host::Isolate`, the third `Host` method question (`nvs-runtime/src/host.rs` § 3).
- `examples/isolate.nvs` prints its five frozen lines — the driver's failing check, `docs/agent/loop-goal.toml`.
- Stage 6's four unwritten `cargo-named` names: `an_unresolvable_class_is_refused_at_the_boundary`,
  `a_contained_panic_in_a_child_leaves_the_parent_running`,
  `a_cancelled_parent_leaves_no_orphan_and_no_leaked_arena`, `a_child_is_cancelled_at_its_next_safepoint`.
- `Core\Secret::reveal()` is still absent at both ends — item 18, `docs/plan/m5.md`.
- The `[context]` gaps above, in `docs/agent/loop-goal.toml`.
