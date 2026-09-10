# Handoff

## State

**Milestone M8, goal `queue-purge`. Every stage is on disk.** Stage 6's fixture
`examples/queue-purge.nvs` prints the eight lines frozen at `docs/agent/loop-goal.toml:7344`, in that
order, on stdout, and exits 0 against the compose PostgreSQL. `python tools/reference.py --check` —
stage 6's other check — already passed. Stage 4's `E0637` is at `crates/nvs-diagnostics/src/lib.rs:1820`,
and stages 3 and 5 were on disk before this session.

**The fixture draws its four queue names at run time**, for `examples/queue.nvs`'s reason, so the
grant `nvs.toml` now holds for its entry is `purge = true` rather than a name list: the scoped
spelling is what `E0637` and its four `-p nvs-types` cases pin, and a queue that exists only while a
program runs is not where that half could be proved. `examples/queue/held.nvs` is the second file the
fixture needed — a job that parks for a second is the only way a row is `Claimed` while a caller asks
to delete it, and `claimed-delete=false` is the line that needs one.

**Both goal files are untouched this session.** `examples/queue-purge.nvs` was already in
`docs/agent/loop-goal.toml`'s `files` list, so that file and `docs/agent/goals/34-queue-purge.toml`
are still byte-identical.

**The sweep's PostgreSQL holds a stale pending job naming `scripts/receipt.nvs`** from some earlier
run. Any queue fixture's in-process worker claims it and reports it in two lines on **stderr**;
stdout is untouched, and so is every `exact` check, which captures the two streams separately.

## Next group

**Stage 6: the two `purge` selectors no case binds** — one file set:
`tests/conformance/core/queue-purge-*.nvst`, `crates/nvs-stdlib/src/queue.rs`.

- [ ] **A case pinning `{before: …}`.** None of the three `queue-purge-*.nvst` cases names it, so
      `crates/nvs-stdlib/src/queue.rs:3078`'s `purge_before_of` and the millisecond conversion under
      it are reached by nothing in the corpus.
      `tests/conformance/core/queue-purge-takes-its-filter-as-one-trailing-shape.nvst:1` is the
      sibling to copy — `rule:concurrency/queue-deletion-is-explicit-and-bounded`.
- [ ] **The dead-letter table's own `{tag: …}`.** `crates/nvs-stdlib/src/queue.rs:3172` binds a tag
      for `Selection::Dead` and nothing asks it: `examples/queue-purge.nvs:114` purges the dead
      letter by state alone, and `rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them` is what
      a tag surviving § 6's move means. Same file set.

## Backlog

- `crates/nvs-stdlib/tests/queue.rs` is the real-server leg — a selector no case binds is a
  statement text no server has ever parsed; that file's module doc owns the reasoning.
- The stale `scripts/receipt.nvs` row above: nothing in this repository removes it, and
  `docs/agent/carried-gaps.md` is where it belongs if the goal switches first.
