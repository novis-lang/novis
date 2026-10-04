//! A child placed `on: "worker"`: what crosses to the core that runs it, and
//! what comes back.
//!
//! `rule:concurrency/on-worker-runs-the-child-on-another-core` is the
//! specification and [`crate::worker`] is the transport; this module is the one
//! thing neither of those can be, which is an isolate rebuilt from parts on a
//! core that never held its parent. ADR 0184 §§ 2–4 is the reasoning.
//!
//! # What crosses, and why it is parts rather than an isolate
//!
//! [`crate::isolate::Isolate`] holds a [`Program`](nvs_runtime::script::Program)
//! and a [`Value`], and neither may leave the core that built it: the program is
//! a boxed Rust closure over the *parent's* resolver, and a refcount is non-atomic
//! precisely because a value is reachable from one core. So a placement crosses
//! four things, each of which is either plain data or a handle every core
//! already reads — the [`Entry`]'s two names, the argument as
//! [`nvs_runtime::graph`]'s bytes, [`nvs_runtime::PlacedIsolate`] (the request
//! tree, the deadline word, the configuration snapshot, the compiled unit's
//! class table and static recipes, and the narrowing the spawn site wrote), and
//! the answer slot [`crate::worker`] owns.
//! The far core builds the program and decodes the argument for itself, which is
//! what [`Entry`]'s own doc means by the core that runs the child being the one
//! that prepares it.
//!
//! The answer comes back the same way, as [`Crossed`]: every field of a
//! [`Completion`] except the one that is a graph, plus that graph's bytes. The
//! parent decodes it into its own arena at the join, against its own class
//! table, so what a placed child answers with is a copy at every node — the cost
//! ADR 0184 § 2 says belongs to the placement and the reason `on: "here"`
//! exists.
//!
//! # Both entry forms cross
//!
//! A path becomes code through a resolver, which is per thread, and a core
//! [`crate::worker`] starts for itself was never on the thread `nvs-cli` booted.
//! What closes that is [`nvs_runtime::script::SharedResolver`]: the process
//! publishes one handle to the compiler it already built — `nvs-cli`'s keeps its
//! path map and its unit map behind `RwLock`s, and a serving fleet hands every
//! core the same `Arc<Compiler>` so that a source compiles once for the process
//! — and a core installs it on its own thread as it starts. So what is per
//! thread stays the seam and not the cache, and a path entry is prepared on the
//! core that runs it exactly as a method entry is.
//!
//! [`crosses`] therefore asks what is true of the *context* rather than of the
//! entry's form, plus the one question a path still raises: whether this process
//! published a resolver at all. A `nvs check` and a test publish none, and a path
//! entry there stays on the parent's core rather than crossing to answer
//! [`nvs_runtime::script::ResolveError::NoResolver`].
//!
//! # What it spends
//!
//! `rule:programs/memory-priority` asks for this out loud. Per placement in
//! flight, charged to the tree that asked for it: the argument's bytes and the
//! answer's bytes, each held once while it crosses, on top of the two copies of
//! the graph itself that the encode and the decode make. That is strictly more
//! than a same-core child, which copies once and holds no bytes, and it is what
//! buys the core.

use std::time::Duration;

use nvs_runtime::graph::GraphError;
use nvs_runtime::host::{Completion, Entry, Failure, Narrowing, Output, Running};
use nvs_runtime::{
    ClassDesc, Ctx, DeclaredHeader, ErrorClass, OpenSpawn, OutputSink, PlacedIsolate, SpawnForm,
    Value,
};

use crate::isolate::{cancelled_completion, close_spawn, hand_over, refused_completion, run_here};
use crate::worker::{Answer, Destination, Posted};

/// The core a child of `entry` would be placed on, or `None` where this spawn
/// stays here.
///
/// Asked **before** anything is built and before the argument is consumed, which
/// is [`crate::worker::Destination`]'s whole reason for existing: a spawn that
/// encoded first and was then refused a core would hold neither a placement nor
/// a value to start where it stands.
///
/// Three questions, and a `None` to any of them is a fall-through to the
/// parent's own core rather than a failure — the placement asked for a core and
/// loses only that:
///
/// * A core to write to, and this task's own way of being woken by it
///   ([`crate::worker::destination`]).
/// * An entry the far core can prepare: a method always, and a path wherever
///   this process published a resolver — the module doc's *Both entry forms
///   cross* owns why those are the same question asked of two forms.
/// * A class table to prepare it *against*: a method entry is a label looked up
///   in the compiled unit's table, and a context holding none could resolve
///   nothing there.
pub(crate) fn destination_for(ctx: &Ctx, entry: &Entry) -> Option<Destination> {
    if !crosses(ctx, entry) {
        return None;
    }
    crate::worker::destination()
}

