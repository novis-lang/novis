---
milestone: M8
---
# Loop goal 34 — a job is removed from the language, by receipt or by tag

`Core\Queue` can create a job and cannot remove one. A succeeded row stays in `nvs_jobs` forever, a
cancelled batch of forty thousand leaves forty thousand rows, and the dead-letter table — which the
runtime is right never to sweep — has no spelling an operator can sweep either. The only answer today
is `Core\Db::execute` against the runtime's own tables, which makes `nvs_jobs`' column names and its
`state` ordinals part of the public contract by use, and hands the removal to every code path holding
`db.connect`.

[ADR 0153](../../decisions/0153.md) is the whole design and this goal is its implementation: one
column, two members, one capability, one diagnostic.
`rule:concurrency/queue-deletion-is-explicit-and-bounded` is the members and their refusals,
`rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them` is why the column is not `key`.
**Neither is this goal's to re-open**, and neither is
`rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`, which is what the `State::Dead` arm
exists to keep true.

Its floor is goal `type-test`'s whole list.

## Why here

The queue can create a job and cannot remove one, so `nvs_jobs` grows with every job the deployment
has ever run and the only answer is raw SQL against tables the runtime owns. It sits here rather
than beside goal `database` because the schema change is what makes it cheap and that is goal `schema`'s converge,
not goal `database`'s hand-written DDL lists: a nullable column with no default grades `Safe`, so a live
deployment takes it through the `nvs queue migrate` it already runs. After goal `signed-urls` and `type-test` because it
adds surface and they are the entries that settled how surface is added; in front of the dossier
because that entry stops adding any.

## The surface, in one block

```php
use Core\Queue;
use Core\Queue\State;

// The group is decided at enqueue. Nothing can group rows that were never grouped.
Queue::push('jobs/export.nvs', {args: {row: $id}, tag: 'export:' . $batch});

Queue::delete($receipt);                                          // one row, by receipt -> bool
Queue::purge('exports', {tag: 'export:' . $batch,
                        state: State::Pending});                  // a cancelled batch  -> uint
Queue::purge('exports', {before: Core\Time::now()->minus(30d)});  // retention: terminal rows only
Queue::purge('exports', {state: State::Dead, tag: 'tenant:7'});   // named, or never touched
```

```toml
[app.capabilities.queue]
purge = ["exports", "email"]     # or `true`; absent is the denial
```

## Stage 1 — the floor

Goal `type-test`'s whole acceptance list, never traded.

## Stage 2 — the keystone: one column, and the converge that carries it

One file set: `crates/nvs-stdlib/src/queue.rs`, `crates/nvs-cli/src/worker.rs`.

1. **`tag`** — a nullable `short()` column on `JOBS_TABLE` and on `DEAD_TABLE`, and an index
   `nvs_jobs_tag` on `(queue, tag)`. Beside `dedupe_key`, and deliberately **not** beside
   `dedupe_pending`: nothing releases a tag, because a tag is not a lock.
2. **`push` writes it.** One more option in the shape, one more bound value in `INSERT_*`, and no
   other statement on the request path changes. The claim statement does not read it; the retry
   statement does not touch it; the dead-letter move carries it across with the rest of the row.
3. **The converge is the proof.** A database holding the pre-`tag` schema must plan **exactly one
   step, graded `Safe`** — `nvs_db::ddl`'s "a nullable column with no default is a catalog write on
   all four" arm. If it plans two, or grades up, the column was declared wrong; that is the check to
   write first, because everything after it assumes a live deployment can take this change through
   the `nvs queue migrate` it already runs.

**The trap here is the worker.** `crates/nvs-cli/src/worker.rs` reads its columns by position out of
the driver rows, per its own `args`-position comment. A column added in the middle of a `select` list
silently shifts every read after it. Add at the end of the list, and read the position tests.

## Stage 3 — the two members, as statements

One file: `crates/nvs-stdlib/src/queue.rs`.

`delete` and `purge` in the two dialects the queue already writes — PostgreSQL and MySQL — beside
`CANCEL_*` and `SUCCEEDED_*`, which are the shapes to copy.

1. **`delete` is keyed on the receipt and on the state**, never on the id alone: `where id = ? and
   state <> 1` is what makes "a claimed job is not removable" a property of the statement rather than
   of a check above it, and the affected-row count is the `bool` the member answers.
2. **`delete` tries `nvs_jobs` and then `nvs_dead_jobs`**, exactly as `STATUS_*` already does across
   both tables. A `Queue\Id` names a job across the dead-letter move, and a member that stopped
   working the moment a job exhausted its attempts would be a receipt that expires without saying so.
3. **`purge` is one `DELETE … LIMIT`** over the state set the call selected, with `tag`, `before` and
   the bound. `state: Dead` reads `nvs_dead_jobs`; every other selection reads `nvs_jobs`; the default
   set is `Succeeded` and `Cancelled` and nothing else.
