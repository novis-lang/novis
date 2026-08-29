# Handoff

## State

**Stage 6's item 20 is two constructs of three, and the third is now specified rather than open.**
`crates/nvs-types/src/expr/isolate.rs` is a module of its own — both `spawn script` and `await` are
checked from there, `crates/nvs-types/src/expr/mod.rs` keeps its charter of dispatch-only — and its
module doc is the one home of the decision: **the handle is a registered `Core` class and the
result is an ADR 0036 § 3 shape.** One fact decides both, and it cuts opposite ways: a `Core`
instance has no property a program can reach, which is what a handle *is* and what the result may
not be.

**`await` answers with the shape today, and still reports `E0776`.** The type is
`{ok: bool, value: mixed, output: string, error: ?{class: string, message: string}}`, one field per
member of `nvs_host::Completion`; `crates/nvs-types/tests/isolates.rs` is four cases over it.
**The refusal may not be removed ahead of the lowering**: `crates/nvs-ir/src/lower/expr.rs:422`'s
roster comment ends in a `panic!`, so a construct the checker accepts and `nvs-ir` has no arm for
crashes the compiler rather than reporting anything.

**`spawn script` still answers `mixed`**, because `Core\Script\Handle` needs a registry row and
`crates/nvs-stdlib/tests/spec_registry_coverage.rs` makes a new `Core` class a
`docs/spec/01-core-library.md` edit too. That is the next group's first slice.

**Orientation gaps, unchanged:** `[context] adrs` prints ADR 0023 § 2 only — ADR 0006's
`## Decision`, *Failure is a value* and *Output is captured by default* are what a Stage 6 slice is
written against and none is in the manifest (this session sliced them by hand). `[context] modules`
has no pattern for `nvs-types/src/expr/`, `nvs-stdlib/src/registry.rs` or `nvs-cli/src/`.

## Next group

**`Core\Script\Handle`, and the typing that needs it.** File set:
`crates/nvs-stdlib/src/registry.rs`, a new `crates/nvs-stdlib/src/script.rs`,
`docs/spec/01-core-library.md`, and `crates/nvs-types/src/expr/isolate.rs`.

- [ ] **`Core\Script\Handle` is a registered class with no members** — the whole point being that
      there is nothing to call on one. `crates/nvs-stdlib/src/channel.rs` is the worked example of a
      `Core` class that is not a namespace; add it to `CLASSES` at
      `crates/nvs-stdlib/src/registry.rs:824`, and expect
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs` to want a `docs/spec/01-core-library.md`
      entry beside it. ADR 0051 § 1 is why the `Core` prefix is claimable here at all.
- [ ] **`spawn script` types to that handle** — `check_spawn_script` in
      `crates/nvs-types/src/expr/isolate.rs:91` returns `env.interner.mixed()`; the row lowers
      through `crates/nvs-types/src/core_lib.rs:327` (`CoreTy::Instance` → `interner.class`). Keep
      the `E0703` report, for the roster reason above.
- [ ] **`await`'s operand is checked against the handle** — same file, `check_await`, plus the
      `E0776` help text. `await 5` should say what it wanted; today the operand is only checked for
      its own sake.

## Backlog

- **`await` lowers, and `spawn script` with it** — the roster comment at
  `crates/nvs-ir/src/lower/expr.rs:422` is the proof, and removing the two reports is the same
  slice. `docs/plan/m5.md`.
- Item 22's fields: `error->code`, `error->trace`, a limit breach's `error->limit`, and the
  top-level `return` that fills `value` — ADR 0006 § *Failure is a value, not an exception*.
- `Core\Script::args()` and `Core\Script::valueOrThrow($result)` — item 22, and the second is the
  one surface cost of the shape.
- `output`'s carrier type under ADR 0088 §§ 3, 5 — typed `string` today, and item 24 owns it with
  `output: capture|inherit`.
- The `[context]` manifest gaps named in `## State`.
