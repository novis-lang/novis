//! A `spawn script` **path** placed `on: "worker"`: the entry form whose code is
//! compiled on the core that runs the child.
//!
//! `rule:concurrency/on-worker-runs-the-child-on-another-core`'s *Both entry
//! forms cross*, end to end and through the seam a program reaches it by:
//! `nvs_runtime::host::Host::start_isolate`, the same call
//! `Core\Script::spawn` makes. What a unit test in `crates/nvs-host/src/placed.rs`
//! can ask is whether a path *would* cross; what only this can ask is what the
//! far core does with one when it gets there.
//!
//! **Why a test binary of its own.** `nvs_host::worker`'s destination set is one
//! `static` per *process*, and a core takes the published resolver as it starts
//! and never again. A path entry landing on a core that some earlier test in the
//! same binary started with nothing published would answer
//! `ResolveError::NoResolver` — at random, depending on the order cargo ran
//! them in. Every core in this process is started by a test below, each of which
//! publishes before it places anything, and the lock they share is what keeps a
//! publication from being withdrawn under a placement in flight.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::ThreadId;

use nvs_runtime::host::{Completion, Entry, Narrowing, Output, Placement};
use nvs_runtime::script::{Program, Resolver, SharedResolver, publish};
use nvs_runtime::{ClassTable, Ctx, ErrorClass, FieldDefault, OutputSink, TaskRoot, Value};

/// The one path [`Scripts`] compiles. Absolute, because `script::resolve`
/// refuses a relative path before any resolver is asked.
const CHILD: &str = "/srv/child.nvs";

/// A path it does not, which is what a resolver refusing one looks like from the
/// far core: the program never exists, so no line of the child runs.
const MISSING: &str = "/srv/gone.nvs";

/// One test at a time, because what they arrange is process-wide: the published
/// resolver is one slot for the process, and a guard dropped while another
/// test's placement is deciding whether a path can cross would decide it wrongly.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// The thread each child ran on, in the order they ran.
///
/// Written by the program [`Scripts`] compiles, on whichever core prepared it,
/// which is the fact these cases are about: a placed child runs on a core other
/// than its parent's.
static RAN_ON: Mutex<Vec<ThreadId>> = Mutex::new(Vec::new());

/// The resolver this process publishes: one script, compiled wherever it is
/// asked for.
///
/// A closure rather than anything compiled, which is all a `Program` is
/// (`nvs_runtime::script::Program`) — the questions here are where it is built
/// and where it runs, and neither of those is a question about the language.
#[derive(Debug)]
struct Scripts;

impl Resolver for Scripts {
    fn resolve(&self, path: &str) -> Result<Program, String> {
        if path != CHILD {
            return Err(format!("`{path}` names no script this test compiled"));
        }
        Ok(Box::new(|_ctx: &mut Ctx, args: Value| {
            ran_on().push(std::thread::current().id());
            // The argument came across as bytes and is read back into this
            // core's arena; the answer makes the same trip the other way.
            Value::int(args.as_int().unwrap_or(-1) + 1)
        }))
    }
}

/// The lock above, taken with a poisoned one taken anyway: a test that panicked
/// holding it has left a `()` behind, and the next one is no worse for it.
fn serially() -> MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner)
}

/// [`RAN_ON`], on the same terms, and never held across a placement — the child
/// takes this lock on its own core.
fn ran_on() -> MutexGuard<'static, Vec<ThreadId>> {
    RAN_ON.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A context a `spawn script` can be written on: the `script.spawn` door open,
/// and a class table for the entry to be resolved against.
///
/// Both are asked before a placement is secured —
/// `nvs_host::placed::destination_for`'s own doc lists them — so a context
/// missing either would fall through to this core and pin nothing.
fn spawning() -> Ctx {
    const SLOTS: [&str; 4] = ["message", "previous", "backtrace", "location"];
    let mut table = ClassTable::new();
    let root = table.define("Throwable", &SLOTS, &[]);
    let mut snapshot = nvs_config::Snapshot::default();
    snapshot.config.capabilities = Some(nvs_config::tree::Capabilities {
        script: Some(nvs_config::tree::CapScript {
            spawn: Some(nvs_config::tree::Setting::Bool(true)),
        }),
        ..nvs_config::tree::Capabilities::default()
    });
    let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
    ctx.set_runtime_error_class(ErrorClass::new(Arc::new(table), root));
    ctx.install_statics(Arc::from(vec![Some(FieldDefault::Int(7))]));
    ctx.set_config(Arc::new(snapshot));
    ctx
}

/// Spawns one child through the host seam and answers what it completed with.
///
/// The loop is the one a server runs (`crates/nvs-cli/src/serve.rs`) and it is
/// not decoration: a parent waiting for another core is a *parked* task, so a
/// single `run_until_idle` returns with the answer still in flight. `parked == 0`
/// is what says the tree is done.
///
/// The resolver is **installed on this thread as well as published**, which is
/// the pair `nvs-cli` arranges for the core it booted on: publishing is what a
/// core started later reads as it starts, and installing is what the thread
/// running now compiles through. A child left `on: "here"` is compiled by the
/// second, and answers `ResolveError::NoResolver` without it.
fn spawn_script(entry: Entry, placement: Placement, argument: i64) -> Completion {
    SharedResolver::new(Arc::new(Scripts)).scoped(|| placed_or_here(entry, placement, argument))
}

