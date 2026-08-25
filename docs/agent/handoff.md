# Handoff

## State

**The `->`-through-a-call-result leak is closed.** `lower_property_access` stages a *temporary* base
on the owned-temporaries stack, retains the value it read out of the slot, then releases the base —
so `$m->make()->name` no longer loses an object per run. The matching half is
`Lowering::aliasing_read`, which now recurses into a property access's own base: a field read off a
temporary is a **fresh producer**, so no consumer retains it a second time. `mwl-ir`'s module doc §
*Refcount insertion is naive and syntactic* owns the rule; the IR shape is pinned by
`a_field_read_off_a_temporary_retains_its_result_and_releases_the_base` and the observable behaviour
by `tests/conformance/lang/a-field-read-through-a-call-result-keeps-its-value.mwlt`.

Verify is green (1531 tests). Valgrind is clean over the new edge (a chain, a `Core` argument, a
method receiver, a non-refcounted field, `?->` on both legs) and over six of the seven `examples/`
fixtures; `collect.mwl` still exits 1 at `Core\Out::capture` with **0 bytes lost**, which is the
known frontier and not a leak.

**The same shape is still open one door along:** `$m->rows()["0"]` leaks the array (112 direct + 248
indirect, measured). `lower_index` releases nothing, exactly as `lower_property_access` did. It is
item 2 below and it is a near-copy of what just landed.

**Spec § 9 still owes only its `Iterable`.**

## Next group — the two remaining `mwl-ir` lowering holes, then § 9's `Iterable`

**Shared file set:** `crates/mwl-ir/src/lower/expr.rs` (`lower_property_access` at `expr.rs:2814`
is the worked example for both of the first two), `crates/mwl-ir/src/lower/mod.rs`
(`aliasing_read` at `mod.rs:1657`, `lower_program`), plus for item 1
`crates/mwl-types/src/{defaults.rs,signatures.rs,layout.rs}`,
`crates/mwl-codegen/src/emit.rs:1367` (`emit_new`) and `crates/mwl-runtime/src/object.rs`.

- [ ] **1. A property's declared default runs.** Bigger and worse than the plan recorded: `public
      int $n = 4;` reads back `0`, `public string $s = "x";` **aborts with a null-pointer
      dereference** in `mwl-runtime`'s `string.rs:258`, and `public int $n = "no";` is not even
      type-checked. Nothing evaluates the expression — `signatures.rs:657` reads `p.default` only to
      decide ADR 0022's definite-assignment obligation. **`InstKind::New` carries the constructor
      call**, so no IR site can splice an initializer between allocation and construction; a
      constructor prologue cannot work either, because the ctor label is the *declaring* class's
      (`new Dog()` runs `Animal::constructor` and would miss `Dog`'s own defaults). The answer is a
      per-class **default image on the descriptor**, which also reaches `NewDynamic` and ADR 0071's
      native decoder. Chain, bottom up, each layer additive: `defaults.rs` gains an
      `eval_property_default` beside `eval_param_default` at `defaults.rs:149` (same literal decoder, plus `= []` →
      the existing `ConstArg::EmptyArray`, plus a property-flavoured diagnostic — next free type
      code is `E0472`); `ClassSignature` (`signatures.rs:299`) gains `property_defaults`;
      `ClassLayout` (`layout.rs:66`) gains a `defaults: Vec<Option<ConstArg>>` parallel to its
      already-flattened `fields`; `ir::Class` (`ir.rs:32`) copies it; `mwl-codegen` hands it to a
      new `ClassTable::set_defaults` (copy `set_codec` at `object.rs:437`); `ClassDesc`
      (`object.rs:159`) holds it and `MwlObj::new` — behind `mwl_object_new` at `object.rs:1031` —
      writes the slots instead of leaving them null. `mwl-ir` emits **no new instruction**, so no
      `print_function` snapshot moves.
- [ ] **2. An index read releases its base when the base is a temporary.** The direct copy of what
      just landed: stage the base with `own_temporary`, retain the element, `release_temporaries_since`,
      and extend `aliasing_read`'s new recursion to `ExprKind::Index`. `ir.rs:789` already documents
      that `ArrayGet` reads without retaining, the same way `FieldGet` does. Probe:
      `.agent-tmp/index-temp.mwl`.
- [ ] **3. § 9's `Iterable`**, which `Core\Heap`/`ObjectMap`/`ObjectSet` all declare — `mwl-stdlib`
      `heap.rs`/`objmap.rs`/`objset.rs`, ADR 0053 § 2.

## Backlog

- `Core\Arr::sort`'s natural order over objects can use `mwl_runtime::dispatch` now — `arr.rs`.
- § 6 owes `decodeAs<T>`; § 10 owes `{previous: $e}`, `$e->location`, `ParseError::issues`.
- § 12 owes `Core\Out::capture` alone — the `examples/collect.mwl` frontier; lands with M4S/ADR 0092.
- ADR 0088's qualifier classification on every `mwl-stdlib` member row — plan's *Open now*.
- `do`/`while` does not lower; an abandoned generator skips its `finally` (`mwl-ir` gap 18).
- Stage 4 counts: conformance 420 of 600, differential 89 of 150.
