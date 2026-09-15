//! The seam a `Core` member reaches its host through — one thread-local, one
//! trait, and one operation shaped like the promise it has to keep.
//!
//! Plus [`Host::sleep`], which is the one thing a member can want from a core
//! that is not a group; § 3 below owns why it is the exception.
//!
//! `rule:concurrency/a-child-belongs-to-the-calling-task`'s
//! `Core\Task::all` runs its fields as children of the calling task, and that
//! task lives on `nvs-host`'s scheduler. A `Core` member is a `nvs-stdlib`
//! helper. This module is what joins those two, and the decisions that join
//! them are recorded here because this is the file neither side can be read
//! without.
//!
//! # 1. The edge is inverted through this crate, not added between the two
//!
//! `nvs-stdlib` does **not** depend on `nvs-host`, and it may not start: the
//! signature registry lives in `nvs-stdlib`, so `nvs-types` and `nvs-codegen`
//! both depend on it, and an edge from there to the host would link `mio`,
//! `corosensei` and `core_affinity` into `nvs check` — a type checker carrying
//! a reactor and a coroutine library to answer a question about a signature.
//!
//! So the trait is declared *here*, in the crate both sides already depend on.
//! `nvs-host` implements it; `nvs-stdlib` calls it; neither names the other.
//! That is the same inversion `nvs-runtime` already runs on the other axis —
//! this crate's `ctx` module holds `Core\Cli\Text`'s name as a constant rather
//! than asking `nvs-stdlib` about it, "the dependency running `nvs-stdlib` →
//! `nvs-runtime` and not back".
//!
//! # 2. A thread-local, not a second opaque pointer in `Ctx`
//!
//! `nvs-host`'s `reactor` module made this same call for the reactor and its
//! module doc owns the general argument; what is repeated here is only the part
//! that decides it for a *scheduler*, because a `Core` member — unlike a
//! `std::io::Read` — does hold a [`Ctx`] and could have read a pointer out of
//! it.
//!
//! **A scheduler is per core; a `Ctx` is per request.** Putting it in the
//! context is one copy of the core's identity per in-flight request, written on
//! every spawn, to say something that is true of the whole thread — and `Ctx`
//! is `#[repr(C)]` with offsets compiled code loads inline, so a field there is
//! an ABI change rather than a struct change. The yielder is in `Ctx` because
//! it genuinely is per *task*: it points into the stack of the one coroutine
//! that is running, and there is no thread-wide answer to what it is. A host is
//! the opposite shape, so it takes the opposite route.
//!
//! [`install`] publishes one for as long as its guard lives — a worker does it
//! once, around everything it runs — and [`with_current`] is how a helper
//! borrows it. It nests and restores, so a scheduler driven from inside another
//! one's task does not clobber the outer one.
//!
//! # 3. What crosses is a group, not a task API
//!
//! [`Host`]'s central operation is [`Host::run_group`], and it takes a whole
//! group. It is deliberately **not** `spawn` / `wait` / `cancel` for
//! `nvs-stdlib` to sequence, and that is the decision worth the most here.
//!
//! `rule:concurrency/nothing-is-still-running-when-a-call-returns`'s guarantee — "control does not leave the call with work still
//! running" — is a property of the *sequence*, not of any one call in it. A
//! seam handing out task ids makes keeping it the caller's diligence again,
//! which is the exact failure `nvs_host::spawn_child` already refuses on the
//! parent link: it reads the running task rather than taking a parent, so § 1's
//! "each is a child of the calling task" is a property of the call. The same
//! argument applies one level up. A group crosses whole, the host owns the
//! waiting and the cancelling, and there is no order of seam calls a future
//! member could get wrong.
//!
//! It is also what lets both members share it: `Task::all` and `Task::map`
//! differ in how their jobs are *built* — a shape literal's fields against one
//! callback over an array — and in nothing about how they run.
//!
//! [`Host::sleep`] is the exception that proves the shape rather than a crack
//! in it: it is not a piece of a group's sequence, it is a member that has to
//! *wait* and would otherwise stall the core for every neighbour on it. Its own
//! doc owns why it is here, and a further method is a decision to make on the
//! same terms — what does this member wait for that a group cannot express —
//! rather than a slot to fill.
//!
//! [`Outcome`] is § 4's table, every row of it, and the last one is a variant
//! for a reason worth stating: "the calling task is cancelled" would be no
//! return at all if the caller could be unwound where it stands. It cannot.
//! The caller of a `Core` member is standing on an `extern "C"` frame, a forced
//! unwind may not cross one ([`crate::HelperFrame`]), so the host resumes the
//! caller and this call returns [`Outcome::Cancelled`] like anything else. The
//! member's answer is [`Ctx::cancel`], which is § 5's teardown by
//! `rule:errors/propagation`'s return status: no
//! `catch` sees it and no script code runs on the way out.
//!
//! # What it spends
//!
//! One machine word pair per thread — a null-checked wide pointer in a
//! thread-local, `const`-initialized and holding no `Drop` type, which is what
//! this crate's own allocator module requires of every one in it. It is
//! O(cores) and does not grow with requests served, per
//! `rule:programs/memory-priority`. A helper that
//! asks pays one thread-local load and one null test.

