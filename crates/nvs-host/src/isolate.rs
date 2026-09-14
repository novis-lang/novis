//! `rule:security/isolate-shares-nothing`'s isolate, as one type: a child task with a heap boundary.
//!
//! `rule:security/isolate-shares-nothing` fixes what an
//! isolate *is* — it shares immutable compiled code and its parent's budget and
//! nothing else, values cross by copy or by move at refcount 1, and a child's
//! failure arrives as data rather than as an unwind.
//! `rule:security/arena-is-an-ownership-root`
//! decides how, and this module is that decision in code: an arena is an
//! ownership root, so building one is [`Ctx::isolate`] and releasing one
//! wholesale is dropping that context.
//!
//! `docs/plan/design.md` § *In-process isolated script execution* asks for
//! **one** `Isolate` type serving both the `spawn script` path and, at M7, an
//! inbound HTTP request. This is that type; what differs between the two is the
//! [`Program`] handed in and the [`Output`] asked for, not the boundary.
//!
//! `rule:concurrency/a-connection-is-a-root-isolate`'s
//! connection is another caller and needs no boundary of its own: it is
//! [`Isolate::start`] from the *connection's* context, with a [`Program`] the
//! upgrading request prepared and handed over before it ended, so the
//! connection is that request's sibling rather than a child of its tree.
//! `nvs-stdlib`'s `socket` module owns that decision and why the preparation
//! cannot happen on this side. What each of
//! `rule:concurrency/two-doors-one-isolate`'s doors adds here is one builder
//! for the thing it was handed — [`Isolate::over_socket`] a peer,
//! [`Isolate::over_event_stream`] a body — and nothing else differs between
//! them.
//!
//! # The program arrives as a closure, not as a path
//!
//! An isolate runs another `.nvs` file, and turning a path into runnable code
//! is [`nvs_runtime::script`]'s seam rather than anything this module can do —
//! that module owns the decision and the reason. What matters here is only the
//! consequence: a [`Program`] is a boxed closure over an already-prepared unit,
//! exactly as [`nvs_runtime::host::Job`] is for a task. Both crossings, the
//! failure classification and the whole context construction live here
//! regardless, because not one of them depends on where the code came from.
//!
//! # Decision: a refused argument is the parent's fault, a refused answer is the child's
//!
//! One walk refuses at both crossings
//! (`rule:classes/graph-copy`), and the two refusals mean different things:
//!
//! * An **argument** that cannot cross was built by the parent, before any child
//!   existed. [`Isolate::run`] answers `Err`, no task is started, and the caller
//!   raises it in the parent — which is where the offending value's name and its
//!   path in the graph mean something.
//! * An **answer** that cannot cross was built by the child. That is `rule:security/isolate-shares-nothing`'s
//!   *failure is a value*: it lands as `ok = false` carrying the walk's own
//!   message, beside an uncaught throw and a limit breach, and the parent keeps
//!   running.
//!
//! A **cancellation** reaches that same `ok = false` by two different routes,
//! and which one a child takes is a fact about its stack rather than about the
//! boundary. A child that is parked, or running with nothing of `nvs-stdlib` on
//! its stack, is force-unwound by the scheduler and never reaches [`finish`]:
//! its slot is unfiled and [`cancelled_completion`] is what the join answers
//! with. A child standing on a [`nvs_runtime::HelperFrame`] may not be unwound
//! there, so it is *told*, answers with [`Ctx::cancel`] and returns — [`finish`]
//! sees [`Ctx::cancelled`] and classifies it, which is the route that keeps
//! what the child echoed before the safepoint. Both are `rule:concurrency/cancellation-runs-no-user-code`, and
//! neither runs a line of the child's own code on the way out.
//!
//! § 2's *unresolvable class* is asked of the answer and not of the argument,
//! and the asymmetry is a known gap rather than a decision. The rule needs the
//! receiving side's class table: at the join the parent's is in hand
//! ([`Ctx::class_table`], taken at the spawn because by then the parent's
//! context is borrowed by the frame parked on the join), while at the spawn
//! the child's does not exist yet — its program's prologue installs it. Closing
//! it means the [`nvs_runtime::script`] seam answering with a unit's table
//! beside its entry point, which is a change to that seam and not to this walk.
//! What the rule *is* — the same class, by descriptor identity, not a class of
//! the same name — is `nvs_runtime::graph`'s `Live::admit`, its one home.
//!
//! # What it spends
//!
//! `rule:security/isolate-budget-is-the-trees` is the one home of the accounting. Per **in-flight** isolate:
//! one [`Ctx`], one pooled task stack, and the values the child allocates.
//! Nothing here is O(isolates created) — a finished isolate's context is dropped
//! as its task ends, and that drop is the wholesale release.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

use nvs_runtime::graph::{GraphError, copy_graph, copy_graph_into};
use nvs_runtime::{
    Ctx, ErrorClass, EventStreamDoor, Fault, Inbound, Limit, OpenSpawn, OutputSink, PeerSocket,
    SpawnForm, SseSlot, TaskRoot, UpgradeSlot, Value,
};

use crate::scheduler::{TaskId, Waiting, Wake, cancel_task, spawn_child, suspend_current};
use crate::watchdog::Registration;

pub use nvs_runtime::script::Program;

// The shapes the crossing is described in are declared on the seam itself, in
// `nvs_runtime::host`, because that is where a `Core` member reaches them from
// and a name written out in two crates is a name that can drift. They
// are re-exported here because this module is where they *mean* something: the
// seam fixes the shape, and everything below decides the behaviour.
pub use nvs_runtime::host::{Completion, Failure, Output, Running};

/// One isolate: a program, the argument crossing into it, and where its output
/// goes.
pub struct Isolate {
    program: Program,
    args: Value,
    output: Output,
    charge: Charge,
    entry: Form,
    /// Boxed, and not for the size of this struct alone: [`Ctx::set_inbound`]
    /// boxes a carrier anyway, so allocating it here hands the same allocation
    /// on rather than moving a wide struct twice. What it also buys is
    /// that `nvs_server::Reply` — an enum with this type in one variant and a
    /// response in the other — stays a value a handler can return without one
    /// arm dwarfing the other.
    inbound: Option<Box<Inbound>>,
    peer: Option<Box<dyn PeerSocket>>,
    /// The writing half of the response this isolate's events are the body of —
    /// [`Isolate::over_event_stream`], and `None` for every isolate that is not
    /// a connection answering one.
    event_stream: Option<nvs_runtime::stream::Emit>,
    /// Where this isolate publishes itself while it runs, so that a thread
    /// which is not this core can charge it against `rule:errors/on-limit`'s
    /// CPU ceiling — [`Isolate::watched_by`], and `None` for every isolate
    /// nobody handed a registration to.
    watch: Option<Rc<Registration>>,
}

/// Which of [ADR 0006](/docs/decisions/0006.md)
/// § *Decision*'s two entry forms built the program this isolate holds.
///
/// It changes exactly one thing, which is why it is a tag here rather than the
/// name the seam carried: which constructor builds the child's context. A path
/// entry has a unit of its own, whose `install_in` arms the child's statics
/// from inside the program; a method entry has none — its code is the parent's
/// unit's — so its context is armed at construction, which is
/// [`Ctx::method_isolate`]. The name itself is
/// [`nvs_runtime::host::Entry`]'s, and it has already become a [`Program`] by
/// the time an isolate exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Form {
    /// A `.nvs` file of its own, which is every isolate but the one below.
    Path,
    /// A `static` method of the parent's unit —
    /// [`Isolate::running_a_method_of_the_parents_unit`].
    Method,
}

/// Whose budget an isolate spends: `rule:security/isolate-shares-nothing`'s answer, and `rule:errors/handler-script`'s one
/// named exception to it.
///
/// Private, and a builder rather than a parameter of [`Isolate::new`], because
/// § 3 states that the exception is not a precedent. Nothing a program writes
/// can reach it — `nvs_runtime::host`'s seam is what `spawn script` arrives
/// through, and it never calls the builder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Charge {
    /// The tree it was spawned from, which is every isolate a program spawns.
    Tree,
    /// The engine's own reserve, which is [`crate::ladder`]'s handler and
    /// nothing else.
    EngineReserve,
}

impl std::fmt::Debug for Isolate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Hand-written: a `Program` is a closure and has no `Debug`.
        f.debug_struct("Isolate")
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

impl Isolate {
    /// Builds one. **Consumes one reference to `args`**, which [`Isolate::run`]
    /// hands to the walk; dropping an `Isolate` without running it leaks that
    /// reference, the same obligation [`nvs_runtime::host::Job`] documents for a
    /// job that is never called.
    #[must_use]
    pub fn new(program: Program, args: Value, output: Output) -> Self {
        Self {
            program,
            args,
            output,
            charge: Charge::Tree,
            entry: Form::Path,
            inbound: None,
            peer: None,
            event_stream: None,
            watch: None,
        }
    }

    /// Publishes this isolate to `watch` for as long as it runs, so the
    /// watchdog can charge it against its CPU ceiling and stop it at one.
    ///
    /// A builder for [`Self::running_a_method_of_the_parents_unit`]'s reason:
    /// an isolate a server takes up on a watched core is the caller that has a
    /// registration to hand over, and every other one in this tree — a `spawn
    /// script` child, a test's program, the engine's own handler — would
    /// otherwise pass a `None` to say it has none.
    ///
    /// **What is published is the tree, and the registration crosses rather
    /// than the publication because the tree is not in hand here.** It is the
    /// context [`Self::start`] is called on: that context's safepoint handle is
    /// the root's, its `Ctx::cpu_limit` is the ceiling the whole tree divides
    /// (`rule:security/isolate-budget-is-the-trees`), and a caller that
    /// published at this line would be publishing whatever context it built the
    /// isolate from. A tree under no cap publishes nothing —
    /// [`Registration::publish`] clears the slot instead of filling it.
    ///
    /// The publication is cleared when this isolate's body ends, however it
    /// ends. One slot per registered thread is the whole of what a registration
    /// holds, so a thread running several isolates at once publishes the one it
    /// took up last and clears the slot at the first of them to finish.
    #[must_use]
    pub fn watched_by(mut self, watch: Rc<Registration>) -> Self {
        self.watch = Some(watch);
        self
    }

    /// Says the program is a `static` method of the *parent's* unit rather than
    /// a file of its own — [`Form`], and ADR 0006 § *Decision*'s second form.
    ///
    /// A builder rather than a parameter of [`Isolate::new`] for [`Charge`]'s
    /// reason inverted: every other caller of this type is a path, so the one
    /// that is not says so at its own call site and no existing one is rewritten
    /// to pass a default. It changes exactly one thing — which constructor
    /// builds the child's context below — and [`Ctx::method_isolate`] owns what
    /// that difference is.
    #[must_use]
    pub fn running_a_method_of_the_parents_unit(mut self) -> Self {
        self.entry = Form::Method;
        self
    }

    /// Gives it the request it is answering, which the child's own context then
    /// carries for `Core\Request`'s members to read
    /// ([`Ctx::set_inbound`](nvs_runtime::Ctx::set_inbound)).
    ///
    /// **The request rides on the isolate rather than on the parent's context**,
    /// because the context that runs the application is the one [`Isolate::start`]
    /// builds at the spawn: the accept loop's own context never runs a line of a
    /// program, so a request installed there would sit where nothing reads it.
    ///
    /// That is also why a `spawn script` child answers no request — nothing calls
    /// this for one, and
    /// `rule:security/isolate-shares-nothing`'s isolate
    /// shares nothing but compiled code, so `Core\Request::method()` inside one
    /// throws exactly as it does in a CLI program. Passing the request down
    /// automatically would be ambient authority crossing the boundary that exists
    /// to stop it; a child that needs a header is handed it as an argument.
    ///
    /// **What it spends:** the carrier is moved, not copied — `nvs_runtime::Inbound`
    /// owns the per-request accounting — and it is released with the child's
    /// context.
    #[must_use]
    pub fn answering(mut self, inbound: Inbound) -> Self {
        self.inbound = Some(Box::new(inbound));
        self
    }

