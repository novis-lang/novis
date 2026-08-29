# Novis — The Web-Native Programming Language: Implementation Plan

<!-- This block has a fixed field set: Status, Done, On disk, Toolchain, ADR slices landed, Open now,
     Blocking. Overwrite a field in place; never add a paragraph or a new field name. That is what
     keeps it bounded as milestones accumulate. Aim for ~400 bytes a field — guidance for you, not a
     check: nothing verifies it, and no session should ever be spent trimming to a number. History
     lives in `git log`, per-crate gaps in each crate's module doc — see AGENTS.md's "Writing docs
     here" section. -->

> **Status:** 2026-08-28. **M4's loop goal is reached** — every check in its acceptance list passes, so
> the language surface is closed and nothing a CLI program reaches for panics below the front end. The
> next target is **the parity program**, orders 1–5 of the milestone table: PHP core feature parity, all
> five SQL drivers, concurrency, governance and the server, run as the six-goal chain in
> [docs/agent/goals/](agent/goals/README.md). Dependencies: `regex` + `fancy-regex` and `jiff` are named
> by the user; the rest the loop picks under ADR 0051 § 4.
>
> **Done:** M0 (setup) and M1 (front end) whole, M2 (HIR, types, IR) and M3 (baseline Cranelift
> backend) whole, **M4 (language completeness) to its loop goal's acceptance list** — its own 1000-case
> corpus figure is the one thing left and it is met through orders 1–4. M4S Part I registered —
> `crates/nvs-stdlib/tests/spec-members-outstanding.txt` holds no keys, which is this project's
> definition of *registered*. M1's own section lists the one grammar addition still owed — the pipeline
> operator, ADR 0098 — which blocks nothing and is scheduled after the current loop goal. Each milestone
> file under [docs/plan/](plan/) states its own acceptance.
>
> **On disk:** the workspace and its CI (three platforms, with miri, asan and fuzz legs), and the
> nine crates — `nvs-diagnostics`, `nvs-syntax`, `nvs-hir`, `nvs-types`, `nvs-ir`, `nvs-runtime`,
> `nvs-stdlib`, `nvs-codegen`, `nvs-cli` — plus `nvs-test`, `fuzz/`, `tools/`, `benches/abi-probe`,
> and the two case trees `tests/conformance` and `tests/differential`. **Each crate's own module doc
> is the authority on what it holds and what it still owes**; `python tools/brief.py` prints one map
> line each, and `python tools/disk.py` the live counts.
>
> **Toolchain:** Rust 1.97.1 stable (pinned), Cranelift 0.135.0, wasmtime 48, MSVC 14.44 + Windows
> SDK 10.0.26100 for linking, PHP 8.5.9 as the differential oracle — on the Windows `PATH` and
> inside the WSL distro alike, at the same version — `cargo-fuzz` 0.13.2 and `valgrind` under a WSL
> nightly toolchain (docs/setup.md is what a machine installs, and why).
>
> **ADR slices landed:** **each ADR's own *Verification* section is the authority on what its slice
> covers, and this field never restates one** — `python tools/brief.py --where <keyword>` routes to
> the ADR that owns a topic, and `python tools/adr.py --stats` shapes the whole set. What a crate
> still owes is its own module doc's `# Known gaps`. What landed in which session is in `git log`.
>
> **Open now:** **The parity program is live, and goal 1's first item list — ADR 0061 § 3's
> enumeration — runs end to end.** Its six goals are written out under
> [docs/agent/goals/](agent/goals/README.md), each with its own `[context]` manifest and acceptance
> TOML; `python tools/loop.py --chain docs/agent/goals/chain.toml` walks them, carrying each goal's
> whole acceptance list forward as the next one's floor through `goal-switch.py`. The switch from
> M4's goal has been made and the chain has finished goal 1 and is running goal 2, whose Stage 2
> keystone is on disk: `crates/nvs-host` is a real crate — one pinned thread, one `!Send` scheduler,
> a run queue of stackful coroutines, and a `Ctx` that carries the yielder as an opaque pointer so
> no signature in the chain is coloured. ADR 0115 specifies the reactor above it, and
> `crates/nvs-host/src/reactor.rs` is now that reactor: `mio` readiness keyed by `TaskId`, § 2's
> five-rule parking contract, and `run_until_idle` joining it to the run queue. How a task reaches
> that reactor is now decided and recorded in `reactor.rs`'s module doc — a thread-local, not a
> second opaque pointer in `Ctx` — and `crates/nvs-host/src/net.rs` is ADR 0115 § 3's stream over
> it: `NvsTcp` is a plain `std::io::Read`/`Write` that issues the syscall first and registers, parks
> and loops only on `WouldBlock`, keeps its registration across parks, and waits on a poll of its
> own when there is no core to hand back. `NvsTcp::connect` parks the same way and then asks
> `SO_ERROR` and a zero-length write whether that readiness *meant* success — a refused connection
> is writable too, and `peer_addr`, which `mio`'s own example uses, answers `Ok(the target)` on
> Windows for a socket that never connected. Deadlines are on the same reactor:
> `crates/nvs-host/src/timer.rs` is Stage 2 item 5, one deadline per task, armed by
> `sleep`/`park_until` and enforced as the timeout of the very poll `Reactor::turn` was about to
> make, so a sleep and a timeout are one mechanism and not two clocks. That mechanism now bounds the
> stream as well: an `NvsTcp` carries an optional deadline, a park files it with the core's timers
> beside the reactor registration, off a core it is the blocking poll's own timeout, and what ends
> the wait is the clock and not the wake — `io::ErrorKind::TimedOut` through the ordinary
> `Read`/`Write` return, since that is the only error channel those traits have.
> `NvsTcp::connect_timeout` is ADR 0074 § 5's `connect_timeout` over it, bounding the handshake and
> lifting the bound before the stream is handed back. Item 4 is closed with the Unix-domain sibling,
> and it is the *same* type rather than a second one: the stream is `NvsStream<S>` generic over
> whatever the reactor can register, `NvsTcp` and `NvsUnix` are type aliases over it, and what stays
> per family turned out to be only the address. `net.rs`'s module doc § *One type over the source,
> not one type per socket family* is that decision's one home. Windows compiles none of the
> `#[cfg(unix)]` half, so the crate is 53 tests under WSL against 49 here. Closing it turned up a
> latent path to `abort()` — a suspended coroutine dropped under `run_task`'s containment boundary —
> which `nvs_runtime::Teardown` now closes. Stage 2 item 6 is closed as well, in two halves. A core
> can now be woken from a thread that is not it: `Reactor::remote_wake` hands out a `Send`, one-shot
> `RemoteWake` that queues a `TaskId` and pokes `mio`'s own waker, `WAKE_TOKEN` is the single token
> in that reactor which is not a task id, and the count of outstanding handles is the third thing
> `Reactor::turn`'s "can anything still wake this core" test asks about — without it a task waiting
> on the pool, which holds neither a registration nor a deadline, would be abandoned rather than
> waited for. `crates/nvs-host/src/blocking.rs` is the pool over it: bounded at twice the core count
> per ADR 0106 § 6, threads started only when work arrives so a worker that never blocks holds none,
> and `blocking::run` is the whole handoff — take the wake, hand the closure to this thread's pool,
> park, and take the answer out of a slot on the way back, with a panic in the work carried back to
> the task's own stack rather than lost off the core. Its `Drop` deliberately does not join: the
> pool lives in a thread-local, and joining from a TLS destructor deadlocks on Windows. Stage 2's
> implementation is complete: `crates/nvs-host/src/watchdog.rs` is item 7 and ADR 0106 § 7 — one
> thread for the process, started with the first core that registers, reading the earliest deadline
> each core publishes and reporting one that has been behind its own clock by a margin. Nothing is
> written for it on any path, which is the item's own constraint: `Timers` publishes its first entry
> into an `AtomicU64` beside the ordered index it already keeps, and because `take_due` drops a
> deadline the moment the core polls after it, a published deadline still a margin in the past is a
> statement about the *core* and not about a slow request. A stall is reported once per deadline
> rather than once per sweep, since a wedged core republishes nothing — ADR 0106 § 10's coalescing
> arriving for free. Firing writes ADR 0020 § 4's floor and does nothing else; the shed half of § 7
> is admission control's and does not exist in this crate yet. What is left of the stage is its
> *acceptance*, and that is now closed: every `nvs-host` test the Stage 2 `cargo-named` checks name
> is on disk, over behaviour that was already landed — the parking pair and the neighbour that keeps
> running (`net.rs`), one core per scheduler with no task ever migrating and the non-atomic refcount
> that buys (`scheduler.rs`), a sleep and a socket timeout in one table (`timer.rs`), and a blocking
> call reaching a pool bounded at twice the core count (`blocking.rs`). Writing the last of those
> found a real abandoned-task bug and closed it: `Reactor::turn` returned `0` — which
> `run_until_idle` reads as *idle* — whenever a poll woke nothing, and a drain that takes several
> pokes' ids at once leaves the surplus pokes to come back ready over an empty queue, so a core with
> the pool saturated abandoned two thirds of its parked tasks. The retry test is now the same test
> the blocking state is entered on: a turn that would block for a reason may not then report that
> reason gone. The last of those names is `a_rustls_session_streams_over_it_unmodified`, and it is
> the one that cost something: a real `rustls` session — handshake, verified self-signed
> certificate, application record — runs over an `NvsTcp` with nothing in it naming this crate, and
> the peer holds the answer's **last byte** back so a park lands strictly inside a record rather
> than tidily between two, which is the half a stream reporting a short read as a complete one would
> fail. `rustls` and `rcgen` are `[dev-dependencies]` of `nvs-host` alone, on the `ring` provider
> rather than the C `aws-lc-rs` default; the workspace `Cargo.toml`'s comment above them is the one
> home of why a test binary is where ground-rules.md's pure-Rust default is spent rather than met,
> and `tools/gen-attribution.py` walks normal and build dependencies only, so no Novis artifact
> links either crate and no notice is owed. What a task *costs* is settled too:
> `crates/nvs-host/src/stack.rs` is ADR 0115 § 4 — 1 MiB of reserved address space per task,
> resident only in the pages its handler touched, pooled per worker and recycled the moment the task
> ends, with the recursion limit armed from that stack instead of asserted from a ceiling on the
> worker's. Stage 3's task tree is on disk beneath all of that. Every task carries a parent and the
> children under it, and the parent is **taken from the task that spawned it** rather than passed in
> — `nvs_host::spawn_child` reads `current_task`, so ADR 0072 § 1's "each is a child of the calling
> task" is a property of the call and not of a caller's diligence. A running task cannot reach the
> `&mut Scheduler` that is resuming it, so the tree is the half of a scheduler a task *can* reach:
> an `Rc<RefCell<TaskTree>>` published in a thread-local for the length of `Scheduler::run`, holding
> the id counter, the parent links, the cancel flags and the children a task asked for but the
> scheduler has not built yet. Cancellation marks and the scheduler tears down: `Scheduler::cancel`
> and `cancel_task` flag a task and every descendant and do nothing else, and the forced unwind runs
> on the scheduler's own stack — before a marked task is resumed, or over the parked set once the
> run queue drains — because unwinding a coroutine from a frame standing on that coroutine's stack
> is not a thing to arrange. A marked task therefore dies at its next safepoint when it is running
> and at once when it is already parked, which is ADR 0072 § 5's rule from both directions, and what
> the unwind runs is native `Drop` and no script code. A task's death cancels whatever it left
> running, so § 4's "control does not leave the call with work still running" holds one level below
> the member that will promise it, and a cancelled task hands back its id rather than a `Finished` —
> its `Ctx` went down with its stack. `scheduler.rs`'s module doc § *The task tree, and what
> cancelling one costs* is that design's only home; no ADR slot was free for it. All five names
> Stage 3's `cargo-named` check asks of `nvs-host` are green, because item 11's channel is on disk:
> `crates/nvs-host/src/channel.rs` is a bounded queue between two tasks on one core, whose `send`
> suspends at the bound instead of growing the queue and whose `recv` suspends on empty. What it
> needed from the scheduler was a wake a *running* task can issue, and `nvs_host::Wake` is that — a
> handle taken while the waiting task is running, which queues a `TaskId` on the task tree and is
> drained into the run queue by `Scheduler::run`, after the resume that filled it or at the start of
> the next turn. It holds the tree rather than reading the thread-local when it fires, which is what
> lets an end of a channel held outside any task wake the task waiting on it and what stops two
> schedulers on one thread crossing wakes, since a `TaskId` is unique only inside its own tree. A
> registration is an RAII guard on the waiting task's own stack, so a cancelled task takes itself
> off the waiter queue under the forced unwind and a `send`'s single wake is never spent on a task
> that can never be resumed — which is what makes waking one waiter rather than all of them correct.
> `channel.rs`'s module doc is that design's only home, as no ADR slot was free. Item 11 is closed
> at its language surface too, and the acceptance program that names it — `examples/channel.nvs` —
> prints `produced=8`, `consumed=8` and `sender suspended`. `Core\Task\Channel<T>` is a registered,
> constructible, iterable `Core` class with two members, `send` and `close`, because every other
> question a program could ask one (`count`, `isFull`) has an answer that is stale before the caller
> reads it. Three decisions are behind it and `crates/nvs-stdlib/src/channel.rs`'s module doc is
> their one home. The **queue is Novis values in the instance's own slots** rather than a handle
> into a table the host keeps: a `Core` instance has no native drop, so nothing would ever tell a
> host-side table that the last reference to a channel had gone, and its footprint would be
> O(channels created) for the life of a worker — a leak rather than a trade-off. Keeping it in slots
> costs the duplication of about thirty lines of "wait when full, wait when empty" against
> `nvs_host::channel` and buys the whole lifetime question, releasing the object releasing every
> value still queued and charging it to the request that made it. The **object is its own
> iterator**, because a `foreach` over a channel may not take a snapshot — the values are not there
> yet — so `iterate()` answers with the receiver itself and `advance()` is where a consumer waits,
> ending the walk the moment the channel is empty *and* closed. And a state change **wakes every
> waiter on the channel rather than one**: `nvs-host`'s own channel wakes one and says why, but that
> argument is about many producers, and here the list is almost always length one and a wrong pick
> is a hang rather than a slow path. What all of that needed from the seam is a park a `Core` member
> can reach, so `nvs_runtime::host::Host` gains `waker()` and `park()` beside `run_group` and
> `sleep`. A `Waker` is a one-shot boxed closure because the host's own wake is a task id plus the
> tree it belongs to and neither is a thing `nvs-runtime` can spell; it is a hint by contract, so
> every member that parks re-checks the state it parked for. The two are separate methods rather
> than one so that a member learns there is **no task beneath it** before it commits to waiting — a
> park with nothing under it can only block the core, which ADR 0106 § 6 forbids outright, and
> `Core\Task\Channel::send` reports that rather than parking. `SchedulerHost` implements both over
> `Wake::current` and `suspend_current(Waiting::Parked)`, and a cancelled waiter takes its own
> registration off the list on the way out, so a wake is never held for a task that cannot be
> resumed. 93 tests in the crate on Windows, up from 83. Stage 4 has now begun from the other end,
> in the compiler rather than the host: `Core\Task` is a registered class and `Core\Task::all`
> type-checks, which is ADR 0072 § 1's heterogeneous typing and the three `nvs-types` names the
> stage's `cargo-named` check asks for. It needed a **second binding site** beside
> `CoreTy::CallableTo`, and that is the whole design: `CoreTy::CallableShapeTo("S")` says "this
> parameter is a shape literal whose every field is a written `fn` literal, and the shape of *their*
> results binds `S`", because the argument's own type is a shape of opaque `callable`s and can
> therefore say nothing about what any of them returns — which is exactly the
> `array<mixed>`-plus-a-cast that § 1 exists to avoid. `nvs_types::expr::args`'
> `bind_callable_shape` is the one place a field is read and the one place `E0773` (the argument is
> not written out) and `E0774` (a field is not an `fn` literal) are reported; `Ty::CallableShapeTo`
> then substitutes to `mixed` rather than to a type, because that position is checked in full there
> and a second assignability pass could only repeat the same mistake. § 3's `{limit?: uint,
> deadline?: Duration}` is the row's options bag, both options defaulting to `Const::Null` because
> neither type has an "unbounded" value in it, and there is no `timeout` member and no `race`. § 2's
> `map` is the second row, and what it cost is the section's own argument for two members rather
> than one: `map(array<T>, callable, {limit?, deadline?}): array<U>` needed no new machinery at all
> — an ordinary `CoreTy::CallableTo` binds `U` from the one callback the whole call shares, so the
> row is the three-line shape `Core\Arr::map` already had plus § 3's options bag, while `all` needed
> a whole second binding site because a shape literal carries a different closure in every field.
> Stage 4 now has **both bodies**, and `crates/nvs-stdlib/src/task.rs` is three steps each: build
> one `nvs_runtime::host::Job` per shape field or per array element, hand the group and § 3's
> `Bounds` to the host, and put the answers back under the field names `all` promised or the keys
> and order `map` promised. **The route to one is now decided and on disk**:
> `crates/nvs-runtime/src/host.rs` is the seam a `Core` member reaches its host through — a
> `&'static dyn Host` published in a thread-local — and its module doc is that decision's one home,
> `scheduler.rs` carrying a pointer to it rather than a copy. Three things were decided there. The
> edge is **inverted through `nvs-runtime`** rather than added between `nvs-stdlib` and `nvs-host`,
> because `nvs-types` and `nvs-codegen` both depend on the signature registry and an edge from there
> to the host would link `mio`, `corosensei` and `core_affinity` into `nvs check` — a type checker
> carrying a reactor to answer a question about a signature. It is a **thread-local rather than a
> second opaque pointer in `Ctx`**, for `reactor.rs`'s reason narrowed to the case where the caller
> does hold a context: a host is per *core* while a `Ctx` is per *request*, so a field there is one
> copy of the core's identity per in-flight request, and `Ctx` is `#[repr(C)]` with offsets compiled
> code loads inline, which makes it an ABI change rather than a struct change. And **what crosses is
> a whole group, not a task API** — one `Host::run_group` taking the jobs and § 3's bounds and
> answering with ADR 0072 § 4's own table as `Outcome` — rather than a `spawn`/`wait`/`cancel` the
> member sequences, because § 4's "control does not leave the call with work still running" is a
> property of the *sequence*, and a seam handing out task ids makes keeping it the caller's
> diligence again, which is the exact failure `spawn_child` already refuses on the parent link.
> `Outcome` has three variants and not five: § 4's last row is not a return at all, since a
> cancelled caller is unwound *through* the call rather than out of it. The host end of that seam is
> now filled in. `crates/nvs-host/src/group.rs` is the one implementor: `SchedulerHost` is a unit
> struct — every scrap of a scheduler's state is already a thread-local of that crate, so a host
> carrying a pointer to any of it would be a second, staler route — and `Scheduler::run` installs it
> beside the task tree's own guard, so a `Core` member can reach a host exactly when it can reach a
> tree to put children in. Its module doc holds the two decisions only an implementor can make. The
> first is **what a child gets for a `Ctx`**, which `spawn_child` forces to be an owned one while §
> 1's children share the request: `Ctx::child` builds a fresh context that **aliases the request's
> static-property base** and owns everything else, because compiled code loads a static inline
> through that word and a child with a store of its own would give one request two copies of every
> static — which is exactly what the acceptance program's `limit` block measures a peak through. The
> origin, the debug flags, the error class and the deadline word are copied; the output buffer and
> the assertion ledger are the child's own and are spliced back **in job order** when the group
> ends, so a child's `echo` cannot make a `Task::map`'s output depend on which child finished first.
> The alias obliges the parent to outlive the child, and that is discharged twice: the call does not
> return while a child is still running, and a parent torn down first cancels its children, which
> the scheduler unwinds without resuming. The second decision is the **sequence**, which is the
> whole of § 4: start what the `limit` allows, park, cancel every sibling on the first throw or on
> the deadline, and **keep parking until the count reaches zero** — a per-child guard whose `Drop`
> counts it out and wakes the parent, rather than a line at the end of a body a cancelled child
> never reaches. `Outcome::Threw` carries a `Thrown` rather than a `Value` now, since that is what
> `Ctx::take_thrown` produces and what `Ctx::raise` takes, and a `Job` may be dropped without ever
> being called — the doc on that type is the one home of the release obligation that puts on whoever
> builds one. Eight tests pin it, 101 in the crate against 93. The other end is filled in too:
> `nvs-cli` depends on `nvs-host`, and `nvs run` runs its program **inside a task** — one
> `Scheduler`, a reactor installed over it, `TaskRoot::Request` as the root, and the `Ctx` read back
> out of `take_finished()` rather than stayed borrowed — so a `Core\Task::all` in a CLI program has
> a calling task to be a child of, which is what ADR 0072 § 1 makes a property of the call rather
> than of the caller. A job's captures are deliberately **borrowed**: the closure a field holds and
> the element `map` passes belong to the member's own arguments, which outlive the group, so the
> release obligation a dropped `Job` carries falls only on `map`'s rendered `$key` — held as a Rust
> `String` so that dropping the job releases it. `examples/tasks.nvs` prints all four of its frozen
> lines — `all=3`, `map=1,4,9,16`, `deadline hit` and `limit held at 2` — because `Core\Time::sleep`
> parks its task rather than blocking the thread, so two children under a `limit` of 2 genuinely
> overlap and the peak the program gauges is the one § 3 describes. What that took was not the
> member: it was that **a cancelled task standing on script frames is resumed rather than unwound**.
> A forced unwind is a panic and an Novis stack is not one it may cross — a `Core` member's entry
> point is `extern "C"` and the compiled frames beneath it carry no unwind tables — so
> `nvs_runtime::HelperFrame` counts a helper frame for as long as one runs, `nvs_host`'s `yield_on`
> reads that count at each suspension, and a cancelled task whose stack is not clear of them is
> handed `Resume::Cancelled` in place of what it was waiting for. The member it is inside answers
> with `Ctx::cancel`, which sets `SafepointFlags::CANCEL` and returns `nvs_safepoint`'s own status,
> so the task dies by ADR 0002's return status with no `catch` and no cleanup on the way out — ADR
> 0072 § 5 reached the only way that stack allows. The notice is delivered **once**, so a park site
> that ignores it is left parked rather than spinning the sweep, and `Drop for Scheduler` leaks such
> a task rather than aborting on it, which is ADR 0106's no-`abort()` rule bought for one stack
> mapping at the death of a worker. § 4's last row is now a variant rather than an unwind through
> the call: `Outcome::Cancelled` is what a group whose *caller* was cancelled returns, and both
> `Core\Task` members turn it into the same `Ctx::cancel`. `nvs_host::group::Child::run` asks
> `Ctx::cancelled()` before it reads a pending message, because a child that stopped by status
> leaves one behind and it is not a throw. Stage 4's acceptance is closed. Stage 5 is on disk from
> the walk outwards. `crates/nvs-runtime/src/graph.rs` is ADR 0023 § 2's graph copy written
> **once**, with a private `Carrier` trait over it and two implementors — `Live`, which builds
> values into the destination, and `Encode`, which appends § 3's closed format — so a rule added to
> the walk reaches both carriers or neither and there is no third place to forget. Three decisions
> are its module doc's, and each is the answer to a question the ADR states behaviourally.
> **Identity is object identity and nothing else**: a string is immutable and an array is
> copy-on-write, so sharing is unobservable in both and a cycle cannot be built through either,
> which leaves the one heap shape with reference semantics as the one that needs an entry in the
> identity map and a back-reference in the format. **A move at refcount 1 is decided per node, not
> at the root**, because a uniquely-owned root can hold a child something else still holds, and
> adopting that child would hand the other side mutable state its source can still see; the walk
> therefore consumes one reference to every value and asks the carrier whether it may adopt the
> allocation. And **an object's declared property names are written before any of their values**,
> which is what lets a decode make § 3's whole mismatch check before it builds anything at all — the
> refusal is `Slot` declares one property where the payload records two, not a half-filled instance.
> `Core\Serialize` is the member surface over it (`crates/nvs-stdlib/src/serialize.rs`):
> `encode(mixed): bytes` and `decode(bytes): mixed`, the second parameter carrying `Qual::Sink` so
> ADR 0023 § 3's tainted-sink rule is a compile-time refusal with no launderer. The two members
> classify one `GraphError` differently on purpose — a payload that is not this format is a
> `ParseError`, and a value the program built that cannot cross is a `LogicError`. Reaching a
> program's own class from a `Core` member needed one new seam, `Ctx::class_desc`, which reads the
> table `set_runtime_error_class` already installed rather than registering a second one.
> `examples/serialize.nvs` prints all four of its frozen lines, and its stale-shape golden is a real
> payload now rather than the placeholder that stood there: the same class name with a second `int
> $extra` beside `$n`, written in the format `graph.rs` emits. Stage 5's checker half is on disk as
> well, and both of the stage's `nvs-types` names are green. `Core\Serialize::decode`'s `Qual::Sink`
> row needed nothing added to enforce it: a classified `CoreTy::Text`/`Blob` lowers to exactly what
> its unclassified spelling lowers to, so a `tainted bytes` argument is refused by ordinary
> assignability and `serialize_decode_refuses_a_tainted_operand` is a test over machinery that was
> already there. ADR 0033 § 4's cross-boundary sink did need a check, and it is
> `nvs_types::expr::quals::reject_secret_boundary_argument` under a new `E0775`. It is a call-site
> rule rather than a parameter type for the reason `Core\Debug::dump`'s already is — `encode`
> declares `mixed`, which a `secret string` satisfies, so the written argument is the last place the
> qualifier is visible — and it is **one** function for both of ADR 0023 § 2's carriers, which is
> how § 4 states the rule: `spawn`/`spawn worker`/`spawn script` reach it rather than growing a
> second one when they lower. The half a call site cannot see is the walk's own: a `secret`-typed
> *property* of a copied object is `graph.rs`'s `field_is_secret` refusal at run time, because an
> argument's static type is the class and not its storage. `Core\Secret::reveal()`, which ADR 0033
> names as the way out and every one of these diagnostics' help text points at, is not in the
> registry yet — it needs a parameter spelling that *accepts* a qualifier and a `Qual::Launder` no
> consumer reads — so item 18's escape hatch is open at both ends. What is left of Stage 5 is
> joining the live carrier to the `spawn` boundary, and Stage 6 has now begun building that
> boundary. ADR 0116 is the goal's second pre-authorized slot, spent: **an isolate's arena is an
> ownership root, not an address range.** Entering one maps nothing and leaving one unmaps nothing —
> allocation keeps going through the process allocator, and what an isolate owns is reachability,
> enforced at the single place a value can move. The region arena the word implies was rejected on
> three facts the tree already holds: freeing a region runs no native drop, so a
> `Core\Db\Transaction` would stay open where ADR 0072 § 5 requires it rolled back; a bump region
> cannot reuse a freed intermediate, so a loop appending to one string would make an isolate's
> footprint O(work done) rather than O(live), which ADR 0004 calls a leak; and the latency a region
> is usually reached for was already collected by `alloc.rs`'s per-thread cache. "Released
> wholesale" is therefore one drain of `crate::release`'s worklist over the isolate's roots —
> iterative, bounded by what is live, running every native teardown — and the same drain a cancelled
> isolate gets from the scheduler's own stack. The payoff for the crossing is § 5's: because both
> sides allocate from the same place, `Live`'s move at refcount 1 is a pointer handoff rather than a
> copy, so a child returning a large array costs the walk and not the array.
> `crates/nvs-host/src/isolate.rs` is that ADR in code, and it is design.md's one `Isolate` type:
> the argument crosses in through `copy_graph`, the program runs as a child task on a stack of its
> own, the answer crosses out **before** the context is dropped, and dropping it is the wholesale
> release. Two decisions are its module doc's. A **program arrives as a closure, not as a path**,
> for the reason `nvs_runtime::host`'s seam already records — resolving a path means the compiler,
> and an edge from the host to `nvs-ir` would link a scheduler, a reactor and a JIT into every `nvs
> check` — so whoever can compile builds one, `nvs-cli` today and the server at M7. And the two
> refusals of the one walk **mean different things**: an argument that cannot cross was built by the
> parent before any child existed, so it is an `Err` with no task started and the caller raises it
> in the parent, while an answer that cannot cross was built by the child and is ADR 0006's
> failure-is-a-value, `ok = false` beside an uncaught throw. What it needed from the runtime is
> `Ctx::isolate`, the other half of the pair `Ctx::child` opens and the one place the two part: a
> task aliases the request's static-property base, an isolate gets its own, and that single word is
> the whole of "a child cannot read or write a parent static". It is safe where `Ctx::child` is
> `unsafe`, because nothing in the returned context points into the parent. Four of Stage 6's eight
> `cargo-named` names are green on it, plus two the module's own asymmetry needs; 116 tests in the
> crate against 110. The other half of the pair — where the code an isolate runs *comes from* — is
> now on disk as well. `crates/nvs-runtime/src/script.rs` is the second seam this crate inverts,
> beside `host.rs`'s: a `Resolver` trait declared where both sides already depend on it, published
> in a thread-local, and one `resolve(path)` answering with a `Program` or with `ResolveError`'s two
> variants. It is a seam of its own rather than a third `Host` method because the two have different
> subjects and different lifetimes — a host is per core and its subject is a task, a resolver is per
> program and its subject is code, and `nvs check` has a resolver's whole toolchain with no
> scheduler while a bare worker thread has the reverse. `nvs_host::Program` is now a re-export of
> that seam's type rather than a second declaration of the same closure, so the argument-transfer
> contract has one home. `crates/nvs-cli/src/script.rs` is the one implementor: the front end,
> `lower_program` and `nvs_codegen::compile` `nvs run` already carries, behind a cache keyed by the
> path **as written**, leaked once per process so that ADR 0006's "an isolate shares immutable
> compiled code" is a property of that cache and of nothing else. Its module doc owns the two
> decisions only an implementor can make — a relative path is anchored at the **working directory**
> rather than at the entry file, which is what `examples/isolate.nvs` is already written against and
> what `require`'s opposite rule (ADR 0021, resolved against the requiring file, because a library
> moves as a unit) is the contrast for; and one unit per written path, at a footprint of O(the
> program's text) rather than O(isolates spawned). Where the transferred argument *lands* is decided
> too: `Ctx::set_isolate_argument` is the isolate's own ownership root holding it, released when
> that context is dropped, which is ADR 0116 § 2's wholesale release reaching the one value a
> program is handed and has nowhere else to put. The route is proved end to end without any of the
> language surface — `examples/isolate/hello.nvs` resolves, runs inside a real `Isolate` and hands
> back `child said hello`, and `throws.nvs` arrives as `ok = false` carrying `RuntimeError` rather
> than as an `Err`. What is left of item 20 is the language surface, and the first of its three
> constructs has landed: `await` is a prefix expression. `ExprKind::Await` is in the AST,
> `crates/nvs-syntax/src/parser/expr.rs:2055` builds it, every walker that matches `ExprKind`
> carries the arm, and `nvs-types` refuses it under a new `E0776` beside `spawn script`'s `E0703` —
> two codes and not one because the two constructs arrive separately and a program writing only one
> should hear about the one it wrote. It is **contextual**, for `crates/nvs-syntax/src/token.rs`'s
> own reason, and disambiguated the way `spawn` is: by what follows it rather than by reserving the
> spelling, so a program using `await` as a constant, a function or a method name still parses. The
> one reading it claims from such a program is `await($x)`, which is the operator over a
> parenthesised operand rather than a call to a function of that name — the trade being that
> excluding `(` would make `await ($handle)` an unknown-function error at the one place a developer
> is most likely to reach for parentheses. `docs/spec/00-overview.md` § 2 carries the production and
> that rule. `examples/isolate.nvs` now parses whole and reports only `E0703` and `E0776`, where it
> used to stop at `E0319` on the `await` line. What `ScriptResult` *is* is now decided and recorded
> in the module doc that owns it: `crates/nvs-types/src/expr/isolate.rs`, which is also where both
> of ADR 0006's constructs are checked from as of this session. **The handle is a registered `Core`
> class and the result is an ADR 0036 shape**, and one fact decides both — a `Core` instance has no
> property a program can reach (`crates/nvs-stdlib/src/registry.rs:775`). That is exactly what a
> handle *is*, since `await` is the only thing a program may do with one, and exactly what the
> result may not be, since `$result->ok` is read on the next line; a shape needs no new machinery at
> all, where a class would need a property resolution in `nvs-types`, a slot read in `nvs-ir` and a
> per-slot type the registry has no spelling for. `await` already answers with `{ok: bool, value:
> mixed, output: string, error: ?{class: string, message: string}}` — one field per member of
> `nvs_host::Completion`, four `nvs-types` tests over it — while still reporting `E0776`, because
> the refusal is what keeps `nvs-ir`'s roster comment true and that roster ends in a `panic!`: a
> construct may not stop being refused before it starts being lowered. The one surface cost is that
> ADR 0006's `$result->valueOrThrow()` cannot be a method on a shape and becomes
> `Core\Script::valueOrThrow($result)`, a static member on the class item 22 introduces anyway. Both
> constructs now carry their type. `spawn script` answers with `Core\Script\Handle`, which
> `crates/nvs-stdlib/src/script.rs` registers with **no members at all** — no `cancel()`, no
> `isDone()`, no `id()`, because cancellation is the parent's own teardown reaching its children
> (ADR 0072 § 5) and every "is it finished yet" answer is stale before the caller reads it, which is
> the argument `Core\Task\Channel` already records for `count`/`isFull`. It is absent from
> `CONSTRUCTORS` too, so `new Core\Script\Handle()` is refused where it is written; the only thing
> that may build one is the lowering of a spawn. It declares no slots either, and that is not an
> oversight — a slot is the layout a helper body reads back by index, so it is declared by whoever
> writes one, and what a handle holds is item 22's to decide. `await` is now checked **against**
> that class, so `await $n` over an `int` is an ordinary `E_TYPE_MISMATCH` naming what it wanted
> rather than only the refusal: nothing but a spawn produces a handle, so an operand that is not one
> cannot have come from one. The class owed `docs/spec/01-core-library.md` nothing, contrary to what
> the item predicted — `spec_registry_coverage.rs` reads §§ 1-12's *table rows* and asks the
> registry for each, so a memberless class is invisible to it, and a row there would demand the very
> member this class must not have. The concurrency language surface belongs to
> `docs/spec/00-overview.md` § 2 by `01-core-library.md` § 19's own routing line, and that is where
> both types are now named and pointed at their module doc. Both refusals stay, for the roster
> reason above. The steps the chain took are in [goals/README.md](agent/goals/README.md) § *Starting
> the chain*. M4's own residue is the 1000-case corpus count, which orders 1–4 meet as the suite
> grows; nothing else about M4 is open. What the program is measured by is `python
> tools/check-migration.py` at 100% classified, which stood at 25% the day the program was scheduled
> and reads 34% now that goal 1's own five domains — dates and times, regular expressions, JSON,
> URLs and paths — carry a row per name. `python tools/gaps.py`, `python tools/holes.py` and `python
> tools/check-migration.py --report` are the three worklists behind it, and no session re-derives
> one.
>
> **Blocking:** Nothing waiting on a decision — every design call orders 1–5 reach is pre-authorized in
> the goal's own § *Standing decisions*, and each goal names the numbered ADRs it may open and no
> others. One external dependency: **goal 5 needs a reachable Docker daemon**, because ADR 0067's
> driver matrix runs MySQL, MariaDB, PostgreSQL and SQL Server as real servers; the driver preflights
> it and stops the run naming it rather than grinding against a check that cannot pass. Picking every
> dependency but the two the user named is pre-authorized under ADR 0051 § 4.

**How the plan relates to the ADRs.** The plan is the record of *what* gets built, in what order, and how
each milestone is verified. It states decisions but does not argue them. The reasoning lives in
[docs/adr/](adr/README.md), and where a decision has its own ADR the plan links to it instead of restating
it — follow the link rather than expecting the argument here. For decisions with no ADR of their own, the
*why* is in [adr/README.md](adr/README.md) under *Decisions taken at project start* and the *mechanics*
are [docs/plan/design.md](plan/design.md) § *Architecture*.

## Milestones

Each milestone ends with something runnable and its own tests. Do not start the next until the current
one's verification passes.

**A milestone's number is its identity, not its position.** The **Order** column is the schedule and it is
the only thing that says what comes next; M15 has carried that distinction since it was written, and as of
2026-08-28 it applies to the whole table. Nothing is renumbered when the order changes, because a number
that moves invalidates ~650 cross-references in `docs/adr/` and every one of them is a link somebody has
already followed.

| Order | Milestone | What it builds | Loop-days |
|---|---|---|---|
| done | [M0](plan/m0.md) | Project setup (~3 days) | 0.3 |
| done | [M1](plan/m1.md) | Front end (~3 weeks) | 0.7 |
| done | [M2](plan/m2.md) | HIR, types, IR (~4 weeks) | 1.5 |
| done | [M3](plan/m3.md) | Baseline Cranelift backend → **Hello World** (~3 weeks) | 0.5 |
| done\* | [M4](plan/m4.md) | Language completeness — a usable CLI language (~10 weeks) | ~3 |
| **1** | [M4S](plan/m4s.md) | The `Core` API contract and its pure half (~5 weeks) | ~1.5 |
| **2** | [M5](plan/m5.md) | Concurrency and script isolates (~5 weeks) | ~3.5 |
| **3** | [M6](plan/m6.md) | Config, limits, capabilities, disk cache (~3 weeks) | ~1 |
| **4** | [M8](plan/m8.md) | Stdlib and databases (~16 weeks) | ~6.5 |
| **5** | [M7](plan/m7.md) | Built-in HTTP server (~4 weeks) | ~2 |
| **6** | [M4B](plan/m4b.md) | Minimal `nvs-lsp`, syntax highlighting and the VS Code extension (~3 weeks) | ~1.5 |
| **7** | [M9](plan/m9.md) | Extension system, and the `nvs:ext@1.0.0` world it freezes (~6 weeks) | ~2.5 |
| **8** | [M10](plan/m10.md) | Developer tooling and IDE integration (~14 weeks; scope shifted by ADR 0040, net change undetermined) | ~8 |
| **9** | [M11](plan/m11.md) | PHP transpiler (~10 weeks) | ~3 |
| **10** | [M15](plan/m15.md) | Packages, the registry and the supply chain (~8 weeks) | ~3 + a calendar floor |
| **11** | [M16](plan/m16.md) | `nvs/web`, `nvs new`, and the framework (~12 weeks) | ~4 |
| ongoing | [M12](plan/m12.md) | Optimising JIT tier (ongoing) | measurement-bound |

\* **M4 reached its loop goal on 2026-08-28** — every check in that goal's acceptance list passes, which is
what closes the language holes. What it has not reached is its own milestone acceptance's **1000 `.nvst`
cases**; that count was deliberately left as a corpus figure to be met as the suite grows through orders
1–4, and [m4.md](plan/m4.md) still carries it unchanged.

**Orders 1–5 are one program, not five independent milestones: PHP core feature parity.** Everything a
program written in PHP reaches for without loading an extension, plus every planned SQL driver, plus the
concurrency, governance and server the capability-bearing half of `Core` cannot exist without. It is
scheduled as one continuous unattended run — see *The parity program* below, and
[docs/agent/goals/README.md](agent/goals/README.md) for the six loop goals it is cut into. PHP's optional
extensions (`gd`, `intl`, `imap`, and the rest of the list in
[02-php-migration.md](spec/02-php-migration.md)) are explicitly not part of it and stay with M9.

Each row is a file under [docs/plan/](plan/). `python tools/plan.py --show M8` prints one
without you needing to know that, and `--show M8:verify` prints only its acceptance paragraph.
The decisions those milestones sit inside, the architecture and the verification strategy are
[docs/plan/design.md](plan/design.md).

## The parity program

Orders 1–5, in that order, are the run that takes Novis from "a usable CLI language" to "everything PHP
does out of the box, and the four databases it does it against". The order inside the program is a
dependency chain rather than a preference: `Core`'s pure half is what everything else is written against;
the reactor is what a socket, a driver and a listener all need; capabilities are what every
capability-bearing member is gated on; the capability-bearing half of `Core` and the databases sit on both;
and the server sits on all of them.

| Loop goal | Milestone | Lands |
|---|---|---|
| [1 core-depth](agent/goals/1-core-depth.md) | M4S tail | `Core` §§ 1–13 depth, `autoload`, the compile-time attribute passes, OpenAPI |
| [2 concurrency](agent/goals/2-concurrency.md) | M5 | the reactor and its parking streams, the scheduler, `spawn`/`await`, `Core\Task`, isolates, `Core\Serialize` |
| [3 governance](agent/goals/3-governance.md) | M6 | the config tree, capability enforcement, limits, the artifact cache, `nvs build --compile` |
| [4 core-part-ii](agent/goals/4-core-part-ii.md) | M8, non-database | `Core\IO`, crypto, `Process`, `Cli`, `Cache`, `RateLimit`, `Log`, `Http\Client`, `Reflect` |
| [5 database](agent/goals/5-database.md) | M8, database | `Core\Db`, five drivers, the pool, the type map, `Core\Queue` |
| [6 server](agent/goals/6-server.md) | M7 | `nvs serve`, the request-facing `Core` classes, mounts, uploads, `Core\Session`, the control socket |

**The program's own stop condition is `python tools/check-migration.py` reporting 100% classified** —
every one of the oracle build's **1151 functions and 253 types** accounted for as a `member`, `language`
or `dropped` row, every `member` row's member registered, and every one of them carrying a conformance
case. It was 25% when the program was scheduled. A count of conformance cases is a proxy for parity; a
table that enumerates the source of truth is not.

**The oracle build gained `mysqli`, `pgsql` and `sqlite3` on 2026-08-29**, which is why that inventory is
1151 rather than the 925 the program was first sized against. It is a better program for it: 236 of the
new names are the three APIs [ADR 0067](adr/0067-core-db.md) exists to replace, so *"one API replaces
`PDO`, `mysqli`, `pgsql` and `sqlite3`"* stops being an assertion about four APIs and becomes an audit of
236 functions, each with a row saying what became of it. The floors in each goal were re-derived against
the new denominator in the same commit.

**The two columns are not the same unit.** The parenthesised weeks are the original estimate, written for
a human team before any code existed; **Loop-days** is what this project's unattended loop actually spends,
elapsed and continuous — M0–M3 are measured, the rest projected. The measured conversion is ~23×, it is
not applied uniformly, and it carries a rework tax and four ways it breaks:
[docs/plan/velocity.md](plan/velocity.md) is the one home for all of that. Summed, the milestones left
come to **~7 weeks** against the ~99 the original column still shows. Neither figure gates anything.