use std::cell::Cell;
use std::time::{Duration, Instant};

use crate::ctx::{Ctx, DeclaredHeader, TraceEvent};
use crate::graph::GraphError;
use crate::script::{Program, ResolveError};
use crate::throwable::Thrown;
use crate::value::Value;

/// One child of the group: what it runs, and the answer it hands back.
///
/// A boxed Rust closure rather than a Novis `callable` [`Value`], because the
/// two members build their jobs differently and the host has no business
/// knowing which: `Task::all` closes over one field's `fn` literal, `Task::map`
/// over the shared callback and one element. What the host is told is that this
/// runs on a child's stack with a child's context and produces a value.
///
/// **A job may be dropped without ever being called**, and the builder owes the
/// release in that case. A `limit` that never lets the last job start, a
/// sibling that threw, and an expired deadline all end the group with jobs
/// still queued, and the host drops them rather than running work whose result
/// § 4 has already decided it will not return. Anything a job captured that
/// owns a reference — the element `Task::map` closed over, the closure both
/// members call — must therefore be released by that capture's own [`Drop`] and
/// not only by the body, or a cancelled group leaks one reference per job it
/// never reached.
pub type Job = Box<dyn FnOnce(&mut Ctx) -> Value>;

/// `rule:concurrency/limit-and-deadline-are-the-only-bounds`'s
/// `{limit?: uint, deadline?: Duration}`, decoded.
///
/// `None` is that section's "unbounded" in both fields, which is why neither is
/// a number with a sentinel: a `limit` of `0` is a real bound meaning "run
/// nothing", and no `Duration` means "never".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bounds {
    /// How many children may be running at once, or `None` for all of them.
    pub limit: Option<u32>,
    /// When cancellation is *requested*, or `None` for no deadline of this
    /// group's own. § 4 is the one home for why that is not when the call
    /// returns.
    pub deadline: Option<Duration>,
}

/// How a group ended — `rule:concurrency/nothing-is-still-running-when-a-call-returns`
/// 's table, and the module docs own why its last row is a variant here
/// rather than an unwind.
///
/// Every variant is reached with **nothing still running**. That is the whole
/// of what the member promises and it is discharged on this side of the seam.
#[derive(Debug)]
pub enum Outcome {
    /// Every child returned. One answer per [`Job`], in the order given.
    Completed(Vec<Value>),
    /// A child threw, every sibling was cancelled, and the call waited for
    /// those cancellations. This is the **first** throw by completion order; a
    /// second is written by the host rather than handed back, because only one
    /// can propagate and neither may be swallowed.
    ///
    /// A [`Thrown`] rather than a [`Value`], because that is what a failure
    /// already is on both sides of this seam: it is what [`Ctx::take_thrown`]
    /// hands the host out of the child's context and what [`Ctx::raise`] takes
    /// from the member on the way out, so neither end has to unwrap an object
    /// pointer out of a tagged value and put it back. It may be null — the
    /// state a bare-message fault with no exception class installed leaves —
    /// and `raise` handles that as it already does everywhere else.
    Threw(Thrown),
    /// The deadline expired, every child was cancelled, and the call waited.
    /// The member turns this into `TimeoutError`; the host does not know that
    /// class.
    TimedOut,
    /// The **calling** task was cancelled, every child was cancelled, and the
    /// call waited — `rule:concurrency/nothing-is-still-running-when-a-call-returns`'s last row.
    ///
    /// A return rather than an unwind because the caller is standing on an
    /// `extern "C"` frame no unwind may cross ([`crate::HelperFrame`]); the
    /// module docs own that. The member's answer is [`Ctx::cancel`], which is
    /// [`Woken::Cancelled`]'s answer too, for the same reason.
    Cancelled,
}

/// How a wait ended — what [`Host::sleep`] answers.
///
/// Two variants because a park has two ways to end and a member has to tell
/// them apart: the clock reached the instant asked for, or the task was
/// cancelled while it waited. A cancelled task standing on script frames cannot
/// be unwound where it parked ([`crate::HelperFrame`]), so its host resumes it
/// instead, and this is what the resume says. The member's answer to
/// [`Woken::Cancelled`] is [`crate::Ctx::cancel`] and an ordinary return: that
/// call is `rule:concurrency/cancellation-runs-no-user-code`'s
/// teardown, asked of the same slow path a safepoint poll asks, and because it
/// is that rather than a throw, no `catch` sees it. The flag it raises is this
/// context's own: the word a poll reads is the request tree's, and one task
/// being cancelled is not the tree being stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Woken {
    /// The wait ran to the end of the duration asked for.
    Elapsed,
    /// The task was cancelled while it waited, and the wait is over early.
    Cancelled,
}

/// A one-shot handle that makes the task it was taken for runnable again.
///
/// A boxed closure rather than a type of its own because there is exactly one
/// thing a holder does with it — fire it — and because the host's own wake is
/// a task id plus the tree it belongs to, neither of which this crate can
/// spell. It is deliberately **not** `Send`: a task never migrates off its
/// core, so a wake that crossed a thread would be naming a task the receiving
/// scheduler does not own.
///
/// Firing one is a *hint*, exactly as `nvs-host`'s own wake is: the task may
/// already have been woken by someone else, or have ended, and being late is
/// not an error. So a member that parks re-checks the state it parked for
/// rather than treating a resume as an answer.
pub type Waker = Box<dyn FnOnce()>;

