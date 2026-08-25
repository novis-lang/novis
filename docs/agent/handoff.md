# Handoff

## State

**Spec § 2 is whole.** `Core\Arr::from` landed, and with it the parameter shape everything that reads a
sequence now declares: `registry::CoreTy::Iterated`, ADR 0053 § 3's three shapes interned as one union
(`array<T>|Iterable<T>|Iterator<T>`). Its own doc comment in `crates/mwl-stdlib/src/registry.rs` owns both
decisions — that a plain `array<T>` satisfies it, and that a helper reads it *by tag* rather than from a
pre-drained array. `crates/mwl-runtime/src/sequence.rs` is the one place such an argument is read: an array
walked directly, a cursor driven by name through its class descriptor's own method table, the way
`call_closure` already reaches a closure's `invoke`. Valgrind is clean over the new refcount edges.

Two checker holes were closed to make that shape work, both generic rather than special cases:
`expr::assign`'s union target now recurses per member (so `?Animal` accepts a `Dog`, which it did **not**
before — that is what `tests/conformance/lang/a-nullable-class-parameter-accepts-a-subclass.mwlt` pins),
and `generics::bind` gained a union arm plus an `implements`-aware one, so `Iterable<T>` binds `T` from a
class that fixed it in its `implements` clause. `bind` therefore takes the graph and the signature table now.

The ratchet (`crates/mwl-stdlib/tests/spec-members-outstanding.txt`) is down to **one key**, `§12
Out::capture`, which lands with M4S's sink work — so every registerable §§ 1-12 member is registered.
Conformance is 418 of 600, differential 89 of 150. `§ 9`'s `Core\Heap` and the `Iterable` its three rows
declare are the only *unwritten* §§ 1-12 members left; § 10 owes the constructor's `{previous: $e}` shape
and `$e->location`.

**A property's declared default is silently ignored** — `public int $n = 4;` reads back `0` unless a
constructor assigns it. Found while writing the cursor for the `from` case; recorded in the plan's *Open
now* and in `playbook.md`. It is a wrong value rather than a missing feature, so it outranks breadth under
AGENTS.md's ordering — take it before the group below if you have the context for `mwl-ir`'s `new`.

## Next group — § 9's collections finish the spec's Part I

**Shared file set:** a new `crates/mwl-stdlib/src/heap.rs`, `crates/mwl-stdlib/src/objset.rs` and
`objmap.rs`, and `crates/mwl-stdlib/src/registry.rs` (`CLASSES` at `registry.rs:709`, `CONSTRUCTORS` at
`registry.rs:749`). `objset.rs` is the model for both: `NEW_SYMBOL` at `objset.rs:19`, the `CoreClass` at
`objset.rs:28` with its `instance:` roster at `:31` and `slots:` at `:96`, and `crate::instance::receiver`
/`build` at `:127`/`:173`.

- [ ] **1. `Core\Heap<T>`** — `docs/spec/01-core-library.md:685` (`push`, `peek`, `pop`, `count`,
      `isEmpty`), ordering by [ADR 0013](../adr/0013-comparable-interface.md)'s `Comparable` or by a
      comparator given at construction. A `Core` instance's slots hold only values MWL already holds, so
      the heap is an `array<T>` in one slot maintained through `identity_store::borrow`/`edit`/`replace`
      — `Core\Hash\Stream` in `crates/mwl-stdlib/src/hash.rs` is the worked example of a *mutable* one.
- [ ] **2. The `Iterable` all three of § 9's rows declare** — `docs/spec/01-core-library.md:683-685`. This
      is the other direction from slice 1 above: a `Core`-owned class *implementing* a compiler-declared
      interface, so `CoreClass` needs to say what it conforms to and `iterate()` has to answer with a
      `Core`-owned cursor instance. `mwl_types::iter_lib` is where the interface's members are seeded, and
      `mwl_types::signatures::resolve_iteration_element` is what a `foreach` asks.

## Backlog

- **A property's declared default never runs** — the plan's *Open now*; a correctness item, not breadth.
- `§12 Out::capture` — the last ratchet key, lands with M4S's sink work (plan's *Open now*).
- § 10 owes the constructor's `{previous: $e}` options shape and `$e->location` — ADR 0071 § 5 needs them.
- `Core\Json::decodeAs<T>` — `mwl_stdlib::json`'s gap 2; the call-site type argument it waited on exists.
- Stage 4's counts are their own work: conformance 418/600, differential 89/150.
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir`'s own gaps).
