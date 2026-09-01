# Handoff

## State

**Stage 9's carrier is registered and its conversion is not.** `Core\Html\Markup` is a
`CoreClass` at `crates/nvs-stdlib/src/html.rs:121` — memberless, one `text` slot at
`nvs_runtime::CARRIER_TEXT_SLOT`, name taken from `nvs_runtime::CARRIER_HTML_MARKUP` so the
class a program writes and the class `value_to_string` renders cannot drift apart. That
const's own doc comment is the home of why it has no constructor.

**`"<b>" as Core\Html\Markup` still ICEs**, at `crates/nvs-ir/src/lower/convert.rs:351`, whose
panic named this class as what it was waiting on. The class existing was not the whole of it:
what the row needs is a way to build a one-slot instance of a registered `Core` class from the
IR, which no conversion has ever needed, and the crate direction (`nvs-stdlib` → `nvs-runtime`,
never back) is what makes it a decision rather than a transcription. The `# Known gaps` section
of `crates/nvs-stdlib/src/html.rs` is the home of that, and of what `Core\Html` still owes.

**The acceptance check that had been failing passes.** `Core\Validate` launders nothing, as a
test rather than as the module doc's claim: all six members answer a `bool`, so there is
nothing a qualifier could be removed from, and the last assertion asks the whole registry that
**no laundering member anywhere answers a `bool`** — laundering is a transformation, validating
is a question about the input.

**The pack still did not print ADR 0024 § 5**, sliced by hand for the second session running.
`[context] adrs` in `docs/agent/loop-goal.toml` wants `0024 § 5`, and `0007 § 2` for the group
below.

## Next group

**ADR 0024 § 5's two remaining halves, over `crates/nvs-ir/src/lower/convert.rs`,
`crates/nvs-ir/src/ir.rs`, `crates/nvs-codegen/src/emit.rs`,
`crates/nvs-types/src/expr/operators.rs` and `tests/conformance/core/`.**

- [ ] **`string as Core\Html\Markup` lowers rather than panicking** — § 5's first bullet, where
      a *source literal* is trusted and nothing computed ever is (`E0417` already refuses the
      second half). The arm is `crates/nvs-ir/src/lower/convert.rs:351`; a helper row would go
      beside `Helper::BytesToString` at `crates/nvs-ir/src/ir.rs:1718` and gets its symbol at
      `crates/nvs-codegen/src/emit.rs:3613`. What it has to produce is
      `crates/nvs-stdlib/src/instance.rs:242`'s one-slot value over
      `crates/nvs-stdlib/src/html.rs:121` — decide helper-versus-registry-symbol *first*, since
      `nvs-runtime` cannot reach `nvs-stdlib`.
- [ ] **`Markup + Markup` is `Markup`** — § 5's composition rule, and the one operator row an
      object type gets. `crates/nvs-types/src/expr/operators.rs:321` is `binary_result` and
      `crates/nvs-types/src/expr/operators.rs:344` is `BinaryOp::Add`'s arm, which refuses every
      other object today; both operands must be the carrier and nothing else.
- [ ] **Three `.nvst` cases over the pair**, in `tests/conformance/core/`: a literal converted
      and echoed raw, a computed string refused at compile time, and two fragments composed.
      The gap paragraph they close is `crates/nvs-stdlib/src/html.rs:20`.

## Backlog

- `Core\Html::sanitize` and ADR 0122's WHATWG parser wait on `Core\Xml`'s tree — ADR 0051 § 3.
- `[context] adrs` wants `0024 § 5` and `0007 § 2` — `docs/agent/loop-goal.toml`.
- `truncate` and `lock` on `Core\IO\File` — the plan's *Open now*, stage 2.
- `Core\Cli::displayWidth` — the plan's *Open now*, stage 3.
- Reading `[log] target` — the plan's *Open now*, stage 7.
