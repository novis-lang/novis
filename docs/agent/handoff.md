# Handoff

## State

**A property's declared default runs.** `public int $n = 4;` reaches the slot of every fresh
instance, inherited defaults included, and `public string $s = "x";` no longer dereferences a null
pointer. The expression is evaluated once, at signature collection, into the same `ConstArg` a
parameter default becomes (`mwl_types::defaults`, whose module doc owns what one may be and why an
initializer cannot be spliced between `InstKind::New`'s allocation and its constructor call); it
travels as data — `ClassSignature::property_defaults` → `ExprTypeTable::property_defaults` →
`ir::Class::defaults` (joined against the flattened slot order in `lower_program`) →
`ClassTable::set_defaults` → `ClassDesc::defaults`, written by `MwlObj::new`. A bad one is **E0472**.
`mwl-ir` emits no new instruction, so no `print_function` snapshot moved.

**The `->`/`[]`-through-a-call-result leak is closed on both legs.** `lower_index` now mirrors
`lower_property_access`: it stages a temporary base, retains the element it read, then releases the
base, and `Lowering::aliasing_read` recurses into an index's own base as well as a property access's.
Pinned by `an_index_read_off_a_temporary_retains_its_result_and_releases_the_base` and by
`tests/conformance/lang/an-index-read-through-a-call-result-keeps-its-value.mwlt`.

Verify is green (1532 tests). Valgrind is clean over both new edges — a string/array property
default through `new`, `clone` and an inherited chain, and `$m->all()["0"]` in a loop.
`examples/collect.mwl` still exits 1 at `Core\Out::capture`, which is the known frontier.

**Spec § 9 still owes only its `Iterable`.**

## Next group — § 9's `Iterable`, then the two § 10 gaps

**Shared file set:** `crates/mwl-stdlib/src/{registry.rs,objmap.rs,heap.rs}` and
`crates/mwl-types/src/{core_lib.rs,iter_lib.rs}` for item 1;
`crates/mwl-types/src/error_lib.rs` plus `crates/mwl-ir/src/lower/exception.rs` for items 2 and 3.

- [ ] **1. § 9's `Iterable`.** `Core\Heap`, `Core\ObjectMap` and `Core\ObjectSet` each *declare*
      `Iterable` and none of them satisfies it, so a `foreach` over one does not compile.
      `mwl_types::iter_lib` seeds the compiler-owned interfaces and
      `signatures::resolve_iteration_element` is what a `foreach` asks; ADR 0053 § 2 owns the
      concrete-type-argument rule. Start by grepping `registry::CLASSES` for the three `Iterable`
      rows and `iter_lib`'s seeding, and settle whether a `Core` instance answers a `foreach`
      through the method table or through a native drive — say which in `mwl-stdlib`'s module doc.
- [ ] **2. § 10's `{previous: $e}` constructor option**, which ADR 0071 § 5's
      one-throw-lists-every-bad-field rule needs. `mwl_types::error_lib` holds the tree's
      synthesized signatures; `mwl_ir::lower::exception` holds the synthesized constructors.
- [ ] **3. § 10's `$e->location`**, the same two files, one slot along.

## Backlog

- `Core\Json::decodeAs<T>` — `mwl-stdlib`'s `json` gap 2; the written call-site type argument it
  waited on exists now.
- `Core\Out::capture` — the last key in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`, and
  `examples/collect.mwl`'s first failing line. Lands with M4S's sink work (ADR 0092).
- `do`/`while` does not lower — `mwl-ir`'s own known-gaps list.
- A `?array<T>` cannot be indexed after a `!= null` guard — `mwl-ir` panics at `lower/expr.rs`;
  the playbook names the three spellings that do lower.
- Stage 4's counts: conformance 421 of 600, differential 89 of 150 — `docs/agent/loop-goal.md`.
- ADR 0088's qualifier classification on `mwl-stdlib`'s member rows — plan § *Open now*.
