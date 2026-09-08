# Handoff

## State

**Goal 14, stage 10 is closed: a full re-analysis of a ~1,000-line document is measured against a
named bound.** `crates/nvs-lsp/tests/latency.rs` generates a 997-line document of ordinary
declarations, opens it as a buffer with no file under it, and takes the fastest of five
`analyse_current` calls. That file's module doc is the bound's home — the goal's § *Standing
decisions* sends a decision this size to the crate's own doc rather than to an ADR — and it holds
both numbers, what they were measured on, and the item-level caching that is the answer if it ever
goes red. `crates/nvs-lsp/src/document.rs`'s `analyse` points at it.

**The ceiling is split by build profile, and that is the one decision here.** 25 ms release, 200 ms
debug, each roughly 10x the 2.7 ms and 22.3 ms measured where it landed. A release-only ceiling
would have to be `#[ignore]`d in a debug build the way `perf_guards.rs` does it, and every gate in
this repository builds `debug` — see the new playbook bullet.

**Stage 9 stays closed**: `nvs lsp-test tests/lsp/` is 189 passed, matrix full.

**Stage 11 is the next red check, and only half of it is red.** `python tools/reference.py --check`
passes as it stands (`docs/novis.md` current, 286 of 286 examples hold); the dossier gate refuses
because neither editor feature is on the roster at all.

## Next group

**Stage 11: the reference chapter** — one file set: a new chapter under `docs/reference/tools/`,
with `tools/dossier.py` read for the derivation only.

- [ ] **The two editor headings exist, so the roster derives them** — the check is
      `docs/agent/loop-goal.toml:5340`, and it fails today with `--only names 2 feature(s) that are
      not on the roster`. `tools/dossier.py:484` is what turns a heading into a feature id: the
      chapter's frontmatter `id:` plus the slug of each `# ` title, so `tools:editor/nvs-lsp` and
      `tools:editor/nvs-lsp-test` need `# nvs lsp` and `# nvs lsp-test` headings in a chapter whose
      `id` is `editor`. `docs/reference/tools/10-cli.md:1` is the shape to copy; the three chapters
      there are numbered, so this one is `40-editor.md`.
- [ ] **The two features owe no proof** — `tools/dossier.py:508` builds the roster and `--gate`
      judges it against the four proofs `tools/dossier.py:2` names (tests, examples, perf, hostile).
      Decide per proof what a *tool* feature can even have, and excuse the rest in
      `tools/data/dossier-policy.toml` rather than by weakening the gate. The refusal asks for
      `--emit-goals`; that appends to `docs/agent/goals/chain.toml`, which is the loop's own
      schedule, so read `tools/dossier.py:1690` before running it and prefer `--dry-run` first.

## Backlog

- Stage 12, `the_workspace_has_no_async_runtime`, a standing guard rather than this goal's work —
  `docs/agent/loop-goal.toml:5352`.
- The latency ceilings come from one machine; a second platform's figure is what a tightening pass
  would need — `crates/nvs-lsp/tests/latency.rs`'s module doc.
- `Foo::class` colours neither half; the `class` keyword is the TextMate layer's —
  `crates/nvs-lsp/src/semantic.rs`'s module doc.
- The matrix is a ratchet now: a case at a construct nobody reached obliges six more rows —
  `crates/nvs-lsp/src/coverage.rs:46`.
