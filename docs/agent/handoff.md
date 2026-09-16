# Handoff

## State

**Goal `unowned-closures`, stage 6 (the register).** `python tools/owners.py` reads 80 items with
`unowned: 6`, `untagged: 0`, `broken-tag: 0`, `unreasoned: 0`, and `--deferrals` green. The six left are
in `nvs-db`, `nvs-runtime` and `nvs-stdlib`, two each.

**`nvs-host`'s whole share of the register is settled, and it took a chain entry rather than a
deferral.** Both placement gaps are one seam and neither waits on a milestone: the table behind a
resolver is already shareable — `nvs_cli::script::Compiler` keeps its two maps behind `RwLock`s and a
serving fleet hands every core the same `Arc<Compiler>` (`crates/nvs-cli/src/serve.rs:703`) — so what is
missing is a seam a core that has not started yet can install. Goal `worker-placement` is now the entry
after `class-scoped-types` and before `gap-zero`, with its prose, manifest, acceptance list and seed
handoff on disk; `placed.rs`, `group.rs` and `worker.rs` tag to it, and their two
`docs/agent/carried-gaps.md` § *Unowned* bullets are rows in that file's § *Owned* table.

**A floor check named the CSRF binding under the wrong crate** — the token's format is
`crates/nvs-runtime/src/csrf.rs`, under both its readers, because `Core\Csrf` and the server door are in
crates that cannot see each other. It is its own `-p nvs-runtime` block now, over the name the tree
holds, and `docs/agent/loop-goal.toml` and `docs/agent/goals/60-unowned-closures.toml` are byte-identical
again.

## Next group

**Stage 6: the register** — one file set: `crates/nvs-runtime/src/`. Each item is a scheduling decision
written into the gap's own `— owner:` line: a goal slug on the chain, or an M9+ deferral whose plan
states the scope (`python tools/owners.py --deferrals` is the gate on the second kind).

- [ ] **A stack ceiling is asserted, not discovered** — `crates/nvs-runtime/src/ctx/mod.rs:65` gap 1.
      `rule:concurrency/a-tasks-recursion-limit-comes-from-its-own-stack` says the limit is armed from
      the stack's **real bounds**, so the code is behind a rule rather than ahead of one; what it needs
      is a platform call this crate has no dependency for, and the gap's own prose names M6, which is
      complete. The decision is which entry owns reading a thread's bounds, not whether to.
- [ ] **A decoded `Core` instance is a `mixed` a program cannot narrow** —
      `crates/nvs-runtime/src/graph.rs:74` gap 1. Both refusals are `nvs-types`' (`E0496` because
      `instanceof` finds no descriptor, `E0711` because `rule:types/conversion` tabulates no conversion
      into one), and the address `instanceof` would test against is the one
      `nvs_stdlib::class_descriptors` already hands the backend, so the owner question is which entry
      carries that address into the checker.

## Backlog

- The four register items left after this group: `crates/nvs-db/src/span.rs:65` gap 1 and
  `crates/nvs-db/src/tds/mod.rs:88` gap 1 (one file set), `crates/nvs-stdlib/src/cache.rs:155` gap 1 and
  `crates/nvs-stdlib/src/response.rs:199` gap 2 (another).
- `crates/nvs-runtime/src/script.rs`'s test-resolver comment says a real implementor is `!Sync` and uses
  `scoped` for that reason; `nvs_cli::script::Compiler` is `RwLock`-backed and shared. Goal
  `worker-placement`'s own handoff carries it.
- `docs/agent/loop-goal.toml`'s `[context]` had drifted ahead of the goal file by three sessions of
  widenings; the copy is restored, and a session that widens one should write both.
