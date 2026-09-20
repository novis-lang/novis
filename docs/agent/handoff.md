# Handoff

## State

Goal `core-arr-1-4` is reached: its acceptance check over the 14 members it names is green, and
`python tools/dossier.py --owed` names no `Core\Arr` member at all. All 56 now carry `about.md`,
three examples, one attack, one bench with a row in `docs/perf/members.ndjson`, and a test from
Novis and from Rust. Nothing is blocked.

`Core\Arr::shapeAs` finished this session. It reads an array as the type written at the call site, so
every proof names its type first: a bare shape cannot open a local declaration (`E0134`), so each
`.nvs` writes `type Signup = {name: string, age: int};` and passes `Signup`. The class form needs
`#[Core\Json\Derive]`; a class without it is refused at compile time with `E0821`, not at run time as
the reference card's `LogicError` row reads. An array type is `array<T>` only, so a keyed array of
text is `array<string>`.

Its two tests were already on disk and uncounted — see the playbook bullet this session added — so the
whole test edit was a `covers:` marker on
`tests/conformance/core/an-array-becomes-the-shape-it-was-asked-for.nvst` and on
`crates/nvs-stdlib/src/arr.rs`'s `an_array_hydrates_into_every_field_the_shape_names`.

`shapeAs` is 304.6 ns/op at a 3.2 ns calibration unit — 94.6 calibration units — and is 5 statements,
0 calls, 7 allocations and 454 bytes per op. It declares `calls 0` and the measurement agrees. It is
the one member in this class that allocates its own result: the 7 are the form the round builds plus
the instance and its fields.

The attack reaches its last step and stops at the request's memory ceiling. Its first three steps
hold: 200,000 keys the shape does not name are left out, three bad fields arrive as three issues on
one `ParseError`, and a value nested 10,000 deep is refused without exhausting the native stack.

## Next group

**Stage 2: the dossier, over the three sibling `Core\Arr` goals** — one file set:
`docs/agent/goals/dossier/91-core-arr-2-4.toml`, `92-core-arr-3-4.toml` and `93-core-arr-4-4.toml`,
plus whatever a red member names under `docs/examples/core/Arr/`, `tests/hostile/core/Arr/` and
`benches/members/core/Arr/`. Every member these three name is already complete, so each should be
green on its first session and the work is to run the check and close the goal;
`rule:testing/feature-proofs` is what each check spells.

- [ ] **Goal `core-arr-2-4` closes on its own check.** Run its `argv` and expect `nothing owed`, `0
      failed`, `0 failed`; the 14 members it names all report `complete` today.
      `docs/agent/goals/dossier/91-core-arr-2-4.toml:76`
- [ ] **Goal `core-arr-3-4` closes the same way.**
      `docs/agent/goals/dossier/92-core-arr-3-4.toml:76`
- [ ] **Goal `core-arr-4-4` closes the same way**, and is the last of the class.
      `docs/agent/goals/dossier/93-core-arr-4-4.toml:76`

## Backlog

- The `dossier: types:exception` floor check failed the driver's last acceptance sweep with `2 failed`
  of 13 hostile files, and passes standalone in 3.8s at 2-at-a-time. It ran at 8-at-a-time under the
  full floor, so this reads as a timeout under load rather than a regression; no file in
  `tests/hostile/types/` changed. `tools/dossier.py`'s hostile timeout owns it.
- `crates/nvs-runtime/src/lib.rs`'s memory-limit gap still says why the surviving reading is that
  small "is not checked" — the gap itself is untouched, only its two doc links were repaired.
