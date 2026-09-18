# Handoff

## State

Goal `config-directives-2-3`, milestone dossier. Eleven of its sixteen items are complete and five
are untouched. Nothing is blocked.

This session landed the four `System` keys carved out of `[limits]` — `directive:limits.fatal_reserve_memory`,
`directive:limits.fatal_reserve_time`, `directive:limits.max_decompressed` and
`directive:limits.max_decompression_ratio` — each with its `about.md`, one example with a blessed
`.out`, one attack, and one Rust case in `crates/nvs-config/tests/directives.rs`. All four keys are
now written in the repository-root `nvs.toml` at exactly the figures the code ships, so nothing in
the tree behaves differently and the four pages have real values to print.

Two proofs found bugs, and both are fixed rather than recorded. The decompression attack's
`array<uint>` of asks panicked the lowerer: an element of an array literal reaches
`lower_int_literal` with no expected type, so digits past `int`'s range took the `int` branch —
`crates/nvs-ir/src/lower/expr.rs:2455` now reads the magnitude too, and
`tests/conformance/lang/an-integer-literal-past-int-lowers-as-uint-wherever-it-is-written.nvst` pins
it. And `crates/nvs-config/src/default.toml` documented these keys at figures the code does not
ship, in both `[limits]` and `[app.limits]`; they are the shipped ones now.

## Next group

**Stage 2: the dossier — one file set: `crates/nvs-config/src/directive.rs` and
`crates/nvs-config/tests/directives.rs`, plus each feature's own three proof paths.** What is left of
`[limits]` and then the `[log]` block, which is the same registry shape one block over.
`python tools/dossier.py --id '<feature>'` prints the three paths; check an item with it before
taking it, since this goal's list was emitted before the previous goal's last sessions committed.

- [ ] **`directive:limits.max_script_depth`** — owes examples, hostile, tests. The recursion ceiling,
      `System` for `rule:security/isolate-budget-is-the-trees`' reason: a script able to raise its own
      would exhaust the tree's heap before any depth stopped it.
      `crates/nvs-config/src/directive.rs:105`
- [ ] **`directive:log`** — the block's blanket row, and the one of these that is `Runtime`: a
      program choosing what it logs is the ordinary case. The contrast with the three `System` keys
      carved out of it is the claim. `crates/nvs-config/src/directive.rs:157`
- [ ] **`directive:log.handler`** — the first carve-out: who the records go to is the deployment's.
      `crates/nvs-config/src/directive.rs:164`

The group after that is the pair at `crates/nvs-config/src/directive.rs:170` and `:171` —
`log.handler_reserve_memory` and `log.handler_reserve_time`, which are `rule:errors/on-limit`'s
reserve one subsystem over and read the same way this session's pair did.

## Backlog

- `crates/nvs-runtime/src/ctx/mod.rs:556` says the CPU-limit timer does not exist yet and the flag is
  raised by tests alone. It does exist: a `nvs run` under a file-set `[limits] cpu_time` was stopped
  cleanly this session, handler first. The sentence is stale — `nvs_host::watchdog` is the timer.
- A `[limits] cpu_time` lowered by `Core\Config::set` mid-request is never sampled against: the
  watchdog carries the ceiling published at request start. The direction is safe — nothing published
  can be raised — so it is a correctness gap in a request tightening itself, not an escape.
- `directive:limits.max_script_depth` and the three `[log]` keys are what is left of this goal.
