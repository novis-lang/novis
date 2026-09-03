# ADR 0138 — a connection future is driven by the coroutine that owns it, and a waker is one wake

- **Status:** Accepted
- **Date:** 2026-09-03
- **Scope:** how a `Future` is driven inside a runtime that is not `async` — the poll loop, what a `Waker`
  is allowed to do, which core re-polls, and what a cancellation ends. It does **not** decide what is
  driven: the listener, the mount table and the response policy are
  [0097](0097-development-server-and-proxied-origin.md)'s, and the parking contract underneath every wait
  here is [0115](0115-the-reactor-reports-readiness-and-a-stream-that-would-block-parks.md) § 2's.
- **Depends on:** [0115](0115-the-reactor-reports-readiness-and-a-stream-that-would-block-parks.md), whose
  reactor issues the one permission this seam carries across a thread boundary.
- **Validated by:** `crates/nvs-host/src/block_on.rs`

> **In short:** a connection is one `Future`, and the coroutine that accepted it drives that future to
> completion in a loop of *clear the flag, poll, park* — `nvs_host::block_on`. A `Waker` here is not a
> scheduling primitive: it is a permission to make **one more poll happen**, delivered as a `RemoteWake`
> against the parked task's own core. Nothing is spawned, nothing is queued and nothing migrates, which is
> the whole of why this is a seam and not a second scheduler.

## Context

`docs/plan/design.md` § *Thread-per-core, shared-nothing runtime* decided that this runtime is ours and is
not `async`, and [0106](0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md) § 6 decided that a
core is never blocked on a syscall. Both hold today: a socket read is a plain `std::io::Read` that suspends
its coroutine, and no function in the chain is coloured.

The HTTP/1 implementation is `hyper`, under [0097](0097-development-server-and-proxied-origin.md) § 1, and
`hyper` is `Future`-shaped: `serve_connection` returns a future, and its I/O traits are `poll_read` and
`poll_write`. It needs no executor — h1 spawns nothing, so no `hyper::rt::Executor` is ever installed — but
something has to *poll* that one future until it is done, and something has to answer the `Waker` it is
handed.

So exactly one place in this tree speaks `Future`, and this ADR is that place. The risk it exists to close
is not that a poll loop is hard to write; it is that a poll loop grown carelessly becomes an executor —
a ready queue, a spawn, work stealing — and then there are two schedulers on one core, each with its own
idea of what may touch a `!Send` arena.

## Decision

### 1. One future per connection, driven by the coroutine that owns it

`nvs_host::block_on` drives a single future on the stack of the task that called it, and its whole body is
the loop: clear the wake flag, poll, and if the answer is `Pending`, park the task. The coroutine already
exists — one per accepted connection — so what this adds is a loop, not a runtime.

Three properties follow, and together they are what "not an executor" means here:

- **There is no queue of futures and no spawn.** A future that wants concurrency asks the *scheduler* for a
  task, exactly as `Core\Task` does; it does not get one from this loop.
- **A future is polled only on the stack that owns it.** Everything `!Send` beneath it — the `Ctx`, the
  arena, a non-atomic refcount — stays as sound as it was before the seam existed.
- **`Pending` suspends the task and not the thread.** The core goes back to its run queue and serves its
  other connections. A `block_on` that blocked the thread would be precisely the thing
  [0106](0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md) § 6 forbids.

The name is the honest one: it blocks *the task*, in the same sense every socket read in
`crates/nvs-host/src/net.rs` already does.

### 2. A `Waker` is a permission to make one more poll happen

[0115](0115-the-reactor-reports-readiness-and-a-stream-that-would-block-parks.md) § 2's rule 2 is that a
wake decides nothing, and this seam is that rule applied to a type `std::task` defines. Waking sets an
atomic flag and, if a permission is installed, delivers it. It does not decide the future is ready, does
not re-poll, and does not touch the run queue itself. **The re-poll belongs to the loop**, and the loop
clears the flag *before* it polls, so a wake fired from inside a poll is observed by the check after it.

That ordering is the whole of the lost-wakeup argument, and it covers the two cases that would otherwise be
bugs:

- **A wake between the poll and the park** finds no park to end, and is the flag alone. The loop re-reads
  the flag after arming its permission and before it suspends, so it goes round again instead of sleeping
  on a wake that has already happened.
- **A waker cloned and held past the call** — `hyper` keeps one for an upgrade — fires into an empty slot
  and sets a flag nobody reads. Inert, which is the reactor's own "a wake is a hint that may have raced
  with the task ending".

### 3. The permission is one `RemoteWake`, re-issued only after it fires

A permission is taken per **call**, not per park, and the reason is the reactor's outstanding count.
`Reactor::remote_wake` raises that count when it issues a handle and lowers it only when the id the handle
queued is drained on the core — which means **every handle issued is a wake delivered**, by `wake` or by
the drop that stands in for it. A handle taken freshly for each park would therefore poke the core once per
readiness edge with a wake nobody asked for.

So the loop installs one permission and leaves it installed across parks; the waker **takes** it out of the
slot when it fires; the next park finds the slot empty and issues a fresh one. A connection whose every
wait ends in socket readiness — the ordinary one, since the reactor wakes the task by id and never through
the waker — issues exactly one permission for the whole connection and delivers it when `block_on` returns.

### 4. A wake from another core wakes the task; it never moves it

