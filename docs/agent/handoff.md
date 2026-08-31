# Handoff

## State

**`Core\Reflect\ClassInfo::call` is a row, a card and a body**, and the whole of ADR 0019 § 2's
visibility rule for it is `nvs_runtime::call_erased_method`: past the two questions a description
owes about its own subject, `call` hands the call to the path an ordinary `$value->name(...)` on a
`mixed` receiver takes. That path already asks every question this member owes — `public`, declared
at all, native, arity, per-parameter tag — and asks them on behalf of a site outside every class by
construction, which is exactly a reflective call site's premise. The helper's own doc comment in
`crates/nvs-stdlib/src/reflect.rs` is the home of why a second check written there would be the copy
that silently stops being the rule.

**`$arguments` is required, not defaulted to `[]`** — `registry::Const` has no array variant, and the
row's comment says so rather than approximating a signature the registry cannot express.

**`subject_of` is now shared by both acting members**, so the "this describes X, and the value is a
Y" `LogicError` and the not-an-object `RuntimeError` are written once.

**Stage 8's remaining named test is the reflective property write.** The shape of the problem is
worth knowing before starting: ADR 0014's hook is emitted **at the call site** by
`nvs_ir::lower::expr::emit_observer_call`, so a native member has no lowering to reach it and
`nvs_object_field_set` is not where the hook lives either. Whatever `set` does, it cannot be
`nvs_object_field_set` plus a retain. Stage 7 still owes the schema-identity test in the backlog.

**The driver's stage-1 warm-start check was failing on a stale `target/release/nvs.exe`, not on the
tree** — `cargo build --release` here, and it now reports `min 7.3 ms` against its 10 ms budget. The
playbook bullet under *Tooling* is the home of why it will recur.

## Next group

**The reflective property write — stage 8's last named item, over `crates/nvs-stdlib/src/reflect.rs`
and whatever seam ADR 0014's hook turns out to be reachable through. ADR 0014 is the specification
and `docs/agent/loop-goal.toml:2474` is the check that names the test.**

- [ ] **Find where a native member can reach ADR 0014's observer, and decide whether `set` can exist
      at all** — the hook is emitted at the call site by
      `crates/nvs-ir/src/lower/expr.rs:3539`'s `emit_observer_call`, and the runtime's own write is
      `crates/nvs-runtime/src/object.rs:2481`'s `nvs_object_field_set`, which knows nothing about it.
      If the answer is a new runtime entry point, that is the slice; if it is that the observer must
      be looked up off the `ClassDesc` the way `unwind` is, say so in the module doc. Decide and
      record — a design call here is pre-authorized.
- [ ] **`Core\Reflect\ClassInfo::set` — the five edits** — ADR 0019 § 2 and ADR 0014.
      `crates/nvs-stdlib/src/reflect.rs:408`'s `subject_of` is the shared subject check already
      written, `crates/nvs-stdlib/src/reflect.rs:715`'s `get` is the reader it mirrors, and the named
      test is `a_reflective_property_write_runs_the_hook_an_ordinary_write_runs`.
- [ ] **Three `.nvst` cases for the write, under `tests/conformance/core/`** —
      `crates/nvs-stdlib/src/reflect.rs:800` is the Rust half's fixture and
      `tests/conformance/core/reflect-call-runs-a-public-method-and-refuses-a-private-one.nvst` the
      `.nvst` shape; `mixed $x = $obj;` is the spelling for an erased receiver (`var $x: mixed` is
      `E0102`).

## Backlog

- Stage 7's schema-identity test — application code and the engine floor produce identical log
  records for one error; `docs/plan/m8.md` § *Verify*.
- Reflective *construction*, the third acting member ADR 0019 § 2 names — `crates/nvs-stdlib/src/reflect.rs`'s known gap 3.
- § 1's remaining `*Info` classes: a description names properties and no methods, so
  `get_class_methods` has no answer — same known-gap list.
- `Core\Ast`'s typed per-production roster and a node's own text —
  `crates/nvs-stdlib/src/ast.rs`'s module doc.
