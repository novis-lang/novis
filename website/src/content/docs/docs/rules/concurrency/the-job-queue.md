---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "The job queue"
description: "Enqueue commits with your own write. Four members, at-least-once delivery, finite attempts, and a dead letter that is kept."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/concurrency/connections/
  label: "Persistent connections"
next:
  link: /docs/rules/concurrency/running-a-job/
  label: "Running a job"
---

<p class="nv-section-lead">Enqueue commits with your own write. Four members, at-least-once delivery, finite attempts, and a dead letter that is kept.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">10</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">8</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">2</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">9</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#enqueue-commits-with-your-write">A <code>push</code> on the queue's connection commits with the write that caused it, or neither happens</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#queue-four-members"><code>Core\Queue</code>'s four request-path members are each asked about a job or about a queue, never about a row number</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-tag-groups-jobs-and-a-key-dedupes-them">A <code>tag</code> names a group of jobs and a <code>key</code> admits one pending job, and they are two columns because they are opposites</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#a-job-names-a-file">A job names a script file, and its payload crosses as a copied value</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-jobs-budget-and-grants-are-recorded-at-enqueue">A job's budget and grants are recorded when it is enqueued, and narrowed from that context rather than widened</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#claiming-is-one-statement">A worker claims a job with one statement the database arbitrates, so a fleet needs no lease protocol</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-job-between-attempts-is-pending">A job between attempts is <code>Pending</code>, and there is no <code>Failed</code> state to ask about</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#delivery-is-at-least-once">Delivery is at-least-once, and idempotency is the job's own obligation</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#attempts-are-finite-and-a-dead-letter-is-kept">Attempts are finite, backoff is exponential and jittered, and an exhausted job is kept rather than discarded</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-budget-overrun-is-a-failed-attempt">A job that exceeds its budget is a failed attempt, reported as that and retried like any other</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="enqueue-commits-with-your-write">

## A `push` on the queue's connection commits with the write that caused it, or neither happens

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#enqueue-commits-with-your-write"><code>concurrency/enqueue-commits-with-your-write</code></a>
</div>

`Core\Queue::push` on the queue's own connection enlists in whatever transaction that connection
already has open. If the transaction rolls back the job was never enqueued; if it commits the job is
durable. There is no window in which the order exists and the job that was to send its receipt does
not, and there is no outbox to write.

That is the property a job being a row buys, and it is the whole reason the storage is a table in a
database `Core\Db` already talks to ([`core-classes/queue-storage-is-a-table`](/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table "The job queue is two tables in a connection the operator names, converged by an explicit command")). Every queue built
on an external broker has an enqueue and a business write in two systems that cannot commit together,
and every team on one rediscovers the outbox pattern — which is to say they rediscover that the
database was the right queue.