    /// Offers `rule:concurrency/a-connection-is-a-root-isolate`
    /// 's upgrade slot to the request this isolate answers, so that
    /// `Core\Socket::upgrade` inside it has somewhere to leave the connection
    /// isolate it prepared.
    ///
    /// It goes through the isolate rather than onto the carrier directly because
    /// of who holds what: the door that knows whether this connection *can* be
    /// upgraded (`nvs_server::serve_connection`) never sees the
    /// [`Inbound`] — the handler it asks for a reply is what builds one — and
    /// this is the one place the two are in the same hand. Nothing else about
    /// the isolate changes, and in particular a connection's own isolate is
    /// built by [`Isolate::new`] with no builder of its own: what it runs is a
    /// [`nvs_runtime::Upgrade`]'s two halves, which are already this type's
    /// first two arguments.
    ///
    /// **A no-op for an isolate answering no request**, which is the fail-closed
    /// direction and the honest one: a `spawn script` child has no request, so
    /// there is nothing an upgrade of its connection would mean.
    #[must_use]
    pub fn offering_upgrade(mut self, slot: UpgradeSlot) -> Self {
        if let Some(inbound) = self.inbound.as_mut() {
            inbound.offer_upgrade(slot);
        }
        self
    }

    /// Offers `rule:concurrency/two-doors-one-isolate`
    /// 's SSE cell to the request this isolate answers, so that
    /// `Core\Sse::upgrade` inside it has somewhere to leave the connection
    /// isolate it prepared.
    ///
    /// Everything [`Self::offering_upgrade`] says about *why it goes through
    /// the isolate* holds here unchanged. What differs is who is offered one:
    /// **every request a server answers gets this cell**, because an event
    /// stream takes nothing of the connection but the response the request
    /// already has, where § 1's slot is offered only where an upgrade was
    /// framed. `nvs_runtime::Inbound::offer_sse` is the one home of that
    /// difference.
    ///
    /// **Still a no-op for an isolate answering no request**, which is the same
    /// fail-closed direction: a `spawn script` child has no response for an
    /// event stream to be written into.
    #[must_use]
    pub fn offering_sse(mut self, cell: SseSlot) -> Self {
        if let Some(inbound) = self.inbound.as_mut() {
            inbound.offer_sse(cell);
        }
        self
    }

    /// Offers `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`
    /// 's request-scoped cell to the request this isolate answers, so that
    /// `Core\Response::stream` inside it has somewhere to leave the head the
    /// connection is to write.
    ///
    /// Offered to **every** request a server answers, exactly as
    /// [`Self::offering_sse`]'s is and for a sharper version of the same
    /// reason: this stream takes nothing of the connection at all. It is the
    /// response the request already has, written in pieces instead of at once,
    /// and the connection reads this cell while the request's own future is
    /// still running rather than after it has ended.
    ///
    /// **Still a no-op for an isolate answering no request**, and that is what
    /// makes the member inert off a connection rather than broken: a CLI
    /// program, a `spawn script` child and a `#[Test]` method each open a
    /// stream that writes to their own output, there being no response to frame.
    #[must_use]
    pub fn offering_response_stream(mut self, cell: nvs_runtime::stream::BodySlot) -> Self {
        if let Some(inbound) = self.inbound.as_mut() {
            inbound.offer_response_stream(cell);
        }
        self
    }

    /// Moves `rule:concurrency/a-connection-is-a-root-isolate`
    /// 's socket into the isolate this builds.
    ///
    /// **The opposite direction from the two above**, and that is the whole of
    /// what separates a connection isolate from the request that asked for one.
    /// Those offer a *request* somewhere to leave an upgrade it prepared; this
    /// hands the isolate the socket the server then framed, so it is called on
    /// the connection's own isolate and never on a request's.
    ///
    /// It is a builder and not an argument of [`Isolate::new`] for
    /// [`Self::running_a_method_of_the_parents_unit`]'s reason: every other
    /// isolate in this tree has
    /// no peer, and a parameter would make every one of those call sites pass a
    /// `None` to say so. What it changes is one field of the child's context —
    /// [`Ctx::set_peer`] owns what that field means and when it may be read.
    ///
    /// **It is called after the upgrade was framed and never before.** § 1's
    /// socket does not exist until the `101` is on the wire, so an isolate
    /// started with the program in hand and the peer still to come would run
    /// its first `receive()` against nothing; `nvs_server::serve_connection`'s
    /// own docs are the home of that ordering.
    #[must_use]
    pub fn over_socket(mut self, peer: Box<dyn PeerSocket>) -> Self {
        self.peer = Some(peer);
        self
    }

    /// Moves the body of a `200 text/event-stream` into the isolate that writes
    /// it — `rule:concurrency/two-doors-one-isolate`'s other hand-over, on the
    /// same terms as [`Self::over_socket`].
    ///
    /// It is the same direction and the same moment: the connection has already
    /// built the half the bytes go through, and hands it to the isolate that is
    /// about to own it. What differs is what was handed over, and that is the
    /// whole of what separates the two doors — an event stream takes no socket,
    /// so this isolate has a body and no peer where § 1's has a peer and no
    /// body.
    ///
    /// **It marks as well as hands over**, because the two are one fact about
    /// this isolate rather than two: [`Ctx::mark_event_stream`] is what
    /// `Core\Sse::current` reads, and a context holding a writing half it was
    /// not told the meaning of is one where that member would answer for an
    /// ordinary streamed body too. `nvs_stdlib::sse`'s `current` is the home of
    /// why the question is neither the peer nor an open body.
    ///
    /// The door it marks is [`EventStreamDoor::Connection`], and that is what
    /// admits this isolate to `Core\Topic`: the stream outlives the request
    /// that opened it, so there is a wait here for a delivery to be drained on
    /// — which is the whole of what a subscription needs and the whole of what
    /// a streaming response lacks.
    #[must_use]
    pub fn over_event_stream(mut self, events: nvs_runtime::stream::Emit) -> Self {
        self.event_stream = Some(events);
        self
    }

    /// Charges it to `rule:errors/handler-script`'s engine-owned reserve instead of to the tree
    /// that spawned it.
    ///
    /// [`Ctx::handler_isolate`] is the one home of what that changes and why
    /// § 3 wants it. Here it changes two things and no more: the context this
    /// isolate runs under, and the tree-depth refusal below, which is asked of
    /// the tree only.
    #[must_use]
    pub fn charged_to_the_engine_reserve(mut self) -> Self {
        self.charge = Charge::EngineReserve;
        self
    }

    /// Runs it to completion and answers with what crossed back.
    ///
    /// The sequence is `rule:security/isolate-teardown-is-a-drain-then-a-sweep` and `rule:security/isolate-values-cross-by-copy`: copy the argument in, build the
    /// isolate's own ownership root, run it as a child task, copy the answer out
    /// **before** that root is released, and release it. Control does not return
    /// while the child is still running, exactly as it does not for a group
    /// (`rule:concurrency/nothing-is-still-running-when-a-call-returns`
    /// ).
    ///
    /// # Errors
    ///
    /// [`GraphError`] when the *argument* has no meaning on the other side. No
    /// child is started in that case; the module doc owns why the same refusal
    /// on the way back is an `ok = false` instead.
    pub fn run(self, ctx: &mut Ctx) -> Result<Completion, GraphError> {
        Ok(self.start(ctx)?.join(ctx))
    }

    /// Starts it and answers with the handle that collects it later —
    /// `spawn script`'s half of [`Isolate::run`].
    ///
    /// The argument crosses here, and the child is a runnable task before this
    /// returns, which is what makes `spawn` and `await` two constructs: a
    /// parent that spawns three and awaits three overlaps them. What is
    /// deferred to [`Running::join`] is only the wait, the answer's crossing
    /// and the [`Output::Inherit`] hand-over — the last of those deliberately,
    /// since the await is the one point at which the ordering against the
    /// parent's own output is a fact rather than a race.
    ///
    /// # Errors
    ///
    /// [`GraphError`] when the *argument* has no meaning on the other side. No
    /// child is started in that case; the module doc owns why the same refusal
    /// on the way back is an `ok = false` instead. A chain already at
    /// `[limits] max_script_depth` is **not** one of these — see the refusal at
    /// the top of the body.
    pub fn start(self, ctx: &mut Ctx) -> Result<Box<dyn Running>, GraphError> {
        let Self {
            program,
            args,
            output,
            charge,
            entry,
            inbound,
            peer,
            event_stream,
            watch,
        } = self;
        // `rule:errors/on-limit`'s ceiling on the tree, ahead of everything else in this
        // body: `Ctx::script_depth_breach` owns why the question belongs to the
        // parent and why it is asked once here rather than polled at a
        // safepoint. Nothing has been built yet at this point, which is the
        // whole reason the check stands above the crossing below — a refusal
        // costs one comparison and leaves no half-made isolate behind.
        let depth_breach = match charge {
            Charge::Tree => ctx.script_depth_breach(),
            // `rule:errors/handler-script`'s handler is not part of the tree this ceiling
            // bounds, and a chain that reached the ceiling is one of the
            // failures it exists to report — so asking here would refuse the
            // report on the grounds of the thing being reported.
            // `Ctx::handler_isolate` owns that reading and gives it a fresh
            // depth to start from.
            Charge::EngineReserve => None,
        };
        if let Some(Fault::Fatal(message)) = depth_breach {
            // The safepoint's memory branch, in its order and for its reason
            // (`nvs_runtime::nvs_safepoint`): § 1's tier 1 runs before the
            // breach becomes the message the ladder prints, so a throw of the
            // handler's own is overwritten by `set_pending` rather than
            // reported in place of the limit that stopped the request. The
            // report it is handed reads `max_script_depth`, which is what
            // separates this from the out-of-memory the heap would deliver
            // otherwise (`docs/plan/m6.md`'s *Verify*).
            ctx.run_limit_handler(Limit::ScriptDepth);
            ctx.set_pending(message.clone());
            // Not a `GraphError`: that error is the *argument's* and this
            // refusal is the tree's, and the module doc holds the argument's to
            // its own meaning. The parent is fatal from here, so the handle
            // this answers with carries the failure the child never ran to
            // produce — a `spawn` whose `await` is never reached sees the
            // `FATAL` first either way.
            return Ok(Box::new(Collected {
                completion: Some(refused_completion(&message)),
                output,
                // No child was started, so there is no spawn to record: the
                // event `rule:observability/spawn-is-its-own-event` asks for is
                // filed where a body begins, and none did.
                open: None,
            }));
        }
        // In, at the spawn — before anything is built, so a refusal costs
        // nothing and leaves no half-made isolate behind. No receiving table
        // is named on this side: the child's own is installed by its program's
        // prologue and does not exist yet, which is the module doc's known gap.
        let crossed = copy_graph(args)?;
        // Taken here rather than at the join, because the answer crosses on the
        // child's own stack and this context is borrowed by then
        // (`Ctx::class_table`).
        let receiving = ctx.class_table();
        // `rule:errors/on-limit`'s CPU ceiling reaches a request that allocates
        // nothing, writes nothing and calls nothing only if a thread that is not
        // this one is charging it, and what such a thread charges is the
        // **tree's** handle and the tree's ceiling. Both are read off this
        // context rather than off the child below, and they have to be:
        // `Ctx::isolate` carries no ceiling across, precisely so that the budget
        // stays the root's, and the word it does carry across is the same word
        // this one hands out. Published before the child is built, so a runaway
        // is published ahead of the code that runs away.
        //
        // [`Unpublished`] is what clears it, and the two are deliberately not
        // symmetrical: the publication is one store made here, and the guard
        // that undoes it goes wherever the body it belongs to ends.
        if let Some(watch) = &watch {
            watch.publish(ctx.safepoint_view(), ctx.cpu_limit());
        }

        // The isolate's own root. Buffered under both options; § 4's fresh
        // statics base is `Ctx::isolate`'s whole reason for existing.
        //
        // *Which* buffer is `rule:tooling/echo-always-has-a-sink`'s table, and this is the one place it
        // is read: an isolate handed a request attaches the HTML sink, because
        // its `echo` is the response body, and one spawned inside a request
        // takes its parent's carrier — that row says so, and it is what keeps a
        // `spawn script` child of a page from handing `Core\Out::capture` back
        // a class its parent's own bytes are not. Everything else — a CLI
        // program, a scheduled script, a job worker, a `#[Test]` method — never
        // reaches either half and so keeps the terminal sink by default.
        let sink = if inbound.is_some() || ctx.carrier() == nvs_runtime::CARRIER_HTML_MARKUP {
            OutputSink::Body(Vec::new())
        } else {
            OutputSink::Buffer(Vec::new())
        };
        // `rule:security/isolate-shares-nothing`'s method entry arms the child from the recipes *this* context
        // is holding, there being no second unit to install and so no
        // `install_in` to run inside the program — `Ctx::method_isolate` owns
        // that reading. The handler's reserve is asked first because it is a
        // question about the budget rather than about the entry, and § 3's
        // handler is a path by construction (`crate::ladder` compiles one).
        let mut isolate_ctx = match (charge, entry) {
            (Charge::Tree, Form::Path) => ctx.isolate(sink),
            (Charge::Tree, Form::Method) => ctx.method_isolate(sink),
            (Charge::EngineReserve, _) => ctx.handler_isolate(sink),
        };
        // The request this child answers, on the context that will run it and
        // before it can run — [`Isolate::answering`] owns why it arrives here
        // rather than on the parent, and `Ctx::set_inbound` why it is written
        // once and never cleared.
        if let Some(inbound) = inbound {
            isolate_ctx.set_inbound(inbound);
        }
        // `rule:concurrency/a-connection-is-a-root-isolate`'s socket, on the same context and for the same reason:
        // it is what this isolate *is*, so it is there before the program's
        // first statement rather than reached back for. [`Isolate::over_socket`]
        // owns why nothing but a connection's own isolate has one.
        if let Some(peer) = peer {
            isolate_ctx.set_peer(peer);
        }
        // `rule:concurrency/two-doors-one-isolate`'s other hand-over, beside it
        // and at the same point for the same reason: an event stream's isolate
        // *is* the body of the response its connection is still writing, so the
        // half the bytes go through is there before the program's first
        // statement — and so is the door, marked as the connection one because
        // this stream outlives the request that opened it, which is what
        // `Core\Topic` means by a connection. Both marks together,
        // [`Isolate::over_event_stream`] owning why they are one fact.
        if let Some(events) = event_stream {
            isolate_ctx.set_body_stream(events);
            isolate_ctx.mark_event_stream(EventStreamDoor::Connection);
        }
        // `rule:observability/spawn-is-its-own-event`'s event, opened where the
        // child starts rather than where it is awaited, so a child that is
        // never joined reads as a spawn with no join instead of as nothing at
        // all. The `Option` is the gate and the timing question at once: with
        // both debug bits off nothing is recorded here and nothing on the
        // child's side reads a clock for a split nobody asked for.
        let open = ctx.open_spawn(SpawnForm::Script);
        Ok(match Wake::current() {
            Some(wake) => start_as_task(
                isolate_ctx,
                program,
                crossed,
                Rc::new(wake),
                output,
                receiving,
                watch,
                open,
            ),
            // No task beneath the call, which takes a host installed by
            // something other than a running scheduler — `run_group`'s own
            // case. The program still has to run and the answer still has to be
            // right, so it runs on the caller's stack, at the spawn rather than
            // at the await: with no scheduler there is no concurrency to defer
            // it for. Nothing about the boundary weakens — it is the context,
            // not the stack.
            None => {
                let _unpublished = watch.map(Unpublished);
                Box::new(Collected {
                    completion: Some(run_here(
                        isolate_ctx,
                        program,
                        crossed,
                        receiving,
                        open.is_some(),
                    )),
                    output,
                    open,
                })
            }
        })
    }
}

