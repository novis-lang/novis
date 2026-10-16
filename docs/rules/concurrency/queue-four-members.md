`Core\Queue` is `push`, `status`, `cancel` and `stats` — the four members a request asks, and the four
that take no capability. `push(string $script, {…})`
answers a `Queue\Id` — a receipt that carries its queue with it, not the row's primary key — and
`status` and `cancel` are asked with that receipt rather than with a number a caller would have to
carry a queue name beside. `stats` is the odd one out on purpose: it is asked about a *queue*, because
what wants watching is a population rather than a job, and it answers a `Queue\Stats` whose five
counters are members read out of one statement, so they describe one instant rather than five. Those
five are what is waiting, what a worker holds and what the jobs still in the queue have attempted,
then how deep the dead-letter table is and what the jobs in it attempted before they were buried:
every attempt is in exactly one of the two sums, because a job's attempts move with its row.

The shape is `rule:core-api/shape-rules` throughout: subject first, one trailing options shape,
nothing mutates, failure throws, absence is `?T`. `push`'s options are the job's own — its queue, its
earliest run time, its attempt ceiling, its backoff base, a dedupe `key` that admits at most one
pending job per key, a `tag` that names a group instead
(`rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them`), and the isolate's limits and grants
(`rule:concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue`).

**Removing a row is not one of these four**, and the two members that do it —
`rule:concurrency/queue-deletion-is-explicit-and-bounded`'s `delete` and `purge` — sit beside them
rather than among them. They are the operator's half of the class: the only members that take a
capability, because they are the only ones that can destroy the record that work existed. `push` takes
none because it names no block and so has nothing for a grant to be about, and `status`, `cancel` and
`stats` read or release what the caller already holds a receipt for.
