# Handoff

## State

**M4's Stage 6 is the frontier. The `mixed` receiver's calling convention is
decided, its *descriptor* half is built, and the checker now records the call;
the lowering and the runtime dispatch are what is left.**
`docs/adr/README.md` § *Decisions taken at project start* has the paragraph
that specifies all of it — the row carries arity and parameter tags,
`nvs_runtime::closure`'s `check_param_tags` is the one implementation both
erased paths share, and a tagged-ABI thunk per method was rejected on ranks 1,
3 and 5 together. Do not re-open it.

- **`ExprInfo::ErasedCall { name }` is what the site records** for a `mixed`
  receiver, and that variant's own doc comment is the home of why the name is
  all it carries. Nothing else was recordable: no signature means no argument
  mapping, so every written argument fills its own position.
- **Three spellings are refused at the site rather than deferred** — `E0712`
  (`name:`), `E0714` (`inout`) and the new `E0732` (ADR 0027's
  `$m->method(...)`, which names a closure value and not a call). Each code's
  doc comment in `nvs-diagnostics` owns its reason; the lowering does not have
  to answer any of them.
- **`nvs_runtime::MethodRow` is the method table's row**: name, code, `arity`,
  `param_tags`, `public`, `native`. That type's own doc comment is the home of
  what each field is for; `ClassDesc::method_row` is the reader the dispatch
  slice wants.
- **`native` is what a `Core` receiver is refused on** — true for exactly
  `nvs_stdlib::instance`'s `Core`-owned rows, which borrow argument 0 and carry
  no signature this crate could read a shape off.
- **Nothing has to be marshalled in either direction**: `nvs-codegen`'s
  `store_value` (`emit.rs:3130`) already writes each argument and each return
  *with* its tag, `load_value` (`:3226`) reads the payload half at the callee's
  representation.
- M4's acceptance still names *Verification* sections for ADRs 0023, 0028 and
  0069; 0014 and 0046 have theirs.

## Next group

**The `mixed` receiver's dispatch half, in this order.** One file set:
`crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-ir/src/ir.rs`,
`crates/nvs-runtime/src/dispatch.rs`, `crates/nvs-codegen/src/emit.rs`, and a
case under `tests/conformance/`. The checker half above is landed, so nothing
here reaches `nvs-types` again.

- [ ] **Lower it** — `ExprInfo::ErasedCall` reaches
      `Lowering::lower_method_call` (`crates/nvs-ir/src/lower/expr.rs:2379`,
      whose panic roster names `mixed` alone) and becomes one helper carrying
      ADR 0002's error edge: the receiver's tagged value, the member name as a
      constant, and the arguments in written order. It is a `Helper` rather
      than an `InstKind` for `CallClosureArray`'s reason — the count is a
      run-time fact — and the call's type is `Ty::Tagged`.
- [ ] **Dispatch it** — the runtime half in `crates/nvs-runtime`, beside
      `closure`'s `check_param_tags`, which is the one implementation both
      erased paths share: a tag that is not an object throws, a name the class
      answers with no row throws, and the three shapes the row cannot describe
      (a non-`public` member, a variadic or `inout` list, a `native` row) throw
      the wording the ADR README paragraph fixes.
- [ ] **The deferral's own `.nvst`** — the call that dispatches, the missing
      name, the non-object receiver, and the agreement between the erased
      spelling and the same call through the declared class, counted.

## Backlog

- A `require` whose path is not a string literal runs nothing at all, silently
  — `nvs_hir::requires`' own known gap.
- ADR 0033's container axis: an `array<T>` element and an ADR 0036 shape field
  carry no `secret` bit — `nvs_stdlib::debug`'s known gap 1.
- ADR 0092 § 6's `Throwable` record producer waits on the crate edge above
  `nvs-runtime`'s fatal path.
- `E0721`'s three unforwardable member shapes have no way out but writing the
  member by hand — `resolve_delegations`' doc comment.
