# Handoff

## State

**M4 — language completeness.** Item 16 is half closed: a **named** or **spread** call
argument now type-checks, with the parameter it fills settled by the checker and handed
down. The lowering half is still open, so a well-typed `f(...$xs)` still panics at
`crates/mwl-ir/src/lower/call.rs:75` — unchanged, not a regression.

The mapping is `mwl_types::expr_table::ArgSlot` (`Param(i)` / `Spread(i)` / `Unresolved`),
one entry per written argument, recorded on `ResolvedCall::arg_slots`. `mwl-ir` cannot
re-derive it: a name resolves against `MethodSig::param_names`, a new
`Option<Vec<String>>` that is `None` for every signature with no source names — every
`Core` row, the synthesized `Throwable` constructor, the seeded interfaces. `None` is not
`Some(vec![])`, and that is the whole reason it is an `Option`: it is what makes
**E0485** ("write ADR 0063 R2's options bag") a different diagnostic from **E0486**
("no such parameter") at a zero-parameter user method.

The rules and their reasons are on `mwl_types::expr::args::map_arguments`
(`crates/mwl-types/src/expr/args.rs:145`), which is reached only by a list containing a
`name:` or a `...` — an all-positional list keeps its own identity mapping and its own
count-against-count arity message, so no existing call site's diagnostic moved. New
codes: **E0485**–**E0489**. Two findings worth not re-deriving: **E0484 has no call-site
twin** (a variadic parameter always supplies an `array<T>` expectation, so the mismatch
is the better `E0401` — `declared_for`'s doc comment owns it, and the item's own text
predicted otherwise), and a spread binds a *generic* variadic tail for free, `array<T>`
against the subject's own array type being the same rule one element at a time.

`verify.py` 6 of 6 green — conformance **586**, differential **163**, 1629 unit tests.
`python tools/holes.py` is unmoved at **34 sites**: this slice added no lowering.

## Next group

**Item 16's lowering half first** — it is the only thing standing between a checked
`f(...$xs)`/`f(name: v)` and a program that runs, and everything it needs is now on disk.
It and the third slice are two file sets; take the third only with room left.

- [ ] **Item 16's lowering half: reorder by `arg_slots`** — `crates/mwl-ir/src/lower/call.rs:60`
      (`lower_call_args`, whose `assert!` at `:75` is the panic) and `:219`
      (`lower_variadic_tail`). Read `ResolvedCall::arg_slots` through
      `ArgSig` (`crates/mwl-ir/src/lower/mod.rs:1193`), which today copies only
      `param_tys`/`by_ref`/`variadic`/`defaults` off the resolved call. The checker
      guarantees the shape: every slot is `Param(i)` or `Spread(i)`, at most one argument
      per fixed parameter, every `Spread` lands at the variadic index with every fixed
      parameter already filled — so the lowering never has to refuse anything, it has to
      *place* each lowered value at its parameter's ABI position and fall back to
      `emit_const_arg` for a parameter no slot named.
- [ ] **The spread's own entries into the variadic tail** — same file. The tail already
      builds one fresh array; a `Spread` contributes its subject's entries to that array,
      which is the `InstKind::ArraySpread` (`crates/mwl-ir/src/lower/expr.rs:3645`) and
      the `mwl_array_spread` helper item 17 landed. Watch the refcount edge item 17
      already paid for: the subject is **borrowed**, so a freshly-built one is staged as
      an owned temporary, and the array under construction is staged and re-pointed
      after every write.
- [ ] **A positional element after an explicit key still numbers from its own position**
      — `crates/mwl-ir/src/lower/expr.rs:3657`'s keyed shape, the one divergence a
      spread-carrying literal no longer shares. `[1, "k" => 2, 3]` puts `3` at `"1"`
      where PHP puts it at `"2"`, and the fix is the `ArrayAppend` the spread path
      already takes. `lower::tests::a_positional_element_after_an_explicit_key_keeps_its_own_position_counter`
      pins the current answer and `ir::InstKind::ArrayNew`'s doc comment records it as
      deliberate.

## Backlog

- `mwl-ir` gap 1: `Class::method(...)` first-class callable panics — `mwl-ir`'s crate docs.
- Calling a closure through the variable holding it does not lower — `mwl-ir`'s crate docs.
- ADR 0007 § 2's `array<T> as array<U>` conversion row does not lower — `lower/expr.rs:877`.
- `lower_decl_type`/`lower_checked_ty` catch-alls: `decimal`, `never`, `iterable`,
  `self`/`static`/`parent`, a shape and an intersection as a *declared* type — no item names them.
- An abandoned generator's `finally` (loop-goal § *Standing decisions*) is still unbuilt.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