/// Clears a thread's publication when the isolate it was made for ends.
///
/// A guard rather than a line at the end of the body, for [`Ended`]'s reason:
/// an isolate torn down by a forced unwind reaches no line, and a slot left
/// filled would charge whatever the thread does next to a tree that has
/// finished. It carries no identity, so it clears the thread's slot whether or
/// not the publication still names this isolate — [`Registration::clear`] owns
/// what that costs.
struct Unpublished(Rc<Registration>);

impl Drop for Unpublished {
    fn drop(&mut self) {
        self.0.clear();
    }
}

/// A child already on a stack of its own, with nobody parked on it yet.
///
/// The state the spawn hands back for the parent to hold until the await, which
/// is what makes those two separate calls: the child's task, where its answer is
/// filed, and whether its body has ended.
struct Started {
    /// The child's task, so that a cancelled parent can reach it.
    id: TaskId,
    /// Where the child files its answer, once.
    slot: Rc<RefCell<Option<Completion>>>,
    /// Set by [`Ended`] however the child's body ended.
    done: Rc<Cell<bool>>,
    /// Whose stream the child's bytes go to at the join.
    output: Output,
    /// The event the spawn filed, which the join closes and an abandoned child
    /// leaves open — `None` for a spawn nobody was observing.
    open: Option<OpenSpawn>,
}

impl std::fmt::Debug for Started {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Started")
            .field("done", &self.done.get())
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

/// A child that already ran, held until it is joined — the no-scheduler case,
/// where "started" and "finished" are the same moment.
#[derive(Debug)]
struct Collected {
    completion: Option<Completion>,
    output: Output,
    /// The event the spawn filed, closed here at the join for [`Started`]'s
    /// reason and by the same call.
    open: Option<OpenSpawn>,
}

impl Running for Collected {
    fn join(mut self: Box<Self>, ctx: &mut Ctx) -> Completion {
        let completion = self.completion.take().unwrap_or_else(cancelled_completion);
        close_spawn(self.open.take(), &completion, ctx);
        hand_over(completion, self.output, ctx)
    }

    fn finished(&self) -> bool {
        // "Started" and "finished" were the same moment for this one, so a
        // poller never sees it unfinished and `join` never parks.
        true
    }

    fn abandon(self: Box<Self>) {
        // The program already ran, and its answer is the only thing left; the
        // drop below is what discards it.
    }
}

impl Started {
    /// Parks until the child's task has ended, however it ends.
    ///
    /// `cancelling` is whether the child has already been told to stop, so that
    /// the one cancellation this loop can issue is not issued twice.
    fn park_until_done(&self, mut cancelling: bool) {
        while !self.done.get() {
            let resumed = suspend_current(Waiting::Parked);
            if !resumed.suspended() {
                // Nothing suspended, so the core was never handed back and the
                // child cannot make progress; looping would spin. The same
                // refusal `suspend` documents.
                break;
            }
            if resumed.cancelled() && !cancelling {
                // This task was cancelled while it waited. The child dies with
                // it, and the loop keeps parking until it has — `rule:concurrency/nothing-is-still-running-when-a-call-returns`'s
                // "control does not leave the call with work still running"
                // holds through a cancellation too.
                cancelling = true;
                cancel_task(self.id);
            }
        }
    }
}

impl Running for Started {
    fn join(mut self: Box<Self>, ctx: &mut Ctx) -> Completion {
        self.park_until_done(false);

        let completion = self.slot.borrow_mut().take();
        let completion = completion.unwrap_or_else(cancelled_completion);
        close_spawn(self.open.take(), &completion, ctx);
        hand_over(completion, self.output, ctx)
    }

    fn finished(&self) -> bool {
        // [`Ended`] is what sets this, and it is a `Drop`, so it is true for a
        // child that returned, one that threw and one that was torn down
        // half-way — every state in which there is nothing left to wait for.
        self.done.get()
    }

    fn abandon(self: Box<Self>) {
        if self.done.get() {
            return;
        }
        cancel_task(self.id);
        if nvs_runtime::Teardown::in_progress() {
            // This task's own stack is being unwound, and a stack being unwound
            // may not park: `suspend_current` here would suspend a task the
            // scheduler is in the middle of taking apart. It is also the one
            // case that needs nothing — `Scheduler::orphan` cancels the
            // children of a task as it retires, so the child dies with this one
            // either way, which is the trait's own "dropping is not a leak".
            return;
        }
        self.park_until_done(true);
        // The completion, if the child filed one before it was told, is dropped
        // with `self.slot`: nobody is left to read an answer, and `rule:security/isolate-values-cross-by-copy`
        // makes discarding the copy the whole of freeing it.
    }
}

/// Closes the event the spawn filed, carrying the child's own wall time as the
/// half the overhead split is computed against.
///
/// One helper for both handles, because they differ in how the child ran and
/// not in what the event says about it, and it takes the completion whole so
/// that the numbers a join reports always come off the answer it is reporting.
/// A child that is abandoned rather than joined reaches none of this and leaves
/// its event open, which is the reading
/// `rule:observability/spawn-is-its-own-event` asks for.
fn close_spawn(open: Option<OpenSpawn>, completion: &Completion, ctx: &mut Ctx) {
    if let Some(open) = open {
        ctx.close_spawn(open, completion.wall);
    }
}

/// Gives an [`Output::Inherit`] child's bytes to the parent's own stream, at the
/// await and nowhere else.
fn hand_over(mut completion: Completion, output: Output, ctx: &mut Ctx) -> Completion {
    if output == Output::Inherit {
        let bytes = std::mem::take(&mut completion.output);
        let _ = ctx.write_output(&bytes);
    }
    completion
}

/// Fires when the child's task ends **however** it ended — returning, throwing,
/// or torn down by a forced unwind without its body ever finishing.
///
/// The same shape `group::Child`'s own `Drop` has, and for the same reason: the
/// awaiting side is parked on this, and a cancelled child that only signalled
/// from the end of its body would leave that park with nothing to wake it.
struct Ended {
    done: Rc<Cell<bool>>,
    wake: Rc<Wake>,
}

impl Drop for Ended {
    fn drop(&mut self) {
        self.done.set(true);
        self.wake.wake();
    }
}

/// The child onto a stack of its own, runnable before this returns.
///
/// A single-child sibling of `group::run_as_children`'s loop rather than a call
/// into it, because that runner gives every child a [`Ctx::child`] — the
/// *aliased* statics base `rule:statements/an-isolate-has-its-own-statics` says an isolate may not have. The
/// park is **not** here: waiting is [`Started::join`]'s, and the guarantee that
/// control does not leave with the child still running is the awaiting call's
/// rather than this one's.
#[expect(
    clippy::too_many_arguments,
    reason = "one isolate's parts, each decided at a different line of `Isolate::start`; a struct \
              to carry them the width of one call would be `Isolate` again"
)]
fn start_as_task(
    isolate_ctx: Ctx,
    program: Program,
    args: Value,
    wake: Rc<Wake>,
    output: Output,
    receiving: Option<ErrorClass>,
    watch: Option<Rc<Registration>>,
    open: Option<OpenSpawn>,
) -> Box<dyn Running> {
    let slot: Rc<RefCell<Option<Completion>>> = Rc::new(RefCell::new(None));
    let done = Rc::new(Cell::new(false));

    let filed = Rc::clone(&slot);
    let ended = Ended {
        done: Rc::clone(&done),
        wake,
    };
    let unpublished = watch.map(Unpublished);
    // The child's half of the event's split, as a flag rather than a flags read
    // on the other side: a request nobody is observing must not read a clock,
    // and the parent asked that question once at the spawn. `group::Child` says
    // the same thing about a `Core\Task` child.
    let timed = open.is_some();
    let spawned = spawn_child(isolate_ctx, TaskRoot::Request, move |child| {
        // Moved in so the guard is dropped with the body — including when the
        // body is torn down half-way through by a forced unwind.
        let ended = ended;
        // Moved in for the same reason, and held past `ended` below: the answer
        // is filed there, while `rule:concurrency/after-response-outlives-the-connection`'s
        // work runs on this stack afterwards and is this request's CPU as much
        // as its body was.
        let _unpublished = unpublished;
        let began = timed.then(Instant::now);
        let answer = program(child, args);
        // Stopped where the body stops and before the crossing below, so the
        // copy-out lands on the parent's side of the split rather than inside
        // the child's own compute.
        let wall = began.map(|at| at.elapsed());
        let mut completion = finish(child, answer, receiving.as_ref());
        completion.wall = wall;
        // `rule:concurrency/a-connection-is-a-root-isolate`: a connection isolate's end **is** the connection's end,
        // and § 7 asks for a defined code rather than the reset a dropped
        // descriptor gives. This is the one place that holds both halves — the
        // socket is a field of this child's context and the answer it ended
        // with is in hand — so a close written anywhere else would be reading
        // one of the two through something that outlived it.
        //
        // Every isolate but a connection's has no peer and takes this branch
        // never, which is why it is asked of the context rather than of a flag
        // this builder would have to carry.
        //
        // **A panic does not reach this line, and that is the answer rather
        // than an omission.** `nvs_runtime::run_task` contains one — the core
        // survives it and [`Started::join`] hands the waiter a cancelled
        // completion — but the unwind runs [`Ended`]'s `Drop` and leaves,
        // closing the descriptor with the child's context and giving the peer
        // the reset § 7 asks this branch to replace. Moving the close into that
        // guard is the only way to answer otherwise and it is refused twice
        // over: the guard holds no context to reach a peer through, and a close
        // is a *write*, which parks — where a stack being unwound may not
        // (`crate::scheduler`'s module doc, `nvs_runtime::HelperFrame`). Nor is
        // it the case § 1 is about: a panic is `rule:errors/escalation-ladder`'s engine floor rather
        // than one of the `[limits]` a connection has a budget of, and every
        // one of *those* arrives here as a fault because
        // `nvs_runtime::run_helper` caught it a frame earlier.
        if let Some(peer) = child.peer() {
            peer.close(if completion.ok {
                nvs_runtime::Closing::Done
            } else {
                // Including a `[limits]` budget the connection ran past, which
                // is what makes this the answer to § 1's "closed with a defined
                // code": the limit machinery has already torn the isolate down
                // by here, so the peer is told why instead of reading a reset.
                nvs_runtime::Closing::Faulted
            });
        }
        // `rule:concurrency/after-response-outlives-the-connection`'s condition, read off the answer this isolate just
        // produced: a program that threw, exited or was torn down runs none of
        // its after-response work, and `nvs_runtime::deferred`'s module doc
        // owns why those registrations are released unrun instead.
        //
        // **Two questions, because an `exit` is not a failure.** Its answer
        // crosses and `Completion::ok` is true, so the flag alone cannot tell it
        // from the last top-level statement having run; the status the program
        // recorded (`Ctx::ending`) is what does. A finish records none, which is
        // the whole of why the fourth ending comes out on this side of the gate.
        let deferred = completion.ok && child.ending().is_ok() && child.has_deferred();
        *filed.borrow_mut() = Some(completion);
        // The answer is filed and the guard publishes it, so from here whoever
        // was waiting may take it and go. Everything below therefore runs on a
        // tree its joiner has already let go of, which is what § 6 means by the
        // connection ending while the request tree does not.
        drop(ended);
        if deferred {
            // Before a line of the work, and in this order for a reason a
            // comment is the only place to keep: a task's death cancels what it
            // left running (`crate::scheduler`), and the task that joined this
            // one is a connection about to return. Nothing has yielded between
            // the wake above and this call, so the connection cannot have
            // reached its own end in between — this is the last moment the link
            // can be cut, and the first at which cutting it costs nothing.
            crate::scheduler::detach_current();
            // Then the core, to whoever was waiting. § 6's work runs *after
            // the response is written*, and on one core that is a scheduling
            // fact rather than a turn of phrase: this task is runnable and the
            // joiner has only just been woken, so a drain started here would
            // put the whole of a request's after-response work in front of the
            // bytes the client is still waiting for. One slice is enough — the
            // joiner resumes inside the poll that was waiting for this answer,
            // and what it does with it either finishes or parks on the socket.
            let resumed = suspend_current(Waiting::Yielded);
            if resumed.cancelled() {
                // `rule:concurrency/cancellation-runs-no-user-code`: no user code runs on the way out of a
                // cancellation, and the registrations are released with the
                // context a few lines from here.
                return;
            }
            nvs_runtime::deferred::run_deferred(child);
        }
    });

    let Some(id) = spawned else {
        // Unreachable from inside a turn: holding a `Wake` is `current_task`
        // and the tree both answering. Nothing ran, so nothing is owed — the
        // event goes on to the handle either way, and the join closes it with
        // no child time, which is what a child that never ran took.
        return Box::new(Collected {
            completion: Some(cancelled_completion()),
            output,
            open,
        });
    };

    Box::new(Started {
        id,
        slot,
        done,
        output,
        open,
    })
}

