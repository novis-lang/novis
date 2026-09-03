# Handoff

## State

**Goal 8 — `Core\Program::id()` — has just started; nothing of it has landed yet.** Goal 7's whole list
is this goal's Stage 1 floor. The design is settled in the goal prose's standing decisions: the formula
is `BLAKE3(unit content hashes in program order ‖ env_hash)`, 64 lowercase hex characters, computed at
program resolution and at the hot-reload swap, exposed as a plain (never `secret`) string; the one design
act is a folded amendment to ADR 0061 giving `Core\Program` its first runtime member and recording why
the id cannot be a compile-time-folded constant.

## Next group

**Stage 2: the combine and its threading** — one file set: `crates/nvs-config/src/cache.rs`,
`crates/nvs-hir/src/requires.rs`, `crates/nvs-runtime/src/ctx.rs`.

- [ ] **`program_id` beside its two inputs** — `crates/nvs-config/src/cache.rs:120` (`content_hash`) and
      `:105` (`env_hash`): BLAKE3 over the unit content hashes in program order, then the env hash,
      reusing the digests the artifact cache already computes. The four named tests of the TOML's
      stage 2 check prove deterministic and complete.
- [ ] **Threaded to the runtime** — computed where `resolve_program`'s answer
      (`crates/nvs-hir/src/requires.rs:182`) and the `env_hash` are both in hand, stored in the
      per-program state, recomputed by the hot-reload swap.

## Backlog

- Stage 3 (the member, the ADR 0061 amendment, the registry card with the goal prose's two
  descriptions, `docs/reference/core/Program.md`, the conformance case, `examples/program-id.nvs`)
  shares no files with stage 2 except the ctx seam — a session that lands stage 2 with headroom starts
  the ADR amendment, which is prose and cheap.
- When this goal's last check goes green the driver takes goal 9 — `Core\Db\Schema`. The chain runs to
  goal 15; M4B is its last four entries.