/// Whether the far core could prepare this child at all — the two questions
/// [`destination_for`] asks before it secures anything.
///
/// Separate from the securing so that what it decides is a fact about the spawn
/// rather than about whether a core happened to be free, which is also the only
/// way to ask it of a context that is on no scheduler.
fn crosses(ctx: &Ctx, entry: &Entry) -> bool {
    if ctx.class_table().is_none() {
        return false;
    }
    // A method is a label in the compiled unit's table, and that table crossed
    // with the seed, so there is nothing further to ask. A path becomes code
    // through a resolver, and the one the far core will hold is the handle this
    // process published — [`nvs_runtime::script::published`], read here rather
    // than off a started core's thread-local because a core takes the handle as
    // it starts and a core is started for the first placement that reaches it.
    // A process that published none is a `nvs check` or a test, and a path entry
    // there stays on the parent's core rather than crossing to answer
    // [`nvs_runtime::script::ResolveError::NoResolver`].
    entry.is_method() || nvs_runtime::script::published().is_some()
}

/// Starts the child on the core `destination` holds, and answers the handle that
/// collects it.
///
/// **Consumes one reference to `args`**, exactly as
/// [`crate::isolate::Isolate::start`] does and at the same point in the body:
/// the crossing is where the parent's value stops being the child's business.
///
/// # Errors
///
/// [`GraphError`] when the *argument* has no meaning on the other side. No child
/// is started in that case and the core is given back unused, which is what
/// dropping a [`Destination`] without posting means.
pub(crate) fn start(
    destination: Destination,
    ctx: &mut Ctx,
    entry: Entry,
    args: Value,
    output: Output,
    narrowing: Narrowing,
) -> Result<Box<dyn Running>, GraphError> {
    // Out, at the spawn, and before the seed below for the reason the refusal
    // stands where it does in a same-core start: a refusal costs one walk and
    // leaves nothing half-made, here not even a core told to expect work.
    let argument = nvs_runtime::graph::encode(args)?;
    // `rule:observability/spawn-is-its-own-event`'s event, opened where the child
    // starts exactly as a same-core one is, so a placement reads as a spawn in
    // the same trace beside them — under the `spawn worker` spelling, which is
    // what tells it apart from the `spawn script` the same source line is when
    // it stays here and is the only way a reading can say what the core cost.
    // It is also the gate on the child reading a clock at all, asked once here
    // and carried rather than asked again over there.
    let open = ctx.open_spawn(SpawnForm::Worker);
    let crossing = Crossing {
        entry,
        argument,
        // The narrowing rides the seed rather than being applied here: the
        // context it holds does not exist until the far core builds one, and the
        // ceilings it is clamped against are the ones this reading just resolved
        // (`nvs_runtime::PlacedIsolate::narrowed_by`).
        seed: ctx.placed_isolate().narrowed_by(narrowing),
        timed: open.is_some(),
    };
    Ok(Box::new(Placed {
        posted: destination.post(move || cross(crossing)),
        output,
        open,
    }))
}

/// One placement's parts, on their way to the core that will assemble them.
struct Crossing {
    entry: Entry,
    argument: Vec<u8>,
    seed: PlacedIsolate,
    /// Whether the parent opened a spawn event, so the child reads a clock only
    /// where somebody is observing — `rule:testing/debug-probes`'s gate, asked
    /// once on the parent and carried rather than asked again here.
    timed: bool,
}

/// A [`Completion`] with the one field that is a graph turned into bytes —
/// what a placed child answers its parent with.
///
/// Every other field is already plain data, which is why this is the whole of
/// the difference: `rule:security/isolate-values-cross-by-copy` has the answer
/// crossing by copy in any case, and across a thread that copy is the encode and
/// the decode rather than one graph walk.
struct Crossed {
    ok: bool,
    value: Vec<u8>,
    output: Vec<u8>,
    content_type: Option<Box<str>>,
    file_body: Option<Box<std::path::Path>>,
    status: Option<u16>,
    headers: Vec<DeclaredHeader>,
    error: Option<Failure>,
    wall: Option<Duration>,
}

