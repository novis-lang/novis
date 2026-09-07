A queue row is removed by `delete`, which names one job by its receipt, or by `purge`, which names a
queue and a filter — and neither ever removes a claimed job, a dead-lettered one, or a pending one
unless the call says so. `delete(Queue\Id): bool` is `cancel`'s twin and its `bool` means the same
thing: this call is what removed it. `purge(string $queue, {state?, tag?, before?, limit?}): uint` is
`stats`' twin and answers the count it removed. Neither invents a shape —
`rule:core-api/shape-rules` is satisfied by copying the two members these sit beside.

**A claimed job is not removable at all.** A worker holds a lease on that row, and there is no
protocol for interrupting work in flight — the sentence
`rule:concurrency/cancel-is-a-race-it-can-lose` already writes about `cancel`. Removing the row under
a running worker would leave the job running to completion with nothing to report into, so `delete`
answers `false` there and `purge` has no options that reach it.

**`Dead` and `Pending` are opt-in, and the default set is what has finished.** A purge naming no state
removes succeeded and cancelled rows and nothing else. The dead-letter table is the record that work
was lost, and `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept` exists because an
unwatched one loses it silently — so a sweep that took it by default would make that rule true of the
runtime and false of every deployment. Naming `State::Dead` in the source is what keeps *nothing is
discarded silently* a property of the system rather than of the runtime alone. `Pending` is opt-in for
the mirror reason: a retention sweep that quietly dropped work still waiting to run is a data-loss bug
wearing a maintenance costume.

**`limit` is finite with nothing written**, per
`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`, and the count answered is what makes a
caller's loop the shape that drains a large table. The first `purge` a deployment runs is against the
table that has been growing since it was deployed, and an unbounded `DELETE` there holds a lock on the
connection the application enqueues through for as long as it takes.

**Both members take the deny-by-default `queue.purge` capability, scoped on queue names**, and they
are the only members of `Core\Queue` that take one. `push` names no block and so has nothing for a
grant to be about (`rule:concurrency/queue-four-members`); these name a queue the program itself
wrote, and they destroy the record that work existed. The grant is what keeps that out of the request
path — a web entry enqueues with no `queue` block at all, and the retention entry is the one place in
a deployment that holds the name. Removal is DML on the queue's own connection, so it enlists in an
open transaction exactly as `push` does and issues no DDL: `nvs queue migrate` remains the only thing
that changes the tables' shape.
