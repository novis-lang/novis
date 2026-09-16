# Handoff

## State

**Goal `unowned-closures`, stage 2.** One gap closed and struck; the rest of the runtime's own
`Decided` list is untouched, and nothing is blocked.

`crates/nvs-runtime/src/graph.rs` gap 1: a closure is its class's `ClassDesc::is_closure` bit, never
a declared `invoke`. `nvs_ir::ir::Class::is_closure` carries it from the two literals
`nvs_ir::lower::closure` mints, `nvs-codegen` hands it to `ClassTable::set_closure`, and both readers
ask the bit — `call_closure`'s `invoke_address` and `graph::refusable`. Native code hand-building a
closure for a `Core` member to call back into sets it at each of its sites. Before this, a class
declaring an `invoke` of its own was refused by `Core\Serialize::encode` as a closure;
`tests/conformance/core/serialize-a-class-declaring-invoke-is-not-a-closure.nvst` pins the bound from
both sides. `rule:types/callable-is-a-closure`'s *There is no `__invoke`* paragraph already promised
this, so the runtime was behind a rule rather than ahead of one and no fragment changed.

Stage 1's floor is goal `m8-stdlib-depth`'s whole list, carried and untouched.

## Next group

**Stage 2: the runtime's own `Decided` list** — one file set: `crates/nvs-runtime/src/ctx/` and the
two crates that install into it.

- [ ] **`Ctx` carries a `Core`-class resolver, installed at boot** — the plumbing half of
      `crates/nvs-runtime/src/graph.rs:61`'s gap 1, whose `Decided:` sentence is "install a
      Core-class resolver on Ctx at boot ... one more table installed the way routes and commands
      are". Mirror `crates/nvs-runtime/src/ctx/wiring.rs:413`'s `set_commands`, and fall back to it
      in `crates/nvs-runtime/src/ctx/error.rs:252`'s `class_desc`, which is the one route every
      `decode` call site already goes through (`crates/nvs-stdlib/src/serialize.rs:162`,
      `crates/nvs-stdlib/src/cache.rs:2500`, `crates/nvs-stdlib/src/session.rs:1010`). The table to
      resolve against is `crates/nvs-stdlib/src/instance.rs`'s leaked process-wide one, so a plain
      `fn` pointer costs nothing per request. Every boot site installs it:
      `crates/nvs-cli/src/main.rs:2175`, `crates/nvs-cli/src/runner.rs:596`,
      `crates/nvs-cli/src/runner.rs:1503`, `crates/nvs-cli/src/script.rs:772` — a missed one is a
      `Core` class that round-trips under `nvs run` and not under `nvs serve`, so count them.
- [ ] **`decode` rebuilds a `Core` instance rather than refusing it** — the behaviour half, and what
      strikes `crates/nvs-runtime/src/graph.rs:61`'s gap 1. `rule:classes/graph-copy`'s "an object
      whose class the receiving side cannot resolve is refused by name" stays true; what changes is
      which side can resolve. A conformance case beside
      `tests/conformance/core/serialize-round-trips-every-shape-the-walk-reaches.nvst` round-trips a
      `Core\Time\Instant`.

## Backlog

- `crates/nvs-runtime/src/lib.rs:199` gap 1 — delete `Tag::Closure` and `Tag::Resource`; same file
  set this session already opened (`value.rs`, `graph.rs:362`'s arm).
- `crates/nvs-runtime/src/lib.rs:206` gap 2 — `concat`/`concat_n` reuse a solely-owned left operand.
- `crates/nvs-runtime/src/record.rs:54`, `metrics.rs:110`, `routes.rs:90`, `decimal.rs:54` — stage
  2's remaining runtime gaps, each its own file set.
- `crates/nvs-runtime/src/graph.rs:69` gap 2 (an object holding a host handle) is `owner: unowned`,
  not this goal's.