/// The far core's whole half: build the context, prepare the program, decode the
/// argument, run it, and encode what it answered.
///
/// It runs as a task on that core — the receptionist in [`crate::worker`] starts
/// it as one — so the child may park, spawn children of its own and reach a
/// reactor, which is the difference between a placement and a pool job.
fn cross(crossing: Crossing) -> Crossed {
    let Crossing {
        entry,
        argument,
        seed,
        timed,
    } = crossing;
    // A method entry's statics are materialized here from the recipes the
    // compiled unit owns, because its code is the parent's unit's and there is
    // no second unit whose prologue would arm them. A path entry's are its own
    // unit's, armed by the program below through `Ctx::install_statics` as every
    // entry file's prologue does, so seeding the parent's first would be a store
    // built to be thrown away.
    let child = if entry.is_method() {
        seed.build_method(OutputSink::Buffer(Vec::new()))
    } else {
        seed.build(OutputSink::Buffer(Vec::new()))
    };
    // The receiving table for both crossings on this side, taken before the
    // context is moved into the run below. It is the table that crossed, so a
    // class in the answer is the same descriptor the parent will compare against
    // — the identity `rule:classes/graph-copy` is written in terms of.
    let receiving = child.class_table();
    let program = match entry.program(&child) {
        Ok(program) => program,
        Err(refusal) => return crossed(refused_completion(&refusal.to_string())),
    };
    let value = match nvs_runtime::graph::decode(&argument, &resolver(receiving.as_ref())) {
        Ok(value) => value,
        // The argument was accepted by the walk on the parent's core and refused
        // by the reader on this one, which is this program disagreeing with its
        // parent about a class rather than the parent having built something
        // that cannot cross. It reaches the parent as `ok = false` for the
        // module doc's reason: no line of the child ran, but the child is what
        // could not receive it.
        Err(refusal) => return crossed(refused_completion(&refusal.to_string())),
    };
    crossed(run_here(child, program, value, receiving, timed))
}

/// Turns what the child answered into what crosses back, spending the
/// completion's one reference on the encode.
///
/// An answer that cannot cross is **the child's failure and not a refusal** —
/// [`crate::isolate`]'s module doc § *A refused argument is the parent's fault*
/// owns that split, and it holds identically across a thread.
fn crossed(mut completion: Completion) -> Crossed {
    let value = std::mem::take(&mut completion.value);
    match nvs_runtime::graph::encode(value) {
        Ok(value) => Crossed {
            ok: completion.ok,
            value,
            output: completion.output,
            content_type: completion.content_type,
            file_body: completion.file_body,
            status: completion.status,
            headers: completion.headers,
            error: completion.error,
            wall: completion.wall,
        },
        Err(refusal) => {
            let mut refused = crossed_failure(&refusal.to_string());
            refused.output = completion.output;
            refused.wall = completion.wall;
            refused
        }
    }
}

/// What crosses for a child that produced no answer worth carrying: a failure
/// value, and the empty encoding of `null` beside it.
fn crossed_failure(message: &str) -> Crossed {
    Crossed {
        ok: false,
        value: Vec::new(),
        output: Vec::new(),
        content_type: None,
        file_body: None,
        status: None,
        headers: Vec::new(),
        error: Some(Failure {
            class: "Error".to_owned(),
            message: message.to_owned(),
        }),
        wall: None,
    }
}

/// Reads `crossed` back into the arena of whichever core is asking, against that
/// core's own class table.
///
/// An empty payload is `null` rather than a short read: [`crossed_failure`]
/// writes one for every ending that has no answer, and a failure's `value` is
/// `null` by [`Completion::value`]'s own contract.
fn received(crossed: Crossed, receiving: Option<&ErrorClass>) -> Completion {
    let value = if crossed.value.is_empty() {
        Value::null()
    } else {
        nvs_runtime::graph::decode(&crossed.value, &resolver(receiving)).unwrap_or_else(|_| {
            // Unreachable from a child that encoded successfully against a table
            // this one shares, and answered as a failure rather than asserted:
            // a child may not end its parent, and the message the walk wrote is
            // already on the completion below when this arm is the one that
            // fires.
            Value::null()
        })
    };
    Completion {
        ok: crossed.ok,
        value,
        output: crossed.output,
        content_type: crossed.content_type,
        file_body: crossed.file_body,
        status: crossed.status,
        headers: crossed.headers,
        error: crossed.error,
        wall: crossed.wall,
        // A `spawn worker` child roots a trace of its own on the core it landed
        // on, so there is nothing of the parent's trace here to carry back:
        // what the parent's records of the child is the `spawn` event it filed
        // itself (`rule:observability/spawn-is-its-own-event`), and that event
        // is already on the parent's own context.
        trace: Vec::new(),
    }
}