/// The child on the caller's own stack, for a host with no scheduler under it.
fn run_here(
    mut isolate_ctx: Ctx,
    program: Program,
    args: Value,
    receiving: Option<ErrorClass>,
    timed: bool,
) -> Completion {
    // The flag the caller asked the question for, and the same reading the task
    // above takes: the clock is read only where an event was opened, and the
    // body is what it measures.
    let began = timed.then(Instant::now);
    let answer = program(&mut isolate_ctx, args);
    let wall = began.map(|at| at.elapsed());
    let mut completion = finish(&mut isolate_ctx, answer, receiving.as_ref());
    completion.wall = wall;
    // `rule:concurrency/after-response-outlives-the-connection`, on the host that has no response and no scheduler either:
    // the isolate's own frame has returned and its answer is in hand, which is
    // the same trigger the task above reads. There is nothing to detach from —
    // this ran on the caller's own stack — and nothing waiting behind it, so
    // the work simply runs before the answer is handed back. The gate is the
    // one above, for the reason stated there: an `exit` crosses as an ordinary
    // answer and only the recorded status says it was one.
    if completion.ok && isolate_ctx.ending().is_ok() && isolate_ctx.has_deferred() {
        nvs_runtime::deferred::run_deferred(&mut isolate_ctx);
    }
    completion
    // `isolate_ctx` is dropped here: `rule:security/isolate-teardown-is-a-drain-then-a-sweep`'s wholesale release.
}

/// Classifies how the isolate ended and copies its answer **out** of the arena
/// it was built in.
///
/// Called on the child's own stack, while its context is still alive, which is
/// the ordering `rule:security/isolate-values-cross-by-copy` requires: the copy reads the child's graph, and the
/// caller's root owns what comes back.
fn finish(isolate_ctx: &mut Ctx, answer: Value, receiving: Option<&ErrorClass>) -> Completion {
    let cancelled = isolate_ctx.cancelled();
    // `Core\Script::finish()` reaches this root as a `THROWN`, because the throw
    // path is the one every `finally` lives on, and it is an ordinary end all
    // the same. Asked before anything is taken, so the failure below still has
    // the real exception to climb the ladder with;
    // `nvs_runtime::is_finish` is the one home of the question.
    let finished = !cancelled
        && isolate_ctx
            .pending_class()
            .as_deref()
            .is_some_and(nvs_runtime::is_finish);
    let failed = !cancelled && !finished && isolate_ctx.pending().is_some();
    let thrown = failed.then(|| isolate_ctx.take_thrown());
    // The marker, off the context and onto this frame. It is a separate binding
    // from the throw above because every gate below reads that one to mean *this
    // isolate failed*, which a finish did not: taking the marker here is what
    // leaves nothing pending for those gates to find, and handing it to the
    // queue below is what makes the report name `Finish` rather than `Normal`.
    let marker = finished.then(|| isolate_ctx.take_thrown());
    if let Some(thrown) = &thrown {
        // `rule:errors/handler-script`'s tier 3, climbed here for the same reason `nvs run`
        // climbs it at the end of a program: an isolate *is* a program, and
        // this throw reached the top of it with nothing left to catch it. It
        // runs **before** the buffer is taken below, so the handler's own
        // output joins the child's and crosses at the await like every other
        // byte the child wrote.
        //
        // **Tier 4 is deliberately not reached from here.** `rule:security/isolate-shares-nothing`'s
        // failure-is-a-value already carries this throw back to the parent,
        // which is a reporter the CLI's own root task does not have, so a line
        // on `stderr` beside it would be a second report of one failure rather
        // than the floor beneath a missing one. What the operator asked for is
        // the handler; what the parent asked for is the value; neither is the
        // floor.
        // `rule:errors/on-uncaught-throw`'s tier 2, at the *isolate* root that section names
        // beside the request one: this child's own registration, fired before
        // the tier below it and before the buffer is taken, so its output
        // crosses at the await with everything else the child wrote. A parent's
        // handler is not reached from here — a registration is request-local
        // (`Ctx::set_uncaught_handler`), and `rule:security/isolate-shares-nothing`'s failure-is-a-value is
        // how this throw reaches the parent at all.
        isolate_ctx.run_uncaught_handler(thrown);
        let record = nvs_runtime::floor::uncaught(thrown);
        crate::ladder::escalate(isolate_ctx, &record);
    }
    // `rule:observability/exit-hooks-run-after-the-ladder-before-teardown`'s order, at the isolate root, which is
    // where a served request ends: every `finally` has run, tiers 2 and 3 have
    // had the failure above, and the queue comes after them so a misbehaving
    // hook cannot starve the report. It is before `end_session` below because a
    // hook is user code that may still write the record, and before the buffer
    // is taken because what a hook writes is the child's own output and crosses
    // at the await with the rest of it. It is also before the deferred work
    // both callers run, which is that rule's last paragraph.
    //
    // A cancellation runs none of it (`rule:observability/a-fatal-and-a-cancellation-run-no-exit-hook`), and which of the
    // remaining endings this is comes from the status the program recorded
    // (`Ctx::set_ending`) — a `FATAL` among them, which the seam refuses. A
    // pending throw with no status recorded is a `Program` written in Rust
    // rather than compiled, and it is the same ending. A finish left no status
    // either and is `THROWN` for the same reason an uncaught throw is: it is the
    // object beside the status that tells the two apart, which is why the marker
    // is handed on below rather than dropped here.
    if !cancelled {
        let outcome = match isolate_ctx.ending() {
            Ok(()) if failed || finished => Err(nvs_runtime::THROWN),
            ending => ending,
        };
        isolate_ctx.drain_exit_hooks(outcome, thrown.as_ref().or(marker.as_ref()));
    }
    // `rule:http-server/a-session-is-loaded-once-and-written-whole`'s write-back, at the end of the program that opened the
    // record: an HTTP request is a root isolate, so this is the line where a
    // request ends, and `Ctx::end_session` is the no-op an isolate that started
    // no session — or only read one — returns from. It runs after the tier-3
    // climb above because a request that threw still changed what it changed,
    // and before the output is taken below so that anything it has to report
    // crosses at the await with the rest of what this child wrote.
    //
    // **Not on the cancelled path.** The send parks on the store, and a task
    // being torn down may not park (`nvs_runtime::HelperFrame`) — the same rule
    // that makes `Running::abandon` the one call that need not wait.
    if !cancelled {
        isolate_ctx.end_session();
    }
    let output = isolate_ctx.take_buffered_output().unwrap_or_default();
    // `rule:security/response-body-is-one-typed-member`'s declaration, taken beside the bytes it describes and on
    // every path out of here — a child that threw still wrote what it wrote,
    // and whether that reaches a peer is the collector's call rather than
    // this one's.
    let content_type = isolate_ctx.take_content_type();
    // Spec § 15's status, taken on the same paths and for the same reason: a
    // child that threw still said what it said, and a child that set a status
    // and then failed is exactly the case where the declaration matters most.
    let status = isolate_ctx.take_status();
    // Spec § 15's headers, taken on the same paths again and for the same
    // reason. A list rather than a word, and empty for the child that set none.
    let headers = isolate_ctx.take_headers();
    // `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
    // request-scoped body, ended here because this is where its request ends:
    // dropping the writing half is what tells the connection the body is over,
    // and a program that streamed one has already written every byte of it by
    // the time this line runs. Beside the three takes above because it is the
    // same act — what is left on the context once the program has stopped
    // running belongs to nobody.
    drop(isolate_ctx.take_body_stream());

    if let Some(thrown) = thrown {
        // `rule:security/isolate-shares-nothing`'s second row: the class name and the message as copied data.
        // The exception object stays in the arena that is about to go.
        let class = class_name(&thrown);
        release(answer);
        return Completion {
            ok: false,
            value: Value::null(),
            output,
            content_type,
            status,
            headers,
            error: Some(Failure {
                class,
                message: thrown.message(),
            }),
            // Filled by whoever ran the body, on every path out of here: this
            // one measures the child, and a throw is a way for a body to end
            // rather than a reason not to have timed it.
            wall: None,
        };
    }
    if cancelled {
        // Not the child's own failure: its parent died or a deadline landed,
        // and `rule:concurrency/cancellation-runs-no-user-code` means nothing of the child's runs on the way out.
        release(answer);
        let mut completion = cancelled_completion();
        completion.output = output;
        completion.content_type = content_type;
        completion.status = status;
        completion.headers = headers;
        return completion;
    }
    // Out, at the await. A refusal here is the child's, so it is a failure
    // value rather than an `Err` — the module doc owns the asymmetry. The
    // parent's table is named, so `rule:classes/graph-copy`'s third bullet is asked here
    // and not only of the payload carrier.
    let resolve = |name: &str| -> Option<*const nvs_runtime::ClassDesc> {
        Some(receiving?.sibling(name)?.desc())
    };
    match copy_graph_into(answer, Some(&resolve)) {
        Ok(value) => Completion {
            ok: true,
            value,
            output,
            content_type,
            status,
            headers,
            error: None,
            wall: None,
        },
        Err(refused) => Completion {
            ok: false,
            value: Value::null(),
            output,
            content_type,
            status,
            headers,
            error: Some(Failure {
                class: "Error".to_owned(),
                message: refused.to_string(),
            }),
            wall: None,
        },
    }
}