/// Where an isolate's `echo` ends up — [ADR
/// 0006](/docs/decisions/0006.md) § *Output is
/// captured by default*.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Output {
    /// On the result, as bytes. The default, because the alternative silently
    /// mixes another script's bytes into a response the parent is responsible
    /// for.
    #[default]
    Capture,
    /// Appended to the parent's own output stream **when the result is
    /// awaited**, which is what keeps the ordering deterministic under
    /// concurrency. The child buffers either way; the two options differ only in
    /// who is handed the bytes at the end.
    Inherit,
}

/// What a spawn **named**, in whichever of [ADR
/// 0006](/docs/decisions/0006.md) § *Decision*'s two entry
/// forms it wrote — a path, or a class and a method.
///
/// The name and not the code, and that is the whole of why this type is here.
/// A [`Program`] is a boxed closure over whatever the *parent's* resolver
/// built, so it is neither `Send` nor meaningful anywhere but the core that
/// made it, while a path and a label are strings. Carrying the name lets the
/// core that is going to **run** the child be the one that prepares it, which
/// is what `rule:concurrency/on-worker-runs-the-child-on-another-core` needs
/// and what `on: "here"` gets for free — [`Entry::program`] is that step, and
/// it happens on the far side of the seam.
///
/// It also decides which constructor builds the child's context, which is a
/// difference nothing above the seam can act on. A path entry has a unit of its
/// own, whose `install_in` arms the child's statics from inside the program; a
/// method entry has none — its code is the parent's unit's — so its context has
/// to be armed at construction, which is [`Ctx::method_isolate`]. Everything
/// else about the two is identical, which is why this is a parameter of the
/// start rather than a second operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// The path a `.nvs` file was named by, for [`crate::script::resolve`] to
    /// turn into code — the form `rule:security/isolate-shares-nothing` is
    /// written around, and the one a resolver answers.
    Path(String),
    /// A `static` method of the unit the parent is already running, named by
    /// the label `nvs-codegen` emitted it under and the parameter names its
    /// `args:` map binds by. `crate::script`'s module doc is the one home of
    /// why that needs no resolver and no second unit, and
    /// [`crate::script::method_program`] is the code it becomes.
    Method {
        /// `"{class}::{method}"`, which [`crate::call_static_bound`] looks up.
        label: String,
        /// The entry's parameters in declaration order, empty for an entry that
        /// declares none.
        names: Vec<String>,
    },
}

impl Entry {
    /// Whether this names a `static` method of the parent's unit rather than a
    /// file of its own.
    #[must_use]
    pub fn is_method(&self) -> bool {
        matches!(self, Self::Method { .. })
    }

    /// Turns the name into the code to run, **on the thread that will run it**.
    ///
    /// Called by the implementor of [`Host::start_isolate`] rather than by the
    /// member that reached it, so that a child placed on another core is
    /// resolved there. `ctx` is the context the spawn is charged to, and it is
    /// read for one thing only: `rule:security/capability-check-at-the-door`'s
    /// `script.spawn` grant, which [`crate::script::resolve`] asks with the
    /// path as its scope.
    ///
    /// # Errors
    ///
    /// [`ResolveError`], for the path form alone. A method entry is code the
    /// parent's unit already holds, so there is nothing left to refuse.
    pub fn program(self, ctx: &Ctx) -> Result<Program, ResolveError> {
        match self {
            Self::Path(path) => crate::script::resolve(ctx, &path),
            Self::Method { label, names } => Ok(crate::script::method_program(label, names)),
        }
    }
}

/// Why no isolate was started — the two questions [`Host::start_isolate`] asks
/// before there is a child, and the only two it can answer with.
///
/// Separate variants because they are different mistakes. The argument is the
/// parent's: it built a value with no meaning on the other side, and
/// `rule:security/isolate-values-cross-by-copy` names what may cross. The entry
/// is the spawn site's, or the configuration's, and every one of its shapes is
/// a [`ResolveError`]. What a caller words each as is the caller's, because the
/// construct it is reporting for — `spawn script`, an upgrade — is the spelling
/// a program recognises.
#[derive(Debug)]
pub enum StartError {
    /// The argument had no meaning on the other side, and nothing was started.
    Argument(GraphError),
    /// The entry did not become a [`Program`]: no resolver, a path the grant
    /// does not cover, or one that did not compile.
    Entry(ResolveError),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Argument(error) => error.fmt(f),
            Self::Entry(error) => error.fmt(f),
        }
    }
}

/// Which core a spawn starts its child on —
/// `rule:concurrency/on-worker-runs-the-child-on-another-core`'s `on:`.
///
/// It reaches the seam because it is a scheduler's question and nothing above
/// one can answer it: the word is written at the spawn site and checked there
/// (`nvs_types::expr::isolate`'s `check_placement`), and the core it names is
/// decided **once**, at the start. A task never migrates
/// (`rule:concurrency/a-wake-never-moves-a-task`), so nothing downstream of the
/// start revisits it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Placement {
    /// The parent's own core, which is where a child that named no placement
    /// runs.
    #[default]
    Here,
    /// A core other than the parent's. The child is its parent's child in every
    /// other respect — the same boundary, the same tree's budget, the same
    /// death with its parent — and what the crossing costs is the move: the
    /// argument and the answer are copied at every node in both directions,
    /// because a refcount is non-atomic.
    Worker,
}

