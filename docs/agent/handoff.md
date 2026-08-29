# Handoff

## State

**Stage 6's item 20 is closed, and every one of the eight `cargo-named` names the Stage 6 acceptance check asks of `nvs-host` is green.** `spawn script` and `await` parse, type-check and lower; the boundary copies in, runs, copies out and releases wholesale; and cancellation is now pinned from both directions in `crates/nvs-host/src/isolate.rs`.

**What the two new tests fix in place.** A cancelled parent parked in `Started::join` tears its child down before the join returns — no orphan in the tree, nothing the child held outliving it, and ADR 0072 § 4's guarantee holding through a cancellation. A *running* child dies at the next safepoint it reaches rather than being unwound out of anyone's frame, and what it echoed before that point still crosses. The two reach the same `ok = false` by different routes, which is a fact about the child's stack and not about the boundary; `isolate.rs`'s module doc § *a refused argument is the parent's fault* now owns it, and it is why `finish`'s cancelled branch and `cancelled_completion` both exist.

**Known gap, unchanged:** the argument going *in* is not asked ADR 0023 § 2's unresolvable-class question, because the child's class table does not exist until its program's prologue installs it. `isolate.rs`'s module doc names the `nvs_runtime::script` seam change that closes it.

**Orientation gaps, unchanged and still real:** `[context] adrs` prints ADR 0023 § 2, ADR 0072 §§ 4-5 and ADR 0106 § 6 — ADR 0006's `## Decision` still has to be sliced by hand, and ADR 0106 § 2 is not printed either. `[context] modules` has no pattern for `nvs-runtime/src/graph.rs`.

## Next group

**The two Stage 8 `.nvst` cases the corpus check names under `tests/conformance/isolate/`, which do not exist yet.** They share that directory and the two cases already in it as models — `a-spawn-that-cannot-start-throws-in-the-parent.nvst` and `a-handle-is-collected-once-and-the-second-await-throws.nvst` — plus `examples/isolate.nvs`, which is the working end-to-end program. **Decide first, and say so in the case's comment: where the child file lives.** A `.nvst` case is one file, a child path is resolved against the *working directory* (`crates/nvs-cli/src/script.rs`'s module doc), and `nvs test tests/conformance/` runs from the repo root — so either the case spawns an existing `examples/isolate/*.nvs`, or a fixture directory beside the cases is introduced and that becomes the convention for every later isolate case.

- [ ] **`tests/conformance/isolate/a-child-shares-nothing-with-its-parent.nvst`** — ADR 0006 § *Decision*, ADR 0116 § 4. The statics base is the whole of it (`crates/nvs-host/src/isolate.rs:158`, `Ctx::isolate`); a child writing a static and the parent reading its own afterwards is the assertion, with the parent's output buffer as the second half.
- [ ] **`tests/conformance/isolate/a-childs-failure-is-a-value-not-an-exception.nvst`** — ADR 0006 § *Failure is a value*. `$result->ok`, `$result->error->class` and `$result->error->message` off the shape `await` answers with (`crates/nvs-types/src/expr/isolate.rs` owns the shape), with the parent running on afterwards — the `.nvst` twin of `an_uncaught_throw_in_a_child_leaves_the_parent_running`.

## Backlog

- Stage 7's four `nvs-test` names (`each_test_runs_in_its_own_isolate_sharing_only_compiled_code` and three others) exist nowhere yet — `docs/agent/loop-goal.toml`, stage 7.
- The argument-in direction of the unresolvable-class rule — `isolate.rs`'s module doc owns the gap and names the seam change it needs.
- `nvs_runtime::graph::walk` returns `Err` from its refusal arms without releasing the value it was handed, while its depth check releases — worth one valgrind pass over `Core\Serialize::encode` of a refused nested value.
- ADR 0006's `$result->valueOrThrow()` is `Core\Script::valueOrThrow($result)` and the member does not exist yet — item 22, `docs/plan/m5.md`.
- `Core\Secret::reveal()` is still absent, so ADR 0033's escape hatch is open at both ends — item 18.
- The 1000-case corpus count is M4's residue, met as the suite grows — `docs/implementation-plan.md`.