/// [`nvs_runtime::graph::decode`]'s class resolver over a table handle.
///
/// The handle keeps the table alive for as long as the Rust closure can be called,
/// which is what makes the descriptor addresses it hands out valid without any
/// lifetime on the pointer — [`ErrorClass`]'s own doc owns that.
fn resolver(table: Option<&ErrorClass>) -> impl Fn(&str) -> Option<*const ClassDesc> {
    move |name| {
        table
            .and_then(|table| table.sibling(name))
            .map(|of| of.desc())
    }
}

/// A child running on another core, and the whole of what its parent holds.
///
/// [`crate::isolate`]'s `Started` for a child on a stack this core cannot see:
/// the task id, the slot and the wake are all [`crate::worker`]'s, so what is
/// left here is the two things a join needs that a placement does not know about
/// — whose stream the bytes go to, and the event the spawn filed.
#[derive(Debug)]
struct Placed {
    posted: Posted<Crossed>,
    output: Output,
    open: Option<OpenSpawn>,
}

impl Running for Placed {
    fn join(mut self: Box<Self>, ctx: &mut Ctx) -> Completion {
        // The parent's table, read here rather than carried from the spawn: this
        // is the one call where the parent's context is in hand, and the answer
        // is decoded on this stack rather than on the child's.
        let receiving = ctx.class_table();
        let completion = match self.posted.collect() {
            Answer::Value(crossed) => received(crossed, receiving.as_ref()),
            // Contained at the far core's own task root and crossed as a value,
            // which is `rule:security/isolate-shares-nothing`'s failure-is-a-value
            // applied to a boundary that is also a thread: resuming the panic
            // here would let a child end its parent.
            Answer::Panicked(panic) => refused_completion(&format!(
                "the isolate panicked on the core it was placed on: {panic}"
            )),
            // The cancellation this parent sent, or a core that closed while it
            // held the work. Both are a child that stopped without answering,
            // which is the same ending a torn-down same-core child has.
            Answer::Stopped => cancelled_completion(),
        };
        close_spawn(self.open.take(), &completion, ctx);
        hand_over(completion, self.output, ctx)
    }

    fn finished(&self) -> bool {
        self.posted.finished()
    }