Outside a transaction, `push` is its own committed statement, which is the ordinary case and needs
nothing said about it. Pointing `[queue] connection` at a separate database is permitted and silently
gives up this property; so does pushing on a different connection than the queue's
([`concurrency/foreign-connection-enqueue-is-counted`](/docs/rules/concurrency/running-a-job/#foreign-connection-enqueue-is-counted "A push on a connection other than the queue's is not transactional, and is counted rather than assumed away")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A queued job and the database write that justified it commit together, where an application on a broker writes an outbox row and relays it afterwards</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table" title="The job queue is two tables in a connection the operator names, converged by an explicit command"><code>core-classes/queue-storage-is-a-table</code></a> <a href="/docs/rules/concurrency/running-a-job/#foreign-connection-enqueue-is-counted" title="A push on a connection other than the queue's is not transactional, and is counted rather than assumed away"><code>concurrency/foreign-connection-enqueue-is-counted</code></a> <a href="/docs/rules/concurrency/running-a-job/#no-broker-and-no-driver-interface" title="There is no broker backend and no pluggable driver, because a driver makes transactional enqueue optional"><code>concurrency/no-broker-and-no-driver-interface</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/queue.rs"><code>crates/nvs-stdlib/tests/queue.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="queue-four-members">

## `Core\Queue`'s four request-path members are each asked about a job or about a queue, never about a row number

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#queue-four-members"><code>concurrency/queue-four-members</code></a>
</div>

`Core\Queue` is `push`, `status`, `cancel` and `stats` — the four members a request asks, and the four
that take no capability. `push(string $script, {…})`
answers a `Queue\Id` — a receipt that carries its queue with it, not the row's primary key — and
`status` and `cancel` are asked with that receipt rather than with a number a caller would have to
carry a queue name beside. `stats` is the odd one out on purpose: it is asked about a *queue*, because
what wants watching is a population rather than a job, and it answers a `Queue\Stats` whose four
counters are members read out of one statement, so they describe one instant rather than four.

The shape is [`core-api/shape-rules`](/docs/rules/core-api/naming-and-shape/#shape-rules "Every Core member obeys the same twenty shape rules, R1–R20") throughout: subject first, one trailing options shape,
nothing mutates, failure throws, absence is `?T`. `push`'s options are the job's own — its queue, its
earliest run time, its attempt ceiling, its backoff base, a dedupe `key` that admits at most one
pending job per key, a `tag` that names a group instead
([`concurrency/a-tag-groups-jobs-and-a-key-dedupes-them`](/docs/rules/concurrency/the-job-queue/#a-tag-groups-jobs-and-a-key-dedupes-them "A tag names a group of jobs and a key admits one pending job, and they are two columns because they are opposites")), and the isolate's limits and grants
([`concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue`](/docs/rules/concurrency/the-job-queue/#a-jobs-budget-and-grants-are-recorded-at-enqueue "A job's budget and grants are recorded when it is enqueued, and narrowed from that context rather than widened")).

**Removing a row is not one of these four**, and the two members that do it —
[`concurrency/queue-deletion-is-explicit-and-bounded`](/docs/rules/concurrency/running-a-job/#queue-deletion-is-explicit-and-bounded "A queue row is removed by delete or purge, never while claimed, and never dead-lettered or pending unless the call says so")'s `delete` and `purge` — sit beside them
rather than among them. They are the operator's half of the class: the only members that take a
capability, because they are the only ones that can destroy the record that work existed. `push` takes
none because it names no block and so has nothing for a grant to be about, and `status`, `cancel` and
`stats` read or release what the caller already holds a receipt for.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The queue is four members of the runtime rather than a framework façade, and what <code>push</code> answers is a handle rather than an integer id</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/naming-and-shape/#shape-rules" title="Every Core member obeys the same twenty shape rules, R1–R20"><code>core-api/shape-rules</code></a> <a href="/docs/rules/concurrency/the-job-queue/#a-job-between-attempts-is-pending" title="A job between attempts is Pending, and there is no Failed state to ask about"><code>concurrency/a-job-between-attempts-is-pending</code></a> <a href="/docs/rules/concurrency/running-a-job/#cancel-is-a-race-it-can-lose" title="cancel answers whether it won the race, and it changes the job's state rather than deleting the row"><code>concurrency/cancel-is-a-race-it-can-lose</code></a> <a href="/docs/rules/concurrency/running-a-job/#queue-deletion-is-explicit-and-bounded" title="A queue row is removed by delete or purge, never while claimed, and never dead-lettered or pending unless the call says so"><code>concurrency/queue-deletion-is-explicit-and-bounded</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0153.md">record 0153</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/queue-push-answers-an-id-and-not-a-number.nvst"><code>tests/conformance/core/queue-push-answers-an-id-and-not-a-number.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/queue-cancel-and-status-are-asked-the-same-way.nvst"><code>tests/conformance/core/queue-cancel-and-status-are-asked-the-same-way.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/queue-status-is-asked-with-the-receipt-and-not-the-row.nvst"><code>tests/conformance/core/queue-status-is-asked-with-the-receipt-and-not-the-row.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/queue-stats-is-asked-about-a-queue-and-not-a-job.nvst"><code>tests/conformance/core/queue-stats-is-asked-about-a-queue-and-not-a-job.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/queue-stats-answers-a-record-of-counters.nvst"><code>tests/conformance/core/queue-stats-answers-a-record-of-counters.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-tag-groups-jobs-and-a-key-dedupes-them">

## A `tag` names a group of jobs and a `key` admits one pending job, and they are two columns because they are opposites

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#a-tag-groups-jobs-and-a-key-dedupes-them"><code>concurrency/a-tag-groups-jobs-and-a-key-dedupes-them</code></a>
</div>

`push`'s `tag` names a group of jobs and `key` admits at most one pending job, and they are two columns
because they are opposites at the point they touch. A tag exists to name **many** rows — a batch, a
tenant, an upload — so that something can later be said about all of them at once. A key exists to
admit **one**, enforced by the unique index over the released-when-claimed column
([`core-classes/queue-storage-is-a-table`](/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table "The job queue is two tables in a connection the operator names, converged by an explicit command")). Folding the two would cap every group at one pending
job, silently, at the enqueue that created it.

A tag is inert on the request path. Nothing claims on it, nothing dedupes on it, and no statement a
worker runs reads it; it is written by `push`, indexed with its queue, and read by
[`concurrency/queue-deletion-is-explicit-and-bounded`](/docs/rules/concurrency/running-a-job/#queue-deletion-is-explicit-and-bounded "A queue row is removed by delete or purge, never while claimed, and never dead-lettered or pending unless the call says so")'s `purge` alone. That is what keeps it a
column rather than a feature: a deployment that never purges pays one nullable column and one index
write per enqueue for it, and nothing else.

**Grouping is decided at enqueue, not at removal.** A caller that wants a batch deletable tags it when
it creates the batch, because nothing can group rows that were never grouped. The alternative — a
predicate over the payload — is refused: `args` is one JSON document in a text column, so selecting
inside it is a different unindexed dialect on each of [`core-classes/db-one-api`](/docs/rules/core-classes/connecting-to-a-database/#db-one-api "Core\Db is the only database API, and every statement it runs is prepared")'s backends, over
the one table in the runtime that grows without bound.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/the-job-queue/#queue-four-members" title="Core\Queue's four request-path members are each asked about a job or about a queue, never about a row number"><code>concurrency/queue-four-members</code></a> <a href="/docs/rules/concurrency/running-a-job/#queue-deletion-is-explicit-and-bounded" title="A queue row is removed by delete or purge, never while claimed, and never dead-lettered or pending unless the call says so"><code>concurrency/queue-deletion-is-explicit-and-bounded</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table" title="The job queue is two tables in a connection the operator names, converged by an explicit command"><code>core-classes/queue-storage-is-a-table</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0153.md">record 0153</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-job-names-a-file">

## A job names a script file, and its payload crosses as a copied value

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-job-names-a-file"><code>concurrency/a-job-names-a-file</code></a>
</div>

A job names a script file — not a class, not a closure, not a static method. It is the third
construct to take that shape, after `spawn script` and a connection upgrade, and the reason it takes
the *narrowest* of the three is the row: a job's target is stored as data and claimed by any host in
the fleet, possibly after a redeploy, and a string in a table can hold a path but not a method
reference.

A payload crossing into the job is a value **copied**, never a reference, and it is decoded on the
other side into declared types through the derived codecs. That makes a job a compiled unit like any
other file — cached, hot-reloadable, traceable and coverable with no special case
([`concurrency/a-job-runs-as-a-root-isolate`](/docs/rules/concurrency/running-a-job/#a-job-runs-as-a-root-isolate "A job runs as a root isolate, and there is no second execution path")) — and it is why reconstructing an object from a
payload is a question the runtime never has to answer.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A job is a file rather than a class implementing an interface, and nothing about it is a serialized object graph waiting to be reconstructed</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/running-a-job/#a-job-runs-as-a-root-isolate" title="A job runs as a root isolate, and there is no second execution path"><code>concurrency/a-job-runs-as-a-root-isolate</code></a> <a href="/docs/rules/concurrency/running-a-job/#a-payload-refuses-secret-and-keeps-its-qualifiers" title="A durable payload refuses secret, and a tainted value comes back tainted"><code>concurrency/a-payload-refuses-secret-and-keeps-its-qualifiers</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0023.md">record 0023</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0071.md">record 0071</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-jobs-budget-and-grants-are-recorded-at-enqueue">

## A job's budget and grants are recorded when it is enqueued, and narrowed from that context rather than widened

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-jobs-budget-and-grants-are-recorded-at-enqueue"><code>concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue</code></a>
</div>

`push` takes the job's `limits` — the memory, CPU and time its isolate may spend — and its `grants`,
and both are recorded with the row. The grants are **narrowed** from those of the context that
enqueued the job and never widened: a request that could not reach a resource cannot enqueue work
that reaches it either, which is what keeps a queue from being a privilege-escalation seam.

The budget is what makes an overrun a bounded failure rather than a fatal
([`concurrency/a-budget-overrun-is-a-failed-attempt`](/docs/rules/concurrency/the-job-queue/#a-budget-overrun-is-a-failed-attempt "A job that exceeds its budget is a failed attempt, reported as that and retried like any other")), and the grants are asked at the same
capability door every other isolate goes through
([`security/capability-check-at-the-door`](/docs/rules/security/capabilities/#capability-check-at-the-door "The capability check lives inside the function that performs the effect, and that door is the only way out of the process")), with the job's own path as the scope.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Background work carries its own memory, CPU and time budget, and its permissions can only shrink from those of the code that enqueued it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/capabilities/#capability-check-at-the-door" title="The capability check lives inside the function that performs the effect, and that door is the only way out of the process"><code>security/capability-check-at-the-door</code></a> <a href="/docs/rules/concurrency/running-a-job/#a-job-runs-as-a-root-isolate" title="A job runs as a root isolate, and there is no second execution path"><code>concurrency/a-job-runs-as-a-root-isolate</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a></dd></div></dl>

</div>

<div class="nv-rule" id="claiming-is-one-statement">

## A worker claims a job with one statement the database arbitrates, so a fleet needs no lease protocol

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#claiming-is-one-statement"><code>concurrency/claiming-is-one-statement</code></a>
</div>

A worker claims a job with a single statement that selects the oldest due job in its queues and marks
it in the same moment: `for update skip locked` on PostgreSQL and MySQL, `readpast` on SQL Server, and
an immediate transaction on SQLite, whose single-writer model makes contention moot. The database
provides the mutual exclusion, so two instances of a fleet cannot claim the same job, and the runtime
writes no lease protocol, no heartbeat and no coordinator of its own.

**On SQLite the exclusion is that transaction itself, so the claim carries no locking clause and
cannot be given one.** That backend has a single writer: inside an immediate transaction no second
connection is writing at all, so two workers cannot come back with one row — which is what `for
update skip locked` buys on a backend that has row locks and concurrent writers to need them from.
The worker that arrives second waits for the write lock, and by the time it proceeds the row the
first one took is no longer due. `skip locked` is a syntax error there, and the reason not to reach
for one is that there is nothing left for it to do. The transaction has to be an *immediate* one
rather than the deferred transaction a bare `BEGIN` opens: a transaction that reads a row and then
writes it back asks to upgrade a shared lock, and SQLite refuses an upgrade without honouring the
busy timeout, so a deferred claim fails under exactly the concurrency it exists to survive.

A claimed job carries a **visibility timeout**: the claim is keyed on the instant it was taken, and a
worker that dies — or that overruns the window — matches no row when it tries to report, so the job
becomes claimable again. That is what makes delivery at-least-once
([`concurrency/delivery-is-at-least-once`](/docs/rules/concurrency/the-job-queue/#delivery-is-at-least-once "Delivery is at-least-once, and idempotency is the job's own obligation")) and what makes bounded retries a rule rather than
advice.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A fleet needs no supervisor, no heartbeat and no coordinator process — the database provides the mutual exclusion</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/connecting-to-a-database/#db-one-api" title="Core\Db is the only database API, and every statement it runs is prepared"><code>core-classes/db-one-api</code></a> <a href="/docs/rules/concurrency/the-job-queue/#delivery-is-at-least-once" title="Delivery is at-least-once, and idempotency is the job's own obligation"><code>concurrency/delivery-is-at-least-once</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0073.md">record 0073</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0170.md">record 0170</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/queue.rs"><code>crates/nvs-stdlib/tests/queue.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/queue_sqlite.rs"><code>crates/nvs-stdlib/tests/queue_sqlite.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-job-between-attempts-is-pending">

## A job between attempts is `Pending`, and there is no `Failed` state to ask about

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-job-between-attempts-is-pending"><code>concurrency/a-job-between-attempts-is-pending</code></a>
</div>

`Queue\State` is `Pending`, `Claimed`, `Succeeded`, `Dead` and `Cancelled`. There is no `Failed`,
because a failed attempt is retried: a job between attempts is `Pending` with its backoff still to
elapse, and it is indistinguishable from one that has never run — which is correct, since both are
waiting to be claimed.

So "did this job fail" is a question about `Dead`, and the answer is in the dead-letter table
([`concurrency/attempts-are-finite-and-a-dead-letter-is-kept`](/docs/rules/concurrency/the-job-queue/#attempts-are-finite-and-a-dead-letter-is-kept "Attempts are finite, backoff is exponential and jittered, and an exhausted job is kept rather than discarded")) rather than in a state the job
passes through.

The state is a closed integer type ([`enums/closed-integer-type`](/docs/rules/enums/#closed-integer-type "An enum declares a new, closed, named integer type")), not the ordinal the row stores
it as: a program compares against a case rather than a magic number, and a `Queue\State` is not
interchangeable with another `Core` enum that happens to share its ordinals.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no failed state to poll for — a job waiting out its backoff is indistinguishable from one that has not run yet, and &quot;did it fail&quot; is a question about the dead-letter table</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/enums/#closed-integer-type" title="An enum declares a new, closed, named integer type"><code>enums/closed-integer-type</code></a> <a href="/docs/rules/concurrency/the-job-queue/#attempts-are-finite-and-a-dead-letter-is-kept" title="Attempts are finite, backoff is exponential and jittered, and an exhausted job is kept rather than discarded"><code>concurrency/attempts-are-finite-and-a-dead-letter-is-kept</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/queue-state-is-its-own-closed-type.nvst"><code>tests/conformance/core/queue-state-is-its-own-closed-type.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/queue-status-answers-a-state-and-not-a-number.nvst"><code>tests/conformance/core/queue-status-answers-a-state-and-not-a-number.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="delivery-is-at-least-once">

## Delivery is at-least-once, and idempotency is the job's own obligation

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#delivery-is-at-least-once"><code>concurrency/delivery-is-at-least-once</code></a>
</div>

A job may run twice, and that is stated rather than implied. The worker can die after doing the work
and before acknowledging it, or the visibility timeout can expire under load while the attempt is
still in flight; in both cases a second worker will claim the job and run it again.

Idempotency is therefore the application's obligation. `key` gives deduplication at *enqueue* time —
at most one pending job per key — and nothing else in the design pretends to give it at execution
time.

Exactly-once is not on offer, from this queue or from any honest one: systems that claim it are
describing at-least-once plus deduplication, which is what `key` and an idempotent job already are.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The guarantee is stated rather than implied, so a job is written to tolerate running twice instead of assuming it will not</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/the-job-queue/#claiming-is-one-statement" title="A worker claims a job with one statement the database arbitrates, so a fleet needs no lease protocol"><code>concurrency/claiming-is-one-statement</code></a> <a href="/docs/rules/concurrency/the-job-queue/#attempts-are-finite-and-a-dead-letter-is-kept" title="Attempts are finite, backoff is exponential and jittered, and an exhausted job is kept rather than discarded"><code>concurrency/attempts-are-finite-and-a-dead-letter-is-kept</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/queue.rs"><code>crates/nvs-stdlib/tests/queue.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="attempts-are-finite-and-a-dead-letter-is-kept">

## Attempts are finite, backoff is exponential and jittered, and an exhausted job is kept rather than discarded

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#attempts-are-finite-and-a-dead-letter-is-kept"><code>concurrency/attempts-are-finite-and-a-dead-letter-is-kept</code></a>
</div>

A job's attempts are finite with nothing configured, because an unbounded retry is an unbounded wait
wearing a different name. Between attempts the delay grows exponentially, is jittered so a fleet does
not retry in lockstep, and is capped.

A job that exhausts its attempts **moves** to the dead-letter table, carrying its payload, every
attempt's error and its timing. Every attempt's error is there because the jobs row accumulates it:
an `errors` array on the job itself gains one entry — when the attempt started, the class thrown and
its message, capped — as each attempt fails, and the move copies the array rather than writing the
last failure alone (`docs/decisions/0187.md` § 4). Attempts are finite and an entry is capped, so the
array is bounded by construction. The runtime never deletes it. `stats` reports the dead-letter depth
beside the pending and claimed counts, because an unwatched dead-letter table is the classic way a
queue silently loses work and a depth nobody reads is the same as no record at all.

An attempt ceiling of zero is refused at the call rather than accepted: it asks for a job dead-lettered
by the enqueue that created it, and attempts are finite, not optional.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Nothing is discarded silently and nothing retries forever — an exhausted job is a row in a dead-letter table rather than a line in a log</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/the-job-queue/#delivery-is-at-least-once" title="Delivery is at-least-once, and idempotency is the job's own obligation"><code>concurrency/delivery-is-at-least-once</code></a> <a href="/docs/rules/concurrency/the-job-queue/#a-budget-overrun-is-a-failed-attempt" title="A job that exceeds its budget is a failed attempt, reported as that and retried like any other"><code>concurrency/a-budget-overrun-is-a-failed-attempt</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table" title="The job queue is two tables in a connection the operator names, converged by an explicit command"><code>core-classes/queue-storage-is-a-table</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0076.md">record 0076</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0187.md">record 0187</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/queue.rs"><code>crates/nvs-stdlib/src/queue.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/queue.rs"><code>crates/nvs-stdlib/tests/queue.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/queue-push-judges-its-options-before-it-opens-anything.nvst"><code>tests/conformance/core/queue-push-judges-its-options-before-it-opens-anything.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-budget-overrun-is-a-failed-attempt">

## A job that exceeds its budget is a failed attempt, reported as that and retried like any other

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-budget-overrun-is-a-failed-attempt"><code>concurrency/a-budget-overrun-is-a-failed-attempt</code></a>
</div>

A job that exceeds its memory, CPU or time budget has had a **failed attempt**. It is recorded as
that, retried on the same ladder as any other failure
([`concurrency/attempts-are-finite-and-a-dead-letter-is-kept`](/docs/rules/concurrency/the-job-queue/#attempts-are-finite-and-a-dead-letter-is-kept "Attempts are finite, backoff is exponential and jittered, and an exhausted job is kept rather than discarded")), and never reported as an
out-of-memory — the job is the unit torn down, and the worker keeps claiming.

A fatal error inside a job follows the ordinary escalation ladder
([`errors/escalation-ladder`](/docs/rules/errors/the-escalation-ladder/#escalation-ladder "A failure escalates through four tiers, and no tier is retried")) with the job as that unit, so a job whose script does not resolve at
all is a failed attempt too rather than a second policy written beside the first. One shape covers a
throw, a refusal and a budget teardown, which is why a dead-letter row can hold any of the three
without a second entry shape.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Exhausting memory in background work is a bounded failed attempt rather than a fatal that takes the process with it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a> <a href="/docs/rules/concurrency/the-job-queue/#attempts-are-finite-and-a-dead-letter-is-kept" title="Attempts are finite, backoff is exponential and jittered, and an exhausted job is kept rather than discarded"><code>concurrency/attempts-are-finite-and-a-dead-letter-is-kept</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a></dd></div></dl>

</div>
