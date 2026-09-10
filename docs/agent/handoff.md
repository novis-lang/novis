# Handoff

## State

**Milestone M8, goal `queue-purge`. Stage 2 is landed but for one Docker-backed case.** `tag` is a
nullable `short()` column on both queue tables with `nvs_jobs_tag` over `(queue, tag)`, `push` writes
it in both dialects, the dead-letter move carries it across, and no statement a worker runs reads it.

**The converge is two steps, not the one the goal's prose promised, and that is deliberate.** The
column is `Safe` on all four dialects — a nullable column with no default is a catalog write — but
`nvs_db::ddl::base_grade` grades every index build `Locking`, so `nvs queue migrate` takes the
columns unasked and the index with `--including-risky`. The alternative was dropping the index, which
`rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them` and ADR 0153 § 3 both mandate, so the
expectation is what gave way. It is written down in `schema()`'s own doc comment
(`crates/nvs-stdlib/src/queue.rs:306`) and pinned from both sides: `nvs-db`'s two named cases assert
the column alone needs no flag and the index is the only step that does, and
`the_pre_tag_queue_schema_converges_by_one_safe_column_add_per_table` asserts it of the live value.

**Open:** one `[[check]]` of stage 2 —
`many_pending_jobs_share_one_tag_and_two_pending_jobs_never_share_one_key` — which needs the
PostgreSQL and MySQL legs and so was not written blind.

## Next group

**Stage 3 — the two members, as statements** — one file set: `crates/nvs-stdlib/src/queue.rs`,
`crates/nvs-stdlib/tests/queue.rs`. The first item is stage 2's leftover, which lives in the same
file as the rest and needs the same servers.

- [ ] **The group-versus-key case** —
      `many_pending_jobs_share_one_tag_and_two_pending_jobs_never_share_one_key`, against a real
      server. `crates/nvs-stdlib/tests/queue.rs:693`'s `push_keyed` is the key half and binds the
      tag slot `None`; it needs a tag-carrying twin, and both dialects' bound arrays already have
      the slot. Many pending rows under one tag, one pending row under one key —
      `rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them`.
- [ ] **`delete`, as two statements** beside `CANCEL_POSTGRES` at
      `crates/nvs-stdlib/src/queue.rs:893` and `CANCEL_MYSQL` at
      `crates/nvs-stdlib/src/queue.rs:905`, which are the shapes to copy. Keyed on the receipt
      **and** on the state — `state <> 1` in the `where`, never a check above it — and tried against
      `nvs_dead_jobs` after `nvs_jobs`, as `STATUS_*` already does.
      `rule:concurrency/queue-deletion-is-explicit-and-bounded`.
- [ ] **`purge`, as one bounded `delete` per dialect**, written beside those two at
      `crates/nvs-stdlib/src/queue.rs:893`, over the state set the call selected, with
      `tag`, `before` and the finite `limit`. `state: Dead` reads `nvs_dead_jobs`; every other
      selection reads `nvs_jobs`; the default set is `Succeeded` and `Cancelled`. Same rule, and the
      bound is the rule while the dialect's spelling is not.
- [ ] **Extend `queue_statements_agree_with_the_state_enum`** at
      `crates/nvs-stdlib/src/queue.rs:3166` rather than writing a second one — two spellings of that
      rule is how the ordinals drift.

## Backlog

- Stage 4: the `queue.purge` capability and `E0635` — `docs/agent/goals/34-queue-purge.md`.
- Stage 5: the two registry rows and their reference cards — same file, § *Stage 5*.
- Stage 6: the fixture, the `.nvst` cases and the reference page — same file, § *Stage 6*.
- `nvs queue migrate`'s own module doc (`crates/nvs-cli/src/queue.rs`) says nothing about a plan
  whose only refused step is an index build; an operator meets that on the first tagged deployment.
- The `[context] shapes` manifest gave the five `Core` member edits, which stage 5 needs and stage 2
  did not; nothing was missing from the pack this session.
