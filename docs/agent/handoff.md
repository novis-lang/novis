# Handoff

## State

**Spec § 9 owes only its `Iterable` now.** `Core\Heap<T>` landed whole —
`crates/mwl-stdlib/src/heap.rs` is one `array<T>` slot kept as a binary heap plus a `comparator` slot,
and its own module doc owns the three decisions: `peek` answers the *smallest* element, `peek`/`pop`
throw on an empty heap rather than answering `?T`, and the ordering is the constructor's comparator,
else ADR 0013's `Comparable::compareTo`, else `crate::ordering::compare_values`. `comparator_sign`
moved to `ordering.rs` (two domains read a verdict now); `arr.rs` keeps a one-line wrapper that
qualifies the member name.

Two capabilities landed under it, both general rather than heap-shaped:

- **A `Core` class's constructor may take arguments.** `registry::CONSTRUCTORS`'s second half is a
  whole `CoreMethod` named `constructor`, `mwl_types::core_lib::seed` seeds it as an ordinary instance
  signature, and `mwl-ir`'s `lower_new` lowers a `Core` `new` with **borrowed** arguments like every
  other `CoreCall`. So `new Core\Heap<int>(3)` is `E0401` from the same machinery every `Core` call
  uses. `Core\ObjectMap`/`ObjectSet` declare zero-parameter rows and are unchanged.
- **A compiled instance member is reachable from native code by name** — `mwl_runtime::dispatch`,
  promoted out of `sequence.rs` (which now uses it) and given an argument list. That is what closes
  ADR 0013's "reaching an instance method from a helper is not built yet", and `Core\Arr::sort`'s
  natural order over objects can now use it too — see the backlog.

Verify is green (1530 tests). Valgrind is clean over the heap's own refcount edges
(`.agent-tmp/heap-probe.mwl`, `heap-comparable.mwl`, `heap-empty.mwl`, `heap-peek-bind.mwl`).

**A call result read straight through `->` is never released** — `$m->make()->name` loses the object
every run, with no `Core` member involved. Found while valgrinding this slice; recorded in the plan's
*Open now* and in `playbook.md`. It is a leak rather than a missing feature, so it outranks breadth,
and it is item 1 below.

## Next group — the two `mwl-ir` lowering holes that produce wrong runtime behaviour

**Shared file set:** `crates/mwl-ir/src/lower/expr.rs` (`lower_property_access` at `expr.rs:2814`, its
`InstKind::FieldGet` at `expr.rs:2878`, the `new` arms at `expr.rs:2331`/`2479`), `lower/mod.rs`'s
owned-temporaries stack, and `crates/mwl-codegen/src/emit.rs:471` (`InstKind::New`).

- [ ] **1. A field read releases its base when the base is a temporary.** The producer hands back a
      fresh reference and a field read consumes nothing, so `$h->peek()->name` leaks. The Core-call
      arm at `expr.rs:2500`-ish is the shape to copy (`own_temporary` + `release_temporaries_since`),
      but the field read must retain its *own* result first — today it borrows out of the object, which
      is why binding to a local works and reading through does not.
- [ ] **2. A property's declared default runs.** `public int $n = 4;` reads back `0` unless a
      constructor assigns it (plan's *Open now*). Decide where it belongs — a synthesized prologue in
      `mwl-ir` before the constructor body, or the slot fill at `emit.rs:471` — and say so in the
      crate's module doc.
- [ ] **3. § 9's `Iterable`**, which all three of its rows declare —
      `docs/spec/01-core-library.md:683-685`. `mwl_runtime::sequence` already drives a cursor by name;
      what is missing is a `Core` class *satisfying* `Iterable<T>`, which is
      `crates/mwl-stdlib/src/instance.rs`'s question (a `Core` descriptor carries no method table).

## Backlog

- `Core\Arr::sort` over objects still throws instead of using `Comparable` — `mwl_runtime::dispatch`
  exists now, so `arr.rs:2710`'s known gap is a small slice (`ordering.rs` owns the message).
- A `Core`-owned instance has no compiled method table, so a heap of `Core\Time\Instant` needs an
  explicit comparator — `heap.rs`'s module doc, *Known gap*.
- § 10 owes the constructor's `{previous: $e}` options shape and `$e->location` — plan's *Open now*.
- § 6 owes `decodeAs<T>` (`json` gap 2); § 12 owes `Out::capture`, the one key left on the ratchet.
- `do`/`while` does not lower; ADR 0043's `by`-delegation is off path — plan's *Open now*.
- Stage 4's counts are their own work: conformance 419 of 600, differential 89 of 150.
