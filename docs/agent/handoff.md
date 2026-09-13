# Handoff

## State

**Goal `gap-register` — stages 1–3 green, stage 4 open.** `python tools/owners.py` counts every
module-doc heading that records owed work outside a `# Known gaps` block as `sections outside Known
gaps: N` (`tools/owners.py:164` is the pattern, `:701` the line). The stage 4 check wants `0`
(`docs/agent/loop-goal.toml:10111`); it stands at **13**, down from 18.

- `crates/nvs-db/src/` is clear. Its five sections are gone: `tds/mod.rs` and `span.rs` each grew a
  `# Known gaps` block they had none of, `lib.rs` and `ddl.rs` kept their prose and lost a heading
  that only claimed to owe something, and the two headings that were stale said so — `lib.rs` still
  read "what is left there is the statement" for TDS, whose statement path is whole
  (`crates/nvs-db/src/tds/mod.rs:74-84`), and "SQLite is then what is left" for a driver that exists.
- **Two new `unowned` gaps, both reasoned.** `crates/nvs-db/src/tds/mod.rs:88` (a `bytes` bind is
  refused; the fix is ADR 0067 § 1's cache-key shape) and `crates/nvs-db/src/span.rs:64` (a span
  renders on `DebugFlags::TRACE` and nothing outside a test sets that flag from the `debug.trace`
  grant). Neither is goal `m8-db-queue`'s: its stage 10 gate is "no `— owner: m8-db-queue` tag left"
  over an enumerated list, so tagging it would make that goal uncloseable. Both reasons are in
  `docs/agent/carried-gaps.md` § *Unowned* with `[until:]` trailers.
- The gate is green: `--check --untagged-is-an-error --reasons` passes with `untagged: 0`,
  `broken-tag: 0`, `unreasoned: 0`.

Nothing is blocked. `[context] modules` still prints no crate module doc, which is the file kind
this stage edits; the driver's sweep picks up what these commits touched.

## Next group

**Stage 4 continued: `nvs-server`'s three sections** — one file set: `crates/nvs-server/src/`, whose
gaps goal `m7-server-surface` already owns three of (`schedule.rs` gaps 1–3). The rule is the goal's
stage 4 and `tools/owners.py`'s module doc § *A heading is not a register*.

- [ ] **`crates/nvs-server/src/route.rs:21` — `# What the answer is read for, and what is still
      missing` splits.** This file already has a `# Known gaps` block (`crates/nvs-server/src/route.rs:30`
      gap 1, `:44` gap 1), so the owed half moves into it as a numbered item and the heading keeps
      only what the answer is read for.
- [ ] **`crates/nvs-server/src/metrics.rs:50` — `# What is not here yet` becomes a `# Known gaps`
      block.** No block exists in this file today; each item needs an owner backed by a goal file or
      a plan, `unowned` with a `docs/agent/carried-gaps.md` § *Unowned* reason otherwise.
- [ ] **`crates/nvs-server/src/serve.rs:32` — `# What this module does not decide yet`.** Check
      first whether it is stale rather than owed: goal `serve-runs-the-queue` landed after it was
      written, and a heading that only claims to owe something is rewritten, not moved.

## Backlog

- `crates/nvs-stdlib/src/` holds four of the remaining sections — `cache.rs:137`, `http.rs:102`,
  `sse.rs:88`, and `test.rs` is already done; one file set, one group.
- `crates/nvs-runtime/src/budget.rs:55` and `dispatch.rs:21`; `budget.rs` gap 1 is already orphaned
  per `docs/agent/carried-gaps.md`.
- `crates/nvs-cli/src/openapi.rs:27` and `script.rs:42`; `script.rs`'s is a `Decision:` heading, so
  it may be the false-positive kind.
- `crates/nvs-host/src/ladder.rs:16`, `crates/nvs-lsp/src/regions.rs:58`,
  `crates/nvs-test/src/lib.rs:148` — three singletons, one session between them.
