# Handoff

## State

**Stage 6's item 20 is three constructs of three, and both types are now on disk.**
`crates/nvs-stdlib/src/script.rs` registers `Core\Script\Handle` — a `Core` class with no members,
no slots and no constructor row — and `crates/nvs-types/src/expr/isolate.rs` types both halves
against it: `spawn script` answers with the handle, `await` takes one and answers with ADR 0006's
`ScriptResult` shape. Seven `nvs-types` tests pin it. Each module doc is the one home of its own
half: why the class declares nothing (`script.rs`) and why the handle is a class while the result is
a shape (`isolate.rs`).

**Both refusals still stand, and may not be lifted before the lowering.**
`crates/nvs-ir/src/lower/expr.rs:422`'s roster comment ends in a `panic!`, so a construct the checker
accepts and `nvs-ir` has no arm for crashes the compiler rather than reporting anything. That is why
the acceptance check `native examples/isolate.nvs [6 isolates]` fails on `E0703`, and it is the next
group's whole subject — the example otherwise parses and checks with exactly three `E0703` and three
`E0776` and nothing else.

**Orientation gaps, unchanged:** `[context] adrs` prints ADR 0023 § 2 only — ADR 0006's
`## Decision`, *Failure is a value* and *Output is captured by default* are what a Stage 6 slice is
written against and none is in the manifest. `[context] modules` has no pattern for
`nvs-types/src/expr/`, `nvs-stdlib/src/registry.rs` or `nvs-cli/src/`; add one for `nvs-ir/src/lower/`
before the next group, which lives there.

## Next group

**The lowering, which is the only thing standing between this goal and its own acceptance check.**
File set: `crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-ir/src/ir.rs`,
`crates/nvs-runtime/src/script.rs` and `crates/nvs-host/src/isolate.rs`. Read
`crates/nvs-cli/src/script.rs` for the resolver already on disk before adding anything.

- [ ] **`spawn script` lowers to an instruction that builds a handle** — ADR 0006 § *Decision*.
      `InstKind` is `crates/nvs-ir/src/ir.rs:328`, the dispatch is
      `crates/nvs-ir/src/lower/expr.rs:50`, and the roster comment that must lose a line is
      `crates/nvs-ir/src/lower/expr.rs:422`. The value it produces is an instance of the class
      `crates/nvs-stdlib/src/script.rs` registers, and **that slice decides the slot** the module doc
      deliberately left undeclared.
- [ ] **`await` lowers to the suspend that collects a `Completion`** — same three anchors, plus
      `nvs_host::Isolate`'s answer (`crates/nvs-host/src/isolate.rs`) and the four-field shape
      `crates/nvs-types/src/expr/isolate.rs:174`'s `script_result` already fixes, in that order.
- [ ] **Drop `E0703` and `E0776` in the same slice as the arm that replaces each**, never before —
      `crates/nvs-types/src/expr/isolate.rs:113` and `:160`. `crates/nvs-types/tests/isolates.rs`
      asserts both codes and is where the change is visible.

## Backlog

- Item 21's request-tree budget accounting, under compiled-in defaults — `docs/agent/loop-goal.md` § Stage 6.
- Item 22's `Core\Script::args()`, the top-level `return` contract and `valueOrThrow($result)` —
  same file; the static class lands in `crates/nvs-stdlib/src/script.rs` beside its handle.
- Item 23's value-crossing refusals, including the cyclic argument that must not hang — ADR 0023 § 2.
- Item 24's `output: capture|inherit`, which owns whether the result's `output` field stays `string`.
- `Core\Secret::reveal()` is still unregistered, so ADR 0033's escape hatch is open at both ends.
- M4's residue: the 1000-case conformance corpus count, met as the suite grows.