4. **`State::Claimed` throws `LogicError` at the call**, beside `push`'s `maxAttempts: 0` — one closed
   enum case out of five, and the throw names it.
5. `queue_statements_agree_with_the_state_enum` already holds the ordinals in these statements to
   `STATE`'s cases. **Extend it rather than writing a second one**; two spellings of that rule is how
   the ordinals drift.

## Stage 4 — the capability, and the one diagnostic

One file set: `crates/nvs-config/src/capability.rs`, `crates/nvs-config/src/tree.rs`,
`crates/nvs-diagnostics/src/lib.rs`, `crates/nvs-types/src/intrinsics.rs`,
`crates/nvs-runtime/src/capability.rs`.

1. **`Cap::QueuePurge`, spelled `queue.purge`** — one variant, one entry in `ALL`, and an arm in
   **both** `grant` and `grant_mut`, which that type writes out twice on purpose: they *are* the
   name-to-field mapping, and its own doc says a shared traversal would be a third thing to keep in
   step. Scoped on queue names, exact only: `takes_host_wildcard` stays `db.open`'s alone, because a
   queue name is a flat string a program picks and is a UUID in this repository's own fixture.
2. **`[app.capabilities.queue] purge`** — one field on one new struct in `tree.rs`. `grant_of` already
   reads `true`, a bare string, a list and an empty list; nothing new is written for the three-way
   grant.
3. **`E0637`** — a *written* `Core\Queue::purge` whose literal queue name the compiling machine's
   grant does not cover. This is `E0618` one class over and is asked under `E0618`'s conditions and no
   others: a literal name, a configuration this machine actually read, the same list walked by the
   same `Capabilities::allows`. A computed name says nothing.
4. **`delete` has no static half** — its queue comes out of a `Queue\Id` at run time, so it is refused
   at the door like any other ungranted act.

## Stage 5 — the registry rows

One file set: `crates/nvs-stdlib/src/queue.rs`, `crates/nvs-stdlib/src/registry.rs`.

The two members as `CoreMethod` rows with their reference cards, and `tag` added to `push`'s
`ParamDoc` set — conventions.md § *A `Core` member — the five edits* is the shape, and this stage is
the two members' four other edits after stage 3's statement.

The doc rows are the *only* home of what an operator reads in `docs/novis.md`, so `purge`'s card must
say the three things a caller gets wrong: the default set is terminal rows, `Dead` and `Pending` are
named or untouched, and the answer is a count to loop on rather than a completion.

## Stage 6 — the fixture, the cases, and the reference

`examples/queue-purge.nvs`, printing one frozen line per property, beside `examples/queue.nvs` rather
than inside it: that fixture is goal `database`'s and its five lines are frozen.

The conformance cases ADR 0153 § *Verification* names. **No differential case** — PHP has no queue,
which is the same reason `examples/queue.nvs` has none.

`nvs.toml` gains the grant for the new fixture's entry, which is also what makes the capability
evidence rather than assumption: every other fixture in the tree runs with no `queue` grant at all.

## Standing decisions

- **This goal opens no new ADR number.** [ADR 0153](../../decisions/0153.md) is accepted and is the
  whole design: one column, two members, one capability, one diagnostic. The three rules the lead
  paragraph names are not this goal's to re-open, and a gap found in one is an edit to that fragment
  through a record whose `changes:` block names it.
- **The bound on `purge` is the rule, and the dialect spelling is not.**
  `rule:concurrency/queue-deletion-is-explicit-and-bounded` is what the member owes; how each of the
  two dialects expresses a bounded delete is an implementation choice a session makes and records in
  `queue.rs`'s statement comments. A dialect that will not take one spelling takes another — dropping
  the bound is the one answer that is not available.
- **The converge is the check written first, and one `Safe` step is the whole of it.** If the planner
  answers two steps or grades up, the column was declared wrong and the fix is the declaration, not the
  expectation: everything after stage 2 assumes a live deployment takes this through the `nvs queue
  migrate` it already runs.
- **`State::Claimed` throws at the call, and the default set is terminal rows.** Both are 0153's and
  stage 3's, restated here because they are what a caller gets wrong and therefore what a session is
  most likely to "fix" in the other direction. `Dead` and `Pending` are named or untouched.
- **`delete` has no static half**, so there is no second diagnostic to design. Its queue name arrives in
  a `Queue\Id` at run time and it is refused at the door like any other ungranted act; `E0637` is
  `E0618` one class over and is asked under `E0618`'s conditions and no others.
- **What this spends**, per `rule:programs/memory-priority`: one nullable column and one index on
  `(queue, tag)` per jobs table, written once per `push` and read by nothing on the request path — the
  claim, retry and dead-letter statements are unchanged. No allocation per job that the queue did not
  already make.
