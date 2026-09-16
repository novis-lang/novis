# Handoff

## State

**Goal `unowned-closures`, stage 2 closed.** Gap 6 is struck: every raise now names the frame it
happened in. The helper half took build (b) of the two the last handoff priced — a seed on the
caught edge, not an operand on the helper ABI — because the ABI operand materializes a blob on the
path where the call *succeeds*, which is the one thing `rule:errors/throw-is-not-slower` prices.

`InstKind::SeedRaiseSite` (`crates/nvs-ir/src/ir.rs:1226`) carries the failing statement's site in
`Inst::raise_site` and is emitted by `Lowering::landing_block`
(`crates/nvs-ir/src/lower/mod.rs:2291`) into the `caught` block alone — the catchable exit of
`Terminator::Catch`, the one exception edge that pushes no frame label. `nvs-codegen` bakes the blob
there (`crates/nvs-codegen/src/emit.rs:1021`) and calls `nvs_raise_site`, whose body is
`Ctx::seed_raise_site` (`crates/nvs-runtime/src/ctx/error.rs:412`): it promotes a pending message
the way `push_frame` does and renders the frame only where `Thrown::has_frame` is false, so a
`throw` caught beside itself and an exception arriving from a callee both pass through untouched.
The frame is provisional, so a clause matching nothing still has its label replaced rather than
doubled.

**The bound, written rather than built**: a helper's fault that *propagates* out of the frame it was
raised in opens its trace at that frame's label as before and its `location` stays empty — a pushed
label is a rendering, not the datum a `location` is read back from. Its home is
`crates/nvs-runtime/src/throwable.rs`'s module header, and `rule:errors/throw-is-not-slower` states
it too (fragment amended, rendered, mirrored to `website/src`).

Nothing is blocked. `python tools/rules.py --check` is still red tree-wide on
`docs/agent/goals/61-class-scoped-types.toml:237`, which passes a `types/class-scoped-alias`
citation to `peek.py` as its own check argument while the rule that check exists to create does not
exist yet; `session.py --wrap` refuses every wrap while that goal is queued, so this session's
commits were made by hand, as the last two were. Stage 1's floor is goal `m8-stdlib-depth`'s whole
list, carried and untouched.

## Next group

**Stage 3: the builds that needed no decision — `require`'s cooker and the autoload probe trace** —
one file set: `crates/nvs-hir/src/requires.rs`, `crates/nvs-types/src/string_lit.rs`,
`crates/nvs-hir/src/autoload.rs` and `crates/nvs-config/src/cache.rs`.

- [ ] **A `require` path decodes every escape its string does** —
      `crates/nvs-hir/src/requires.rs:1271`'s `cook_quoted` recognises a practical subset and leaves
      octal, hex and unicode escapes un-cooked, so a `require` and an ordinary string literal
      disagree about the same bytes. The gap at `crates/nvs-hir/src/requires.rs:86` defers this to
      "once something besides this module needs it" — and that cooker now exists at
      `crates/nvs-types/src/string_lit.rs:601`, so the code is ahead of the deferral and the build
      is routing `cook_quoted` through it. The check names the test:
      `a_require_path_decodes_every_escape_its_string_does`, under `-p nvs-hir`.
- [ ] **A file created where autoload probed invalidates the unit** —
      `rule:packaging/autoload-probes-fold-into-the-cache-key`'s trace is produced and dropped
      (`crates/nvs-hir/src/requires.rs:101`, the trace being
      `crates/nvs-hir/src/autoload.rs:146`'s `Probe`), so a class that failed to autoload stays
      failed after the file that would have satisfied it appears. The key it folds into is
      `crates/nvs-config/src/cache.rs:140`'s `env_hash`. Test:
      `a_file_created_where_autoload_probed_invalidates_the_unit`.

## Backlog

- `crates/nvs-hir/src/requires.rs:77`'s first gap — folding literal concatenations and `const`s
  before the graph walk — is `Decided:` and owned by this goal, and no stage-3 check names it yet.
- A propagating helper fault still names no `location`; closing it is the helper-ABI operand, priced
  in `crates/nvs-runtime/src/throwable.rs`'s module header.
- `previous` cannot be set at all yet — owned by `nvs_types::error_lib`, which is why the
  synthesized constructor takes only a message.
- `docs/agent/goals/61-class-scoped-types.toml:237` keeps `rules.py --check` red tree-wide, so every
  wrap has to be applied by hand until that goal is reached.
