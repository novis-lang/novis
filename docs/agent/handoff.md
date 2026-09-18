# Handoff

## State

**Goal `test-doubles`, stage 3 is complete and green.** The structural check between a double's shape of
closures and the interface its type argument names now asks all three of its questions in
`nvs_types::conformance::check_double_answers` (`crates/nvs-types/src/conformance.rs:558`), hooked into
the static-call arm at `crates/nvs-types/src/expr/calls.rs:420`:

- `E0825` a method the interface requires and the shape leaves out (`double` only).
- `E0826` a field naming no method the interface declares, a `private` interface method included.
- `E0827` a type argument that resolved to something other than an interface — a deviation from the
  stage's prose, which planned to draw `E0465`; that code says a type argument is not a class, which is
  false of the class this refusal names.
- `E0828` a field that answers a method it cannot stand in for, new this session
  (`crates/nvs-types/src/conformance.rs:@check_answer_signatures`). It is a fourth code rather than the
  argument position's own mismatch because `$answers` is declared `object`, so `report_mismatch` there
  would say a shape is not an object; the code comment holds that call. The relation is plain
  assignability over `Ty::CallableSig`, so `rule:types/callable-arity` and `rule:types/callable-variance`
  are the whole of what a double is held to, and a field typed bare `callable` is the accepted case.
  `rule:testing/doubles`' fragment now states the third refusal.

Seven `cargo test -p nvs-types --test testing` cases and three `tests/conformance/reject/` cases cover it.
The runtime half of a double is already on disk — the rows at `crates/nvs-stdlib/src/test.rs:666` and
their helpers at `crates/nvs-stdlib/src/test.rs:3293` — and the module doc at
`crates/nvs-stdlib/src/test.rs:236` says slot 0 of a double's fields is the call record.

**Stage 4 is untouched**, and it is the driver's failing acceptance check: nothing reads that record yet.

## Next group

**Stage 4: the two assertions** — one file set: `crates/nvs-stdlib/src/test.rs`,
`crates/nvs-types/src/conformance.rs`, `tests/conformance/core/`, `tests/conformance/reject/`.

- [ ] **`assertCalled` and `assertNeverCalled` are registry rows with bodies** — goal prose stage 4.
      Two rows beside `double`'s at `crates/nvs-stdlib/src/test.rs:666`, their cards in the block after
      the class, their helpers beside `nvs_core_test_double` at `crates/nvs-stdlib/src/test.rs:3293`, and
      an `address()` arm each (conventions § *A `Core` member* — five edits). What they read is the call
      record the double already keeps, slot 0 of its fields per
      `crates/nvs-stdlib/src/test.rs:236`; `crates/nvs-stdlib/src/test.rs:1083` is the `double` card
      already promising it. `assertCalled`'s options are `{times, with}` and both are optional.
      `rule:testing/interaction-after-the-fact`.
- [ ] **A method reference is checked against the interface it names** — goal prose stage 4, and the
      refusal `tests/conformance/reject/a-method-reference-names-a-method-the-interface-lacks.nvst` the
      acceptance check names. The member argument is the first-class-callable spelling `T::method(...)`
      the standing decisions settle on; the membership question is the one
      `crates/nvs-types/src/conformance.rs:558` already answers for a field name, so decide there whether
      it reuses `E0826` or takes `E0829` (next free in the band).
- [ ] **The two conformance cases** — `assert-called-reads-the-record-by-count-and-arguments.nvst` and
      `assert-never-called-fails-naming-the-call-it-found.nvst` under `tests/conformance/core/`, the
      exact names the stage's check lists, shaped like
      `tests/conformance/core/a-double-answers-with-its-closures.nvst:1`. Never an `--ORACLE--` section
      under `tests/conformance/`.

## Backlog

- Stage 5, `assertCompletes` over a closure — goal prose, `crates/nvs-stdlib/src/test.rs`.
- `rule:testing/doubles` still reads *designed, not yet shipped*; flip it when stage 5 lands.
- The four proofs beyond ADR 0079's own bullets are goal `dossier`, not this one.