/// The two narrowings a spawn site wrote, as the child is to be held to them —
/// `rule:security/isolate-budget-is-the-trees`' sub-cap and
/// `rule:security/isolate-shares-nothing`'s grant list.
///
/// **Text, and not numbers.** A sub-cap is written in the spelling `nvs.toml`
/// writes the same directive in, and what applies it is the child's own
/// configuration overlay: `nvs_config::Request::set` already refuses a value
/// wider than what is in force, so carrying the word gets *tighter than what
/// remains, never wider* out of one reader rather than a second one grown
/// beside it. A count is rendered to that same spelling for the same reason.
///
/// A default is the spawn that narrowed nothing, which is what
/// [`Host::start_isolate`] is handed for a program that wrote neither option.
/// The empty grant list is **not** that: `Some(vec![])` is a child that may ask
/// for no capability at all, where `None` leaves the parent's set as it stands.
///
/// **What it spends** (`rule:programs/memory-priority`): one short `Vec` per
/// spawn that wrote an option, released once the child's context is built, and
/// one empty `Vec` — no allocation — for every spawn that wrote neither.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Narrowing {
    /// `[limits]` keys and the values written beside them, in the order the
    /// program wrote them.
    pub limits: Vec<(String, String)>,
    /// The capability names the child may still ask for, and `None` for a spawn
    /// that named no `grants:`.
    pub grants: Option<Vec<String>>,
}

/// What a child's failure looks like on the parent's side: data, never an
/// exception object (ADR 0006 § *Failure is a value*).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    /// The rendered class name of what the child threw, or `Error` where the
    /// failure was not a throw at all.
    pub class: String,
    /// The message, rendered on the child's side. Copying the exception
    /// *object* across is what `rule:security/isolate-shares-nothing` rejected.
    pub message: String,
}

/// What an isolate answers with — the native half of the `ScriptResult` the
/// language surface presents.
#[derive(Debug)]
pub struct Completion {
    /// True exactly when the child ended **without failing** and its answer
    /// crossed — which is the last top-level statement having run, an `exit`,
    /// and a `Core\Script::finish()`, since none of the three is a failure.
    ///
    /// So this is not the question "may this child's after-response work run":
    /// `rule:concurrency/after-response-outlives-the-connection` admits one of
    /// those three and `nvs_host::isolate`'s gate reads [`Ctx::ending`] beside
    /// this flag to tell them apart.
    pub ok: bool,
    /// The child's returned value, copied into the caller's ownership root.
    /// `null` whenever `ok` is false.
    ///
    /// **One reference, and collecting a `Completion` is taking it on.** The
    /// copy is made while the child's context is still alive and is rooted in
    /// the *caller's* ownership, so the arena teardown that follows reclaims
    /// nothing of it: a collector either moves this value somewhere that owns
    /// it — `Core\Script`'s `ScriptResult` is the one that does — or calls
    /// [`crate::release`] on it. A collector with nobody to answer, `nvs
    /// queue`'s worker being the one, is still a collector, and dropping the
    /// field there leaks a graph per child that returned one.
    pub value: Value,
    /// What the child wrote. Emptied by [`Output::Inherit`], which has already
    /// handed the bytes to the parent's own stream.
    pub output: Vec<u8>,
    /// What the child declared [`Self::output`] to *be* —
    /// `rule:security/response-body-is-one-typed-member`
    /// 's `Content-Type`, or `None` where nothing declared one.
    ///
    /// Beside the output rather than inside it, and on a `Completion` rather
    /// than on an HTTP type, because this is the one channel out of a finished
    /// isolate: `nvs-server` runs a request as one and never holds its `Ctx`.
    /// A child that is not answering a request still carries the field and
    /// nobody reads it, which is the same shape `Core\Server::isDraining`
    /// takes off a server.
    pub content_type: Option<Box<str>>,
    /// The file the child declared its body **is** — `Core\Response::sendFile`'s
    /// path, or `None` where the body is [`Self::output`].
    ///
    /// The field above owns the reasoning for both, and this is the one that
    /// carries a name instead of bytes: whoever answers opens it and streams it
    /// under the static-file policy, which is what makes a response of any size
    /// cost one path here. A child that sent a file wrote nothing, so
    /// [`Self::output`] is empty beside it.
    pub file_body: Option<Box<std::path::Path>>,
    /// What the child declared this response *means* — spec § 15's status code,
    /// or `None` where nothing set one.
    ///
    /// The field above owns the reasoning for both: this is the same one
    /// channel out of a finished isolate, carrying the second of the two words
    /// a response is declared with. A child that is not answering a request
    /// carries it and nobody reads it.
    pub status: Option<u16>,
    /// What else the child said this response carries — spec § 15's
    /// `setHeader` and `addCookie`, in the order it declared them, and empty
    /// where it declared none.
    ///
    /// The two fields above own the reasoning for all three; this is the one
    /// of them that is a list, because a header is a map and a status is a
    /// word. Whoever answers applies these **after** the headers it wrote for
    /// itself, which is the whole of what `rule:http-server/policy-headers-are-runtime-class-and-setheader-wins` means by an override,
    /// and applies each row the way [`DeclaredHeader::append`] says — a name
    /// this list carries twice is a peer that sees it twice.
    pub headers: Vec<DeclaredHeader>,
    /// Present exactly when `ok` is false.
    pub error: Option<Failure>,
    /// How long the child's own body ran, as the child measured it — the half
    /// `rule:observability/spawn-is-its-own-event`'s overhead split is computed
    /// against, so a spawn reads as real child compute beside the scheduling
    /// and copy-out cost around it rather than as one opaque total. It stops
    /// where the body does, ahead of the answer's crossing, which is the other
    /// side of that subtraction.
    ///
    /// `None` where the parent opened no spawn event. The gate
    /// `rule:testing/debug-probes` names is that a request nobody is observing
    /// reads no clock at all, so the parent asks once at the spawn and the
    /// child is handed the answer rather than reading the flags a second time.
    pub wall: Option<Duration>,
    /// What the child filed for
    /// `rule:observability/trace-events-carry-a-kind`'s kinds, moved off its
    /// context on the way out where `rule:observability/sampling-is-head-based`
    /// 's flag says somebody is recording this trace — and empty where nobody
    /// is, which is every request under a `[trace] sample` of `0.0` and every
    /// child that is not answering one.
    ///
    /// They leave on the completion for [`Self::content_type`]'s reason, and it
    /// is the load-bearing one here: this is the **one** channel out of a
    /// finished isolate, and a door that derives spans from a request it just
    /// answered never holds that request's [`Ctx`]. `nvs_server::trace::spans`
    /// is what derives them (`rule:observability/four-kinds-become-a-span`),
    /// where a `call` event among them becomes no span.
    ///
    /// **What it spends** (`rule:programs/memory-priority`): a sampled
    /// request's events, for as long as its completion is held, and nothing at
    /// all for an unsampled one — the vector is moved rather than copied, and
    /// it is moved only under the flag, so the request that pays is the one
    /// somebody asked to record. [`Ctx::take_sampled_trace`] is where the
    /// question is asked.
    pub trace: Vec<TraceEvent>,
}

