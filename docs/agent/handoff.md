# Handoff

## State

**Goal 14, stage 11 is closed: the editor has a reference chapter and both of its features carry
their proofs.** `docs/reference/tools/40-editor.md` is `id: editor` with the two `#` headings
`nvs lsp` and `nvs lsp-test`, so `tools/dossier.py`'s roster derives `tools:editor/nvs-lsp` and
`tools:editor/nvs-lsp-test` with nothing registered anywhere. Both stage-11 checks are green:
`--only … --gate` owes nothing, and `python tools/reference.py --check` regenerated `docs/novis.md`
with 286 of 286 examples holding.

**What a tool feature owes is one test, one example and one attack** (`POLICY["tool"]`), and each
sits where its tree puts it: `covers:` markers on
`crates/nvs-lsp/tests/handshake.rs:97` and `crates/nvs-lsp/src/suite.rs:623`, one example each
under `docs/examples/tools/editor/`, and three attacks under `tests/hostile/tools/editor/` — the
document at the parser's legal depth, the same one past it under `expect-refusal`, and a document
that smuggles the suite's own `--SECTION--` headers into its strings and comments.

**Stage 12 is the last red check and its test does not exist under that name.** The check names
`the_workspace_has_no_async_runtime`; what is on disk is
`crates/nvs-runtime/tests/manifest_policy.rs:171`'s
`tokio_appears_in_neither_the_manifest_nor_the_lockfile`, which the stage-1 check at
`docs/agent/loop-goal.toml:3461` already names — so renaming the landed test turns stage 1 red and
the repair is a second, wider test rather than a rename.

## Next group

**Stage 12: the standing guard** — one file set: `crates/nvs-runtime/tests/manifest_policy.rs`,
with `docs/plan/m4b.md` for the claim it pins.

- [ ] **The workspace-wide async-runtime guard exists under the name the check uses** — write
      `the_workspace_has_no_async_runtime` beside the tokio one at
      `crates/nvs-runtime/tests/manifest_policy.rs:171`, asserting the family rather than the one
      crate: no manifest of ours names an async runtime and none is a dependency of ours in the
      lock file. `manifest_code()` at `crates/nvs-runtime/tests/manifest_policy.rs:84` and
      `locked_packages()` at `crates/nvs-runtime/tests/manifest_policy.rs:135` are the two walks it
      reads; `rule:concurrency/one-scheduler` is what it pins, and the goal's § *Standing
      decisions* pre-authorizes the claim.
- [ ] **M4B's *Verify* stops asserting something false** — `docs/plan/m4b.md:159` still ends "`tokio`
      appears in neither `Cargo.toml` nor `Cargo.lock`" and `docs/plan/m4b.md:14` calls that "still
      exactly true", while `hyper` has put `tokio` in the lock file since goal 6. Amend both to the
      property actually checked — no crate of ours depends on a runtime, the only route is
      `hyper`'s, and only `sync` compiles — with `python tools/plan.py --amend M4B`. That is the
      `[until:]` condition of the playbook bullet at `docs/agent/playbook.md:1206`, so the wrap
      drops it the day this lands.

## Backlog

- The other three tool chapters' features owe every proof — `python tools/dossier.py --group
  tools:cli` is 14 features at zero, and no goal gates on them yet.
- The latency ceilings come from one machine; a second platform's figure is what a tightening pass
  would need — `crates/nvs-lsp/tests/latency.rs`'s module doc.
- `Foo::class` colours neither half; the `class` keyword is the TextMate layer's —
  `crates/nvs-lsp/src/semantic.rs`'s module doc.
- The matrix is a ratchet now: a case at a construct nobody reached obliges six more rows —
  `crates/nvs-lsp/src/coverage.rs:46`.
