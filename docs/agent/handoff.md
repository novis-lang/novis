# Handoff

## State

**Goal `resource-ceilings` — stage 6 is landed, and only stage 7 is left.** `budget::Detached`
(`crates/nvs-runtime/src/budget.rs:216`) is the accounting bracket: an allocation or a release made
while one is held moves `budget::detached_bytes` and leaves `live_bytes`, every request's reading and
every armed ceiling where it found them. `Core\Cache`'s local tier takes it at
`crates/nvs-stdlib/src/cache.rs:@store_put`, which now copies the entry into an allocation of its own
so that the store's bytes are allocated and freed on the same balance — the symmetry a bracket owes,
and the playbook's newest bullet. Both stage 6 checks are green.

**The store this goal did not name is the compiled-pattern cache, and it must stay unbracketed** —
`crates/nvs-stdlib/src/regex.rs`'s gap 4 is the finding in full: its entry is an `Rc` the compiling
request holds too, so nothing can put the allocation and the release on one balance, and bracketing it
anyway turns a credit bounded by `CACHE_CAPACITY` into an unbounded one. It waits on M6's arena, which
is the standing decision's *does not solve provenance* reaching a second store.

**The stage 1 floor is red on a file no session of this goal wrote.** `python tools/rules.py --check`
fails on `docs/agent/goals/47-webcrypto.toml:146` and `:153`, which cite the two rules that goal
ships; the tree does not hold them yet, and nothing in this goal can write them.

## Next group

**Stage 7: the record and the rulebook** — one file set, `docs/decisions/` and `docs/rules/`, with
`docs/agent/loop-goal.md` § *Standing decisions* as the specification for every claim in both. No
code: the three conformance cases stage 7's suite check names are already on disk.

- [ ] **The record, at `docs/decisions/0174.md`** — 0174 is the next free number at this commit
      (`docs/decisions/0173.md:1` is the highest on disk, so re-derive before claiming it). Its shape
      is `docs/agent/conventions.md` § *A decision record*, its load-bearing claim is the standing
      decision's — a ctx-less primitive's degenerate return is sound because the request is already
      dead — and it cites `rule:errors/on-limit` and `rule:programs/memory-priority` as the rules it
      works inside. No existing record is amended.
- [ ] **The fragments its `changes.creates` names, under `docs/rules/`** — the refusal and its
      degenerate return, the accounting boundary, and the expansion rule, each a JSON entry plus a
      `<topic>/<slug>.md`, then `python tools/rules.py --render`. What the accounting boundary states
      is `crates/nvs-runtime/src/budget.rs:216`, and it sits beside
      `docs/rules/concurrency/cache-memory-is-charged-to-the-core.md:1`, which says the same thing
      about the tier alone and is what the bracket now enforces.

## Backlog

- The floor above: goal `webcrypto`'s two undesigned rules, at `docs/agent/loop-goal.toml:6497`.
- `crates/nvs-stdlib/src/queue.rs` and `queue_sqlite.rs` were not examined for a store that outlives a
  request — `docs/agent/loop-goal.md` § *Standing decisions* says a session that finds one brackets it.
- One allocation larger than the whole remaining budget is still made before anything notices —
  `crates/nvs-runtime/src/budget.rs`'s own `# Known gap`, owner `resource-ceilings`.
- The stack ceiling is asserted rather than discovered until M6 sizes the request's stack —
  `docs/agent/loop-goal.md` § *Standing decisions*, *out of scope*.
