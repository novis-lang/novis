# Handoff

## State

**Goal `gap-register` — stages 1–3 green, stage 4 open.** `python tools/owners.py`'s
`sections outside Known gaps` line stands at **10**, down from 13; the stage 4 check wants `0`
(`docs/agent/loop-goal.toml:10111`).

- `crates/nvs-server/src/` is clear, and the three sections went three different ways. `route.rs`
  kept its prose under `# What the answer is read for` and its two gaps became numbered items under
  `# Known gaps` (`crates/nvs-server/src/route.rs:30`). `metrics.rs` split into `# What it spends`
  and a new `# Known gaps` block (`crates/nvs-server/src/metrics.rs:58`). `serve.rs` owed nothing —
  every bullet under it names the module the answer belongs to — so only the title changed
  (`crates/nvs-server/src/serve.rs:32`).
- **Two owners came out of the goal files rather than out of `unowned`.** `metrics.rs` gap 1 is
  `m7-server-surface`'s (its gap list names this section, stages 10 and 11) and gap 2 is
  `m8-stdlib-depth`'s (stage 12, `Core\Metrics`'s three rows). `route.rs` gap 2 stays `unowned` even
  though the same goal names it: what has to be decided there is which crate owns the registry, and
  the goal's § *Standing decisions* leaves that kind to goal `unowned-closures`.
- The gate is green: `--check --untagged-is-an-error --reasons` passes with `untagged: 0`,
  `broken-tag: 0`, `unreasoned: 0`.

Nothing is blocked. `[context] modules` still prints no crate module doc, which is the file kind
this stage edits; the driver's sweep picks up what these commits touched.

## Next group

**Stage 4 continued: `nvs-stdlib`'s three sections** — one file set: `crates/nvs-stdlib/src/`. None
of the three has a `# Known gaps` block yet, so each item writes the block as well as the items, the
way `nvs-db`'s `span.rs` did. The rule is the goal's stage 4 and `tools/owners.py`'s module doc
§ *A heading is not a register*.

- [ ] **`crates/nvs-stdlib/src/sse.rs:88` — `# What is not here yet` becomes a `# Known gaps`
      block.** Goal `event-streams` cites the section by its title at
      `docs/agent/goals/42-event-streams.md:20`, so that line is part of the slice.
- [ ] **`crates/nvs-stdlib/src/http.rs:102` — `# What is not here yet, and why each is deliberate
      rather than forgotten` splits.** The "deliberate" half is scope and keeps a heading saying so;
      what is owed moves into `# Known gaps`. `docs/agent/goals/48-http-client.md:48` cites the
      title and already names goal `http-client` as the owner of the request body and reply headers.
- [ ] **`crates/nvs-stdlib/src/cache.rs:137` — same shape.**
      `docs/agent/goals/49-process-cache.md:38` cites the title and names goal `process-cache`
      stage 4 as the owner of "A TTL and a `forget`".

## Backlog

- One section each, in seven more crates, after `nvs-stdlib`: `crates/nvs-cli/src/openapi.rs:27`,
  `crates/nvs-cli/src/script.rs:42`, `crates/nvs-host/src/ladder.rs:16`,
  `crates/nvs-lsp/src/regions.rs:58`, `crates/nvs-runtime/src/budget.rs:55`,
  `crates/nvs-runtime/src/dispatch.rs:21`, `crates/nvs-test/src/lib.rs:148`. `python
  tools/owners.py`'s tail is the live list; this one is a snapshot of it.
- A section whose bullets are all "decided elsewhere" needs no gap block, only an honest title —
  `serve.rs` was one and `dispatch.rs`'s *What a caller owes* may be another.
- `docs/agent/carried-gaps.md` § *Unowned* holds `route.rs`'s two gaps as one bullet; goal
  `unowned-closures` is where that decision lands.
