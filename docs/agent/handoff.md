# Handoff

## State

**Goal `test-doubles`, stage 2 is closed.** `Core\Test::double<T>(object $answers): T` and
`Core\Test::partial<T>(T $real, object $answers): T` are registered, built and reachable: three
`tests/conformance/core/` cases compile and run them, `cargo test -p nvs-stdlib` is green including
the coverage floor of three per member, and the two keys they owed are struck from
`crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt`.

On disk in `crates/nvs-stdlib/src/test.rs`: the trampoline table (one native entry per slot against
`METHOD_CEILING`, `crates/nvs-stdlib/src/test.rs:3570`), the leaked descriptor cache keyed by
`(interface, real class, overridden names)`, the ledger a call is recorded in, and the two helpers.
The module doc at `crates/nvs-stdlib/src/test.rs:1` is still the specification and now also records
the one fact this session had to derive: **an interface's descriptor carries no `MethodRow` for a
method it merely declares**, so the shape's fields are the only list a `double` has and a `partial`
reads its delegated half off `$real`'s own method table.

**Nothing refuses a bad shape yet — that is stage 3.** `$answers` is typed `CoreTy::Object` because
what it really has to match is the interface the call site wrote, which no registry row can name. So
`double<Clock>({tomorrow: fn() => 1})` compiles today and its `now()` lands on the fallback a
bodiless declaration names; `rows_of` publishes `(0, 0)` for a field that is not a closure at all
rather than inventing a shape for it.

## Next group

**Stage 3: the refusals** — one file set: `crates/nvs-types/src/conformance.rs`,
`crates/nvs-types/src/expr/args.rs`, `crates/nvs-diagnostics/src/lib.rs`,
`tests/conformance/reject/`.

- [ ] **The two diagnostics, and the walk that compares a shape to an interface** —
      `crates/nvs-types/src/conformance.rs:99`'s `E_INTERFACE_METHOD_MISSING` is the existing
      home of "this does not implement that", and it already holds the interface's declared
      members; the call site that has the written target and the shape argument together is
      `crates/nvs-types/src/expr/args.rs:1708`, the branch
      `crate::derive::stands_in_for_a_class` guards. Two new `E08xx` codes — re-derive the next
      free pair from `python tools/brief.py` before claiming them. `rule:testing/doubles`.
- [ ] **`a_double_of_anything_but_an_interface_is_refused`** — the same walk's third refusal, a
      written type argument that names a class or a shape rather than an interface, beside the
      test at `crates/nvs-types/src/conformance.rs:566`. `rule:testing/doubles`.
- [ ] **The two `reject/` cases** — `a-double-missing-a-method-is-refused.nvst` and
      `a-double-declaring-a-method-the-interface-lacks-is-refused.nvst`, whose `--EXPECTF-ERROR--`
      has to reproduce the diagnostic's own indentation; the shape to copy is
      `tests/conformance/reject/a-bare-return-is-refused-where-the-declaration-is-not-void.nvst:1`
      and the bodies are `tests/conformance/core/a-double-answers-with-its-closures.nvst:6`'s
      interface with a field struck or added. `rule:testing/doubles`.

## Backlog

- Stage 4's two assertions (`assertCalled`, `assertNeverCalled`) read the ledger
  `crates/nvs-stdlib/src/test.rs`'s `record` writes: keyed by method name, one list of arguments
  per call, absent key for a method never called — `rule:testing/interaction-after-the-fact`.
- Stage 5's `assertCompletes` is untouched; its key is still in the outstanding file.
- `METHOD_CEILING` is 32 and its refusal is `Core\Test::double(): … at most 32`, asked by no case;
  a partial of a fat real class spends a slot per method behind it.
- `Core\BigInt` is goal `bigint`, still ahead of `gap-zero` — `docs/implementation-plan.md`.
