`Core\Queue` is `push`, `status`, `cancel` and `stats`, and nothing else. `push(string $script, {…})`
answers a `Queue\Id` — a receipt that carries its queue with it, not the row's primary key — and
`status` and `cancel` are asked with that receipt rather than with a number a caller would have to
carry a queue name beside. `stats` is the odd one out on purpose: it is asked about a *queue*, because
what wants watching is a population rather than a job, and it answers a `Queue\Stats` whose four
counters are members read out of one statement, so they describe one instant rather than four.

The shape is `rule:core-api/shape-rules` throughout: subject first, one trailing options shape,
nothing mutates, failure throws, absence is `?T`. `push`'s options are the job's own — its queue, its
earliest run time, its attempt ceiling, its backoff base, a dedupe `key` that admits at most one
pending job per key, and the isolate's limits and grants
(`rule:concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue`).