This is the question the seam has to answer sharply, because `std::task::Waker` promises `Send + Sync` to
whoever holds it while nothing else in `nvs-host` is either.

What crosses the thread boundary is a `RemoteWake`: a `TaskId` and a poke of one core's poller, and nothing
else — no reference to a scheduler, a stack or an arena. Firing it from any thread queues that id on
**that** core and ends the poll that core is asleep in; the scheduler there moves the task from parked to
ready and resumes it. The re-poll therefore happens on the core that parked, on the coroutine's own stack,
under the same thread-local reactor.

**A task never migrates** — `crates/nvs-host/src/lib.rs`'s corollary to the one rule that crate exists to
hold — so "the future woke on a different core" is not a state this design has. The only object that is
ever on two cores is the permission, and it carries an id.

It is also why the waker's payload is an `Arc` of a flag and a slot rather than the scheduler's own `Wake`,
which is `Rc`-based and deliberately `!Send`: a `Waker` built over that would break `std`'s contract at the
first clone `hyper` makes.

### 5. A cancellation ends the drive, in whichever of the scheduler's two ways applies

The drive adds nothing to the cancellation model; it inherits it, and both of the scheduler's answers end
the loop.

**The ordinary one is a teardown, and `block_on` never returns from it.** A parked coroutine whose stack may
be unwound is dropped where it stands, and the unwind drops the future with it — closing the connection
through Rust's own drops, running no Novis frame, which is
[0072](0072-core-task-structured-concurrency.md) § 5's rule kept by construction.

**The other is a resume.** A stack standing on a helper frame cannot be unwound, so the scheduler hands it
`Resumed::Cancelled` — once — on the way back in. There the loop stops and `block_on` answers `None`. The
two alternatives are both worse in the way the ADRs above already name: parking again would wait on a wake
that is not coming, which is a wedged coroutine rather than a slow one, and ignoring the answer would go on
serving a request the client is no longer attached to.

### 6. Off a core it blocks the thread, which is the same not-bent rule the stream states

Three routes, decided once when the drive starts, because a task never migrates and so the answer cannot
change underneath it:

| Where the drive is standing | What a park is | What a wake is |
|---|---|---|
| a core with a reactor — the server | `Waiting::Parked` | the installed `RemoteWake` |
| a core with no reactor — a scheduler-only test | `Waiting::Yielded` | nothing to arrange; the loop re-polls |
| no core at all — `nvs run`, a unit test | `std::thread::park` | `Thread::unpark` |

The middle row makes progress by spinning through the run queue, which is deliberate: there is nothing on
that thread to arrange a wake *with*, and a park nothing can end is a wedge. The bottom row is `net.rs`'s
§ *Off a core, it blocks* for the same reason it gives — there is no coroutine to suspend and no neighbour
to starve, so blocking the thread starves nobody.

## Consequences

- **What it spends**, per [0004](0004-memory-for-simplicity.md): one `Arc` holding a flag and a mutex slot,
  one `Waker`, at most one `RemoteWake`, and the future itself on the coroutine's own stack — per
  connection, O(in-flight). Nothing is allocated per poll and nothing per park.
- **Latency**: a park costs what every park in `nvs-host` costs. The one charge this design adds over a
  hand-written state machine is the poke delivered when the last permission is dropped — **one per
  `block_on` call**, not one per wait.
- A `Future` in this tree is a *consumer* of the runtime and never a producer: nothing in `crates/` grows an
  `async fn`, and the `Waker` is the only `Send` type any of it touches.
- The day something asks for h2, it does not get an executor from here. That is
  [0097](0097-development-server-and-proxied-origin.md) § 1's decision to reopen, not this one's.

## Alternatives rejected

- **An executor: a per-core ready queue of futures.** Two schedulers on one core, each with its own answer
  to which stack may touch the arena, and a work-stealing question that thread-per-core spent its whole
  design avoiding. Everything it would buy — concurrency inside one connection — the scheduler already
  offers as a task.
- **A `poll_read` that suspends the coroutine instead of returning `Pending`.** Tempting, because then
  there is no waker at all. It deadlocks: `hyper` drives the read and write halves of one connection in a
  single future, so a read that suspends until the peer sends more holds back the response that would have
  made it send more. It survives only for exchanges small enough to fit a socket buffer, which is to say it
  passes the first test anyone writes.
- **Waking through `scheduler::Wake`.** It is the natural same-core route and it is `!Send` by design; a
  `Waker` built over it would be sound exactly until a clone crossed a thread, and unsound with no
  diagnostic afterwards.
- **A `RemoteWake` per park.** Costs a delivered poke per readiness edge, for the reason § 3 gives.
- **`futures::executor::block_on`.** A third-party drive that parks the *thread*: right off a core and
  precisely wrong on one. It would also still need this crate's `RemoteWake` to be reachable from its
  waker, so the dependency buys none of the sixty lines it would replace.

## Verification

`crates/nvs-host/src/block_on.rs`'s tests, one per claim above: a ready future is polled once and never
parks; a future that wakes itself inside its own poll is re-polled without a park (§ 2's ordering); a task
on a core parks and is driven to completion by a wake fired from another thread, with the core asleep in
its poll (§ 4); the permission count returns to zero when the drive ends and no second permission is issued
while the first is still installed (§ 3); a cancelled task is torn down where it parked and the unwind
drops its future, while one standing on a helper frame answers `None` rather than parking again (§ 5).
