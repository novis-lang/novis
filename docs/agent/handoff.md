# Handoff

## State

**Spec § 9 is whole: all three collections answer a `foreach`.** A `Core` receiver reaches ADR 0053's
iteration protocol through its own descriptor's method table — the decision, the snapshot semantics and
the one convention difference (a member reached by name is handed its receiver's reference rather than
borrowing it) are `mwl_stdlib::cursor`'s module doc; `mwl_stdlib::instance`'s `DISPATCH_ROSTER` is the one
place a `Core` class's method table is written, and `registry::ITERABLES` is the checker-side roster
`core_lib` seeds a `ClassSignature::implements` entry from. A map yields its **keys**, a set its members,
a heap `pop` order, all non-destructive and re-iterable; `docs/spec/01-core-library.md` § 9 states that,
and its `Heap` row is amended to declare `Iterable` (PHP's heap iteration empties the heap; this one does
not, and without it a heap's contents were unreachable except by emptying it).

`Core\Cursor` has **no registry row** on purpose — nothing names it in a signature, since `iterate()`'s
return type is the seeded `Iterator<T>`. `mwl_types::expr::iteration::with_subject_args` is what turns
the `K`/`T` an `implements` clause names into the receiver's own argument.

Verify is green (1536 tests). Valgrind is clean over both new edges — a `foreach` over each of the three,
with `break`, a second pass, a removal inside the body, and a comparator-ordered heap.
`examples/collect.mwl` still exits 1 at `Core\Out::capture`, which is the known frontier.

## Next group — § 10's three gaps, which are one file set

**Shared file set:** `crates/mwl-types/src/error_lib.rs` (the seeded shape) and
`crates/mwl-ir/src/lower/exception.rs` (the synthesized constructors). That module doc's own *known gaps*
list is the specification for all three, and ADR 0071 § 5 is what item 3 exists for.

- [ ] **1. `{previous: $e}` on the constructor.** The slot exists and always yields `null`
      (`error_lib.rs:130` interns `Throwable|null`, `error_lib.rs:161` is the one-parameter
      `constructor(string $message)`); the seeded signature needs the options bag and
      `exception_constructor` needs the second parameter — `exception.rs:445`, whose slot order and
      transfer rules are written out at `exception.rs:432`.
- [ ] **2. `$e->location`.** The property is seeded (`error_lib.rs:136`) and
      `Lowering::write_throw_location` (`exception.rs:56`) fills it at the `throw`, so what is missing is
      the *read*: check what a program gets today for `$e->location` and pin it, since the synthesized
      constructor leaves it empty and a construction-site value is deliberately not what it holds.
- [ ] **3. `ParseError::issues` becomes readable.** `error_lib.rs:98` types it `array<Core\Issue>` and
      `mwl_stdlib::issue` builds the entries, but a case cannot read one out (the list is built and
      counted only). ADR 0071 § 5's one-throw-lists-every-bad-field rule is what needs it.

## Backlog

- `Core\Out::capture` — § 12's last member, and `examples/collect.mwl`'s first failure
  (`docs/implementation-plan.md` *Open now*; lands with M4S's sink work).
- `Core\Json::decodeAs<T>` — § 6's last member (`mwl_stdlib::json` gap 2).
- A `Core` collection is still not assignable to an `Iterable<T>` **parameter**, so
  `Core\Arr::from($set)` does not type-check: `expr::assign::class_satisfied` asks
  `mwl_hir::hierarchy::implements_interface`, which knows only the user `ClassGraph`. The runtime half
  already works (`mwl_runtime::sequence::drain` drives the same three names).
- `compareTo` on a `Core` instance is still invisible to `Core\Heap`'s ordering — the mechanism is now
  there (`instance`'s dispatch roster), so it is a row per class plus the transfer wrapper
  (`mwl_stdlib::heap`'s own known gap).
- Stage 4's counts are their own work: conformance 426 of 600, differential 89 of 150.
- `docs/spec/02-php-migration.md` is 31% classified, one pass per PHP domain
  (`python tools/check-migration.py`).
