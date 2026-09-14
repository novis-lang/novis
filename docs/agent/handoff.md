# Handoff

## State

**Goal `m4-refusals` — Stage 7's union and intersection row is landed; its other two rows are open.**
`python tools/holes.py` still reports **3** refusal sites, `UNATTRIBUTED: 0`, **15** guarded, and
`crates/nvs-ir/tests/refusals.rs`'s `CEILING` is **3** to match: `lower_type_test`'s panic still stands
for the rows below, with union and intersection struck from what it names.

- `crates/nvs-ir/src/lower/expr.rs:5012`'s `emit_test_shape` is now the one `is` comparison in this
  crate — every arm `lower_type_test` used to hold, reachable a second time because a union's and an
  intersection's rows are their members'. `crates/nvs-ir/src/lower/expr.rs:5195`'s `emit_test_chain` is
  that walk, and its `decided` parameter is the *whole* difference between the two: `true` for a union,
  `false` for an intersection, with the branch's edges swapped. No `Env` is merged, a member test
  binding nothing.
- `test_shape` (`crates/nvs-ir/src/lower/expr.rs:5589`) answers a union and an intersection by mapping
  over the interned members; one member with no row of its own makes the whole type `None`, so
  `is array<Foo>|int` stays a single known gap rather than a chain half of which lowers.
- The subject is lowered once and released once however many members read it, and a row that retains
  releases in the block it emitted into — `tools/leak-check.sh` over a 200-iteration
  `is array<int>|string` loop reports `0 failure(s)`.
- Nothing is blocked.

## Next group

**Stage 7: `is` over `iterable` and over `callable`** — file set:
`crates/nvs-ir/src/lower/expr.rs`, plus `crates/nvs-ir/src/lower/closure.rs` and
`crates/nvs-ir/src/lower/mod.rs` for the one marker `callable` needs. `docs/agent/loop-goal.md`
§ *Stage 7* is the spec and `rule:types/type-test` is the rule. Both rows go in one slice because
the acceptance case names them together.

- [ ] **`is iterable` — a chain of rows that already exist** — `TestShape::Any` of
      `Tag(Ty::Array)`, `Class("Iterable")` and `Class("Iterator")`, added beside the union arm in
      `crates/nvs-ir/src/lower/expr.rs:5589`. Both interface labels are already in every program's
      class table (`crates/nvs-ir/src/lower/tests.rs:3071`), so this needs nothing new —
      `rule:iteration/two-interfaces`, `rule:iteration/foreach-subjects`.
- [ ] **`is callable` — a closure needs a marker the descriptor carries** —
      `crates/nvs-ir/src/lower/closure.rs:296` and `:744` build the two closure classes with
      `conforms: Vec::new()`, and `Tag::Closure` is reserved and unused
      (`crates/nvs-runtime/src/value.rs:67`), so a closure is an ordinary object. `FN_INVOKE` is
      `"invoke"` (`crates/nvs-ir/src/lower/mod.rs:3562`) and a user class may declare a method of
      that name, so the method table is not the test.  **Recommended:** a reserved
      `fn#callable` interface — pushed once beside `classes.extend(synthesized)`
      (`crates/nvs-ir/src/lower/mod.rs:798`) and named in each closure class's `conforms` — which
      makes the row the `InstanceOf` walk every other class row is. It costs one descriptor per
      process and re-takes the `lower/snapshots` dumps; `rule:types/callable-is-a-closure`.
- [ ] **`is` over a shape, and over an `array<T>` whose element type no tag decides** —
      `crates/nvs-ir/src/lower/expr.rs:5589` (`test_shape`), whose `None` and known gap go with it.
      **Read the playbook bullet first:** `as {x: int}` is `E0711`, so there is no shape walk to
      call in answering mode and this row builds one. `rule:types/type-test`.

## Backlog

- Stage 7's prose in `docs/agent/loop-goal.md:163` says the shape row is "the field walk `as`
  performs for the same type"; `as` performs none, and the stage owns that sentence.
- `crates/nvs-ir/src/lower/expr.rs:5589`'s known gap still names an `array<T>` element no tag
  decides — it goes when that row lands, not before.