impl Completion {
    /// Discharges [`Completion::value`]'s obligation for a collector that has
    /// nowhere to move it, leaving `null` in its place.
    ///
    /// This exists so that "nobody reads the answer" is spellable **safely**.
    /// `nvs-cli` denies `unsafe_code` outright, and the collector that most
    /// needs this — the queue worker, which runs a job for nobody — lives
    /// there, so without a safe discharge the only two options open to it were
    /// the crate's first `unsafe` block or the leak. Idempotent: the field is
    /// `null` afterwards, and releasing a `null` is a no-op.
    pub fn discard_value(&mut self) {
        // `take`, not a `replace` with `Value::null()`: `Value`'s `Default` is
        // that null, and clippy refuses the longer spelling of it.
        let value = std::mem::take(&mut self.value);
        #[expect(
            unsafe_code,
            reason = "the field carries exactly the one reference the copy out \
                      of the child rooted here, and it is replaced with `null` \
                      in the same breath so nothing can give it up twice"
        )]
        // SAFETY: `value` is the reference `finish` copied into this
        // completion's ownership, and the `replace` above is what makes this
        // the last read of it.
        unsafe {
            value.release();
        }
    }
}

/// An isolate that has already been started and has not been collected yet —
/// what `spawn script` answers with, and the only thing `await` consumes.
///
/// Opaque on purpose: everything an implementor holds for one is a task id and
/// a slot on its own scheduler, neither of which this crate can spell. What the
/// seam fixes is that the two halves exist separately, which is what makes
/// `spawn` and `await` two constructs rather than one blocking call wearing two
/// names.
///
/// **Dropping one without joining it is legal and is not a leak**: the child is
/// a task under the caller's own task, so it dies with it
/// (`rule:security/isolate-shares-nothing`'s "the
/// isolate is a child task"). What it costs is that the child's answer is
/// discarded rather than crossing, which is exactly what a program that spawned
/// and never awaited asked for.
pub trait Running: std::fmt::Debug {
    /// Waits until the child has ended and answers with what crossed back.
    ///
    /// Suspends the calling task while it waits, exactly as
    /// [`Host::run_group`] does, and for the same reason: control does not
    /// leave this call with the child still running. `ctx` is the **parent's**,
    /// and is borrowed rather than held so that an [`Output::Inherit`] child's
    /// bytes reach the parent's stream at the one point where the ordering is a
    /// fact rather than a race.
    fn join(self: Box<Self>, ctx: &mut Ctx) -> Completion;

