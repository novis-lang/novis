A queue row is removed by `delete`, which names one job by its receipt, or by `purge`, which names a
queue and a filter — and neither ever removes a claimed job, a dead-lettered one, or a pending one
unless the call says so. `delete(Queue\Id): bool` is `cancel`'s twin and its `bool` means the same
thing: this call is what removed it. `purge(string $queue, {state?, tag?, before?, limit?}): uint` is
`stats`' twin and answers the count it removed. Neither invents a shape —
`rule:core-api/shape-rules` is satisfied by copying the two members these sit beside. They are two
members because their subjects are a receipt and a queue name, and they are named for what they do:
`delete` removes the thing named, `purge` removes what a filter selects.

**A claimed job is not removable at all.** A worker holds a lease on that row, and there is no
protocol for interrupting work in flight — the sentence
`rule:concurrency/cancel-is-a-race-it-can-lose` already writes about `cancel`. Removing the row under
a running worker would leave the job running to completion with nothing to report into, so `delete`
answers `false` there and `purge` has no options that reach it: a `purge` naming `State::Claimed`
throws a `LogicError` at the call.

**`Dead` and `Pending` are opt-in, and the default set is what has finished.** A purge naming no state
removes succeeded and cancelled rows and nothing else. The dead-letter table is the record that work
was lost, and `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept` exists because an
unwatched one loses it silently — so a sweep that took it by default would make that rule true of the
runtime and false of every deployment. Naming `State::Dead` in the source is what keeps *nothing is
discarded silently* a property of the system rather than of the runtime alone. `Pending` is opt-in for
the mirror reason: a retention sweep that quietly dropped work still waiting to run is a data-loss bug
wearing a maintenance costume.

**One member reaches both tables.** A `purge` naming `State::Dead` deletes from the dead-letter table
and every other selection from the jobs table. `delete` looks for the receipt in the jobs table and
then in the dead-letter table, as `status` does, because a receipt names a job across the move and
one that stopped working when the job ran out of attempts would expire without saying so.

**`limit` is finite with nothing written**, per
`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`, and the count answered is what makes a
caller's loop the shape that drains a large table. The first `purge` a deployment runs is against the
table that has been growing since it was deployed, and an unbounded `DELETE` there holds a lock on the
connection the application enqueues through for as long as it takes. A default that ran to
completion would make the correct call and the catastrophic one look identical in source.

**Both members take the deny-by-default `queue.purge` capability, scoped on queue names**, and they
are the only members of `Core\Queue` that take one. `push` names no block and so has nothing for a
grant to be about (`rule:concurrency/queue-four-members`); these name a queue the program itself
wrote, and they destroy the record that work existed. The grant is what keeps that out of the request
path — a web entry enqueues with no `queue` block at all, and the retention entry is the one place in
a deployment that holds the name. `delete` takes the same grant rather than a lesser one: the grant
answers whether this program may remove queue records, and one row at a time is the same answer at
a different rate. A `purge` whose queue name is a string literal outside this machine's grant does not
compile; a computed name, and every `delete`, whose queue comes out of a receipt at run time, are
refused at the call. Removal is DML on the queue's own connection, so it enlists in an open
transaction exactly as `push` does and issues no DDL: `nvs queue migrate` remains the only thing
that changes the tables' shape.

**Removal is never the runtime's own act.** There is no default lifetime, no sweeper and no
configuration key that expires anything, so a deployment that never calls `purge` keeps every row. A
runtime that dropped completed rows on a timer would be deleting an operator's audit trail on a
default nobody chose.

**Declined:**
- **No member, and SQL against the runtime's tables.** It makes the table and column names and the
  state ordinals a public contract by use, and it gives removal to everything holding `db.connect`.
- **A retention key with a sweeper in the worker.** It answers the age case and neither a cancelled
  batch nor a deleted tenant, and it puts an unbounded delete on a timer inside the worker loop. If one
  is ever wanted, it sits on `purge` rather than beside it.
- **`delete` alone.** A caller with tens of thousands of receipts would write the loop the database
  should have run.
- **`purge` alone, with the receipt in its options.** A receipt already carries its queue, so the
  queue name and the receipt would be two spellings of one fact that can disagree, and the subject
  belongs in the first parameter (`rule:core-api/shape-rules`).
- **Deleting the row when a job succeeds.** It removes the growth at its source and costs `status`:
  a receipt would answer "succeeded" and "never existed" identically, and
  `rule:concurrency/delivery-is-at-least-once` makes "did it run" a question programs ask.
- **A separate capability for `delete`.** Two names for one question; the roster keeps two
  capabilities apart only when they answer different questions (`rule:security/capability-roster-is-closed`).
- **Wildcards in the grant.** A queue name is a flat string the program picks, so there are no labels
  for a pattern to match at, and a prefix grant would cover queues nobody has named yet. A grant is
  exact names, or `true`.
