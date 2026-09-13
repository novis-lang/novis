# Handoff

## State

**Goal `unowned-closures` — every unowned gap is built to the answer its decision sheet gave — has just
started; nothing of it has landed yet.** Goal `m8-stdlib-depth`'s whole list is this goal's Stage 1 floor.

Settled before the first session: every gap here carries a `Decided:` sentence in its module doc, written
from the user's decision sheet. **Build to it; never re-open it.** A gap with no `Decided:` sentence and
no obvious build is a `BLOCKED` naming it.

## Next group

**Stage 2: the lowering and the runtime, security first** — one file set: `crates/nvs-ir/src/lower/`,
`crates/nvs-runtime/src/`.

- [ ] **A `secret` compared against a `mixed` is constant-time** — `crates/nvs-ir/src/lower/operator.rs:896`,
      `crates/nvs-ir/src/lib.rs:405`.
- [ ] **One allocation past the budget, to its decision** — `crates/nvs-runtime/src/budget.rs:89`.
- [ ] **A hooked property is reached through an erased key** — `crates/nvs-runtime/src/object.rs:3403`.

## Backlog

- The rest of stage 2's decided items — `crates/nvs-runtime/src/lib.rs:199`.
- Stage 3, the checker and front end — `crates/nvs-hir/src/requires.rs:84` first.
- Stage 4, the library — `crates/nvs-stdlib/src/compress.rs:47` first.
- Stage 5, server/config/cache/schema — `crates/nvs-config/src/cache.rs:51` first.
- Stage 6, the five M10 retags — `python tools/owners.py --deferrals`.
- When this goal's last check goes green the driver takes goal `gap-zero`.
