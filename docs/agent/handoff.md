# Handoff

## State

**Goal `decided-closures`, stage 3 — the checker and the front end.** `python tools/owners.py
--closes decided-closures` now names **4**, and every one of them is stage 3:
`crates/nvs-diagnostics/src/embedded.rs:30`, `crates/nvs-syntax/src/lib.rs:96`,
`crates/nvs-types/src/defaults.rs:58`, `crates/nvs-types/src/response.rs:29`. **Stage 4 — the
library — owns no gap any more.** The other two stage-6 gates stay green.

**What the help page does now, and where its layout is stated.** `page_for` takes the handler's
`nvs_runtime::MethodRow` as a parameter (`crates/nvs-stdlib/src/command.rs:860`) and `signature_of`
(`crates/nvs-stdlib/src/command.rs:844`) resolves it from the row's `Class::method` label through
`Ctx::class_desc`; a caller with no context passes `None` and the page prints no declaration column.
An entry line is `spelling  type = default  about`, both halves of the declaration or neither, and a
flag names its type alone. The module doc § *What a page looks like* is that layout's one home
(`crates/nvs-stdlib/src/command.rs:11`).

**`Core\Reflect\ParameterInfo` answers a type now** (`crates/nvs-stdlib/src/reflect.rs:1106`), which
is what that class's own doc comment said would happen the day the spelling reached the descriptor —
`null` only for a row the table named no type for. It is `PropertyInfo::type`'s pair, and a promoted
constructor parameter's two answers are pinned as agreeing.

## Next group

**Stage 3: the checker's last two register items** — one file set: `crates/nvs-types/src/` alone,
plus `crates/nvs-ir/src/lower/call.rs` for the second.

- [ ] **Strike `response.rs` gap 1 as the stated bound it already is —
      `crates/nvs-types/src/response.rs:29`.** Its `Decided:` is *keep the run-time default*, so
      there is nothing to build: rewrite the paragraph as the module's own prose — a mount's entry
      script runs a `.nvs` file's top-level frame, so § 4's sixth row is answered at run time by § 3's
      default (`echo` means `text/html`, the last body member wins the `Content-Type`) — and delete
      the "Known gap" framing with its `Decided:` and `— owner:` lines. Check
      `rule:security/response-body-is-one-typed-member` does not promise a static refusal there; amend
      the fragment in this slice if it does.
- [ ] **Build `defaults.rs` gap 1: a named constant at a parameter default —
      `crates/nvs-types/src/defaults.rs:58`.** `Decided:` is *allow it, the call-site emitter carries
      the parameter's own IR type*. `literal_default` (`crates/nvs-types/src/defaults.rs:488`) is what
      widens, and the reason it could not before is
      `nvs_ir::lower::emit_const_arg` (`crates/nvs-ir/src/lower/call.rs:606`), which would hand a
      `ConstArg::Int` (`crates/nvs-types/src/defaults.rs:100`) to a `nvs_ir::ty::Ty::Enum` position —
      so the emitter takes the position's type first and the decoder follows. The property half
      already accepts it, so the cases are the parameter half of what
      `rule:types/literal-types` bounds: an enum case and another class's `const` as a parameter
      default, and the `uint $n = Limits::MAX` above `i64::MAX` that stays refused.

## Backlog

- `crates/nvs-diagnostics/src/embedded.rs:30` and `crates/nvs-syntax/src/lib.rs:96` — the other two
  stage-3 register gaps, each with its own `Decided:` sentence in its module doc.
- `crates/nvs-runtime/src/object.rs:654` — `param_types` is empty for every native `Core` row, on
  `param_names`' terms; `nvs_stdlib::registry::CoreMethod::params` is where a spelling for one would
  have to come from if a reader ever needs it.
