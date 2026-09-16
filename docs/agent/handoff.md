# Handoff

## State

**Goal `unowned-closures`, stage 2.** Two gaps closed and struck; the rest of the runtime's own
`Decided` list is untouched, and nothing is blocked.

A `Ctx` now carries a `Core`-class resolver. `Ctx::class_desc` asks the compiled unit's class table
first and that resolver second, so a payload naming a `Core` class resolves instead of being refused:
`Core\Serialize::decode` rebuilds a `Core\Time\Date` with its slots intact
(`tests/conformance/core/serialize-a-core-class-resolves-on-the-way-back.nvst`). The resolver is a
plain `fn` over `nvs_stdlib::instance`'s leaked process-wide descriptors — one word per context,
nothing per request — and it is installed in `nvs_codegen::Unit::install_in`, the call every embedder
already makes, rather than beside each `set_routes`; a derived context copies the word wherever it
copies the program's error class. `crates/nvs-runtime/src/graph.rs`'s gap 1 is struck and its gap 2
is the half that is left: a program still cannot **name** the class it got back, because `instanceof`
on a `Core` class is `E0496` and `as` is `E0711`. Both refusals are `nvs-types`'.

Stage 1's floor is goal `m8-stdlib-depth`'s whole list, carried and untouched.

## Next group

**Stage 2: the runtime's own `Decided` list, continued** — one file set:
`crates/nvs-runtime/src/lib.rs` and the two representation modules its gaps name.

- [ ] **Delete `Tag::Closure` and `Tag::Resource`** — `crates/nvs-runtime/src/lib.rs:199`'s gap 1,
      whose `Decided:` sentence is "Delete both tags — closures stay objects, handles stay `Core`
      classes, and the tag roster shrinks". The two rows are
      `crates/nvs-runtime/src/value.rs:67` and `:69`; nothing constructs either, and the arms that
      name them are the ones to delete with them — `rule:types/callable-is-a-closure` is why a
      closure needs no tag of its own, and `crates/nvs-runtime/src/graph.rs:61`'s gap 1 is why a
      host handle is a `Core` class rather than a `Resource`. The roster is a discriminant list, so
      check whether any of the remaining values is written down anywhere outside this crate before
      renumbering rather than leaving holes.
- [ ] **A runtime-raised exception captures a full backtrace** —
      `crates/nvs-runtime/src/lib.rs:223`'s gap 6, built at `crates/nvs-runtime/src/throwable.rs:413`
      where `Thrown::new` fills `message` and empties `backtrace`. The goal's § *Standing decisions*
      pre-authorizes this one against the sheet's recommendation, so it lands **with**
      `rule:errors/throw-is-not-slower` amended in the same slice to state what a raise now costs —
      that rule's guard is `benches/abi-probe/tests/perf_guards.rs`, and its 0.85 ns figure is about
      the propagation check rather than the raise, so read which of the two the guard measures before
      changing a number in the fragment.

## Backlog

- `tests/conformance/core/jwt-a-token-verifies-for-its-whole-lifetime-and-not-one-second-past-it.nvst`
  flakes on the second boundary — the playbook bullet under *Writing a test case* has the fix.
- `crates/nvs-runtime/src/graph.rs:77`'s gap 2 (a decoded `Core` instance cannot be narrowed) is
  `nvs-types`' and is nobody's yet; it is what makes the round trip usable from a program.
- `crates/nvs-runtime/src/graph.rs:61`'s gap 1, an object holding a host handle, is still unowned.
- `crates/nvs-runtime/src/lib.rs:216` and `:236` both decide the same near-ceiling collector, so
  they are one slice whenever they are taken.
