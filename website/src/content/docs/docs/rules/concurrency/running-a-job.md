---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Running a job"
description: "A job is a root isolate on a recorded budget. Which process runs it is configuration, and there is no pluggable driver."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/concurrency/the-job-queue/
  label: "The job queue"
next:
  link: /docs/rules/concurrency/the-scheduler/
  label: "The scheduler underneath"
---

<p class="nv-section-lead">A job is a root isolate on a recorded budget. Which process runs it is configuration, and there is no pluggable driver.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">7</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">4</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">3</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">7</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#cancel-is-a-race-it-can-lose"><code>cancel</code> answers whether it won the race, and it changes the job's state rather than deleting the row</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-job-runs-as-a-root-isolate">A job runs as a root isolate, and there is no second execution path</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#who-runs-a-job-is-configuration">Which process runs a job is configuration, and every spelling drives the identical isolate</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-payload-refuses-secret-and-keeps-its-qualifiers">A durable payload refuses <code>secret</code>, and a <code>tainted</code> value comes back <code>tainted</code></a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#foreign-connection-enqueue-is-counted">A <code>push</code> on a connection other than the queue's is not transactional, and is counted rather than assumed away</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#queued-work-is-not-scheduled-work">Queued work is durable, retried and declared by the application; scheduled work is a clock tick and none of those</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#no-broker-and-no-driver-interface">There is no broker backend and no pluggable driver, because a driver makes transactional enqueue optional</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="cancel-is-a-race-it-can-lose">

## `cancel` answers whether it won the race, and it changes the job's state rather than deleting the row

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#cancel-is-a-race-it-can-lose"><code>concurrency/cancel-is-a-race-it-can-lose</code></a>
</div>

`cancel` answers a `bool`, and the `bool` says whether *this* call is what took the job out of the
queue. A worker may claim a pending job at any moment, so a late cancel finding the work already
running is the ordinary outcome rather than an unlucky one, and answering `false` is how that is
reported. Throwing would make the commonest race an exception.

