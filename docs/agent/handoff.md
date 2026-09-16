# Handoff

## State

**Goal `unowned-closures`, stage 6 — the register.** `python tools/owners.py` reports `unowned: 20`,
`goal-owned: 64`, `milestone-owned: 15`, `past-milestone: 8`, `untagged: 0`, `broken-tag: 0`,
`unreasoned: 0` and `sections outside Known gaps: 0`, over 107 items in 65 blocks. The stage wants
`unowned` at 0, and § *UNOWNED* is the roster of the 20.

**`nvs-ir`'s gap 21 is fixed and struck.** `as ?T` now finds its `?` wherever the annotation writes
it — on the whole of it, on a union member (`?"a"|"b"`, which parses as `Union[Nullable("a"), "b"]`
because `?` binds the atom), or spelled out as `"a"|"b"|null` — and all three run
`rule:expressions/nullable-conversion`'s membership chain. The crate's `# Known gaps` block now holds
14 (M10-bound, see below) and 18; 7 stays M12.

**The goal's stages do not cover the roster, and that is what stands between this stage and
`unowned: 0`.** Of the 20 unowned items, the goal file names six as work: `crates/nvs-hir/src/requires.rs`
gap 2 (stage 3), `crates/nvs-runtime/src/graph.rs` gaps 1–2 (stage 2), `crates/nvs-stdlib/src/regex.rs`
gap 3 and `crates/nvs-stdlib/src/task.rs` gap 1 (stage 4), `crates/nvs-lsp/src/hints.rs` gap 1
(stage 6). The other 14 — `crates/nvs-cli/src/openapi.rs` gaps 1–5, `crates/nvs-cli/src/runner.rs`
gap 1, `crates/nvs-db/src/span.rs` gap 1, `crates/nvs-db/src/tds/mod.rs` gap 1, the three
`crates/nvs-host/src/` gaps, `crates/nvs-runtime/src/ctx/mod.rs` gap 1,
`crates/nvs-stdlib/src/cache.rs` gap 1, `crates/nvs-stdlib/src/response.rs` gap 2 — are named nowhere
in `docs/agent/goals/60-unowned-closures.md` and carry no `Decided:` sentence (`grep -c Decided:` is
0 in every one of those files). By the goal's own stage 0 each is a `BLOCKED` when a session reaches
it with no obvious build; their reasons are already written in `docs/agent/carried-gaps.md`
§ *Unowned*. Nothing is blocked while the six above are still open.

## Next group

**Stage 6: the honest deferrals** — one file set: `crates/nvs-lsp/src/hints.rs`,
`crates/nvs-ir/src/lib.rs`, `crates/nvs-runtime/src/lib.rs` and `docs/plan/m10.md`.

- [ ] **Retag `crates/nvs-lsp/src/hints.rs:64`'s gap 1 to `M10`** — the block heading is
      `crates/nvs-lsp/src/hints.rs:62`, and `docs/plan/m10.md:19` is where the scope sentence goes if
      it does not already cover a call the checker recorded as anything but `ExprInfo::Call`.
      `docs/agent/goals/60-unowned-closures.md:115` is the stage that schedules it; it is one of the
      20 and takes the roster to 19.
- [ ] **Split the `DEBUG_BREAK` half from the collector half** — `crates/nvs-ir/src/lib.rs:614`
      gap 14 and `crates/nvs-runtime/src/lib.rs:209` gap 5 both cover two safepoint flags at once.
      `DEBUG_BREAK` waits on `nvs dap` and is M10's; `COLLECT` waits on the in-flight collector, which
      `docs/agent/carried-gaps.md:140` holds as an open decision rather than an unclosed gap. Retag
      only what M10 actually buys, and leave the collector where its decision lives.
- [ ] **Prove it with `python tools/owners.py --deferrals`** — the stage's second check, green today
      and the thing a retag can turn red. `crates/nvs-lsp/src/index.rs` carries no `# Known gaps`
      block at all and `crates/nvs-lsp/src/completion.rs:191` is a bold `**Known gaps.**` run already
      tagged `M10`, which `owners.py` does not count; the stage text names both, so neither is work.

## Backlog

- The 14 unowned items no stage names — a sheet the user has not answered; `docs/agent/carried-gaps.md` § *Unowned*.
- `crates/nvs-hir/src/requires.rs` gap 2, the one unowned item with a build stage 3 already specifies.
- `crates/nvs-runtime/src/graph.rs` gaps 1–2, listed under stage 2's **Decided** with no `Decided:` sentence in the file.
- `crates/nvs-stdlib/src/regex.rs` gap 3 and `crates/nvs-stdlib/src/task.rs` gap 1, stage 4's two builds.
- The 8 `past-milestone` deferrals, which `owners.py` says are owed by a goal or nobody.