/// The answer for an isolate that was refused before it was built — today the
/// `[limits] max_script_depth` ceiling, and whatever else `Isolate::start`
/// learns to refuse without starting anything.
///
/// `Error` for the class rather than a named one, as [`Failure`]'s own field doc
/// asks: this failure is not a throw, and `rule:errors/escalation-ladder`'s rule that a `FATAL` never
/// reaches a `catch` is the reason there is no class here that a program could
/// name.
fn refused_completion(message: &str) -> Completion {
    Completion {
        ok: false,
        value: Value::null(),
        output: Vec::new(),
        content_type: None,
        status: None,
        headers: Vec::new(),
        error: Some(Failure {
            class: "Error".to_owned(),
            message: message.to_string(),
        }),
        // No body ran, so there is no child time for a split to subtract.
        wall: None,
    }
}

/// The answer for an isolate that never reported: cancelled, or torn down before
/// its body finished.
fn cancelled_completion() -> Completion {
    Completion {
        ok: false,
        value: Value::null(),
        output: Vec::new(),
        content_type: None,
        status: None,
        headers: Vec::new(),
        error: Some(Failure {
            class: "Error".to_owned(),
            message: "the isolate was cancelled".to_owned(),
        }),
        // The body never reported, so nothing measured it — the join that reads
        // this still closes the event, with the wall it observed alone.
        wall: None,
    }
}

/// The rendered class name behind a [`nvs_runtime::Thrown`], or `rule:errors/escalation-ladder`'s
/// generic one where there is no descriptor to read.
///
/// The answer is `nvs_runtime::Thrown`'s own, read from below rather than
/// derefed a second time here: `rule:errors/log-write`'s tier-4 floor asks the same
/// question of the same value, and the `unsafe` behind it belongs to the crate
/// that publishes the descriptor.
fn class_name(thrown: &nvs_runtime::Thrown) -> String {
    thrown.class_name()
}

