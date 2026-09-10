# Handoff

## State

**Milestone M8, goal `queue-purge`. Stage 3's statement half is closed: every text
`rule:concurrency/queue-deletion-is-explicit-and-bounded` needs is on disk, and a real server has
run all six of them.** The two deletes and the four purge texts were sent to PostgreSQL, MySQL and
MariaDB by `python tools/db-matrix.py`, and the four new cases pass on each leg.

**`purge` is four constants, not two** — `PURGE_POSTGRES` (`crates/nvs-stdlib/src/queue.rs:1027`) and
`PURGE_MYSQL` over the jobs table, `PURGE_DEAD_POSTGRES` and `PURGE_DEAD_MYSQL` over the other one,
because ADR 0153 § 6's `state: Dead` selection is a different table and `DEAD_TABLE` has no `state`
column to select on. Each constant's doc owns its own shape decision; the two that a session is most
likely to re-open are that `before` reads `created_at` and that the bound is `order by id limit`.

**Open:** the two members themselves. `Core\Queue::delete` and `::purge` still have no registry row,
no card, no body and no `address()` arm, and until they exist `examples/queue-purge.nvs` — the
acceptance fixture the driver is failing on — cannot be written.

## Next group

**Stage 3: the two members, as the five edits each** — one file set:
`crates/nvs-stdlib/src/queue.rs`, `tests/conformance/core/`.

- [ ] **`Core\Queue::delete(Queue\Id $job): bool`**, as the five edits
      `docs/agent/conventions.md` § *A `Core` member* lists: the row in `CLASS` at
      `crates/nvs-stdlib/src/queue.rs:1192`, a card beside `CANCEL_DOC` at
      `crates/nvs-stdlib/src/queue.rs:1432`, a body in `nvs_core_queue_cancel`'s shape at
      `crates/nvs-stdlib/src/queue.rs:2660` — which is the twin, down to reading the receipt and
      picking a dialect — and the `address()` arm at `crates/nvs-stdlib/src/queue.rs:2844`. The
      `bool` is whether a row came back on PostgreSQL and the affected count on the framed dialect;
      `counted_row` is where `cancel` already does that reading.
      `rule:concurrency/queue-deletion-is-explicit-and-bounded`, ADR 0153 § 1.
- [ ] **`Core\Queue::purge(string $queue, {state?, tag?, before?, limit?}): uint`**, the same five
      edits at the same four anchors, starting from the row at
      `crates/nvs-stdlib/src/queue.rs:1192`. Its `args: [5]` is one slot per option
      (`crates/nvs-stdlib/src/queue.rs:2660`'s macro counts a bag flattened), `State::Claimed` is
      refused at the call rather than answered `0`, `state: Dead` picks the `PURGE_DEAD_*` text at
      `crates/nvs-stdlib/src/queue.rs:1046`, and `limit` is finite with nothing written.
      ADR 0153 §§ 1, 2 and 4.
- [ ] **Three `.nvst` cases per member** under `tests/conformance/core/`, in
      `tests/conformance/core/queue-cancel-and-status-are-asked-the-same-way.nvst`'s shape and named
      for the corpus's flat `queue-…` convention. The floor is
      `crates/nvs-stdlib/tests/conformance_coverage.rs:155`, and each of the three asks a different
      question — the refusal of `State::Claimed` is one of them.
      `rule:concurrency/queue-four-members`.

## Backlog

- The `queue.purge` capability, deny-by-default and scoped on queue names, and `E0635` — ADR 0153 § 5.
- `examples/queue-purge.nvs`, the acceptance fixture, which needs both members first — ADR 0153 § *Verification*.
- `[context] adrs` in `docs/agent/goals/34-queue-purge.toml` is missing `0153 § 1` (the two
  signatures) and `0153 § Verification` (the fixture's frozen lines); both were sliced by hand here.
- `[context]` printed no window on `crates/nvs-stdlib/src/queue.rs:306`'s `schema()`, which is the
  one home for the column set every filter is written against.
