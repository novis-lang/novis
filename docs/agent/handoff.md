# Handoff

## State

**Goal `m4-refusals` — Stage 7's shape row is decided and not yet built.** The `iterable`, `callable`,
union and intersection rows are landed. `python tools/holes.py` reports **3** refusal sites,
`UNATTRIBUTED: 0`, **15** guarded, and `crates/nvs-ir/tests/refusals.rs`'s `CEILING` is **3** to match.

- **Which walk answers a shape: neither of the two that exist**, and that is written into
  `crates/nvs-ir/src/lower/expr.rs:5581`'s `# Known gaps`, which is the decision this item owed. `as`
  performs no shape walk (`crates/nvs-ir/src/lower/convert.rs:1158` has no arm for one), and
  `Core\Arr::shapeAs`'s reads an `NvsArray` and builds an object out of it — its strict half
  (`nvs_stdlib::json::Reading::Wire`, `crates/nvs-stdlib/src/json.rs:1322`) is the per-field read `is`
  wants, over a document's keys rather than a receiver's fields.
- **So the row is an object field walk.** `rule:types/shape-type` makes a shape compile-time-only and
  structural with width subtyping, so the run-time question is whether this object carries the named
  fields at the named types: an O(n) `InstKind::SlotGet` walk, one `TestShape` per field, chained by
  `TestShape::All` behind the object tag so a subject holding no object declines before a field is read.
- **What blocks building it is a presence probe that does not throw.** `AbsentKey::Null`
  (`crates/nvs-ir/src/ir.rs:831`) answers an absent field with `null`, which a `{a: ?int}` field cannot
  tell from an `a` holding one, and both `AbsentKey` arms throw on a slot never written.
  `rule:types/type-test` makes `is` total, so a subject missing a field answers `false` and raises nothing.
- **The floor's `examples/queue-purge.nvs` failure after session 0009 is a fixture race, not a
  regression** — green 14/14 here (6 serial, 8 concurrent), a healthy run 1.2s against that failure's
  16s, and its three waits now name themselves on timeout. The playbook bullet holds the triage.
- Nothing is blocked.

## Next group

**Stage 7: the shape row — its presence probe, then its field walk** — one file set:
`crates/nvs-ir/src/ir.rs` and `crates/nvs-ir/src/lower/expr.rs`, plus `crates/nvs-runtime/src/object.rs`
and `crates/nvs-codegen/src/emit.rs` for the helper the probe calls. `docs/agent/loop-goal.md`
§ *Stage 7* is the spec and `rule:types/type-test` is the rule.

- [ ] **A presence probe that answers instead of throwing** — a third `AbsentKey` arm, or an
      `InstKind` of its own, answering present / absent / unwritten without an error edge, since `is`
      is total. `crates/nvs-ir/src/ir.rs:831` is the enum's doc and `crates/nvs-codegen/src/emit.rs:2714`
      picks the helper off it; the runtime pair it joins is
      `crates/nvs-runtime/src/object.rs:3249` and `crates/nvs-runtime/src/object.rs:3313`.
- [ ] **The shape row over that probe** — a `TestShape` variant carrying one `(name, required, TestShape)`
      per field, built in `test_shape` from `Ty::Shape`'s `Vec<ShapeField>` and emitted as the
      `All`-chain above. The arm goes beside the `iterable` one at
      `crates/nvs-ir/src/lower/expr.rs:5592`, and `crates/nvs-ir/src/lower/expr.rs:5012`'s
      `emit_test_shape` is where it lowers.
- [ ] **The stage's case** — `tests/conformance/lang/an-is-test-against-a-shape-walks-its-fields.nvst`,
      which stage 7's check names and nothing writes yet; width subtyping, a missing required field and
      a `{a: ?int}` against both an absent `a` and a present `null` are the four rows.
      `crates/nvs-ir/src/lower/expr.rs:5592` is the row under test.

## Backlog
- The element row — `array<Foo>`, an array of shapes, an array of unions — and then the panic and
  `CEILING`; `crates/nvs-ir/src/lower/expr.rs:4985` (`docs/agent/loop-goal.md` § *Stage 7*).
- `rule:types/callable-signature`'s written signature is stage 7's third open row
  (`crates/nvs-ir/src/lower/expr.rs:5581`).
- `examples/queue/flaky.nvs` logs its throw twice under `maxAttempts: 1`; not investigated, and it
  does not move `dead-survives=1` (`examples/queue-purge.nvs:106`).