/// One reference, dropped.
fn release(value: Value) {
    #[expect(
        unsafe_code,
        reason = "this is the last owner of the reference the program \
                  transferred; no slot will take it"
    )]
    // SAFETY: the program transferred this reference and the failure path means
    // nothing else will read it.
    unsafe {
        value.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{Scheduler, current_task};
    use nvs_runtime::{
        ClassDesc, ClassTable, DebugFlags, FieldDefault, MethodRow, NvsObj, TraceKind,
    };

    /// A parent that looks like a request: armed statics, a buffer of its own,
    /// and an error class, so a child's bare-message failure becomes a `Thrown`
    /// carrying a name rather than a null one.
    fn parent() -> Ctx {
        const SLOTS: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut table = ClassTable::new();
        let root = table.define("Throwable", &SLOTS, &[]);
        let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        ctx.set_runtime_error_class(nvs_runtime::ErrorClass::new(
            std::sync::Arc::new(table),
            root,
        ));
        ctx.install_statics(std::sync::Arc::from(vec![Some(FieldDefault::Int(7))]));
        ctx
    }

    /// Runs one isolate inside a real task, so `Wake::current` answers and the
    /// child gets a stack of its own — the shape every caller will have.
    fn run(isolate: Isolate, ctx: &mut Ctx) -> Result<Completion, GraphError> {
        let mut sched = Scheduler::new();
        let held: Rc<RefCell<Option<Result<Completion, GraphError>>>> = Rc::new(RefCell::new(None));
        let out = Rc::clone(&held);
        let back: Rc<RefCell<Option<Ctx>>> = Rc::new(RefCell::new(None));
        let returned = Rc::clone(&back);
        // The parent context is the caller's, so it goes into the task and comes
        // back out: every assertion below is about what it looks like after.
        let moved = std::mem::replace(ctx, Ctx::new(OutputSink::Sink));
        let mut cell = Some(isolate);
        sched.spawn(moved, TaskRoot::Request, move |parent_ctx| {
            let answer = cell.take().expect("run once").run(parent_ctx);
            *out.borrow_mut() = Some(answer);
            *returned.borrow_mut() =
                Some(std::mem::replace(parent_ctx, Ctx::new(OutputSink::Sink)));
        });
        sched.run();
        if let Some(parent_ctx) = back.borrow_mut().take() {
            *ctx = parent_ctx;
        }
        held.borrow_mut().take().expect("the task ran to the end")
    }

    /// A closure, spelled the way `nvs_runtime::graph`'s walk recognizes one:
    /// a class carrying the `invoke` method. Leaked, because a descriptor's
    /// address is its identity.
    fn closure_value() -> Value {
        let mut table = ClassTable::new();
        let id = table.define("Closure", &["arity"], &[]);
        table.set_methods(
            id,
            vec![MethodRow {
                name: nvs_runtime::closure::CLOSURE_INVOKE.to_owned(),
                code: std::ptr::dangling(),
                arity: 0,
                param_tags: 0,
                public: true,
                native: false,
            }],
        );
        let table: &'static ClassTable = Box::leak(Box::new(table));
        #[expect(unsafe_code, reason = "the leaked table outlives the object")]
        // SAFETY: the table is leaked, so the descriptor outlives every value.
        unsafe {
            Value::object(NvsObj::new(table.desc(id)))
        }
    }

    /// `rule:errors/on-limit`'s CPU ceiling is charged by a thread that is not
    /// the one running the request, so an isolate a server takes up publishes
    /// itself for exactly as long as its body runs: the tree it joined and the
    /// ceiling that tree divides, cleared at the end however the body ends.
    #[test]
    fn a_watched_isolate_publishes_its_tree_while_it_runs_and_clears_at_the_end() {
        let mut ctx = parent();
        ctx.set_cpu_limit(60_000_000_000);
        let dog = Rc::new(crate::Watchdog::new());
        // The half of the watched set that answers a *request*: this test's
        // thread is no core, and `register_requests` owns why such an entry
        // takes no deadline view.
        let registered = Rc::new(dog.register_requests());

        // Read from inside the body, because that is the only moment the
        // assertion is about: a publication that arrived after the program ran
        // would stop nothing.
        let seen: Rc<Cell<usize>> = Rc::new(Cell::new(usize::MAX));
        let recorded = Rc::clone(&seen);
        let reading = Rc::clone(&dog);
        let program: Program = Box::new(move |_ctx, _args| {
            recorded.set(reading.running().len());
            Value::null()
        });
        let done = run(
            Isolate::new(program, Value::null(), Output::Capture)
                .watched_by(Rc::clone(&registered)),
            &mut ctx,
        )
        .expect("the argument crossed");
        assert!(done.ok, "the child failed");

        // A platform with no per-thread clock enforces no CPU ceiling, so the
        // publication there is the clearing one — `crate::cpuclock`'s docs own
        // that answer, and this case asserts the wiring either way.
        let expected = usize::from(crate::ThreadClock::current().is_some());
        assert_eq!(
            seen.get(),
            expected,
            "the running isolate was not the one published"
        );
        assert!(
            dog.running().is_empty(),
            "a finished isolate was still offered as the request to stop"
        );
    }

    /// `rule:statements/an-isolate-has-its-own-statics`, which is `rule:security/isolate-shares-nothing`'s "globals, class statics and
    /// runtime-defined constants are fresh": an isolate's statics base is its
    /// own, so nothing it writes can reach the parent's slot. This is the
    /// assertion the whole boundary rests on, because compiled code reaches a
    /// static through that one word and through nothing else.
    #[test]
    fn a_child_cannot_read_or_write_a_parent_variable_or_static() {
        let mut ctx = parent();
        let parent_base = ctx.statics_base();
        assert!(!parent_base.is_null(), "the parent armed one slot");

        let seen: Rc<Cell<*mut Value>> = Rc::new(Cell::new(std::ptr::dangling_mut()));
        let recorded = Rc::clone(&seen);
        let program: Program = Box::new(move |child: &mut Ctx, _args| {
            // Before the child arms anything of its own the base is null: it is
            // not the parent's, and there is nothing to have aliased it to.
            recorded.set(child.statics_base());
            child.install_statics(std::sync::Arc::from(vec![
                Some(FieldDefault::Int(1)),
                Some(FieldDefault::Int(2)),
            ]));
            Value::int(i64::try_from(child.statics_len()).expect("two slots"))
        });

        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("a null argument crosses");

        assert!(done.ok, "{:?}", done.error);
        assert_eq!(done.value.as_int(), Some(2));
        assert!(
            seen.get().is_null(),
            "an isolate starts with no statics base at all, let alone the parent's"
        );
        assert_eq!(
            ctx.statics_base(),
            parent_base,
            "and the parent's own base is untouched by the child arming two slots"
        );
        assert_eq!(ctx.statics_len(), 1, "the parent still has its one slot");
    }

    /// `rule:tooling/echo-always-has-a-sink`
    /// 's third row: a `spawn script` isolate's `echo` reaches a buffer of
    /// its own, and what carries those bytes is the **parent's** carrier — so a
    /// child spawned by a CLI program, a scheduled script, a job worker or a
    /// `#[Test]` method takes `Core\Cli\Text` and its substitution, which is
    /// that sink's neutralization.
    ///
    /// The request half is asserted beside it because the rule is one `if` and
    /// a test of half of it would pass on a runtime that attached the HTML sink
    /// to everything. `nvs_server::serve`'s
    /// `the_html_sink_is_attached_by_a_request_and_by_nothing_else` is the same
    /// claim over a socket; this is it at the line that decides.
    #[test]
    fn an_isolates_echo_takes_the_terminal_sinks_neutralization() {
        let carrier_of = |ctx: &mut Ctx, answering: Option<Inbound>| -> &'static str {
            let seen: Rc<Cell<&'static str>> = Rc::new(Cell::new(""));
            let recorded = Rc::clone(&seen);
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                recorded.set(child.carrier());
                Value::null()
            });
            let isolate = Isolate::new(program, Value::null(), Output::Capture);
            let done = run(
                match answering {
                    Some(inbound) => isolate.answering(inbound),
                    None => isolate,
                },
                ctx,
            )
            .expect("a null argument crosses");
            assert!(done.ok, "{:?}", done.error);
            seen.get()
        };

        let mut ctx = parent();
        assert_eq!(
            carrier_of(&mut ctx, None),
            nvs_runtime::CARRIER_CLI_TEXT,
            "an isolate answering no request took something other than the terminal sink"
        );
        assert_eq!(
            carrier_of(&mut ctx, Some(Inbound::new("GET", "/page", ""))),
            nvs_runtime::CARRIER_HTML_MARKUP,
            "an isolate handed a request did not attach the HTML sink"
        );
    }

    /// `rule:security/isolate-values-cross-by-copy`'s copy is rooted in the **collector's** ownership, so a
    /// completion still holds the answer after the child's arena has gone and
    /// discarding it is what frees the graph. The collector that has to act on
    /// that is the one with nobody to answer — `nvs queue`'s worker, which runs
    /// a job and reports only whether it threw.
    ///
    /// Counted rather than read off a field, because a boundary that handed
    /// back a *borrow* of the child's arena would answer `ok` and `value`
    /// exactly as this one does while leaking one graph per job served. The
    /// null-answering half is the control: the same run owning nothing frees
    /// nothing, so the bytes below are the answer's and not the fixture's.
    #[test]
    fn a_completions_answer_is_held_here_until_it_is_discarded() {
        let freed_by_discarding = |answer: fn() -> Value| -> isize {
            let mut ctx = parent();
            let program: Program = Box::new(move |_: &mut Ctx, _args| answer());
            let mut done = run(
                Isolate::new(program, Value::null(), Output::Capture),
                &mut ctx,
            )
            .expect("a null argument crosses");
            assert!(done.ok, "{:?}", done.error);
            let holding = nvs_runtime::budget::live_bytes();
            done.discard_value();
            holding - nvs_runtime::budget::live_bytes()
        };

        let array = freed_by_discarding(|| {
            let mut out = nvs_runtime::NvsArray::new();
            out.set(nvs_runtime::NvsStr::new(b"receipted"), Value::bool(true));
            Value::array(out)
        });
        assert!(
            array > 0,
            "the child is gone and its answer is alive in this frame, so the \
             discard is what frees it — it freed {array} bytes"
        );
        assert_eq!(
            freed_by_discarding(Value::null),
            0,
            "and an answer owning nothing costs nothing to discard"
        );
    }

    /// The child buffers into its own context, so nothing it writes reaches the
    /// parent's stream — under `Capture` the bytes come back as data instead.
    #[test]
    fn a_child_cannot_see_the_parents_output_buffer() {
        let mut ctx = parent();
        ctx.write_output(b"parent first\n").expect("a buffer");

        let program: Program = Box::new(|child: &mut Ctx, _args| {
            child.write_output(b"child stdout").expect("a buffer");
            Value::null()
        });

        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("a null argument crosses");

        assert!(done.ok, "{:?}", done.error);
        assert_eq!(done.output, b"child stdout");
        assert_eq!(
            ctx.take_buffered_output().unwrap_or_default(),
            b"parent first\n",
            "the parent's stream carries only the parent's own bytes"
        );
    }

    /// `Output::Inherit` is the same machinery with the other sink, and the
    /// bytes arrive at the *await* — which is what makes their position in the
    /// parent's stream a fact rather than a race.
    #[test]
    fn inherited_output_reaches_the_parents_stream_at_the_await() {
        let mut ctx = parent();
        ctx.write_output(b"before ").expect("a buffer");

        let program: Program = Box::new(|child: &mut Ctx, _args| {
            child.write_output(b"child").expect("a buffer");
            Value::null()
        });

        let done = run(
            Isolate::new(program, Value::null(), Output::Inherit),
            &mut ctx,
        )
        .expect("a null argument crosses");

        assert!(done.ok, "{:?}", done.error);
        assert!(
            done.output.is_empty(),
            "inherited bytes are not also captured"
        );
        ctx.write_output(b" after").expect("a buffer");
        assert_eq!(
            ctx.take_buffered_output().unwrap_or_default(),
            b"before child after"
        );
    }

    /// `rule:classes/graph-copy`'s refusals reach this boundary because it is the same walk.
    /// A closure is the shape a test at this level can build without a compiler,
    /// and what is asserted is the *classification*: no child is started, and
    /// the caller is handed the refusal to raise in the parent.
    #[test]
    fn a_closure_a_reference_or_a_resource_is_refused_at_the_boundary() {
        let mut ctx = parent();
        let ran = Rc::new(Cell::new(false));
        let started = Rc::clone(&ran);
        let program: Program = Box::new(move |_: &mut Ctx, _args| {
            started.set(true);
            Value::null()
        });

        let refused = run(
            Isolate::new(program, closure_value(), Output::Capture),
            &mut ctx,
        )
        .expect_err("a closure has no meaning in another arena");

        assert!(
            refused.to_string().contains("closure"),
            "the diagnostic names the offending value: {refused}"
        );
        assert!(
            !ran.get(),
            "the refusal lands before the child exists, so nothing was started"
        );
    }

    /// An instance of `name`, from a class table of its own. Leaked, for
    /// `closure_value`'s reason: a descriptor's address is its identity, and
    /// this fixture exists to have an address the parent's table never
    /// hands out.
    fn instance_of(name: &str) -> Value {
        let mut table = ClassTable::new();
        let id = table.define(name, &["x"], &[]);
        let table: &'static ClassTable = Box::leak(Box::new(table));
        #[expect(unsafe_code, reason = "the leaked table outlives the object")]
        // SAFETY: the table is leaked, so the descriptor outlives every value.
        unsafe {
            Value::object(NvsObj::new(table.desc(id)))
        }
    }

    /// A read-only handle on the object a [`Value`] holds, for reading one
    /// field back off it.
    ///
    /// [`NvsObj::from_raw`] takes a reference **over**, and the one here belongs
    /// to the caller's `value` — so the handle is never dropped, which is how
    /// `nvs_runtime::graph`'s own fixtures borrow one. Its counterpart there is
    /// private to that module, so this is a copy rather than an import.
    fn borrowed(value: Value) -> std::mem::ManuallyDrop<NvsObj> {
        let ptr = value.obj_ptr().expect("an object value");
        #[expect(
            unsafe_code,
            reason = "the caller's `value` owns the reference this borrows"
        )]
        // SAFETY: `value` keeps the object alive for as long as the caller holds
        // it, and this handle is never dropped, so the reference is neither
        // released here nor counted twice.
        let handle = unsafe { NvsObj::from_raw(ptr) };
        std::mem::ManuallyDrop::new(handle)
    }

    /// `rule:security/isolate-values-cross-by-copy`'s cycle rule, driven
    /// through a real spawn rather than over the walk alone: `$a['self'] = $a`
    /// crosses **in** as the argument, and what the child hands back is cyclic
    /// in the same place.
    ///
    /// `nvs_runtime::graph`'s `a_cyclic_value_crosses_without_hanging` pins the
    /// termination at the line that performs it. What a spawn adds is that
    /// there are *two* walks and they are not the same walk: the argument copy
    /// names no receiving table, and the answer copy names the parent's — so
    /// the cycle has to close on each side, and the second one closes in a root
    /// the child's arena is about to be dropped out from under.
    ///
    /// Both are asserted, the first from inside the child, because a boundary
    /// that unrolled the argument one level and handed back something acyclic
    /// is green against either half alone. The ring is built from the parent's
    /// own descriptor for
    /// [`an_unresolvable_class_is_refused_at_the_boundary`]'s reason: a class
    /// the parent cannot name is refused on the way back, and this case is
    /// about the cycle rather than about that refusal. `previous` is the slot it
    /// closes through, being the one a `Throwable` holds another `Throwable` in.
    #[test]
    fn a_cyclic_argument_crosses_a_real_spawn_and_comes_back_with_its_cycle() {
        let mut ctx = parent();
        let desc = ctx
            .class_desc("Throwable")
            .expect("the parent's own table declares it");
        #[expect(unsafe_code, reason = "the parent's table outlives this isolate")]
        // SAFETY: the descriptor came out of the parent's own live table.
        let ring = unsafe { NvsObj::new(desc) };
        // `$ring->previous = $ring`, which is where a naive walk never returns —
        // and which is also the second owner that makes the crossing below a
        // copy rather than the move a uniquely-owned value gets.
        ring.set_field(1, Value::object(ring.clone()));
        let argument = Value::object(ring);
        let ring_ptr = argument.obj_ptr().expect("an object value");

        let closed: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let inside = Rc::clone(&closed);
        let program: Program = Box::new(move |_: &mut Ctx, args: Value| {
            inside.set(borrowed(args).field(1).obj_ptr() == args.obj_ptr());
            args
        });

        let done = run(Isolate::new(program, argument, Output::Capture), &mut ctx)
            .expect("a cycle is not an argument that has no meaning on the other side");

        assert!(done.ok, "{:?}", done.error);
        assert!(
            closed.get(),
            "the argument's cycle closes on the child's own copy, in the child's arena",
        );
        assert_ne!(
            done.value.obj_ptr(),
            Some(ring_ptr),
            "and the answer is a copy: the parent's own object never crossed",
        );
        assert_eq!(
            borrowed(done.value).field(1).obj_ptr(),
            done.value.obj_ptr(),
            "the answer's cycle closes on the answer, rather than on an arena that is gone",
        );

        // Each ring holds itself, which is the leak a refcount cannot see. The
        // child's went with its arena; these two are cleared by hand, as
        // `nvs_runtime::graph`'s fixtures clear theirs.
        borrowed(done.value).set_field(1, Value::null());
        release(done.value);
        #[expect(unsafe_code, reason = "the ring is still held by its own field")]
        // SAFETY: the self-reference is the last one, so the object is live, and
        // this handle is never dropped — the owner taken from it below is.
        let held = std::mem::ManuallyDrop::new(unsafe { NvsObj::from_raw(ring_ptr) });
        // The parent's ring is down to that one reference, the argument's having
        // crossed, so it takes one more owner before the clear: releasing a
        // self-reference at a count of one takes the object apart half-way
        // through the very store that would have made it safe to free.
        let owner = (*held).clone();
        owner.set_field(1, Value::null());
        drop(owner);
    }

    /// `rule:classes/graph-copy`'s third bullet, at this boundary: an object whose class the
    /// receiving side does not have is refused naming the class, and never
    /// arrives as a stub.
    ///
    /// Three legs, because the rule `Live::admit` records is *identity* and not
    /// the name: a class the parent never declared is refused, a class of a
    /// name the parent does declare but from another table is refused too —
    /// the copy keeps the descriptor it was built with, so admitting it would
    /// hand the parent the child's layout and the child's compiled methods —
    /// and the parent's own class crosses, which is what keeps the first two
    /// from passing for the wrong reason.
    #[test]
    fn an_unresolvable_class_is_refused_at_the_boundary() {
        let mut ctx = parent();

        let program: Program = Box::new(|_: &mut Ctx, _args| instance_of("Point"));
        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("the argument crossed; only the answer did not");
        assert!(!done.ok, "a class the parent cannot name may not cross");
        let failure = done.error.expect("a failure is a value, not an `Err`");
        assert!(
            failure.message.contains("Point"),
            "the refusal names the class: {failure:?}"
        );
        assert!(ctx.pending().is_none(), "the parent is not failing");

        let program: Program = Box::new(|_: &mut Ctx, _args| instance_of("Throwable"));
        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("the argument crossed; only the answer did not");
        assert!(!done.ok, "the same name is not the same class");
        let failure = done.error.expect("a failure is a value, not an `Err`");
        assert!(
            failure.message.contains("Throwable"),
            "the refusal names the class: {failure:?}"
        );

        let desc = ctx
            .class_desc("Throwable")
            .expect("the parent's own table declares it");
        let program: Program = Box::new(move |_: &mut Ctx, _args| {
            #[expect(unsafe_code, reason = "the parent's table outlives this isolate")]
            // SAFETY: the descriptor came out of the parent's own live table.
            unsafe {
                Value::object(NvsObj::new(desc))
            }
        });
        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("the argument crossed");
        assert!(done.ok, "the parent's own class crosses: {:?}", done.error);
        release(done.value);
    }

    /// `rule:http-server/containment-does-not-end-at-the-helper`'s third fault, at this boundary: a **panic** inside the
    /// child is contained by the child's own task, so it arrives as one more
    /// `ok = false` beside an uncaught throw and a refused answer, and the
    /// parent is still running and still usable afterwards.
    ///
    /// Nothing here catches anything: the child's task is the containment
    /// boundary (`scheduler::run_task`), and what tells the awaiting side that
    /// the child is over is `Ended`'s `Drop` — which fires *because* it is a
    /// drop, half-way through the unwind, where a line at the end of the body
    /// would never be reached. The join then finds an unfiled slot, which is
    /// the same state a cancelled child leaves and is reported as such: the
    /// panic's own message is the scheduler's to carry, and does not reach a
    /// `Failure` a program can read.
    #[test]
    fn a_contained_panic_in_a_child_leaves_the_parent_running() {
        let mut ctx = parent();
        let program: Program = Box::new(|_: &mut Ctx, _args| panic!("the child gave up loudly"));

        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("a panic is not an argument that could not cross");

        assert!(!done.ok, "a panicking child did not answer");
        assert!(done.error.is_some(), "the failure is a value");
        assert!(ctx.pending().is_none(), "the parent is not failing");

        // Still usable: the parent runs a second isolate to the end on the
        // same context, which is the whole of "leaves the parent running".
        let program: Program = Box::new(|child: &mut Ctx, _args| {
            child.write_output(b"still here").expect("a buffer");
            Value::null()
        });
        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("the argument crossed");
        assert!(done.ok, "{:?}", done.error);
        assert_eq!(done.output, b"still here");
    }

    /// A child that answers with a value that cannot cross is the *other* half
    /// of the asymmetry the module doc names: its own fault, so `ok = false`
    /// rather than an `Err` the parent raises.
    #[test]
    fn an_answer_that_cannot_cross_is_a_failure_value_and_not_a_refusal() {
        let mut ctx = parent();
        let program: Program = Box::new(|_: &mut Ctx, _args| closure_value());

        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("the argument crossed; only the answer did not");

        assert!(!done.ok);
        let failure = done.error.expect("a failure is a value");
        assert!(failure.message.contains("closure"), "{failure:?}");
        assert!(ctx.pending().is_none(), "the parent is not failing");
    }

    /// ADR 0006 § *Failure is a value*: the child's throw is data on the result,
    /// the parent is not unwound, and the parent is still usable afterwards —
    /// which is the whole claim the boundary is built for.
    #[test]
    fn an_uncaught_throw_in_a_child_leaves_the_parent_running() {
        let mut ctx = parent();
        let program: Program = Box::new(|child: &mut Ctx, _args| {
            child.write_output(b"got as far as here").expect("a buffer");
            child.set_pending("the child gave up");
            Value::null()
        });

        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("a null argument crosses");

        assert!(!done.ok);
        let failure = done.error.expect("a failure is a value");
        assert_eq!(failure.message, "the child gave up");
        assert_eq!(
            failure.class, "Throwable",
            "the name is copied, not the object"
        );
        assert_eq!(
            done.output, b"got as far as here",
            "what it echoed before it failed, it did echo"
        );
        assert!(ctx.pending().is_none(), "the parent is not failing");

        ctx.write_output(b"parent still running").expect("a buffer");
        assert_eq!(
            ctx.take_buffered_output().unwrap_or_default(),
            b"parent still running"
        );
    }

    /// Spec § 15's two declarations cross on **every** path out of [`finish`],
    /// the throwing one included — which is the whole of what this boundary
    /// decides about them.
    ///
    /// What a *peer* is told is a different question with a different answer:
    /// `nvs_server::answer` reaches its failure path first and sends `500`
    /// carrying neither. Pinning the two apart is what keeps that policy
    /// editable without editing this crate, and it is why the assertions below
    /// are about the completion rather than about a response.
    ///
    /// The second `declare_header` is the case-insensitive replace, asserted
    /// here because this is the first place a declared header is observable:
    /// two calls under one name are one header, it keeps the position and the
    /// spelling of the first, and it carries the value of the last. A name the
    /// child *appended* crosses once per call instead — the two members mean
    /// opposite things about a name already present, and this boundary is where
    /// a collapsed `Set-Cookie` would become impossible to recover.
    #[test]
    fn a_declared_status_and_header_cross_even_when_the_child_threw() {
        let mut ctx = parent();
        let program: Program = Box::new(|child: &mut Ctx, _args| {
            child.declare_status(201);
            child.declare_header("X-Request-Id", "9f2");
            child.declare_header("x-request-id", "3b7");
            child.append_header("Set-Cookie", "sid=1");
            child.append_header("Set-Cookie", "theme=dark");
            child.set_pending("the child gave up after saying what it meant");
            Value::null()
        });

        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("a null argument crosses");

        assert!(!done.ok);
        assert_eq!(
            done.status,
            Some(201),
            "a status declared before a throw did not cross"
        );
        let carried = vec![
            nvs_runtime::DeclaredHeader::set("X-Request-Id", "3b7"),
            nvs_runtime::DeclaredHeader::add("Set-Cookie", "sid=1"),
            nvs_runtime::DeclaredHeader::add("Set-Cookie", "theme=dark"),
        ];
        assert_eq!(
            done.headers, carried,
            "one name set twice crossed as two headers, or one appended twice crossed as one"
        );
    }

    /// `rule:concurrency/nothing-is-still-running-when-a-call-returns` and `rule:concurrency/cancellation-runs-no-user-code` from the *parent's* side: the task parked in
    /// [`Started::join`] is cancelled, and the boundary owes three things
    /// afterwards — the child's task is out of the tree, the join did not
    /// return while the child was still running, and what the child was holding
    /// is released rather than outliving the request charged for it.
    ///
    /// The parent stands on a [`nvs_runtime::HelperFrame`] because a real one
    /// does — `await` lowers to a `CoreCall` — so its stack is one no forced
    /// unwind may cross and the cancellation arrives as the notice
    /// [`Started::join`]'s loop reads rather than as a teardown of the parent.
    /// That is what makes this a test of the loop and not only of the
    /// scheduler's own sweep.
    #[test]
    fn a_cancelled_parent_leaves_no_orphan_and_no_leaked_arena() {
        /// What the child holds, and the flag it sets when it lets go. An `Rc`
        /// stands in for the arena because it is the one holder a test can
        /// count — the same stand-in `scheduler`'s own
        /// `a_cancelled_tasks_arena_is_released` uses.
        struct Held {
            released: Rc<Cell<bool>>,
            _arena: Rc<Vec<u8>>,
        }
        impl Drop for Held {
            fn drop(&mut self) {
                self.released.set(true);
            }
        }

        let arena = Rc::new(vec![0_u8; 64]);
        let held = Rc::clone(&arena);
        let released = Rc::new(Cell::new(false));
        let in_child = Rc::clone(&released);
        let at_join = Rc::clone(&released);
        let answer: Rc<RefCell<Option<Completion>>> = Rc::new(RefCell::new(None));
        let filed = Rc::clone(&answer);
        let over_at_join = Rc::new(Cell::new(false));
        let observed = Rc::clone(&over_at_join);

        let mut sched = Scheduler::new();
        let parent_id = sched.spawn(parent(), TaskRoot::Request, move |parent_ctx| {
            let program: Program = Box::new(move |_child: &mut Ctx, _args| {
                let _holding = Held {
                    released: in_child,
                    _arena: held,
                };
                // Parked on something that never arrives: the child is alive
                // and owes nothing, which is the state a cancellation has to
                // reach through the parent rather than through the child.
                suspend_current(Waiting::Parked);
                Value::null()
            });
            let _frame = nvs_runtime::HelperFrame::enter();
            let done = Isolate::new(program, Value::null(), Output::Capture)
                .run(parent_ctx)
                .expect("a null argument crosses");
            // Read at the join and nowhere else: § 4's "control does not leave
            // the call with work still running", holding through a
            // cancellation rather than only through a return.
            observed.set(at_join.get());
            *filed.borrow_mut() = Some(done);
        });

        sched.run();
        assert_eq!(
            sched.tracked_tasks(),
            2,
            "the isolate is a child of the awaiting task"
        );
        assert_eq!(sched.parked_count(), 2, "both are waiting on the other");
        assert_eq!(
            Rc::strong_count(&arena),
            2,
            "the child is not holding what it allocated"
        );

        assert_eq!(
            sched.cancel(parent_id),
            2,
            "the mark stopped at the awaiting task"
        );
        let report = sched.run();

        assert!(
            over_at_join.get(),
            "the join returned while the child was still running"
        );
        assert_eq!(report.cancelled, 1, "the child was not torn down");
        assert_eq!(
            sched.tracked_tasks(),
            0,
            "an orphan outlived the task that spawned it"
        );
        assert_eq!(sched.parked_count(), 0, "an orphan is still parked");
        assert_eq!(
            Rc::strong_count(&arena),
            1,
            "the cancelled isolate's memory outlived it"
        );

        let finished = sched.take_finished();
        assert_eq!(
            finished.len(),
            1,
            "the child's `Ctx` was filed instead of going down with its stack"
        );
        assert_eq!(
            finished[0].id, parent_id,
            "the parent did not reach its end"
        );

        let done = answer
            .borrow_mut()
            .take()
            .expect("the awaiting task ran to its end");
        assert!(!done.ok, "a cancelled isolate answered as if it had run");
        assert!(done.error.is_some(), "the failure is a value");
    }

    /// `rule:concurrency/cancellation-runs-no-user-code` from the *child's* side. A child that is running rather
    /// than parked is not unwound out of anybody's frame: it is told, and it
    /// dies at the next safepoint it reaches, having run nothing of its own on
    /// the way out. `scheduler`'s module doc § *The task tree, and what
    /// cancelling one costs* is the mechanism; what this pins at the boundary
    /// is the three consequences — the child stops at the safepoint and takes
    /// no further step, [`Ctx::cancelled`] is what its own body reads to know
    /// it must stop, and [`finish`] therefore classifies it as cancelled rather
    /// than as an answer, so the awaiting parent gets `rule:security/isolate-shares-nothing`'s failure value.
    ///
    /// The cancellation comes from a **sibling** task, because that is the only
    /// place it can come from while the child is still runnable: the parent is
    /// parked on the join, and a caller outside the scheduler can only act once
    /// the whole core has gone idle.
    #[test]
    fn a_child_is_cancelled_at_its_next_safepoint() {
        let steps = Rc::new(Cell::new(0_u32));
        let counted = Rc::clone(&steps);
        let watched = Rc::clone(&steps);
        let read_back = Rc::new(Cell::new(false));
        let noticed = Rc::clone(&read_back);
        let child_id: Rc<Cell<Option<TaskId>>> = Rc::new(Cell::new(None));
        let published = Rc::clone(&child_id);
        let at_cancel = Rc::new(Cell::new(0_u32));
        let stopped_at = Rc::clone(&at_cancel);
        let answer: Rc<RefCell<Option<Completion>>> = Rc::new(RefCell::new(None));
        let filed = Rc::clone(&answer);

        let mut sched = Scheduler::new();
        sched.spawn(parent(), TaskRoot::Request, move |parent_ctx| {
            let program: Program = Box::new(move |child_ctx: &mut Ctx, _args| {
                published.set(current_task());
                child_ctx.write_output(b"as far as here").expect("a buffer");
                // A `Core` member's stack is one no forced unwind may cross, so
                // a cancellation reaches this child as the notice its next
                // suspension answers rather than as a teardown — and answering
                // it is `Ctx::cancel`, `rule:errors/propagation`'s return status, with no
                // cleanup of the child's own running on the way out.
                let _frame = nvs_runtime::HelperFrame::enter();
                loop {
                    if suspend_current(Waiting::Yielded).cancelled() {
                        let _status = child_ctx.cancel();
                        break;
                    }
                    counted.set(counted.get() + 1);
                }
                noticed.set(child_ctx.cancelled());
                Value::null()
            });
            let _frame = nvs_runtime::HelperFrame::enter();
            let done = Isolate::new(program, Value::null(), Output::Capture)
                .run(parent_ctx)
                .expect("a null argument crosses");
            *filed.borrow_mut() = Some(done);
        });
        // The sibling: it lets the child get going, cancels it while it is on
        // the run queue, and records how far it had got — which is the count
        // the child may not move past.
        sched.spawn(parent(), TaskRoot::Request, move |_| {
            loop {
                if watched.get() > 0 {
                    let id = child_id.get().expect("the child never published an id");
                    assert_eq!(cancel_task(id), 1, "the child was already marked");
                    stopped_at.set(watched.get());
                    return;
                }
                suspend_current(Waiting::Yielded);
            }
        });

        sched.run();

        assert!(at_cancel.get() > 0, "the child never ran at all");
        assert_eq!(
            steps.get(),
            at_cancel.get(),
            "the child took a step past the safepoint it was cancelled at"
        );
        assert!(
            read_back.get(),
            "the child's own body could not tell it had been cancelled"
        );
        assert_eq!(sched.tracked_tasks(), 0, "the tree kept a dead task");
        assert_eq!(sched.parked_count(), 0, "the awaiting task was left parked");

        let done = answer
            .borrow_mut()
            .take()
            .expect("the awaiting task ran to its end");
        assert!(!done.ok, "a cancelled child answered as if it had finished");
        let failure = done.error.expect("the failure is a value");
        assert!(failure.message.contains("cancelled"), "{failure:?}");
        assert_eq!(
            done.output, b"as far as here",
            "what it echoed before the safepoint, it did echo"
        );
    }

    /// [`Running::finished`] is the whole of what a caller that may not park
    /// can ask, and both answers matter: `false` while the child has not run,
    /// `true` once it has, with no suspend of the asking task in either. That
    /// is the built-in server's request future — it polls this and answers
    /// `Pending`, because the task it is standing on owes `hyper` its next
    /// read.
    ///
    /// The join at the end is the other half of the pair: asked only once the
    /// answer is `true`, it has nothing left to wait for, so a caller that
    /// polled its way here never parks at all.
    #[test]
    fn a_started_child_answers_finished_without_a_join() {
        let seen: Rc<Cell<(bool, bool)>> = Rc::new(Cell::new((true, false)));
        let recorded = Rc::clone(&seen);
        let answer: Rc<RefCell<Option<Completion>>> = Rc::new(RefCell::new(None));
        let filed = Rc::clone(&answer);

        let mut sched = Scheduler::new();
        sched.spawn(parent(), TaskRoot::Request, move |parent_ctx| {
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                child.write_output(b"ran").expect("a buffer");
                Value::int(7)
            });
            let running = Isolate::new(program, Value::null(), Output::Capture)
                .start(parent_ctx)
                .expect("a null argument crosses");
            let before = running.finished();
            // The core goes back rather than the task parking on the child:
            // this stands in for the poll a future makes, and what ends it is
            // the same question answering differently.
            let mut after = false;
            for _ in 0..8 {
                if running.finished() {
                    after = true;
                    break;
                }
                if !suspend_current(Waiting::Yielded).suspended() {
                    break;
                }
            }
            recorded.set((before, after));
            *filed.borrow_mut() = Some(running.join(parent_ctx));
        });
        sched.run();

        let (before, after) = seen.get();
        assert!(!before, "a child that had not run yet answered finished");
        assert!(after, "a child that had ended never answered finished");
        let done = answer.borrow_mut().take().expect("the task ran to the end");
        assert!(done.ok, "{:?}", done.error);
        assert_eq!(done.value.as_int(), Some(7));
        assert_eq!(
            done.output, b"ran",
            "the join that follows a poll collects the same answer"
        );
    }

    /// `rule:concurrency/nothing-is-still-running-when-a-call-returns` on the one path a join never reaches: a caller that will
    /// **not** await the child still may not leave with it running.
    /// [`Running::abandon`] is that path — the server's request future takes it
    /// when `hyper` drops the service out from under a request — and it owes
    /// both halves, the cancellation and the wait.
    ///
    /// The wait is what is asserted, by reading the child's own teardown *at
    /// the point the abandonment returns*: a call that cancelled and went on
    /// would read `false` there while the child was still on its stack, which
    /// is the rule quietly gone rather than kept.
    #[test]
    fn an_abandoned_child_is_torn_down_before_the_call_returns() {
        /// The stand-in for whatever the child holds, and the flag its drop
        /// sets — the same shape as the cancelled-parent case above.
        struct Held(Rc<Cell<bool>>);
        impl Drop for Held {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }

        let torn = Rc::new(Cell::new(false));
        let in_child = Rc::clone(&torn);
        let read_back = Rc::clone(&torn);
        let at_abandon = Rc::new(Cell::new(false));
        let observed = Rc::clone(&at_abandon);
        let started = Rc::new(Cell::new(false));
        let running_now = Rc::clone(&started);

        let mut sched = Scheduler::new();
        sched.spawn(parent(), TaskRoot::Request, move |parent_ctx| {
            let program: Program = Box::new(move |_child: &mut Ctx, _args| {
                running_now.set(true);
                let _holding = Held(in_child);
                // Parked on something that never arrives, so what is abandoned
                // is a child that is genuinely running rather than one that was
                // about to finish anyway.
                suspend_current(Waiting::Parked);
                Value::null()
            });
            let running = Isolate::new(program, Value::null(), Output::Capture)
                .start(parent_ctx)
                .expect("a null argument crosses");
            // Hand the core back until the child has reached its own park: a
            // single yield re-queues this task ahead of the child it just
            // spawned, and what is abandoned has to be a child that is
            // genuinely running.
            for _ in 0..8 {
                if started.get() {
                    break;
                }
                suspend_current(Waiting::Yielded);
            }
            assert!(started.get(), "the child never got a slice");
            assert!(!running.finished(), "the parked child reported finished");
            running.abandon();
            observed.set(read_back.get());
        });
        let report = sched.run();

        assert!(
            at_abandon.get(),
            "the abandonment returned with the child still running"
        );
        assert_eq!(report.cancelled, 1, "the child was not torn down");
        assert_eq!(sched.tracked_tasks(), 0, "the tree kept a dead task");
        assert_eq!(
            sched.parked_count(),
            0,
            "the abandoning task was left parked"
        );
    }

    /// A one-field class for [`build_a_cycle`] to allocate from. Leaked for
    /// [`closure_value`]'s reason — a descriptor's address is its identity, so
    /// it has to outlive every object made against it — and built **once** per
    /// test, outside every measured region: a table leaked per request would
    /// grow the very reading the two cases below hold flat.
    fn node_class() -> *const ClassDesc {
        let mut table = ClassTable::new();
        let id = table.define("Node", &["next"], &[]);
        let table: &'static ClassTable = Box::leak(Box::new(table));
        table.desc(id)
    }

    /// One pair of objects that hold each other and that nothing else holds —
    /// the smallest graph no reference count can reclaim, and the one
    /// `rule:security/isolate-teardown-is-a-drain-then-a-sweep`
    /// 's teardown sweep exists for.
    ///
    /// An `extern "C"` entry reached through [`nvs_runtime::call`] rather than
    /// a plain Rust call from the program closure, because that call is what
    /// installs the child's context as the current one: an object links itself
    /// into the live list of whichever context is running, and a pair allocated
    /// with none current would sit in no list for the sweep to walk. Slot 0
    /// carries the class to allocate from, the way `rule:statements/static-is-a-member-modifier`'s late static
    /// binding hands a static method its own.
    #[expect(
        unsafe_code,
        reason = "a compiled function's entry is `extern \"C\"`, and the class \
                  `node_class` leaked outlives every object made here"
    )]
    unsafe extern "C" fn build_a_cycle(
        _ctx: *mut Ctx,
        args: *const Value,
        _out: *mut Value,
    ) -> i32 {
        // SAFETY: `nvs_runtime::call` is handed a one-element slice below, so
        // slot 0 is a live, initialized `Value`.
        let class = unsafe { *args }
            .as_class_desc()
            .expect("slot 0 carries the class to allocate from");
        // SAFETY: the table `node_class` leaked is never freed, so the
        // descriptor outlives every object and every context in this binary.
        unsafe {
            let left = NvsObj::new(class);
            let right = NvsObj::new(class);
            left.set_field(0, Value::object(right.clone()));
            right.set_field(0, Value::object(left.clone()));
        }
        nvs_runtime::OK
    }

    /// Runs one isolate whose program builds `pairs` cycles, and answers what
    /// this thread was holding at the moment the child had them all — the
    /// reading a caller compares its own baseline against.
    ///
    /// The [`Completion`] is dropped before this returns, so a caller's reading
    /// afterwards is charged nothing of the answer; the program returns `null`,
    /// which is the one value there is nothing to discard.
    fn a_cycle_building_request(ctx: &mut Ctx, class: *const ClassDesc, pairs: usize) -> isize {
        let peak: Rc<Cell<isize>> = Rc::new(Cell::new(0));
        let inside = Rc::clone(&peak);
        let program: Program = Box::new(move |child: &mut Ctx, _args| {
            for _ in 0..pairs {
                nvs_runtime::call(build_a_cycle, child, &[Value::class_desc(class)])
                    .expect("the builder answers OK");
            }
            inside.set(nvs_runtime::budget::live_bytes());
            Value::null()
        });
        let done = run(Isolate::new(program, Value::null(), Output::Capture), ctx)
            .expect("a null argument crosses");
        assert!(done.ok, "{:?}", done.error);
        peak.get()
    }

    /// `rule:security/isolate-teardown-is-a-drain-then-a-sweep`
    /// at the boundary `rule:security/isolate-shares-nothing` gives a request: what the root drain leaves
    /// is swept when the arena is dropped, so a request that built a cycle
    /// still gives every byte back. `nvs_runtime::object`'s
    /// `a_cyclic_object_graph_is_reclaimed_when_its_context_drops` is the same
    /// mechanism at the line that performs it; this is it one boundary out,
    /// where the arena is torn down by the host on the child's way off the run
    /// queue rather than by the case itself.
    ///
    /// **Filed `-p nvs-host` and not `-p nvs-server`, which is where the
    /// acceptance check named it.** A request *is* this isolate — the goal's
    /// standing decision — but the cycle has to be built by hand, and
    /// `NvsObj::new` is `unsafe` while `crates/nvs-server` inherits the
    /// workspace's `unsafe_code = "forbid"`, which no `#[expect]` can open. The
    /// claim is about the arena's teardown, and the arena's teardown is here.
    ///
    /// The peak is read from **inside** the child rather than inferred, because
    /// a fixture that allocated nothing would satisfy the flat reading below on
    /// its own.
    #[test]
    fn a_request_that_builds_cycles_returns_its_bytes_at_teardown() {
        const PAIRS: usize = 64;
        let class = node_class();
        let mut ctx = parent();
        // One request run and thrown away before the baseline is taken: the
        // scheduler's state, the child's stack and this thread's allocator are
        // all first-touch costs that a cold run pays and nothing frees, and
        // charging them to the sweep would be measuring the fixture.
        // `nvs_runtime::object`'s own cycle guard opens with the same line.
        a_cycle_building_request(&mut ctx, class, PAIRS);

        let before = nvs_runtime::budget::live_bytes();
        let peak = a_cycle_building_request(&mut ctx, class, PAIRS);

        assert!(
            peak - before >= (2 * PAIRS * nvs_runtime::FIELD_STRIDE).cast_signed(),
            "the request held {} bytes over the baseline, less than the {} \
             objects it was asked for can weigh — nothing was built, and the \
             flat reading below would be about nothing",
            peak - before,
            2 * PAIRS
        );
        assert_eq!(
            nvs_runtime::budget::live_bytes(),
            before,
            "the request is over and its cycles are not reachable from anything \
             — no reference count in one ever reached zero, so what gave the \
             bytes back is the arena's own sweep or nothing did"
        );
    }

    /// The same claim under load, which is the whole memory story in one
    /// reading: a request's cycles die with the request, so live bytes track
    /// what is **in flight** and never what has been served.
    ///
    /// A leak of one pair per request is invisible in the single reading the
    /// case above takes, and it is the failure this shape exists for. Asserted
    /// by **counting** the readings that moved rather than by comparing the
    /// last one, so a drift that only shows up late in the soak fails as loudly
    /// as one that shows up immediately.
    #[test]
    fn live_bytes_are_flat_across_a_cycle_building_soak() {
        const REQUESTS: usize = 100;
        const PAIRS: usize = 16;
        let class = node_class();
        let mut ctx = parent();
        // Sized up front and never grown, so recording a reading cannot move
        // the next one.
        let mut readings: Vec<isize> = Vec::with_capacity(REQUESTS);
        a_cycle_building_request(&mut ctx, class, PAIRS);

        let before = nvs_runtime::budget::live_bytes();
        let mut peak = before;
        for _ in 0..REQUESTS {
            peak = peak.max(a_cycle_building_request(&mut ctx, class, PAIRS));
            readings.push(nvs_runtime::budget::live_bytes());
        }

        assert!(
            peak > before,
            "no request in the soak ever held anything, so a flat reading says \
             nothing about the sweep"
        );
        let drifted = readings.iter().filter(|held| **held != before).count();
        assert_eq!(
            drifted,
            0,
            "{drifted} of {REQUESTS} requests left the thread holding bytes it \
             had not held before them, the worst by {} — memory is tracking \
             traffic rather than what is in flight",
            readings.iter().copied().max().unwrap_or(before) - before
        );
    }

    /// One event for the spawn, filed where the child starts and closed at the
    /// await with the child's own wall time beside the parent's, which is the
    /// overhead split `rule:observability/spawn-is-its-own-event` asks a
    /// `spawn script` for — and the form it carries is this construct's
    /// spelling rather than a task's.
    ///
    /// The second half is the gate every probe in the tree shares
    /// (`rule:testing/debug-probes`): with both bits off there is no event, and
    /// nothing on either side of the boundary reads the clock that would have
    /// filled one. `group::an_untraced_spawn_records_no_spawn_event` is the
    /// same claim for a `Core\Task` child.
    #[test]
    fn a_traced_spawn_script_records_one_spawn_event_closed_at_its_join() {
        let program: Program = Box::new(|_child: &mut Ctx, _args| Value::null());
        let mut ctx = parent();
        ctx.set_debug_flags(DebugFlags::TRACE);

        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut ctx,
        )
        .expect("the argument crosses");

        assert!(done.ok);
        assert!(
            done.wall.is_some(),
            "the parent opened an event and the child reported no time of its own"
        );
        let events: Vec<&str> = ctx
            .trace()
            .iter()
            .filter(|event| event.kind == TraceKind::Spawn)
            .map(|event| event.callee.as_str())
            .collect();
        assert_eq!(events.len(), 1, "one event per spawn, not one per await");
        let event = events[0];
        assert!(
            event.starts_with("spawn script started="),
            "the event names another construct: {event}"
        );
        assert!(
            event.contains(" joined=") && event.contains(" child="),
            "the await left the event open, or closed it with no split: {event}"
        );

        let program: Program = Box::new(|_child: &mut Ctx, _args| Value::null());
        let mut untraced = parent();

        let done = run(
            Isolate::new(program, Value::null(), Output::Capture),
            &mut untraced,
        )
        .expect("the argument crosses");

        assert!(done.ok);
        assert!(
            untraced.trace().is_empty(),
            "a request nobody is observing recorded an event"
        );
        assert!(
            done.wall.is_none(),
            "a request nobody is observing read a clock for a split nobody asked for"
        );
    }
}