    /// Whether the child has already ended, asked **without waiting** for it.
    ///
    /// The question a caller that may not suspend has to be able to ask, and
    /// the whole of why it exists: a `Future` polled by somebody else's loop —
    /// the built-in server's service, driven by `hyper` under
    /// `rule:concurrency/one-future-per-connection`
    /// — answers `Pending` while this is `false` and calls
    /// [`Running::join`] only once it is `true`, at which point that call has
    /// nothing left to wait for and does not park. A caller with no such
    /// constraint never asks: `join` is the whole of `await`.
    ///
    /// `false` says only that the child had not ended when it was asked. What
    /// tells a poller to ask again is a **wake** — the child's own, delivered
    /// to the task that started it as its body ends — and never a re-read of
    /// this on a loop, which would be the spin the seam exists to avoid.
    fn finished(&self) -> bool;

    /// Ends the child now, and does not return while it is still running.
    ///
    /// The counterpart to the plain drop above, for a caller whose *own* frame
    /// is the child's structure rather than its task: a future that owns a
    /// request may be dropped by the loop polling it while its isolate is still
    /// going, and letting that drop return with the child running would leave
    /// work outside anything that can be proven finished — which is exactly
    /// what `rule:concurrency/nothing-is-still-running-when-a-call-returns`
    /// refuses. So this cancels and then waits, as `join` does for a
    /// cancelled parent, and discards whatever the child had produced: nobody
    /// is left to read an answer.
    ///
    /// **The one case it may not wait for is its own teardown.** A stack being
    /// force-unwound is not a stack that may park, and it does not need to be:
    /// the child is a task under this one, so the scheduler cancels it as this
    /// task retires. An implementation is expected to make that test itself.
    fn abandon(self: Box<Self>);
}

/// Whatever is running tasks on this thread, as much of it as a `Core` member
/// is allowed to want.
///
/// Exactly one implementor is ever expected — `nvs-host`'s scheduler. The
/// module docs own why the trait is declared here rather than there, and why it
/// carries a whole group rather than a task API.
///
/// `Debug` is a supertrait so that [`Installed`] can derive it; an implementor
/// is expected to be a unit struct, since every scrap of a scheduler's state is
/// already reachable from the thread it belongs to.
pub trait Host: std::fmt::Debug {
    /// Runs `jobs` as children of the calling task under `bounds`, and returns
    /// only once none of them is still running.
    ///
    /// The calling task is suspended while they run, so this borrows its
    /// context rather than holding anything across the switch. What each child
    /// gets for a context, how the `limit` is applied, and what a cancellation
    /// waits for are all the implementor's, and `nvs-host`'s scheduler module
    /// is their home.
    fn run_group(&self, ctx: &mut Ctx, jobs: Vec<Job>, bounds: Bounds) -> Outcome;

    /// Gives the core back for `duration`, resuming the calling task no earlier
    /// than the end of it.
    ///
    /// The smallest thing that could be here: a member that waits for the
    /// *clock* has no readiness to register and no group to hand over, so it
    /// cannot reach a host through
    /// [`Host::run_group`] and would otherwise call
    /// [`std::thread::sleep`] — which stalls every task pinned to the same
    /// core, `rule:http-server/a-core-is-never-blocked-on-a-syscall`
    /// 's tier-B failure, where the requests that lose are the neighbours.
    /// It is also what a `Core\Task::all` under a `limit` needs in order to be
    /// a shaper at all: with a blocking sleep no two children ever overlap, so
    /// the limit is unobservable and so is the concurrency it bounds.
    ///
    /// No `Ctx`, because a park is the task's own business and the yielder is
    /// reached from the thread rather than from the context — `nvs-host`'s
    /// `reactor` module owns that route. A host with no task beneath the call
    /// still owes the wait, and blocking is the right answer there.
    ///
    /// [`Woken::Cancelled`] is how a cancellation reaches the member, and it is
    /// the whole reason this answers anything at all. A task parked here is
    /// standing on an `extern "C"` helper frame, so its host may not unwind it
    /// ([`crate::HelperFrame`]); it resumes the task instead, and the member
    /// turns that into [``rule:errors/propagation``](/docs/decisions/0002.md)'s
    /// return status at the next safepoint. A host with no task beneath the
    /// call still owes the wait, blocking is the right answer there, and it
    /// answers [`Woken::Elapsed`] because nothing could have cancelled it.
    fn sleep(&self, duration: Duration) -> Woken;

    /// A handle that makes the **calling** task runnable again, once, or
    /// `None` when there is no task beneath the call.
    ///
    /// Taken *before* the state that will be waited on is released, and fired
    /// by whoever changes that state. That order is the whole contract: a
    /// handle taken after the release could be registered by a task the peer
    /// has already run past, which is the one way a park becomes a hang.
    ///
    /// `None` is what a member with nowhere to park refuses on, and it is the
    /// reason this is separate from [`Host::park`] rather than folded into it:
    /// a park with no task under it can only block the core, which
    /// `rule:http-server/a-core-is-never-blocked-on-a-syscall`
    /// forbids outright, so the member has to learn there is no task
    /// *before* it commits to waiting.
    fn waker(&self) -> Option<Waker>;

