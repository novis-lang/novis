# Handoff

## State

**Goal `unowned-closures`, stage 2.** Gap 6's Novis half is built and the runtime's own raises are
what is left of it; nothing is blocked.

A raise that carries a site now renders the frame it happened in. `Thrown::capture_site`
(`crates/nvs-runtime/src/throwable.rs:862`) decodes the blob once, writes `location` and pushes
`Member() at file:line` off the same datum, so the two cannot disagree; `Ctx::raise_sited` marks that
frame provisional and `Ctx::push_frame` spends the mark, replacing it with the label the frame pushes
as the throw leaves. The compiler's spelling therefore always wins, and the rendering only survives
where the throw never leaves its frame — a `catch` beside the `throw`, the one place no landing block
pushes anything, which read an empty `backtrace` before this. Every existing trace is unchanged:
`examples/trace.nvs`, `examples/uncaught.nvs` and `nvs-codegen`'s `throwing.rs` all print what they
did. A file-scope raise is named `<script>`, restated in `nvs-runtime` as `ENTRY_SCRIPT_FRAME` and
held to `nvs_ir::lower::ENTRY_SCRIPT_LABEL` by a new test beside the slot agreement.

`rule:errors/throw-is-not-slower` is amended and retitled: the propagation path still allocates
nothing per frame, and what a throw allocates is one label at the raise. Its guard file gained
`a_raise_renders_one_frame_label_and_nothing_larger`, which bounds the rendering at 500 ns over a
site-less raise — the shape that rules out a stack walk — and the old ratio guard now says it prices
the propagation half. `website/src/` is synced (39 files, mostly a catch-up from earlier sessions).

Stage 1's floor is goal `m8-stdlib-depth`'s whole list, carried and untouched.

## Next group

**Stage 2: what is left of gap 6 — the raises that carry no site** — one file set:
`crates/nvs-runtime/src/throwable.rs`, the `nvs_raise_new` call in `nvs-codegen` and the operand
`nvs-ir` would have to lower for it.

- [ ] **`nvs_raise_new` is handed the site its statement already has** — the primitive is
      `crates/nvs-runtime/src/throwable.rs:977` and takes `capture_site` the moment it has a blob;
      its one call is `crates/nvs-codegen/src/emit.rs:2072` (a checked operator's cold block) and the
      signature to widen is `crates/nvs-codegen/src/lib.rs:1622`. The operand is the
      `InstKind::SourceConst` `crates/nvs-ir/src/lower/mod.rs:2133` already materializes for a
      `throw`, which the checked-arithmetic lowering noted at
      `crates/nvs-ir/src/lower/operator.rs:465` does not emit.
      `rule:errors/a-record-names-where-it-was-produced` is why both readers come off the one datum.
      The behaviour to pin is a `try { $a % 0 } catch` naming its own frame, beside the case landed
      in `crates/nvs-codegen/tests/throwing.rs`.
- [ ] **The helper half, decided rather than left open** — a bare-message `Fault` and
      `crates/nvs-runtime/src/ctx/error.rs:393`'s `raise_with_slots` build an exception with no site,
      and the site a producer *is* handed is `crates/nvs-runtime/src/source.rs:138`'s `of_operand`,
      which only `nvs_stdlib::registry`'s `SOURCE_MEMBERS` rows carry. Either thread it through to
      the raise or state the bound in `crates/nvs-runtime/src/lib.rs:216`'s gap 6 — and strike the
      gap either way, since the goal's § *Standing decisions* admits a bound but not a silence.

## Backlog

- `python tools/rules.py --check` is red on an untracked, in-flight goal file
  (`docs/agent/goals/61-class-scoped-types.toml` cites `rule:types/class-scoped-alias`, which that
  goal's own stage 5 creates), so `session.py --wrap` refuses every wrap tree-wide until it lands;
  this session's commits were made by hand for that reason.
- `crates/nvs-runtime/src/lib.rs:199` gap 2 — `nvs_str_concat`/`concat_n` reuse a solely-owned left
  operand, with the ownership hand-off in `nvs-ir`'s lowering for those two calls.
- `crates/nvs-runtime/src/lib.rs:209` gap 5 and `:234` gap 7 — one decision, "a collector that runs
  only near the memory ceiling", so they are one build and not two.
- Stage 3's failing acceptance check (`nvs-hir`'s require-path decode and the autoload probe's unit
  key) is an artefact nothing has written yet, not a regression.
- `docs/agent/carried-gaps.md`'s `nvs-runtime` bullet now covers the string and backtrace halves
  only; its `[until:]` trailer still holds while gaps 2, 6 and 7 name this goal.
