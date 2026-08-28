# Handoff

## State

**M4's Stage 6 is the frontier. The `mixed` receiver's calling convention is
decided and its *descriptor* half is built; the dispatch half is not.**
`docs/adr/README.md` § *Decisions taken at project start* has the paragraph
that specifies all of it — the row carries arity and parameter tags,
`nvs_runtime::closure`'s `check_param_tags` is the one implementation both
erased paths share, and a tagged-ABI thunk per method was rejected on ranks 1,
3 and 5 together. Do not re-open it.

- **`nvs_runtime::MethodRow` is the method table's row**: name, code, `arity`,
  `param_tags`, `public`, `native`. That type's own doc comment is the home of
  what each field is for; `ClassDesc::method_row` is the reader the dispatch
  slice wants and `ClassDesc::method` is written over it.
- **`native` is the one field the ADR paragraph implied rather than named** —
  true for exactly `nvs_stdlib::instance`'s `Core`-owned rows, which borrow
  argument 0 and carry no signature this crate could read a shape off. It is
  what a `Core` receiver is refused on, so slice [2] does not have to find
  another way to ask.
- **Nothing has to be marshalled in either direction** and that is still
  load-bearing: `nvs-codegen`'s `store_value` (`emit.rs:3130`) already writes
  each argument and each return *with* its tag, `load_value` (`:3226`) reads
  the payload half at the callee's representation.
- M4's acceptance still names *Verification* sections for ADRs 0023, 0028 and
  0069; 0014 and 0046 have theirs.

## Next group

**The `mixed` receiver's dispatch half, in this order.** One file set:
`crates/nvs-types/src/expr/calls.rs`, `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-ir/src/ir.rs`, `crates/nvs-runtime/src/dispatch.rs`, and a case
under `tests/conformance/`. The descriptor half above is landed, so nothing
here reaches `nvs-codegen`'s class tables again.

- [ ] **Stop refusing, and record what the site wrote** — `nvs_types` already
      exempts `mixed` from `report_method_on_erased_receiver`
      (`crates/nvs-types/src/expr/calls.rs:623`, whose doc comment says so);
      what `infer_method_call` (`:48`) must now record is an `ExprInfo` the
      lowering can read — the member name and the argument list, checked as
      far as a receiver naming no class allows, with the call's type `mixed`.
      A `name:` argument has no signature to resolve against, exactly as
      through a `callable` (`E0712`), and takes that refusal.
- [ ] **Lower it** — `crates/nvs-ir/src/lower/expr.rs:2379`
      (`lower_method_call`), whose panic at `:2390` names this receiver alone.
      A new `Helper` beside `CallClosureArray` (`crates/nvs-ir/src/ir.rs:2078`)
      carrying ADR 0002's error edge, answering `Ty::Tagged`; the runtime end
      goes beside `crates/nvs-runtime/src/dispatch.rs:50`, reads
      `ClassDesc::method_row`, and throws catchably for a non-object tag, a
      missing name, `!row.public`, `row.native`, and an arity or tag mismatch
      through the same `check_param_tags` (`crates/nvs-runtime/src/closure.rs:393`)
      `call_closure` (`:132`) uses.
- [ ] **The deferral's own `.nvst`** — the call that dispatches, the missing
      name, the non-object receiver, the `private` member, a `Core` receiver,
      a wrong-tagged argument, and the agreement between the `mixed` spelling
      and the declared-class one counted rather than read off a line.

## Backlog

- A variadic or `inout` callee cannot be reached through an erased receiver:
  packed and written back at the call site — `E0721`'s limit, and the ADR
  paragraph names it. State it where the dispatch refuses it.
- `nvs_stdlib::instance`'s rows carry `arity: 0`/`param_tags: 0` because that
  crate holds no signature for a native member — `dispatch_table`'s own doc.
- ADRs 0023, 0028 and 0069 still owe M4 a *Verification* section
  (`docs/plan/m4.md`).
- `require` with a non-literal path runs nothing, silently, in both forms
  (`nvs_hir::requires`' own known gap).
