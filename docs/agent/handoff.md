# Handoff

## State

**Stage 7 — ADR 0048's bundler — is closed at both of its acceptance checks**, and with it every
stage the goal names: `python tools/try.py --bundle examples/hello.nvs --expect "Hello, World!"`
passes, and `a_bundle_carries_the_statically_resolved_require_graph_as_source` and
`a_bundled_executable_runs_identically_to_nvs_run` are in `crates/nvs-cli/tests/bundle.rs`. The
status is `CONTINUE` rather than `DONE` only because the driver, not a session, decides that: no
open stage remains, so the next iteration's acceptance run is the verdict.

`nvs build --compile <entry> [-o <path>]` joins `--openapi` under a required clap `ArgGroup`, so
`nvs build` with neither still refuses. `crates/nvs-cli/src/bundle.rs`'s module doc owns the
payload layout, the rebundle-from-a-bundle truncation, and the three known gaps (no `.nvsx`
entries, macOS signing warns rather than fails, a bundle still reads `./nvs.toml`).

**The read-back side is one new byte source, not a second interpreter.**
`nvs_diagnostics::embedded` is a write-once process-global installed before clap parses anything;
`SourceMap::load` and `nvs-hir`'s new `requires::canonicalize` answer out of it first, and every
later phase is unchanged. That is ADR 0048 § 4's "the same `nvs run` code path with one different
byte source", and its module doc owns why a global rather than a field. **`autoload` is the known
gap**: ADR 0061's probing lists real directories and is not routed through the table, so a bundled
program reaching a name only through an autoload root does not resolve it.

## Next group

**Closing ADR 0048's two remaining gaps.** One file set: `crates/nvs-hir/src/autoload.rs` with
`crates/nvs-hir/src/requires.rs` and `crates/nvs-cli/tests/bundle.rs`. Before starting, add ADR
0048 §§ 2-5 to `[context] adrs` at `docs/agent/loop-goal.toml:85` — this session had to open the
ADR by hand, which is a call and ~7k the manifest should have saved.

- [ ] **Route ADR 0061's autoload probing through `nvs_diagnostics::embedded`** so a bundled
      program resolves a name reachable only through an autoload root — ADR 0048 § 3's
      closed-world rule reaches `autoload` for the same reason it reaches `require`, and
      `bundle.rs`'s *Known gaps* records that it does not yet. The two `read_dir` sites are the
      work; `requires::canonicalize` is the shape to copy. Anchors:
      `crates/nvs-hir/src/autoload.rs:402`, `crates/nvs-hir/src/autoload.rs:519`,
      `crates/nvs-hir/src/requires.rs:447`.
- [ ] **A `.nvst`-free regression for a bundle whose `require` escapes the entry's directory** —
      `bundle::common_root` picks the deepest ancestor every file sits under, and a
      `require "../lib/x.nvs"` is the case that distinguishes it from "the entry's parent"; the
      fixture set is beside the existing one. Anchor: `crates/nvs-cli/tests/bundle.rs:29`.

## Backlog

- ADR 0048 § 6: a `.nvsx` in the payload — blocked, Tier 1 extensions do not load yet
  (`docs/adr/0003-extension-system.md`).
- M6 *Verify*'s "on all three platforms" for the bundle: only the Windows leg has run
  (`docs/plan/m6.md`).
- ADR 0078's snapshot-swap rows need `nvs ctl reload`, which is goal 6
  (`docs/adr/0078-config-reload-and-control-socket.md` § 3).
- `orient.py`'s `[context] modules` pattern `crates/nvs-host/src/budget.rs` matches no module —
  it moved or the glob is wrong (`docs/agent/loop-goal.toml`).
