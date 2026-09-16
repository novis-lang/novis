# Handoff

## State

**Goal `unowned-closures`, stage 2.** Three gaps closed and struck; the rest of the runtime's own
`Decided` list is untouched, and nothing is blocked.

`nvs_runtime::Tag` no longer carries `Closure` or `Resource`. A closure is an ordinary object — one
field per capture, one `invoke` method (`rule:types/callable-is-a-closure`) — and an engine-owned
handle is a `Core` class holding a key into its own context's table, so neither shape ever needed a
row: nothing constructed either tag, and every arm that named one now names the shape instead.
Discriminants 8 and 9 are left as **holes** rather than closed up, which is the one judgement call in
the slice: `nvs_ir::lower::param_tag_nibble` writes 10 and 11 down in a crate that cannot name `Tag`
(they are held together only by `nvs-codegen`'s `param_tag_nibbles_are_the_runtime_tag_bytes`), and
compiled code embeds a tag byte, so a discriminant that moves moves in two crates at once and in
every artifact already built against the old one. `crates/nvs-runtime/src/lib.rs`'s gap 1 is struck,
and the plan's § *Value representation* (`docs/plan/design.md:269`) now states the roster the code
has rather than the one it was drafted with.

Stage 1's floor is goal `m8-stdlib-depth`'s whole list, carried and untouched.

## Next group

**Stage 2: the runtime's own `Decided` list, continued** — one file set:
`crates/nvs-runtime/src/throwable.rs`, the crate doc's gap 6, the rule it amends and the guard that
prices it.

- [ ] **A runtime-raised exception captures a full backtrace** — `crates/nvs-runtime/src/lib.rs:216`'s
      gap 6, whose `Decided:` sentence is "Capture a full backtrace at every raise". `Thrown` is
      `crates/nvs-runtime/src/throwable.rs:389` and the raise it is reached from is
      `crates/nvs-runtime/src/throwable.rs:902`; the frames to capture are the ones
      `nvs_trace_push` already pushes (exported at `crates/nvs-runtime/src/lib.rs:386`), not an OS
      backtrace, so what has to be decided is only where the copy is taken and what it holds.
      `rule:errors/throw-is-not-slower` is the rule it contradicts.
- [ ] **The rule and the guard are re-priced in the same slice** —
      `rule:errors/throw-is-not-slower` says a throw allocates nothing, and the goal's § *Standing
      decisions* already overrides it for a runtime-raised exception, so the fragment states what a
      raise now costs; `benches/abi-probe/tests/perf_guards.rs:118`'s throw/return ratio is the
      assertion that fails first if it is not.

## Backlog

- `crates/nvs-runtime/src/lib.rs:199` gap 2 — `nvs_str_concat`/`concat_n` reuse a solely-owned left
  operand, with the ownership hand-off in `nvs-ir`'s lowering for those two calls.
- `crates/nvs-runtime/src/lib.rs:209` gap 5 and `:229` gap 7 — one decision, "a collector that runs
  only near the memory ceiling", so they are one build and not two.
- Stage 3's failing acceptance check (`nvs-hir`'s require-path decode and the autoload probe's unit
  key) is an artefact nothing has written yet, not a regression.
- `docs/agent/carried-gaps.md`'s `nvs-runtime` bullet now covers the string and backtrace halves
  only; its `[until:]` trailer still holds while gaps 2, 6 and 7 name this goal.
