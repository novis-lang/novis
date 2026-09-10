# Handoff

## State

**Milestone M8, goal `queue-purge`. Stage 2 is closed, and stage 3's two `delete` statements are on
disk with no caller yet.** `many_pending_jobs_share_one_tag_and_two_pending_jobs_never_share_one_key`
(`crates/nvs-stdlib/tests/queue.rs:1599`) ran against a live PostgreSQL: four pending rows under one
tag, one of them also holding the key, and the second keyed push answering the first job's own id off
`INSERT_POSTGRES`'s `existing` arm. It is the first case in this file to bind `$1` at all, so the
dedupe arm every other push leaves null has now met a server.

**`DELETE_POSTGRES` (`crates/nvs-stdlib/src/queue.rs:939`) and `DELETE_MYSQL` (`:975`) are one
statement each, and both were run by hand against the live containers before they were written
down** — pending row removed, claimed row refused and still there afterwards, dead-lettered row
removed out of the other table, wrong queue nothing. The PostgreSQL text is two data-modifying CTEs
and the framed one is a multi-table delete driven from a derived row; the playbook holds why the
framed shape is not the obvious one. What is missing is a case in the suite that *sends* them, which
is the first item below.

**Open:** the rest of stage 3 — the two server cases, `purge`'s statements, and both members
themselves (`Core\Queue::delete` and `::purge` have no registry row, card, body or `address()` arm).

## Next group

**Stage 3: the deletes, sent to a server and then given their members** — one file set:
`crates/nvs-stdlib/src/queue.rs`, `crates/nvs-stdlib/tests/queue.rs`.

- [ ] **`delete_answers_false_for_a_claimed_job_and_true_for_a_pending_one`**, sending
      `DELETE_POSTGRES` at `crates/nvs-stdlib/src/queue.rs:939` from a new case in the PostgreSQL
      block beside `crates/nvs-stdlib/tests/queue.rs:1599` — `push_marked` writes the row and
      `claim` is what makes one of them claimed. Both sides of the bound in one case: the pending
      row goes, the claimed one is still there afterwards, and the answer is the row the statement
      returned. `rule:concurrency/queue-deletion-is-explicit-and-bounded`.
- [ ] **`delete_finds_a_receipt_in_the_dead_letter_table_the_way_status_already_does`** — the framed
      twin, sending `DELETE_MYSQL` (`crates/nvs-stdlib/src/queue.rs:975`) from a case beside
      `crates/nvs-stdlib/tests/queue.rs:2607`, whose `cancel` helper is the shape for a statement
      read off its affected count. `a_framed_exhausted_job_moves_to_the_dead_letter_table_in_one_transaction`
      is what puts a row in the other table to find. Same rule, ADR 0153 § 6.
- [ ] **`purge`, as one bounded `delete` per dialect**, written beside the two at
      `crates/nvs-stdlib/src/queue.rs:975`: the state set the call selected, `tag`, `before`, and
      the finite `limit` that is the rule rather than the spelling. `state: Dead` reads
      `nvs_dead_jobs`, every other selection reads `nvs_jobs`, and the default set is `Succeeded`
      and `Cancelled`. ADR 0153 §§ 2 and 4.
- [ ] **Extend `queue_statements_agree_with_the_state_enum`** at
      `crates/nvs-stdlib/src/queue.rs:3291` with `purge`'s ordinals rather than writing a second
      test — `delete`'s `Claimed` line is already in it.

## Backlog

- The `queue.purge` capability and `E0635` under `E0618`'s conditions — stage 4, `docs/agent/loop-goal.toml`.
- Both members as `Core` rows: registry, card, body, `address()` arm — `docs/agent/conventions.md` § *A `Core` member*.
- `DELETE_MYSQL`'s multi-table spelling has never met MariaDB: `python tools/db-matrix.py --driver mariadb`.
- `.nvst` conformance cases for either member, once they exist — `docs/agent/conventions.md` § *A `.nvst` test case*.