/// [`spawn_script`]'s body, under the resolver it installed.
fn placed_or_here(entry: Entry, placement: Placement, argument: i64) -> Completion {
    let mut sched = nvs_host::Scheduler::new();
    let _installed =
        nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
    let answered: Rc<RefCell<Option<Completion>>> = Rc::new(RefCell::new(None));
    let collected = Rc::clone(&answered);
    sched.spawn(spawning(), TaskRoot::Request, move |ctx| {
        let started = nvs_runtime::host::with_current(|host| {
            host.start_isolate(
                ctx,
                entry,
                Value::int(argument),
                Output::Capture,
                placement,
                Narrowing::default(),
            )
        });
        let running = started
            .expect("a task on a core reaches this crate's host")
            .expect("the argument crosses, and a path that cannot be compiled fails on the child");
        *collected.borrow_mut() = Some(running.join(ctx));
    });
    loop {
        let report = nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        if report.parked == 0 {
            break;
        }
    }
    let completion = answered.borrow_mut().take();
    completion.expect("the spawning task never reached its join")
}

/// A path entry is started on a core other than the one that wrote it, and its
/// answer comes back.
///
/// The rule's *started* is the whole of the placement, so the thread the program
/// ran on is the assertion. That it answers at all is the other half: the
/// argument crossed as bytes, the far core built the program from the published
/// resolver, and what it returned crossed back the same way.
#[test]
fn a_path_entry_is_placed_on_another_core() {
    let _serial = serially();
    let _published = publish(SharedResolver::new(Arc::new(Scripts)));
    ran_on().clear();

    let completion = spawn_script(Entry::Path(CHILD.to_owned()), Placement::Worker, 41);

    assert!(
        completion.ok,
        "the child failed where it was placed: {:?}",
        completion.error
    );
    assert_eq!(
        completion.value.as_int(),
        Some(42),
        "the argument or the answer did not survive the crossing",
    );
    let ran = std::mem::take(&mut *ran_on());
    assert_eq!(ran.len(), 1, "one child, run once");
    assert_ne!(
        ran[0],
        std::thread::current().id(),
        "a child asked for a core ran on the one that spawned it",
    );
}

/// The same path, placed and left here, answers the same thing.
///
/// Which core ran a child is not readable from the child
/// (`rule:concurrency/on-worker-runs-the-child-on-another-core`), so the two
/// completions are the observable equality the rule promises, and the threads
/// underneath them are what says the placement happened at all. A case asserting
/// only the first would pass against a placement that quietly fell through to
/// this core, which is the failure the goal's standing decisions forbid.
#[test]
fn a_placed_path_child_answers_what_a_same_core_child_answers() {
    let _serial = serially();
    let _published = publish(SharedResolver::new(Arc::new(Scripts)));
    ran_on().clear();

    let here = spawn_script(Entry::Path(CHILD.to_owned()), Placement::Here, 41);
    let placed = spawn_script(Entry::Path(CHILD.to_owned()), Placement::Worker, 41);

    assert_eq!(placed.ok, here.ok, "one of the two forms failed");
    assert!(here.ok, "the same-core child failed: {:?}", here.error);
    assert_eq!(
        placed.value.as_int(),
        here.value.as_int(),
        "a placed child answered something its same-core twin did not",
    );
    assert_eq!(placed.value.as_int(), Some(42));

    let ran = std::mem::take(&mut *ran_on());
    assert_eq!(ran.len(), 2, "two children, each run once");
    assert_eq!(
        ran[0],
        std::thread::current().id(),
        "`on: \"here\"` left the core it was written on",
    );
    assert_ne!(
        ran[1],
        std::thread::current().id(),
        "`on: \"worker\"` stayed on the core it was written on",
    );
}

/// A path the far core's resolver cannot compile is the child failing, and it
/// reaches the parent as a value.
///
/// The goal's standing decisions: a placement that cannot be honoured surfaces
/// as `ok = false` with the reason in `error.message`, never as a silent
/// fall-through to this core and never as an unwind —
/// `rule:security/isolate-failure-is-a-value` across a thread. The parent
/// reaching the assertions below is that second half.
#[test]
fn a_placed_path_no_resolver_can_compile_fails_as_a_value() {
    let _serial = serially();
    let _published = publish(SharedResolver::new(Arc::new(Scripts)));
    ran_on().clear();

    let completion = spawn_script(Entry::Path(MISSING.to_owned()), Placement::Worker, 0);

    assert!(
        !completion.ok,
        "a path no resolver compiled answered as though it had run",
    );
    let failure = completion
        .error
        .expect("a failed completion names why it failed");
    assert!(
        failure.message.contains(MISSING),
        "the refusal does not name the path it refused: {failure:?}",
    );
    assert!(
        ran_on().is_empty(),
        "no line of the child runs when its program was never built",
    );
}
