# Handoff

## State

**Spec § 8 is whole**: `crates/mwl-stdlib/src/path.rs` registers all nine `Core\Path` members plus
`Path::SEPARATOR` — the first `Core` constant that is a `string` — over no dependency at all. That
module's own docs own the three decisions it settled and the two path shapes it does not model (UNC,
drive-relative); the plan's *Open now* names them in one sentence rather than restating them.

`python tools/verify.py` green, 1407 tests; `mwl test tests/conformance/` is 363 cases, all passing.
The two new cases are `tests/conformance/core/path-decomposes-a-path-without-touching-the-disk.mwlt`
and `path-join-normalize-and-relative-to.mwlt`.

**`bytes` has no runtime representation, and that blocks spec § 7 outright.** `mwl_ir::Ty::Bytes`
exists and `mwl_types::Ty::Bytes` interns, but `mwl_runtime::Tag` has no `Bytes` variant and
`mwl_ir::ty`'s own doc says nothing constructs a *fresh* `bytes` value yet. So `Core\Encoding`,
`Core\Bytes`, `Random::bytes` and `Hash::of` cannot be built before a runtime tag exists — the
first `Core` member returning `bytes` is what that decision has to be made for. The next group
routes around it.

## Next group — `Core\Random`, spec § 11's first table

**Shared file set:** `crates/mwl-stdlib/src/random.rs` (new), `crates/mwl-stdlib/src/lib.rs`
(`pub mod path;` at `lib.rs:194`, the `.or_else` at `lib.rs:240`), `crates/mwl-stdlib/src/registry.rs`
(`CLASSES` at `registry.rs:597`), `crates/mwl-stdlib/Cargo.toml` + the root `[workspace.dependencies]`,
and `tests/conformance/core/`. The spec table is `docs/spec/01-core-library.md:705-720`. Copy the
module shape from `path.rs` — `pub const CLASS` with one `CoreMethod` row per member, then
`pub(crate) fn address(symbol: &str)`.

- [ ] **`Random::int`, `Random::float` and `Random::token`, with the CSPRNG dependency picked.**
      Spec § 11's opening line makes this a CSPRNG *always* — there is no insecure tier to add later —
      so the pick is against `docs/adr/0051-standard-library-tiers.md` § 4's two questions and owes
      three things (AGENTS.md): the `[workspace.dependencies]` line saying why that crate,
      `cargo deny check`, and `python tools/gen-attribution.py`. `int($min, $max)` is inclusive at
      both ends and throws when `$min > $max` (R4); `float()` is uniform in `[0, 1)`; `token` defaults
      to 32 bytes via `CoreMethod::defaults`, and is hex, so it needs no `bytes` value to exist.
- [ ] **`Random::pick`, `sample` and `shuffle`** — the three over `array<T>`, which is
      `CoreTy::Var("T")` in the parameter and in the return (`registry.rs`'s `CoreTy::Var` docs own
      how it binds). `pick` answers `?T` over an empty array rather than throwing, per R5. Read a
      borrowed argument array through `crate::arr::borrowed` (`arr.rs:727`) — never
      `MwlArray::from_raw` directly, which would release the caller's reference on drop.
- [ ] **Conformance cases under `tests/conformance/core/`** — one per group above. **A random answer
      cannot be asserted**, so pin the invariants: `Random::int(5, 5)` is `5`, a drawn value compared
      against its bounds, `Core\Arr::count(Core\Random::shuffle($a))` unchanged, `sample`'s count, and
      `pick` over a one-element array. Both cases are needed *in the same slice as the rows*, not
      after — `playbook.md` § *Writing a test case*'s first bullet says why.

## Backlog

- `bytes` needs a `mwl_runtime::Tag` and a fresh producer before spec § 7 or `Hash::of` — that
  module's own doc (`mwl-ir/src/ty.rs:140`) states the gap; the decision is `mwl-runtime`'s to record.
- `Core\Uuid` (§ 11's second table) is a `Core`-owned instance type, so it needs `CoreTy::Instance`
  and a `slots` roster — `registry.rs`'s `CoreTy::Instance` docs own the shape.
- §§ 9 and 12 are the rest of `examples/collect.mwl`: `ObjectSet`/`ObjectMap` additionally need
  `new Core\X<T>()` to parse (`docs/implementation-plan.md` *Blocking*).
- `Core\Str`'s twelve remaining § 1 rows and `Arr::diff`/`intersect` — `mwl-stdlib`'s own gap 1.
- ADR 0088 owes the registry a qualifier classification per member — plan *Open now*.
- An abandoned generator never runs the `finally` it is suspended inside — `mwl-ir` gap 18.

## Gaps in this goal's `[context]` manifest

`orient.py`'s **modules** map printed no `mwl-stdlib` and no `mwl-runtime` line, which is the crate
every Stage 3 slice is written in; `loop-goal.toml`'s `[context] modules` needs those two patterns.
Its **shapes** selection printed the `.mwlt` case, the diagnostic and the commit message, but not the
`Core` member shape, which is what a `Core` slice actually writes. Both cost a session a handful of
reads it should not have paid for.