Cancelling changes the job's state rather than deleting its row, so a caller that cancels and then
asks `status` is answered `Cancelled` instead of being refused. The receipt still names something
`status` can answer about, which is what keeps the two members usable in the order a program actually
writes them ([`concurrency/queue-four-members`](/docs/rules/concurrency/the-job-queue/#queue-four-members "Core\Queue is four members, and each is asked about a job or about a queue, never about a row number")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Cancelling is a race against a worker rather than a command, and the answer says which side won it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/the-job-queue/#claiming-is-one-statement" title="A worker claims a job with one statement the database arbitrates, so a fleet needs no lease protocol"><code>concurrency/claiming-is-one-statement</code></a> <a href="/docs/rules/concurrency/the-job-queue/#queue-four-members" title="Core\Queue is four members, and each is asked about a job or about a queue, never about a row number"><code>concurrency/queue-four-members</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/queue-cancel-answers-whether-it-won-the-race.nvst"><code>tests/conformance/core/queue-cancel-answers-whether-it-won-the-race.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/queue-a-cancelled-job-is-still-one-status-answers-about.nvst"><code>tests/conformance/core/queue-a-cancelled-job-is-still-one-status-answers-about.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-job-runs-as-a-root-isolate">

## A job runs as a root isolate, and there is no second execution path

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-job-runs-as-a-root-isolate"><code>concurrency/a-job-runs-as-a-root-isolate</code></a>
</div>

A job runs as a **root isolate**: its own arena, its own budget, its own grants, sharing only compiled
code. It is the same isolate a `spawn script` builds, reached through the same door — there is no
second execution path, and no part of the enqueuing request's heap, statics or session is visible
from inside it.

A job holds the compiled unit it started with, exactly as a request and a persistent connection do,
so a redeploy mid-drain does not change what a running job is executing. A queue draining ten jobs off
one script compiles it once.

Output is captured rather than written through, because a job's `echo` landing in the middle of what
the server or the run's own script is writing is exactly the mixing capture exists to prevent.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A job shares nothing with the request that enqueued it — no statics, no session, no superglobals — because it is a fresh isolate rather than the same process continuing</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/the-job-queue/#a-job-names-a-file" title="A job names a script file, and its payload crosses as a copied value"><code>concurrency/a-job-names-a-file</code></a> <a href="/docs/rules/concurrency/the-job-queue/#a-jobs-budget-and-grants-are-recorded-at-enqueue" title="A job's budget and grants are recorded when it is enqueued, and narrowed from that context rather than widened"><code>concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0017.md">record 0017</a></dd></div></dl>

</div>

<div class="nv-rule" id="who-runs-a-job-is-configuration">

## Which process runs a job is configuration, and every spelling drives the identical isolate

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#who-runs-a-job-is-configuration"><code>concurrency/who-runs-a-job-is-configuration</code></a>
</div>

Whether jobs run inside the server process or in a worker of their own is configuration, not a
different mechanism. `[queue] workers` is a count per *instance* and running workers in-process is the
default shape; `workers = 0` makes an instance enqueue-only, which is how a deployment separates the
machines that accept requests from the ones that drain the queue.

Both spellings drive the identical isolate ([`concurrency/a-job-runs-as-a-root-isolate`](/docs/rules/concurrency/running-a-job/#a-job-runs-as-a-root-isolate "A job runs as a root isolate, and there is no second execution path")) over the
identical claim statement ([`concurrency/claiming-is-one-statement`](/docs/rules/concurrency/the-job-queue/#claiming-is-one-statement "A worker claims a job with one statement the database arbitrates, so a fleet needs no lease protocol")), so moving work between them
is an operational decision and never a behavioural one. There is nothing to install beside the runtime
and no supervisor to keep alive.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Workers are the runtime's own and start from configuration — there is no separate daemon or process manager to install beside the server</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table" title="The job queue is two tables in a connection the operator names, converged by an explicit command"><code>core-classes/queue-storage-is-a-table</code></a> <a href="/docs/rules/concurrency/running-a-job/#a-job-runs-as-a-root-isolate" title="A job runs as a root isolate, and there is no second execution path"><code>concurrency/a-job-runs-as-a-root-isolate</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-payload-refuses-secret-and-keeps-its-qualifiers">

## A durable payload refuses `secret`, and a `tainted` value comes back `tainted`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-payload-refuses-secret-and-keeps-its-qualifiers"><code>concurrency/a-payload-refuses-secret-and-keeps-its-qualifiers</code></a>
</div>

A durable row is an output, and an output refuses `secret` ([`security/secret-qualifier`](/docs/rules/security/secrets/#secret-qualifier "secret is a second, independent compile-time qualifier, written before tainted and in that order alone")). So a
`secret` cannot enter a job payload — the enqueue does not compile. A job that needs a credential
reads it from configuration when it runs, which is where credentials live anyway and which keeps them
out of a table, a backup and a replica.

A payload's other qualifiers are recorded with it and restored on decode: a `tainted` value enqueued
comes back `tainted` ([`security/tainted-qualifier`](/docs/rules/security/tainted-data/#tainted-qualifier "tainted is a compile-time qualifier on string and bytes, spellable in any declaration and erased before codegen")), so the analysis survives the round trip
instead of being laundered by a database. The queue's table is trusted exactly as far as the rest of
the application's database is — an attacker who can write to it has already won — which is a better
answer than returning every field `tainted` and training every job to launder reflexively.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A credential cannot enter a job payload at all, and taint survives the round trip through the database instead of being laundered by it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/secrets/#secret-qualifier" title="secret is a second, independent compile-time qualifier, written before tainted and in that order alone"><code>security/secret-qualifier</code></a> <a href="/docs/rules/security/tainted-data/#tainted-qualifier" title="tainted is a compile-time qualifier on string and bytes, spellable in any declaration and erased before codegen"><code>security/tainted-qualifier</code></a> <a href="/docs/rules/concurrency/the-job-queue/#a-job-names-a-file" title="A job names a script file, and its payload crosses as a copied value"><code>concurrency/a-job-names-a-file</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0023.md">record 0023</a></dd></div></dl>

</div>

<div class="nv-rule" id="foreign-connection-enqueue-is-counted">

## A `push` on a connection other than the queue's is not transactional, and is counted rather than assumed away

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#foreign-connection-enqueue-is-counted"><code>concurrency/foreign-connection-enqueue-is-counted</code></a>
</div>

A `push` issued while a transaction is open on a *different* connection than the queue's is not
transactional. The job commits on its own, the business write commits on its own, and there is a
window between them — the failure mode
[`concurrency/enqueue-commits-with-your-write`](/docs/rules/concurrency/the-job-queue/#enqueue-commits-with-your-write "A push on the queue's connection commits with the write that caused it, or neither happens") exists to remove.

Nothing at compile time can see this: the queue's connection name is operator-owned configuration and
the program never writes it, so the checker has no way to compare the two. The runtime therefore
**records** it, and `stats` reports non-transactional enqueues, so a deployment that has quietly lost
the property can find out by reading a counter rather than by losing a job.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Losing atomicity between a write and its job is a measured deployment fact rather than an unstated one</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/the-job-queue/#enqueue-commits-with-your-write" title="A push on the queue's connection commits with the write that caused it, or neither happens"><code>concurrency/enqueue-commits-with-your-write</code></a> <a href="/docs/rules/concurrency/the-job-queue/#queue-four-members" title="Core\Queue is four members, and each is asked about a job or about a queue, never about a row number"><code>concurrency/queue-four-members</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a></dd></div></dl>

</div>

<div class="nv-rule" id="queued-work-is-not-scheduled-work">

## Queued work is durable, retried and declared by the application; scheduled work is a clock tick and none of those

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#queued-work-is-not-scheduled-work"><code>concurrency/queued-work-is-not-scheduled-work</code></a>
</div>

Two things run outside a request, and they are not variants of one mechanism. A `[[schedule]]` entry
is triggered by a clock, declared by the operator in root-owned configuration, carries no data, is not
retried, and a missed tick is simply missed. A queued job is triggered by a program calling `push`,
declared by the application, carries a payload, is durable until it succeeds or dead-letters, and is
retried within bounds.

"Every night at 03:00" is a schedule. "Because this request happened" is a job. A scheduled entry may
of course `push`, and that is the intended way to enqueue a nightly batch's worth of work: the clock
decides when the batch is created and the queue decides how each piece of it is run, retried and
recorded.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Cron-style entries and background jobs are two mechanisms with two guarantees, rather than one worker loop doing both</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/the-job-queue/#delivery-is-at-least-once" title="Delivery is at-least-once, and idempotency is the job's own obligation"><code>concurrency/delivery-is-at-least-once</code></a> <a href="/docs/rules/concurrency/the-job-queue/#attempts-are-finite-and-a-dead-letter-is-kept" title="Attempts are finite, backoff is exponential and jittered, and an exhausted job is kept rather than discarded"><code>concurrency/attempts-are-finite-and-a-dead-letter-is-kept</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0073.md">record 0073</a></dd></div></dl>

</div>

<div class="nv-rule" id="no-broker-and-no-driver-interface">

## There is no broker backend and no pluggable driver, because a driver makes transactional enqueue optional

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#no-broker-and-no-driver-interface"><code>concurrency/no-broker-and-no-driver-interface</code></a>
</div>

There is no Redis, NATS, SQS or AMQP backend, and no pluggable driver interface. A driver interface
would make transactional enqueue a property of *one* driver rather than of the API
([`concurrency/enqueue-commits-with-your-write`](/docs/rules/concurrency/the-job-queue/#enqueue-commits-with-your-write "A push on the queue's connection commits with the write that caused it, or neither happens")), which is the guarantee the whole design exists
to hold, and it would multiply the surface to specify and test across backends nobody has asked for.

The throughput this buys is a database's throughput — thousands of jobs per second, bounded by write
contention on one table. That ceiling is real and it is documented rather than discovered.

The seam that stays open is the storage layer's internal boundary and not a public interface. Someone
genuinely bounded by database write throughput has outgrown what the queue promises, and a
broker-backed queue would have to say plainly which guarantee it drops.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no Redis, SQS or AMQP backend to choose and nothing to configure between the application and its jobs</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/the-job-queue/#enqueue-commits-with-your-write" title="A push on the queue's connection commits with the write that caused it, or neither happens"><code>concurrency/enqueue-commits-with-your-write</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table" title="The job queue is two tables in a connection the operator names, converged by an explicit command"><code>core-classes/queue-storage-is-a-table</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a></dd></div></dl>

</div>
