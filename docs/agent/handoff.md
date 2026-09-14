# Handoff

## State

**Goal `m4b-editor` is met: stages 0 through 8 are all on disk and every one of its checks passes
locally.** This session closed stages 6, 7 and 8.

- **Stage 6.** `tools/ci-changes.py:56` holds the `editor` lane over `editors/`, the crates and both
  manifests, and `.github/workflows/ci.yml` emits it from `changes` and gates two jobs on it:
  `extension` (the same three platforms as `test` — build `nvs-cli`, `npm ci` against
  `editors/vscode/.nvmrc`, lint, `test:headless` with `NVS_BIN` on that build, `npm run package`, and
  `nvs.vsix` uploaded from the Linux leg alone) and `extension-host` (Linux, `xvfb-run -a npm run
  test:host`, caching the downloaded editor build alone). CI is not running, so the files are the proof;
  goal `gap-zero`'s green run is what proves the jobs themselves.
- **Stage 7.** WSL already had a nightly toolchain and `cargo-fuzz`, so no install was needed. The
  300 s `prefix` run was made here: `Done 246000 runs in 301 second(s)`, no panic, so nothing under
  `crates/nvs-syntax/` changed. `docs/agent/commands.md` § *Fuzzing and callgrind on Windows* now names
  the target and the command the check's `argv` copies.
- **Stage 8.** The three rules are `shipped` with `guardedBy` filled in `docs/rules/ide.json`, rendered.

`python tools/verify.py` is 11 of 11 (the extension's headless tier at 102 passing),
`python tools/verify.py --doc` is green, and `python tools/reference.py --check` says `docs/novis.md` is
current after the status flips.

The pack never prints the goal's own `## Stage N` prose, which is where each stage's exact markers are,
and `[context]` has no field that reaches it — not `rules`, `adrs`, `modules`, `shapes`, `playbook` or
`milestones`. One `peek.py` target on `docs/agent/loop-goal.md:"## Stage N"` is the whole fix, but it has
to be remembered.

## Next group

**Nothing of this goal is open** — the status line is `DONE` and the next session is the chain's next
goal, which `python tools/brief.py` names after `tools/goal-switch.py` runs. If the driver's own sweep
declines the claim, the work is whichever check it names, and only these two could be it:

- [ ] **A red stage 3–5 host check is a regression, never new work**
      (`docs/agent/loop-goal.toml:9915` is the isolation check; one `npm run test:host -- --nvs
      target/debug/nvs.exe` from `editors/vscode` reproduces all twelve).
- [ ] **A red stage 8 check means the render is stale**: `python tools/rules.py --render` after any edit
      to `docs/rules/ide.json:690`, because `docs/rules/ide.md` and `docs/ground-rules.md` are generated.

## Backlog

- A test of `tools/loop.py:1706`'s `EDITOR_READS`: `tools/` has no suite to host one
  (`docs/agent/loop-goal.md` § *The acceptance cache sees an extension change*).
- Publishing the `.vsix` to the Marketplace or Open VSX, or attaching it to a release —
  `rule:ide/one-server-two-thin-clients` § *Revisiting* keeps it open.
- A pixel tier for the colour assertions; PhpStorm and every M10 editor feature — not this goal.
- The stale trivia paragraph in `docs/plan/m4b.md`, which is goal `plan-truth`'s.
- The `unowned` module-doc gaps in `crates/nvs-lsp/src/index.rs` and `hints.rs`, goal
  `unowned-closures`'s.
