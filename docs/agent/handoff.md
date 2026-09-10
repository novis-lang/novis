# Handoff

## State

**Goal `unowned-sweep`, stage 4 — both checks green, and every test name in every stage of
`loop-goal.toml` now exists on disk** (checked name by name). `[limits] max_output` bounds a capture
at both readers: `crates/nvs-stdlib/src/process.rs:420`'s `drain` holds a child's two pipes to it and
kills the child at the bound, and `crates/nvs-stdlib/src/io.rs:2648`'s `slurp` holds a whole-file read
to it. Stage 5's `owners.py --unowned --check` is green; the two `nvs-suite` runs are what
`verify.py` covers, so the goal is met if that run is.

**The item's premise was stale and the correction is the useful part.** `[limits] max_output` already
had a reader — `Ctx::output_limit`, `crates/nvs-runtime/src/ctx/limits.rs:112` — and had since it
landed. What was missing was the *unit* a member needs: the response ceiling is a running total per
request, and a capture is one buffer per call. So the number is now read twice, by
`Ctx::intake_limit` / `intake_bound` / `intake_breach`
(`crates/nvs-runtime/src/ctx/limits.rs:153`), and the field doc at
`crates/nvs-runtime/src/ctx/mod.rs:327` names both readings.

**The tradeoff, stated because it changes a landed member.** `Core\IO::read` of a file larger than
`max_output` now throws where it used to succeed, and a chatty child is killed mid-write. Both
refusals are **catchable** and are not `rule:errors/on-limit`'s `FATAL`: nothing reached the response,
so the request exceeded no limit — a member declined to hold more than the request may produce.
`rule:core-classes/process-run` is the home of that reading. Per request this spends nothing; it only
lowers what a capture may hold.

## Next group

**Stage 5: the two suites, and the `.nvst` half of the bound nothing pins yet** — one file set:
`tests/conformance/core/` beside the two members that just changed,
`crates/nvs-stdlib/src/process.rs` and `crates/nvs-stdlib/src/io.rs`. Both members' `-p nvs-stdlib`
cases assert the bound; no conformance case does, and `rule:core-classes/process-run`'s guard list is
`.nvst` cases. A case that configures anything is a multi-file case — the playbook's *A multi-file
`.nvst` case* bullet owns the shape, and `try.py` cannot drive one.

- [ ] **A conformance case pins the refusal a program sees when a child writes past the ceiling.**
      `tests/conformance/core/process-a-capture-is-whole-not-a-pipes-worth.nvst:1` is the sibling to
      copy the shape from, and `crates/nvs-stdlib/src/process.rs:420` is what it exercises. The
      refusal is a catchable `RuntimeError` naming `max_output` and the member —
      `rule:core-classes/process-run`.
- [ ] **The same for `Core\IO::read`, and that the two messages agree.**
      `tests/conformance/core/io-read-and-read-text-name-one-door.nvst:1` already asks one question of
      both doors and is the shape; `crates/nvs-stdlib/src/io.rs:2648` is the reader, and
      `readText`/`lines` inherit the ceiling through it without a case of their own.
- [ ] **A case for the accepted side, so the ceiling is pinned on both sides in the corpus too.**
      `crates/nvs-runtime/src/ctx/limits.rs:166` is why one byte past is where a read stops: a file
      exactly at the ceiling is read whole and must stay that way.

## Backlog

- No `.nvst` case configures `[limits] max_output` at all — the whole ceiling is Rust-side only.
- `Core\Process::spawn` and `ProcessOptions` are still `(designed)` — ADR 0044 §§ 2–3.
- `drain` spends one OS thread per `run` for the child's lifetime; nothing measures it
  (`crates/nvs-stdlib/src/process.rs:420` says what it buys).
- ~106 module-doc gaps are still unowned — `docs/agent/carried-gaps.md` § *Unowned*.
