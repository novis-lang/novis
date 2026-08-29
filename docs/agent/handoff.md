# Handoff

## State

**Stage 8's two isolate corpus cases are on disk and green**, so `tests/conformance/isolate/` now holds four: the two that were already there, plus a child that shares neither a class static nor an output buffer with its parent (ADR 0006 § *Decision*, ADR 0116 § 4) and a child whose uncaught throw is data on the parent's result rather than an exception unwinding into it (§ *Failure is a value, not an exception*, with the success row named beside the failure one).

**Where a child file lives is settled and needed no decision**: `a-handle-is-collected-once-and-the-second-await-throws.nvst` already had the answer — a `--FILE child.nvs--` section, which the runner lands in the workdir the case runs from. The playbook's *Writing a test case* bullet is that fact's home; no fixture directory was introduced.

**The acceptance check the driver reports failing is Stage 7's, and it is an item still open rather than a regression.** `crates/nvs-test` is the `.nvst` runner (`case.rs`, `expect.rs`, `run.rs`) and has no `tests/` directory at all, so none of the four names that check asks for — starting with `each_test_runs_in_its_own_isolate_sharing_only_compiled_code` — can exist yet: ADR 0079's `#[Test]` surface is unbuilt. That is a stage of work, not a slice, and it is why this session took the handoff's group instead.

**ADR 0117's seam landed outside the loop, in an interactive session**: `crates/nvs-stdlib/src/registry.rs` has `MethodDoc`/`ParamDoc`/`ShapeKeyDoc`/`ErrorDoc` and `CoreMethod::doc`, every row says `doc: None` except `Str::length`, `Json::encode` and `Regex::match`, and `crates/nvs-cli/src/meta.rs` is `nvs meta --json` with its golden in `crates/nvs-cli/tests/meta.rs`. An options bag is documented as one `ParamDoc` per option under the option's name — the website's `ParamDocs` component looks a row up that way — and `ShapeKeyDoc` is proven by the emitter's unit test because no implemented member declares a keyed shape parameter. `cd website && npm run sync:core` reports `registry docs: nvs meta — 3 member(s) carry registry docs` with 0 warnings; publishing the site stays a user-fired step.

**Known gap, unchanged:** the argument going *in* is not asked ADR 0023 § 2's unresolvable-class question, because the child's class table does not exist until its program's prologue installs it. `crates/nvs-host/src/isolate.rs`'s module doc names the `nvs_runtime::script` seam change that closes it.

**Orientation gaps:** `[context] adrs` prints neither ADR 0006 § *Decision* nor § *Failure is a value, not an exception* nor ADR 0116 § 4 — all three had to be sliced by hand, and every remaining isolate case wants them. `[context] shapes` prints the `.nvst` shape without the `--FILE <path>--` auxiliary section, which is the whole answer to the child-file question above. `[context] modules` still has no pattern for `nvs-runtime/src/graph.rs`.

## Next group

**The three `tests/conformance/reject/` cases the Stage 8 corpus check names, none of which exists.** They share that directory, the `--EXPECTF-ERROR--` shape (which must reproduce the diagnostic's own indentation, and it widens with the line number), and the three refusal sites in `crates/nvs-types`. Each already has a Rust twin under `cargo test -p nvs-types`, so what a case adds is the diagnostic a developer actually reads.

- [ ] **`tests/conformance/reject/a-task-all-field-holding-a-callable-variable-is-refused.nvst`** — ADR 0072 § 1 and M5's own *Verify* paragraph, which names this as the easy one to skip. `E0773` is reported at `crates/nvs-types/src/expr/args.rs:799` (`bind_callable_shape`), the one place a `Core\Task::all` field is read; `E0774` (a field that is not an `fn` literal) is the sibling refusal at the same site and is worth the second block of the same case.
- [ ] **`tests/conformance/reject/a-secret-value-does-not-cross-a-boundary.nvst`** — ADR 0033 § 4. `E0775` is `reject_secret_boundary_argument` at `crates/nvs-types/src/expr/quals.rs:363`, called from `crates/nvs-types/src/expr/calls.rs:250`; it is a call-site rule for the reason `Core\Debug::dump`'s already is, so the case must write the argument out rather than declare a parameter.
- [ ] **`tests/conformance/reject/serialize-decode-refuses-a-tainted-operand.nvst`** — ADR 0023 § 3 and `docs/spec/01-core-library.md` § 13. No dedicated code: `Core\Serialize::decode`'s parameter carries `Qual::Sink`, so a `tainted bytes` argument is refused by ordinary assignability and the case pins the message that produces.

## Backlog

- The five `tests/conformance/task/` cases Stage 8's check names, all still missing — `docs/agent/loop-goal.toml` stage 8.
- `tests/conformance/core/a-graph-copy-round-trips-a-cyclic-value.nvst` — ADR 0023 § 2, over `Core\Serialize`.
- Stage 7 in full: ADR 0079's `#[Test]` surface and the four `nvs-test` names — `docs/plan/m5.md`.
- The corpus floors: conformance 970 against 1000, differential 200 against 205 — `docs/agent/loop-goal.toml` stage 8.
- Item 22: `Core\Script::args()` and `Core\Script::valueOrThrow($result)` — `crates/nvs-types/src/expr/isolate.rs`'s module doc.
- `benches/isolation.rs` and its guard test — M5's *Verify* paragraph.
- The 342 registry rows still at `doc: None` — ADR 0117, one `MethodDoc` beside each row, in the shape `crates/nvs-stdlib/src/str.rs`'s `LENGTH_DOC` has; a documented member's `ParamDoc` names must match the spec's or `npm run sync:core` warns.
