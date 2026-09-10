# Handoff

## State

**Goal `queue-purge` — a job is removed from the language, by receipt or by tag — has just started; nothing of it
has landed yet.** Goal `type-test`'s whole list is this goal's Stage 1 floor.

The design is finished and is not this goal's to re-open. [ADR 0153](../decisions/0153.md) holds all
of it, `rule:concurrency/queue-deletion-is-explicit-and-bounded` is the two members and their
refusals, and `rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them` is why the group is a new
column rather than a second meaning for `key`.

**Three things a session must not re-decide:**

1. **`Dead` and `Pending` are opt-in, and a claimed job is not removable at all.** The default set is
   `Succeeded` and `Cancelled`. This looks like conservatism and is not: the dead-letter table is the
   record that work was lost, and `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`
   exists because an unwatched one loses it silently — a purge that swept it by default would make
   that rule true of the runtime and false of every deployment. The claimed arm is stronger still: a
   worker holds a lease, there is no protocol for interrupting work in flight
   (`rule:concurrency/cancel-is-a-race-it-can-lose` already says so about `cancel`), and removing the
   row under it turns a job the caller wanted deleted into one that runs to completion and reports
   into nothing. Put the exclusion **in the `where` clause**, not in a check above it.
2. **`tag` is not `key`, and neither is a weaker version of the other.** `key` admits at most one
   pending job under a value, enforced by the unique index over `dedupe_pending`; a tag exists to name
   many. Reusing `key` as the group would cap every group at one pending job, silently, at the enqueue
   that created it. Nothing releases a tag — it is not a lock, so it gets no `_pending` twin.
3. **Selecting inside `args` is refused, and the reason outlives the request that asked for it.**
   `args` is one JSON document in a `text` column, so a predicate over it is a different unindexed
   dialect on each of `rule:core-classes/db-one-api`'s backends, over the one table in the runtime
   that grows without bound. `tag` is the answer for the case that motivates it.

## Next group

**Stage 2, whole** — the column and the converge that carries it, in one slice, because the schema
value and the statement that writes it are the same fact and a tree where one has it and the other
does not is a tree with a hole in it. One file set: `crates/nvs-stdlib/src/queue.rs`,
`crates/nvs-cli/src/worker.rs`.

- [ ] **The column** — `tag`, nullable `short()`, on `JOBS_TABLE` and `DEAD_TABLE` in
      `crates/nvs-stdlib/src/queue.rs:@schema`, beside `dedupe_key` and deliberately not beside
      `dedupe_pending`. Plus the index `nvs_jobs_tag` on `(queue, tag)`.
- [ ] **`push` writes it** — one option in the shape, one bound value in `INSERT_POSTGRES` and
      `INSERT_MYSQL`. **Add at the end of the column list.** `crates/nvs-cli/src/worker.rs` reads its
      columns by position out of the driver rows — its own `args`-position comment is the warning —
      so a column inserted in the middle shifts every read after it, silently, in both dialects.
- [ ] **Nothing else on the request path changes.** The claim statement does not read `tag`, the retry
      statement does not touch it, and the dead-letter move carries it across with the rest of the
      row. A diff that touched `CLAIM_*` or `RETRY_*` is a diff that went wrong.
- [ ] **The converge is the proof, and it is the check to write first** — a database holding the
      pre-`tag` schema must plan **exactly one step, graded `Safe`**, via `base_grade`'s "a nullable
      column with no default is a catalog write on all four" arm in `crates/nvs-db/src/ddl.rs`. Two
      steps, or a grade above `Safe`, means the column was declared wrong and everything after this
      stage is built on a change no live deployment can take.

## Backlog

- **Stage 3** — `delete` and `purge` as statements in both dialects, beside `CANCEL_*` and
  `SUCCEEDED_*`. `delete` keyed on the receipt *and* the state; both members reaching `nvs_jobs` and
  `nvs_dead_jobs` the way `STATUS_*` already does; `State::Claimed` throwing `LogicError` at the call
  beside `push`'s `maxAttempts: 0`. **Extend `queue_statements_agree_with_the_state_enum`** rather
  than writing a second holder of the ordinals.
- **Stage 4** — `Cap::QueuePurge` (`queue.purge`): one variant, one `ALL` entry, and an arm in **both**
  `grant` and `grant_mut`, which `crates/nvs-config/src/capability.rs` writes out twice on purpose.
  `[app.capabilities.queue] purge` in `tree.rs`; `grant_of` already reads all three spellings. `E0635`
  is `E0618` one class over — a *written* queue name only, under `E0618`'s exact conditions.
- **Stage 5** — the registry rows and the reference cards, per conventions.md § *A `Core` member — the
  five edits*. `purge`'s card is the only home of the three things a caller gets wrong: the default set
  is terminal rows, `Dead`/`Pending` are named or untouched, and the answer is a count to loop on.
- **Stage 6** — `examples/queue-purge.nvs` (a new fixture beside `examples/queue.nvs`, whose five
  lines are goal `database`'s and frozen), the conformance cases ADR 0153 § *Verification* names, and the
  `nvs.toml` grant for the new entry — which is what makes the capability evidence rather than
  assumption, since every other fixture in the tree runs with no `queue` grant at all. **No
  differential case**: PHP has no queue.
