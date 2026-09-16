# Handoff

## State

**Goal `unowned-closures`, stage 2.** Two gaps closed and struck.

`crates/nvs-runtime/src/budget.rs` gap 1: `nvs_runtime::affordable` — the one seam every count-shaped
`Core` argument passes through — asks `budget::affords` in front of the allocation, so no single
operation takes a request past its ceiling before a poll sees it. Two tiers, and they are not the same:
a size no process could hold is still a catchable throw, and one *this request* cannot hold is
`rule:errors/on-limit`'s FATAL, because `affords` has recorded the breach by the time it is worded.
`crates/nvs-runtime/tests/refusal.rs` guards both.

`crates/nvs-ir/src/lib.rs` gap 21: an erased read runs the property's `get` hook.
`read_erased_property_hinted` takes the `Ctx` its callers already hold and asks `ClassDesc::hook_row`
before it reaches a slot, which `write_erased_property` already did for the `set`, so
`rule:types/erased-member-access`'s last clause now holds for every spelling of the read.
`crates/nvs-codegen/tests/property_hooks.rs` guards both directions.

Stage 2's `cargo-named` check also names
`a_secret_compared_against_a_mixed_string_takes_the_constant_time_helper`, which existed under a
near-miss name and is renamed — both of the check's tests exist now.

Stage 1's floor is goal `m8-stdlib-depth`'s whole list, carried and untouched. Nothing is blocked.

## Next group

**Stage 2: the runtime's own `Decided` list** — one file set: `crates/nvs-runtime/src/`.

- [ ] **A command's union-typed capture reads the union's members** —
      `crates/nvs-runtime/src/commands.rs:57`'s gap 1, whose `Decided:` sentence is "support it: read
      the union's members in the command table". `nvs_types::routes::closed_set` answers `None` for a
      union and is deliberately left that way — it also serves
      `rule:routing/a-capture-narrows-to-a-closed-set`'s route captures, whose spelling is
      `Core\Router::match`'s and out of this goal — so the members are read here, where the command
      table owns the spelling, rather than by widening that function under a second caller.
- [ ] **An array header's element-type descriptor** — `crates/nvs-runtime/src/array.rs:221`'s gap 1,
      whose `Decided:` sentence is "add it with the first reader". Nothing builds an array from `mixed`,
      `json_decode` or an isolate boundary yet, so the item is to settle whether that reader exists
      today: if it does not, the goal's § *Standing decisions* "state it as a bound" answer strikes the
      gap as a bound in `ArrayHeader`'s own prose rather than leaving it open, and `rule:types/arrays`
      is what the bound is written against.
- [ ] **The route walk's linear scan** — `crates/nvs-runtime/src/routes.rs:88`'s gap 1, whose `Decided:`
      sentence is "measure on benches/serve-proxied.json first, build only if it shows". The measurement
      comes before any build, and `Routes::match_request` is the one function a trie would replace —
      `rule:routing/path-grammar`.

## Backlog

- Stage 2's remaining runtime `Decided` items: `crates/nvs-runtime/src/lib.rs` gaps 1, 2, 6, 7, plus
  `decimal.rs` and `graph.rs` — `docs/agent/loop-goal.md` § *Stage 2*.
- `crates/nvs-ir/src/lib.rs` gap 2 (a fresh value leaking on the throw path) is the stage's "no choice
  left" work on its own now — `crates/nvs-ir/src/lower/mod.rs:2223` `landing_block` is the boundary, and
  gap 1 was struck by an earlier goal.
- `crates/nvs-ir/src/lib.rs` gap 18 (a throw escaping an abandoned generator's `finally`) is
  `nvs-runtime`'s build, not the lowering's — `Ctx::with_pending_set_aside` and `object::dismantle`.
- `crates/nvs-ir/src/lib.rs` gap 14 is stage 6's, retagged `M10` rather than built — `docs/plan/m10.md`.
