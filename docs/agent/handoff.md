# Handoff

## State

**M4's Stage 6 is the frontier, and the `mixed` receiver's calling convention
is decided but not built.** `docs/adr/README.md` § *Decisions taken at project
start* has the paragraph: the descriptor's method row carries the callee's
arity and parameter tags, `nvs_runtime::closure::check_param_tags` is the one
implementation both erased paths share, and a tagged-ABI thunk per method was
rejected on ranks 1, 3 and 5 together. That paragraph is the specification for
the three slices below; do not re-open it.

- **`nvs-ir` still panics at `crates/nvs-ir/src/lower/expr.rs:2389`**, whose
  roster names `mixed` alone. Every other receiver naming no class is `E0477`.
- **Nothing has to be converted in either direction, and that is load-bearing.**
  `nvs_runtime::abi::NvsFn` takes an array of 16-byte tagged `Value`s and one
  tagged `out` slot; `nvs-codegen`'s `store_value` (`emit.rs:3130`) writes each
  argument and each return *with* its tag and `load_value` (`:3226`) reads only
  the payload half at the callee's representation. What is missing is the
  callee's declared shape, not the marshalling.
- M4's acceptance still names *Verification* sections for ADRs 0023, 0028 and
  0069; 0014 and 0046 have theirs.

## Next group

**The `mixed` receiver, built.** Two file sets, taken in this order — the
descriptor half is `nvs-runtime`/`nvs-codegen`, the dispatch half is
`nvs-types`/`nvs-ir`, and only the first slice reaches `nvs_ir::ir`.

- [ ] **Grow the descriptor's method row to carry the callee's declared shape** —
      `crates/nvs-runtime/src/object.rs:236` (`methods: Vec<(String, *const u8)>`)
      and `:832` (`ClassTable::set_methods`), joined in
      `crates/nvs-codegen/src/lib.rs:957` (`bind_method_tables`) off
      `ClassEntry::methods` (`:471`). Arity and the nibble word come off
      `nvs_ir::ir::Function::params` (`crates/nvs-ir/src/ir.rs:184`) through
      `nvs_ir::lower::param_tag_nibble` (`crates/nvs-ir/src/lower/mod.rs:3019`) —
      `nvs-codegen` compiles each function and can record the pair under the very
      `{declaring}::{method}` label that function already joins on. The `public`
      bit is the one thing with no source below the front end: it needs a third
      element on `nvs_ir::ir::Class::methods` (`crates/nvs-ir/src/ir.rs:125`),
      today a `(name, declaring)` pair copied from
      `nvs_types::layout::ClassLayout::methods`. Do the whole row in one go —
      splitting it means rewriting `set_methods`' signature twice.
- [ ] **Stop refusing, and record what the site wrote** — `nvs_types` exempts
      `Ty::Mixed` at `crates/nvs-types/src/expr/calls.rs:90`, so the call falls
      through with no `ExprInfo` at all. It answers `mixed` (ADR 0007 § 2's one
      unchecked position) and needs a new `ExprInfo` variant
      (`crates/nvs-types/src/expr_table.rs:226`) carrying the method name and the
      argument list, there being no `ResolvedCall` to record.
- [ ] **Lower it** — `crates/nvs-ir/src/lower/expr.rs:2379` (`lower_method_call`),
      one new variadic `Helper` (`crates/nvs-ir/src/ir.rs:1397`) taking the
      receiver, the method name as a `Tag::Str` constant and then the arguments.
      `nvs_runtime::closure::call_closure` (`closure.rs:132`) is the precedent for
      the retain-then-`abi::call` half and `check_param_tags` (`:393`) for the tag
      half; `nvs_call_closure_array` (`:251`) is the shape of a variadic helper.
      Every failure is a catchable throw on ADR 0002's edge, worded as the
      diagnostic that names the same mistake statically.
- [ ] **The deferral's own `.nvst`** — the call that dispatches, the missing name,
      the wrong arity, the wrong argument tag, the non-object receiver, and the
      agreement between the erased spelling and the declared-class one, counted.

## Backlog

- A `require` whose path is not a string literal runs nothing at all, silently,
  in both forms — `nvs_hir::requires`' own known gap.
- M4 acceptance owes *Verification* sections for ADRs 0023, 0028 and 0069 —
  `docs/plan/m4.md`.
- `[context] modules` in `docs/agent/loop-goal.toml` names no `nvs-runtime`
  pattern, so `object.rs`, `closure.rs` and `abi.rs` — the whole file set this
  goal's current item is about — printed no map line. `nvs-codegen/src/emit.rs`
  is named but `lib.rs` is not.
- `[context] playbook` names two selectors that match nothing and `orient.py`
  said so twice: `Writing a test case > nvs-codegen has` and
  `Writing a test case > emitbinop's ordering`.
- Three live tools are the worklist: `tools/holes.py`, `tools/loop.py --list`,
  `tools/check-migration.py`.
