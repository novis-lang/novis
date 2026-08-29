# Handoff

## State

**A decision was reversed interactively on 2026-08-29, and it is this goal's next group.** ADR 0063 R2 now
says every `Core` parameter is callable by the `$name` `docs/spec/01-core-library.md` writes, and the
trailing options bag by `options` — the same rules a user-declared method already has. The ADR's body,
ADR 0117's scope and gap-3 paragraph, `docs/adr/ground-rules.md`, the spec's *How to read an entry*, and
the comments at every site that used to call the missing names "a decision, not a gap"
(`crates/nvs-types/src/core_lib.rs`, `signatures.rs`, `crates/nvs-diagnostics` E0485,
`crates/nvs-stdlib/src/lib.rs` § 3, `registry.rs`'s `ParamDoc::name`) are rewritten and say "Stage 0b lands
it". **Nothing about any member's shape changes** — no parameter is added, removed, reordered or renamed —
and the standing decision in `loop-goal.md` says what to do when a row's arity and the spec disagree.
`loop-goal.md` has the stage as items 28–30, `loop-goal.toml` has its three checks in the catch-up class
right after Stage 0's, and `[context]` carries four extra modules and `0063 §1` **that must be removed
when the three checks are green** — the comments there say so.

**Stage 8's two isolate corpus cases are on disk and green**, so `tests/conformance/isolate/` now holds four: the two that were already there, plus a child that shares neither a class static nor an output buffer with its parent (ADR 0006 § *Decision*, ADR 0116 § 4) and a child whose uncaught throw is data on the parent's result rather than an exception unwinding into it (§ *Failure is a value, not an exception*, with the success row named beside the failure one).

**The acceptance check the driver reports failing is Stage 7's, and it is an item still open rather than a regression.** `crates/nvs-test` is the `.nvst` runner (`case.rs`, `expect.rs`, `run.rs`) and has no `tests/` directory at all, so none of the four names that check asks for — starting with `each_test_runs_in_its_own_isolate_sharing_only_compiled_code` — can exist yet: ADR 0079's `#[Test]` surface is unbuilt. That is a stage of work, not a slice.

**ADR 0117's seam is on disk**: `crates/nvs-stdlib/src/registry.rs` has `MethodDoc`/`ParamDoc`/`ShapeKeyDoc`/`ErrorDoc` and `CoreMethod::doc`, every row says `doc: None` except `Str::length`, `Json::encode` and `Regex::match`, and `crates/nvs-cli/src/meta.rs` is `nvs meta --json` with its golden in `crates/nvs-cli/tests/meta.rs`. An options bag is documented as one `ParamDoc` per option under the option's name.

**Known gap, unchanged:** the argument going *in* is not asked ADR 0023 § 2's unresolvable-class question, because the child's class table does not exist until its program's prologue installs it. `crates/nvs-host/src/isolate.rs`'s module doc names the `nvs_runtime::script` seam change that closes it.

**Orientation gaps:** `[context] adrs` prints neither ADR 0006 § *Decision* nor § *Failure is a value, not an exception* nor ADR 0116 § 4. `[context] shapes` prints the `.nvst` shape without the `--FILE <path>--` auxiliary section. `[context] modules` still has no pattern for `nvs-runtime/src/graph.rs`.

## Next group

**Stage 0b — every `Core` member callable by name, items 28–30 of `docs/agent/loop-goal.md`.** One file
set: `crates/nvs-stdlib/src/registry.rs` and the `params: &[` tables in `crates/nvs-stdlib/src/*.rs`,
`crates/nvs-types/src/{core_lib,error_lib,iter_lib,signatures}.rs` and `expr/args.rs`,
`crates/nvs-diagnostics/src/lib.rs`, `crates/nvs-cli/src/meta.rs`. Read ADR 0063 § 1's R2 row and its
*Alternatives rejected* first bullet — that is the whole rule — and the `lang` case the `core` case is
modelled on. The three checks are `loop-goal.toml` stage `0b named arguments`; every test and case they
name is still to be written.

- [ ] **Item 28 — the names, on the row.** `CoreMethod::names`, one per positional slot; the bag is
      `options` once, on `CoreTy::Options`. Source is the spec's signature column, filled by a scratch
      script emitting one `splice.py --patch`, reviewed by the guard test that parses the same column
      (`every_registry_rows_names_are_the_specs_signature_column`). Where the script finds a row whose
      arity is not the spec's, the standing decision applies: the spec's names, the row's `params`, and a
      Backlog line here. Extend the doc-consistency test so a documented row's `ParamDoc::name`s equal its
      `names`.
- [ ] **Item 29 — the resolution.** `core_lib.rs` ~L97 reads `names` (+ `"options"` for a trailing bag)
      into `param_names`; `error_lib.rs` ~L195 becomes `["message", "options"]`; `iter_lib.rs` ~L157
      takes the spec's. Then no producer writes `None`: drop the `Option`, the `else` arm in
      `named_slot` (`expr/args.rs` ~L315), and retire E0485 the way `nvs-diagnostics` retires a code —
      grep `E0485` first. Prove with a scratch run, not by reading, that a reordered name, a skipped
      defaulted positional and a name at `Str::format`'s tail behave at a helper call; `meta.rs` emits
      `names` for every row. Rewrite the "Stage 0b lands it" comments at each site to the present tense.
- [ ] **Item 30 — the three cases** the `conformance (Core by name)` check names, in `core/`, `error/`
      and `reject/`. Then remove the four Stage 0b modules and `0063 §1` from `[context]` in
      `loop-goal.toml` *and* `docs/agent/goals/2-concurrency.toml`, which mirror each other.

## Backlog

- **The three `tests/conformance/reject/` cases the Stage 8 corpus check names** — the group this one
  displaced; they share `--EXPECTF-ERROR--` and the refusal sites in `crates/nvs-types`: `E0773`/`E0774`
  at `expr/args.rs` (`bind_callable_shape`), `E0775` at `expr/quals.rs` (`reject_secret_boundary_argument`),
  and `Core\Serialize::decode`'s `Qual::Sink` refusal by ordinary assignability.
- The five `tests/conformance/task/` cases Stage 8's check names, all still missing — `docs/agent/loop-goal.toml` stage 8.
- `tests/conformance/core/a-graph-copy-round-trips-a-cyclic-value.nvst` — ADR 0023 § 2, over `Core\Serialize`.
- Stage 7 in full: ADR 0079's `#[Test]` surface and the four `nvs-test` names — `docs/plan/m5.md`.
- The corpus floors: conformance 970 against 1000, differential 200 against 205 — `docs/agent/loop-goal.toml` stage 8.
- Item 22: `Core\Script::args()` and `Core\Script::valueOrThrow($result)` — `crates/nvs-types/src/expr/isolate.rs`'s module doc.
- `benches/isolation.rs` and its guard test — M5's *Verify* paragraph.
- The 342 registry rows still at `doc: None` — ADR 0117, one `MethodDoc` beside each row, in the shape `crates/nvs-stdlib/src/str.rs`'s `LENGTH_DOC` has; after Stage 0b a documented row's `ParamDoc` names must equal its `names`.