    /// Gives the core back until one of this task's [`Waker`]s fires,
    /// `deadline` passes, or the task is cancelled.
    ///
    /// [`Woken::Elapsed`] here means "the wait ended for its own reason" —
    /// a waker fired, the deadline came up, or nothing could hold the task
    /// parked at all. There is no third answer to give: a waker is a hint, so
    /// the caller re-checks the state it was waiting on either way, and a park
    /// that could not be entered is then one more turn of that loop rather than
    /// a case of its own. **The deadline is no exception to that**, and that is
    /// why it is not a variant: the caller already holds the instant it passed
    /// in and reads its own clock against it, which is an answer no wake can
    /// race and no host has to be trusted for. [`Woken::Cancelled`] is the only
    /// answer that means something different, and it means what it means for
    /// [`Host::sleep`]: the task is standing on an `extern "C"` helper frame
    /// ([`crate::HelperFrame`]), so its host resumed it rather than unwinding
    /// it, and the member's answer is [`Ctx::cancel`].
    ///
    /// `None` is a wait with no bound on it, for a member waiting on state only
    /// a peer can change; `Some` bounds the same wait without changing what it
    /// waits *for*, which is why this is one method and not two. A deadline
    /// already in the past returns without parking at all.
    ///
    /// This is deliberately not [`Host::sleep`] with a wake bolted on. A sleep
    /// is a wait *for the clock* and may not end early; this is a wait for a
    /// peer that may not run forever. A member that has both — `Core\Db`'s
    /// `acquire`, waiting for a pooled connection under a timeout — has exactly
    /// one thing to call.
    fn park(&self, deadline: Option<Instant>) -> Woken;

    /// Starts what `entry` names as an isolate under the calling task and
    /// answers with the handle that collects it later.
    ///
    /// This is the half of [ADR
    /// 0006](/docs/decisions/0006.md)'s spawn that
    /// only a scheduler can do, and it is deliberately **eager**: the child is
    /// a runnable task before this returns, so a parent that spawns three and
    /// then awaits three overlaps them. Deferring the start to the join would
    /// make `spawn`/`await` a pair of names for one blocking call and would buy
    /// the language nothing for the second construct it charges a reader for.
    ///
    /// `entry` is the child's program **named and not carried** — the path, or
    /// the class and method — and [`Entry::program`] is what this side turns it
    /// into, because the core that runs a child is the core that prepares it.
    /// `placement` says which core that is. Those are the facts about a child
    /// only this side can act on; see [`Entry`] and [`Placement`].
    ///
    /// `narrowing` is what the spawn site wrote to hold the child to less than
    /// its parent holds — [`Narrowing`], applied to the child's own context
    /// before its first statement runs. It reaches the seam rather than being
    /// applied above it because the context it narrows is built here, and a
    /// narrowing applied anywhere else would be one the placed child on another
    /// core never saw.
    ///
    /// `args` is **transferred on the success path**, where the graph copy
    /// takes the one reference the caller handed over. An `Err` leaves that
    /// reference with the caller, whose own fault edge releases it:
    /// `nvs_ir::lower`'s `lower_spawn_script` is the one `CoreCall` site that
    /// keeps its transferred temporary on the stack across the call rather than
    /// forgetting it first, and it does that for exactly this. `ctx` is the
    /// parent's, and is borrowed only for the
    /// length of the call: the isolate's own ownership root is built here
    /// (`rule:security/isolate-teardown-is-a-drain-then-a-sweep`
    /// ) and is nothing the parent can reach.
    ///
    /// # Errors
    ///
    /// [`StartError::Entry`] when the name did not become code, which is asked
    /// **first**: a path outside the `script.spawn` grant is refused whether or
    /// not the argument could have crossed. [`StartError::Argument`] when the
    /// argument has no meaning on the other side. No child is started either
    /// way, which is what makes both the parent's fault to raise rather than
    /// `rule:security/isolate-shares-nothing`'s failure-is-a-value; the refusal on
    /// the way *back* is an `ok = false` instead, and `nvs-host`'s `isolate`
    /// module owns that asymmetry.
    fn start_isolate(
        &self,
        ctx: &mut Ctx,
        entry: Entry,
        args: Value,
        output: Output,
        placement: Placement,
        narrowing: Narrowing,
    ) -> Result<Box<dyn Running>, StartError>;
}

thread_local! {
    /// The host running tasks on this thread, or `None` when nothing is.
    ///
    /// A `Cell` of a `Copy` wide pointer so it is `const`-initialized and
    /// carries no destructor — see the module docs' *What it spends*, and
    /// the `alloc` module for why that is a rule in this crate rather than a
    /// preference.
    static CURRENT: Cell<Option<&'static dyn Host>> = const { Cell::new(None) };
}

/// Restores whatever host was installed before, when dropped.
///
/// Held on the installer's own stack, which is why the `Drop` here does not
/// break the thread-local's no-destructor rule: the slot holds a pointer, this
/// guard holds the previous one.
#[derive(Debug)]
#[must_use = "the host is uninstalled the moment this guard is dropped"]
pub struct Installed {
    previous: Option<&'static dyn Host>,
}

impl Drop for Installed {
    fn drop(&mut self) {
        CURRENT.with(|slot| slot.set(self.previous.take()));
    }
}

/// Publishes `host` as this thread's host for as long as the guard lives.
///
/// Nesting is restoration, not replacement: a scheduler driven from inside
/// another one's task installs over it and puts the outer one back on the way
/// out.
pub fn install(host: &'static dyn Host) -> Installed {
    let previous = CURRENT.with(|slot| slot.replace(Some(host)));
    Installed { previous }
}