    fn abandon(self: Box<Self>) {
        // The wait is [`Posted::abandon`]'s, teardown carve-out included: the
        // trait's contract is that the child has already died when this returns,
        // and across a thread that is an acknowledgement rather than a scheduler
        // fact.
        self.posted.abandon();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactor::{self, Reactor, run_until_idle};
    use crate::scheduler::Scheduler;
    use nvs_runtime::{ClassTable, DebugFlags, FieldDefault, TaskRoot, TraceKind};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A context carrying a compiled unit's class table, which is what a method
    /// entry is looked up in.
    fn with_a_class_table() -> Ctx {
        const SLOTS: [&str; 4] = ["message", "previous", "backtrace", "location"];
        let mut table = ClassTable::new();
        let root = table.define("Throwable", &SLOTS, &[]);
        let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::new(table), root));
        ctx.install_statics(std::sync::Arc::from(vec![Some(FieldDefault::Int(7))]));
        ctx
    }

    /// A resolver with nothing behind it, which is all a `crosses` case needs:
    /// the question is whether the process published one, and no path here is
    /// ever compiled.
    #[derive(Debug)]
    struct Publishable;

    impl nvs_runtime::script::Resolver for Publishable {
        fn resolve(&self, path: &str) -> Result<nvs_runtime::script::Program, String> {
            Err(format!("`{path}` is not a script"))
        }
    }

    /// Both entry forms cross once the process has published a resolver, and a
    /// path stops crossing the moment it withdraws one.
    ///
    /// The forms differ in where the code comes from — a method is in the unit
    /// the seed carries, a path is compiled on the far core — and the module
    /// doc's *Both entry forms cross* owns why that is now a difference in the
    /// preparation rather than in the placement. A path crossing to a core with
    /// no resolver would answer `ResolveError::NoResolver`, which is a failure
    /// value where the program asked for a core, so the published handle is what
    /// the second half of this asserts.
    #[test]
    fn both_entry_forms_cross_once_a_resolver_is_published() {
        let ctx = with_a_class_table();
        let method = Entry::Method {
            label: "Work::run".to_owned(),
            names: Vec::new(),
        };
        let path = Entry::Path("child.nvs".to_owned());
        assert!(crosses(&ctx, &method), "a method entry may not cross");
        let published = nvs_runtime::script::publish(nvs_runtime::script::SharedResolver::new(
            std::sync::Arc::new(Publishable),
        ));
        assert!(
            crosses(&ctx, &path),
            "a path entry stayed here with a resolver every core reads"
        );
        drop(published);
        assert!(
            !crosses(&ctx, &path),
            "a path entry crossed to a core that could not resolve it"
        );
    }

    /// A context holding no class table places nothing, whatever the entry form.
    ///
    /// A method entry is a label, and the far core resolves it against the table
    /// that crossed with the seed. Placing one with no table to carry would post
    /// work that could only answer "this program declares no such static method".
    #[test]
    fn an_entry_with_no_class_table_to_resolve_it_against_stays_here() {
        let ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        let method = Entry::Method {
            label: "Work::run".to_owned(),
            names: Vec::new(),
        };
        assert!(
            !crosses(&ctx, &method),
            "a method entry crossed with no table to look it up in"
        );
    }

    /// Everything a placement crosses with is `Send`, asserted where a change
    /// that broke it would otherwise fail deep inside `worker::post`'s bounds.
    ///
    /// It is the whole boundary this module is built around: a `Program` and a
    /// `Value` may not leave the core that made them, so what crosses is the
    /// entry's names, the argument's bytes, the seed's shared handles and the
    /// answer's bytes — and nothing that is added to those types later may be a
    /// `Value` again.
    /// A placement's trace event names `spawn worker`, which is the one of
    /// `rule:observability/spawn-is-its-own-event`'s three forms that says a
    /// core was crossed.
    ///
    /// The same source line is a `spawn script` when it stays here, so a
    /// reading that spells both the same way can say what a child cost but not
    /// what the crossing did — which is the split the rule exists for.
    ///
    /// The entry names a method this program declares nothing under, so the far
    /// core answers a refusal rather than a value. That is the child's business
    /// and not the event's: the parent opens the event where it posts the work
    /// and closes it at the join it reached, whichever way the child ended.
    #[test]
    fn a_placed_childs_event_names_the_worker_form() {
        let mut sched = Scheduler::new();
        let _installed = reactor::install(Reactor::new().expect("the OS refused a poll"));

        let recorded: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let collected = Rc::clone(&recorded);
        sched.spawn(with_a_class_table(), TaskRoot::Worker, move |ctx| {
            ctx.set_debug_flags(DebugFlags::TRACE);
            let entry = Entry::Method {
                label: "Work::run".to_owned(),
                names: Vec::new(),
            };
            let destination = destination_for(ctx, &entry).expect("a task on a core secured none");
            let running = start(
                destination,
                ctx,
                entry,
                Value::null(),
                Output::Capture,
                Narrowing::default(),
            )
            .expect("the argument crosses");
            running.join(ctx);
            collected.borrow_mut().extend(
                ctx.trace()
                    .iter()
                    .filter(|event| event.kind == TraceKind::Spawn)
                    .map(|event| event.callee.clone()),
            );
        });
        run_until_idle(&mut sched).expect("the loop failed");

        let events = recorded.borrow();
        assert_eq!(
            events.len(),
            1,
            "one event per placement, not one per await"
        );
        assert!(
            events[0].starts_with("spawn worker started="),
            "a placement read as another construct: {}",
            events[0]
        );
        assert!(
            events[0].contains(" joined="),
            "the join left the placement's event open: {}",
            events[0]
        );
    }

    #[test]
    fn what_crosses_is_send() {
        const fn assert_send<T: Send>() {}
        assert_send::<Crossing>();
        assert_send::<Crossed>();
        assert_send::<PlacedIsolate>();
    }
}