/// Borrows this thread's host, or answers `None` when there is none.
///
/// `None` is not a fault. A `nvs run` of a CLI program has no scheduler under
/// it, and a member that needs one says so through its own diagnostic rather
/// than assuming one is there.
pub fn with_current<R>(f: impl FnOnce(&dyn Host) -> R) -> Option<R> {
    CURRENT.with(Cell::get).map(f)
}

/// Whether this thread has a host, without borrowing it.
#[must_use]
pub fn is_installed() -> bool {
    CURRENT.with(|slot| slot.get().is_some())
}

#[cfg(test)]
mod tests {
    use super::{
        Bounds, Duration, Host, Installed, Instant, Job, Outcome, Woken, install, is_installed,
        with_current,
    };
    use crate::ctx::{Ctx, OutputSink};

    /// A host that answers every group with an empty completion and counts the
    /// calls, which is all the route itself has to be proved against.
    #[derive(Debug)]
    struct Recording(std::cell::Cell<u32>);

    // The trait is `&'static dyn Host`, and a `static` is the only way to hand
    // one over. `Cell` is not `Sync`, so this test's own implementor is a
    // thread-local rather than a plain `static` — which is also the shape a
    // real host has, its state being per core.
    thread_local! {
        static RECORDING: &'static Recording =
            Box::leak(Box::new(Recording(std::cell::Cell::new(0))));
    }

    impl Host for Recording {
        fn run_group(&self, _ctx: &mut Ctx, jobs: Vec<Job>, bounds: Bounds) -> Outcome {
            self.0.set(self.0.get() + 1);
            assert!(bounds.limit != Some(0), "a limit of zero is a real bound");
            drop(jobs);
            Outcome::Completed(Vec::new())
        }

        fn sleep(&self, _duration: Duration) -> Woken {
            // Counted with the groups: what the route has to prove is that the
            // call arrives, and a test that really waited would only be slow.
            self.0.set(self.0.get() + 1);
            Woken::Elapsed
        }

        fn waker(&self) -> Option<super::Waker> {
            // No task under this host at all, which is exactly the answer a
            // member with nowhere to park has to be able to read.
            None
        }

        fn park(&self, _deadline: Option<Instant>) -> Woken {
            self.0.set(self.0.get() + 1);
            Woken::Elapsed
        }

        fn start_isolate(
            &self,
            ctx: &mut Ctx,
            entry: super::Entry,
            args: crate::value::Value,
            _output: super::Output,
            _placement: super::Placement,
            _narrowing: super::Narrowing,
        ) -> Result<Box<dyn super::Running>, super::StartError> {
            // No scheduler here, so "started" is "already finished": the route
            // is what this proves, and a boundary needs a task tree that a unit
            // test of the seam does not have. The name becomes code on this
            // side, exactly as it does on a scheduler's.
            self.0.set(self.0.get() + 1);
            let program = entry.program(ctx).map_err(super::StartError::Entry)?;
            let value = program(ctx, args);
            Ok(Box::new(Started(std::cell::Cell::new(Some(value)))))
        }
    }

    /// The `Recording` host's handle: the answer, held until it is joined.
    #[derive(Debug)]
    struct Started(std::cell::Cell<Option<crate::value::Value>>);

    impl super::Running for Started {
        fn join(self: Box<Self>, _ctx: &mut Ctx) -> super::Completion {
            super::Completion {
                ok: true,
                value: self.0.take().unwrap_or_else(crate::value::Value::null),
                output: Vec::new(),
                content_type: None,
                file_body: None,
                status: None,
                headers: Vec::new(),
                error: None,
                wall: None,
                trace: Vec::new(),
            }
        }

        fn finished(&self) -> bool {
            // The comment above: with no scheduler under it, "started" and
            // "finished" are the same moment, so a poller never waits.
            true
        }

        fn abandon(self: Box<Self>) {}
    }

    fn recording() -> &'static Recording {
        RECORDING.with(|host| *host)
    }

    fn install_recording() -> Installed {
        install(recording())
    }

    #[test]
    fn a_thread_with_no_host_answers_none_rather_than_panicking() {
        assert!(!is_installed());
        assert!(with_current(|_| ()).is_none());
    }

    #[test]
    fn an_installed_host_is_reachable_from_a_free_function() {
        let before = recording().0.get();
        let _guard = install_recording();
        assert!(is_installed());

        // Exactly the shape a `Core` member has: no argument carries the host
        // in, and the call site names no scheduler type.
        let mut ctx = Ctx::new(OutputSink::Sink);
        let outcome = with_current(|host| host.run_group(&mut ctx, Vec::new(), Bounds::default()));

        assert!(matches!(outcome, Some(Outcome::Completed(answers)) if answers.is_empty()));
        assert_eq!(recording().0.get(), before + 1);
    }

    #[test]
    fn installing_nests_and_restores_rather_than_clobbering() {
        let outer = install_recording();
        assert!(is_installed());
        {
            let _inner = install_recording();
            assert!(is_installed());
        }
        assert!(is_installed(), "the outer host is back, not gone");
        drop(outer);
        assert!(!is_installed());
    }
}
