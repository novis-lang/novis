//! `nvs-cli`'s half of [`nvs_runtime::script`]: the path a `spawn script`
//! wrote, compiled into a [`Program`] an isolate can run.
//!
//! The seam's own module doc owns why the compiler is reached this way round
//! rather than from `nvs-host`. This file owns what only an implementor can
//! decide.
//!
//! # Decision: a relative path is anchored at the working directory
//!
//! Not at the entry file.
//! `rule:config/an-application-is-its-entry-file-path`
//! makes an application an entry-file path and says nothing about a child, so
//! there was a choice to make, and the working directory is the one a reader of
//! the program can already predict: it is what every other path a CLI program
//! writes is relative to — `require`'s spelling excepted, which `rule:statements/require-is-the-only-inclusion-construct`
//! resolves against the requiring file because a library has to move as a unit.
//! A `spawn script` target is not a library; it is a second program, named the
//! way the shell that started this one would name it. `examples/isolate.nvs`
//! is written against exactly this and says so in its own comment.
//!
//! # Decision: one unit per written path, swapped when its content moves
//!
//! `rule:security/isolate-shares-nothing`'s "an isolate shares immutable compiled code" is a property of this
//! cache and of nothing else — the seam hands over a closure and has no opinion
//! about what is behind it. So a path is compiled once and every later isolate
//! over it runs the same pages, which is what makes spawning a child cheap
//! enough to be worth doing, until the file behind it changes.
//!
//! It is keyed by the path **as written**, so two spellings of one file compile
//! twice. Canonicalizing would buy the sharing back at the cost of a syscall on
//! every spawn and of a failure mode before the front end has run — and what a
//! program controls is its own text, which is the thing this key already is.
//!
//! **What it spends:** at most three compiled units per distinct written path —
//! the one in force, the one it replaced, so that undoing the last change is a
//! pointer swap and not a compile, and, while an edit does not compile, the
//! failure the next resolve of that same content is answered with — plus, per
//! unit, its key and its [`Trace`]. O(the program's text), never
//! O(isolates spawned) and never O(edits), per
//! `rule:programs/memory-priority` — and freed with
//! the resolver, which is a local of `nvs run` published through
//! [`nvs_runtime::script::scoped`] rather than leaked.
//!
//! # Decision: `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s five steps, and the shared cache they make
//!
//! `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s § *Decision* is implemented here whole, because this is the
//! tree's only in-memory unit table: a [`PathEntry`] holding the digest and the
//! stamp the last check observed, in front of a table keyed by
//! [`UnitKey`]`{ path, program_digest, probe_hash, env_hash }`. A request for a
//! path with a pointer is step 1 alone: a map lookup, and no file-system call.
//! [`Compiler::revalidate`] walks the other four for every pointer, on
//! [`watch`]'s thread once per `revalidate_freq`: `stat`, and re-read the
//! source only where the stamp cannot answer; wait until the program has been
//! quiet for `settle`; compile only content this table has not seen; move the
//! pointer on success; leave it alone on failure, and name the failure the
//! path's requests are answered with ([`PathEntry::failed`]). A path with no
//! pointer yet is walked by the request that names it, as a cold compile.
//!
//! **A unit is keyed on its whole program, which only a compile can name**, so
//! the table is addressed in two moves rather than one. A program is every file
//! its `require`/`autoload` graph reached, and
//! `rule:packaging/autoload-probes-fold-into-the-cache-key` adds the paths its
//! `autoload` resolution probed — the misses included, because a file appearing
//! *in front of* the one that was reached changes the answer without touching a
//! byte anything hashed. Both lists exist only once the front end has run.
//! [`Compiler::traces`] is the index that closes the circle: a resolve reads the
//! [`Trace`] recorded for this path's entry-file content and spells its key
//! with it — the whole-program digest and the probe digest — and a content
//! nothing here has compiled yet spells the entry file's own digest and
//! [`ProbeHash::unrecorded`]. The compile then publishes its unit under the key
//! its own trace gives. What the second move re-keys is the table entry, so a
//! cold path still costs one compile and not two.
//!
//! **Every path a trace names is revalidated like the entry file.** A check
//! looks at every file the compile read, under the same `validate`, every path it
//! looked for and did not find, and every path its `autoload` resolution
//! probed. Where one of them moved, the trace is no longer current, and that
//! sends the content to a compile, because the key it would be answered under
//! is now nobody's. That covers the edits no entry-file digest moves: an edit
//! to a `require`d file, and writing `src/Thing.nvs` where `App\Thing` resolves
//! through `vendor/compat/Thing.nvs`. One entry-file content can have a trace
//! for each unit the table keeps, and a check that finds the current one moved
//! looks at the others before it gives up, so undoing an edit to a `require`d
//! file finds the unit compiled before it.
//!
//! **A failed compile records a trace too.** The front end reports what it read
//! and missed however it ends, so a broken `require`d file, or a deleted one,
//! is watched like any other, and the edit that fixes it reaches the next
//! resolve. Without that trace the failure would stay keyed to an entry file
//! that never changes.
//!
//! **A reader never waits behind a compile.** Both maps are [`RwLock`]s: a
//! resolve that hits takes the read half and contends with nothing, and the
//! write half is taken for a single `insert` *after* the front end and the
//! backend have already finished. **Neither guard is ever held across a
//! compile**, which is what makes step 3 re-entrant — the program being
//! compiled may itself `spawn script` back into this resolver — and is the
//! whole of what keeps a serving core free where that rule keeps a thread free
//! with a compile pool.
//!
//! **A compile is single-flighted, so the fleet pays for one.** That rule
//! specifies a `Compiling`/`Ready`/`Failed` broadcast every racing caller waits
//! on, and [`CompileState`] holds all three. The caller that claims a content's
//! key under the write guard is the one that compiles it; every caller that
//! arrives while that runs finds the [`Flight`] it left in the table, waits on
//! that, and is then answered out of the table like any other hit. This is what
//! makes `docs/plan/m7.md`'s "ten thousand cold requests compile it exactly
//! once" a claim about the fleet rather than about one core, and why
//! [`Compiler::compiles`] counts contents rather than workers.
//!
//! **A waiter gives up its thread, and that is the cheaper of its two
//! options.** The wait is a condition variable rather than a suspension, so the
//! core stops for as long as one compile — but what it replaces is that same
//! core running the same front end itself, which stops it for at least as long
//! and burns a whole compile doing it. Neither map's guard is held across
//! either, so nothing else this cache answers waits behind a flight.
//!
//! **What it spends:** one mutex and one condition variable per compile in
//! flight, dropped with the entry that compile's result replaces —
//! O(contents being compiled right now), which the cores bound, and never
//! O(requests waiting behind one).
//!
//! Step 4 races on the same terms: [`Compiler::advance`] publishes only where
//! the pointer still names what the resolve read on its way in, so the slower
//! of two revalidations cannot roll the fresher one back. Two revalidations
//! that observed *different* content are two compiles by definition, and the
//! loser's [`Compiler::record`] can still sweep the winner's entry out of the
//! unit table on the way past, costing the next resolve of that path one
//! recompile. What holds however a resolve arrives is the cheaper half of the
//! same claim: one landing on content that has already failed is answered from
//! the table rather than compiled again.
//!
//! **A `Ready` entry is one unit, published rather than copied.** [`Compiled`]
//! holds an [`Arc`] of an [`nvs_codegen::Unit`], and that `Unit` is [`Send`]
//! and [`Sync`] for the reasons its own type doc states, so every core
//! resolving the same content reads the pointer this cache published instead
//! of compiling the file for itself. That is what turns the locks above from
//! the shape of a shared cache into one: nothing in an entry is thread-affine,
//! and what a resolve hands back is a clone of the published pointer.
//!
//! **Every worker publishes to and reads from one of these.** `crate::serve`
//! builds it before it binds anything and hands each core it spawns a clone of
//! the [`Arc`], so the sharing above is what a served request actually reaches
//! rather than a shape only this module's own tests exercise.
//!
//! **A failure renders its spans once.** The front end writes diagnostics to
//! standard error as it compiles (see [`Resolver::resolve`]), so the resolve
//! that reached a broken edit is the one that printed it; a later resolve of
//! the same content gets the one-line summary out of the table. That is what
//! step 5's shared `Failed` entry means, and it is the difference between a
//! request storm against a broken file costing one compile and costing one per
//! request.
//!
//! This is also the mechanism
//! `rule:concurrency/connection-bounds-are-finite`'s
//! second bullet is a statement about. The swap is a write to the *table*: a
//! [`Program`] already handed out owns its unit's pages through its own `Arc`,
//! so a connection isolate runs to completion on the code it began with while
//! the next resolve of that path hands the new unit to whoever asks next.
//!

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{
    Arc, Condvar, Mutex, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak,
};
use std::time::{Duration, SystemTime};

use nvs_config::cache::{
    Digest, EnvHash, ProbeHash, Revalidation, UnitKey, Validate, content_hash, discovery_hash,
    env_hash,
};
use nvs_config::tree::Config;
use nvs_runtime::script::{Program, Resolver};
use nvs_runtime::{Ctx, Value};

/// One compiled unit, and the one compile product a *caller* of this cache
/// still needs beside it.
///
/// `rule:routing/matched-once-before-the-handler`'s table is not something the unit's code can be asked for: it
/// is matched against **before** any of that code runs, by the door, so it has
/// to be reachable without running the program. Holding it here is what makes
/// "the compiled unit's route table" a thing the server can have — the cache
/// that already answers "which unit serves this file" is the one place both
/// halves of that answer exist.
#[derive(Debug)]
pub(crate) struct Compiled {
    /// The unit itself, whose `Arc` is what keeps its pages mapped and what
    /// lets one compile of them serve every core.
    unit: Arc<nvs_codegen::Unit>,
    /// The routes it declared, already crossed into the runtime's own shape.
    /// Empty for a program with no `#[Route]`, which is `rule:routing/table-is-opt-in`'s opt-in
    /// rule and is one case rather than an `Option`'s two.
    routes: Arc<nvs_runtime::routes::Routes>,
}

/// What one written path resolved to last, and when that was checked — [ADR
/// 0017]'s `PathEntry`, the pointer an edit swaps.
///
/// Copied out of the map rather than read under a guard held across the `stat`
/// and the compile below it, which is why every field is [`Copy`]: a guard
/// spanning a front-end run would deadlock against the `spawn script` that run
/// may itself perform.
///
#[derive(Clone, Copy, Debug)]
struct PathEntry {
    /// The unit in force: the digest of the entry-file content this path last
    /// *compiled* to, and the two key fields that compile's [`Trace`] gave.
    unit: Generation,
    /// The unit in force before [`Self::unit`], which the table keeps beside
    /// it so that undoing the last change is a pointer swap and not a compile.
    /// `None` until the pointer has moved once.
    replaced: Option<Generation>,
    /// The failure a request of this path is answered with instead of
    /// [`Self::unit`], while the disk holds a program that does not compile
    /// (`rule:config/a-broken-edit-fails-the-requests-that-resolve-it`).
    /// `None` again once the pointer moves.
    failed: Option<Generation>,
    /// What `validate = "mtime"` compares against, and `None` where the file
    /// system answered with neither — a path whose stamp cannot be read is
    /// re-hashed rather than trusted. Always the stamp [`Self::unit`]'s
    /// content was observed with.
    stamp: Option<Stamp>,
}

impl PathEntry {
    /// What a request of this path is answered with: the failure, while there
    /// is one, and otherwise the unit in force.
    fn serving(&self) -> Generation {
        self.failed.unwrap_or(self.unit)
    }
}

/// One compiled program of one path: the entry file's digest, and the
/// whole-program digest and probe digest its [`Trace`] gave. With the path and
/// the environment it is a [`UnitKey`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Generation {
    content: Digest,
    program: Digest,
    probes: ProbeHash,
}

/// The traces [`Compiler::traces`] keeps for one path at one entry-file
/// content, and which of them that content is answered under now.
///
/// There can be more than one because the entry file is not the whole
/// program: an edit to a `require`d file and its revert leave the entry file's
/// digest where it was, and each of the two programs has a trace of its own.
#[derive(Debug, Default)]
struct Traced {
    /// The program digest and probe digest of the trace that still describes
    /// the disk, and `None` where the last check found every trace here moved.
    current: Option<(Digest, ProbeHash)>,
    traces: Vec<Trace>,
}

/// One stamp to keep per file, or per listed directory, in order.
type Stamps = Vec<Option<Stamp>>;

/// What a resolve gives its caller: the program and its route table, or the
/// one-line summary of why it did not compile.
type Answer = Result<(Program, Arc<nvs_runtime::routes::Routes>), String>;

/// The `mtime`/size pair `rule:config/an-edit-reaches-the-next-request-without-a-restart` calls the cheap
/// pre-filter: enough to say a file did *not* change, never enough to say what
/// it now holds.
///
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Stamp {
    modified: SystemTime,
    len: u64,
}

/// What one compile of one entry-file content read: the two key fields only it
/// could produce, and the paths a later resolve checks again.
///
/// A failed compile has one too, built from what the front end read before it
/// stopped, so that the edit fixing it is noticed.
///
/// **What it spends:** per unit, one path, one stamp and one digest for each
/// file the program read or missed, one path and one bool for each `autoload`
/// probe, and one path, one stamp and the listed names for each directory a
/// discovery scan listed. O(the program's files), never O(edits).
#[derive(Clone, Debug)]
struct Trace {
    /// The whole-program digest the unit is published under: the on-disk
    /// artifact cache's [`crate::cache::program_digest`] for a compile that
    /// reached the backend, and [`failed_program`]'s fold for one that did not.
    program: Digest,
    /// The other key field, [`discovery_hash`] over the probed paths and the
    /// listings below.
    probes: ProbeHash,
    /// Every path the `autoload` resolution probed, in probe order, with
    /// whether it existed
    /// (`rule:packaging/autoload-probes-fold-into-the-cache-key`).
    ///
    /// The bool is the **negative entry**, and it is the whole point of keeping
    /// the misses: a path probed and not found is what says the answer was a
    /// question, so a file created there later resolves differently with no
    /// file that was compiled having changed. A path that *was* found is kept
    /// on the same terms, because a deletion moves the answer to the next root
    /// just as an addition moves it to the earlier one.
    ///
    /// **The answers are taken as the front end returns**, not as it probed,
    /// so a file created between the probe and the end of that compile is
    /// recorded as having been there, and the unit stands until something else
    /// moves.
    answers: Vec<(PathBuf, bool)>,
    /// Every file the program read other than the entry file, which
    /// [`Compiler::paths`] already watches, and every path it looked for and
    /// did not find.
    files: Vec<Read>,
    /// Every directory a discovery scan listed
    /// (`rule:packaging/autoload-probes-fold-into-the-cache-key`): a
    /// `discover` glob's base, and every directory `implementing` walked.
    listed: Vec<Listed>,
    /// The entry file's path through every symlink and junction ([`real_path`]),
    /// which is the path the compile read the program through. Where the entry
    /// path resolves to another one now, a `current` link was switched, and the
    /// trace no longer describes the disk whatever the files behind it hold.
    real: PathBuf,
}

/// One directory a discovery scan listed, and what it held.
#[derive(Clone, Debug)]
struct Listed {
    dir: PathBuf,
    /// What `validate = "mtime"` compares against, with [`Read::stamp`]'s
    /// meaning: `None` re-lists the directory on the next check.
    stamp: Option<Stamp>,
    /// [`nvs_hir::autoload::listed_names`] as the compile saw it. A directory
    /// is re-listed when its stamp moves, and only a change here moves the
    /// unit, so a file the scan passes over recompiles nothing.
    names: Option<Vec<String>>,
}

/// One file a compile read, or one path it looked for and found nothing at.
#[derive(Clone, Debug)]
struct Read {
    path: PathBuf,
    /// What `validate = "mtime"` compares against. `None` means the next check
    /// re-reads the file: its stamp could not be read, or it was taken too
    /// close to the read to vouch for it ([`vouching`]).
    stamp: Option<Stamp>,
    /// The digest of the text the compile read, and `None` for a path it found
    /// nothing at.
    digest: Option<Digest>,
}

/// `rule:config/an-edit-reaches-the-next-request-without-a-restart` step 3's state machine, whole.
///
#[derive(Debug)]
enum CompileState {
    /// A compile of this content that some caller is running now, and the thing
    /// every other caller of it waits on instead of running a second one.
    Compiling(Arc<Flight>),
    /// The unit, and the route table beside it — one pointer, which is what
    /// every core resolving this content is handed a clone of.
    Ready(Arc<Compiled>),
    /// The one-line summary this content failed with, kept so that every later
    /// resolve landing on the same [`UnitKey`] is answered rather than
    /// recompiled.
    Failed(String),
}

/// One compile in flight, as the two things a caller waiting on it needs: a
/// flag it can read and a signal it can sleep on.
///
/// **It is only ever waited on from another thread.** Nothing between the claim
/// and the landing runs Novis code — the front end and the backend compile a
/// program, they do not execute one — so the caller holding a flight cannot
/// re-enter this resolver for the content it is compiling, which is the one
/// shape that would have it wait on itself.
///
/// It deliberately carries **no result**. What the compile publishes is the
/// [`CompileState`] under its own key, which is where every other path through
/// [`Compiler::compiled`] reads an answer from, and this only says when to go
/// and look. One copy of an answer cannot disagree with itself, and it is what
/// lets a caller woken by a compile that died mid-flight simply find nothing
/// and compile the content for itself.
#[derive(Debug, Default)]
struct Flight {
    landed: Mutex<bool>,
    lands: Condvar,
}

impl Flight {
    /// Blocks until the compile behind this flight has finished, whatever it
    /// finished as.
    ///
    /// Poison is stepped over for [`shared`]'s reason: the only thing under
    /// this mutex is a `bool` that goes one way, so a thread that panicked
    /// elsewhere has not made it untrue — and refusing to read it would wedge
    /// every later resolve of this content on a compile that already ended.
    fn wait(&self) {
        let mut landed = self.landed.lock().unwrap_or_else(PoisonError::into_inner);
        while !*landed {
            landed = self
                .lands
                .wait(landed)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Releases everyone waiting. Set under the mutex and signalled after it,
    /// so a caller between its read of the flag and its `wait` cannot miss this.
    fn land(&self) {
        *self.landed.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.lands.notify_all();
    }
}

/// The landing a compile owes the callers waiting on its flight, taken on the
/// way in so that it is paid on the way out.
///
/// A guard rather than a call because the way out includes a panic beneath the
/// front end: `Cargo.toml`'s profiles all unwind, so this drop runs, and what a
/// caller woken by it finds is a table with nothing under the key — which sends
/// it to compile the content itself rather than to wait again on a compile that
/// is not happening.
struct Landing<'a>(&'a Flight);

impl Drop for Landing<'_> {
    fn drop(&mut self) {
        self.0.land();
    }
}

/// What a caller that reached step 3 does about the content it is there to
/// compile — [`Compiler::claim`]'s answer, decided under one write guard.
enum Claim {
    /// Nobody holds this content: this caller compiles it, and the flight it
    /// left in the table is what everyone arriving meanwhile waits on.
    Mine,
    /// Someone else's compile of it is already running.
    Behind(Arc<Flight>),
    /// It landed between this caller's own lookup and its claim.
    Landed,
}

/// What one resolve saw of the file behind a path.
struct Observed {
    content_hash: Digest,
    stamp: Option<Stamp>,
}

/// The one implementor: the front end and the backend `nvs run` already
/// carries, plus `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s two maps in front of them.
///
#[derive(Debug)]
pub(crate) struct Compiler {
    /// Written path to what the last check of it observed. Behind a lock
    /// because the seam borrows a resolver shared, and a cache that could not
    /// be written on a hit would not be one — and a [`RwLock`] rather than a
    /// mutex because a hit only reads it, which is every resolve after the
    /// first.
    paths: RwLock<HashMap<PathBuf, PathEntry>>,
    /// The unit table proper, addressed by content rather than by path, so that
    /// two paths holding the same source compile once and a reverted edit is a
    /// hit rather than a recompile.
    units: RwLock<HashMap<UnitKey, CompileState>>,
    /// What each compile of a path's entry-file content read — the index a
    /// lookup needs in front of the table above, since the whole-program digest
    /// and the probe digest that complete a [`UnitKey`] are produced by the
    /// compile the key is meant to spare.
    ///
    /// A lookup reads the [`Traced::current`] trace to spell the key it is
    /// about to ask with. Content this process has not compiled, or whose every
    /// trace has moved, has none, and spells the entry file's digest and
    /// [`ProbeHash::unrecorded`] — the key a compile then claims under and
    /// re-keys away from ([`Self::record`]). A trace stays here exactly as long
    /// as the unit it addresses stays in `units`.
    ///
    /// The paths a trace names are **not** entries in [`Self::paths`]. That map
    /// answers *what a written path last compiled to* — one entry per path
    /// resolved as an entry point, each carrying the digest of a unit — and a
    /// `require`d file has no unit while being perfectly able to become an
    /// entry point later, through a `spawn script` of its own. Two meanings in
    /// one map is how the second one comes to read the first one's stamp.
    ///
    /// **What it spends:** one [`Trace`] per unit in `units`, swept by `units`'
    /// own rule, so the pair stays O(the program's files) rather than O(edits)
    /// — `rule:programs/memory-priority`, and `nvs_hir::autoload::ProbeTrace` is
    /// where the probe list's length is bounded.
    traces: RwLock<HashMap<(PathBuf, Digest), Traced>>,
    /// The environment half of every key here — `rule:config/the-extension-set-is-in-every-unit-key`'s digest, taken
    /// from the configuration this process is serving.
    ///
    /// Behind a lock because `[[extension]]` is a reloadable directive and the
    /// digest is the configuration's whole contribution to that rule's key, so
    /// a reload that changes the set has to be able to move it ([`Self::rekey`]).
    /// The read is one uncontended [`RwLock`] read on a path that already takes
    /// two of them, and the write happens once per reload that changes the set.
    ///
    env: RwLock<EnvHash>,
    /// `[opcache] validate`, `revalidate_freq` and `settle`. All three are
    /// `System`-class, so no request can move them, and all three reload, so a
    /// reload can ([`Self::reconfigure`]). Behind a lock for [`Self::env`]'s
    /// reason: one uncontended read per check, and a write per reload.
    revalidation: RwLock<Revalidation>,
    /// The signal of the [`watch`] thread checking this cache, so that
    /// [`Self::reconfigure`] can wake it. Empty until [`watch`] runs, and after
    /// its thread ends.
    watcher: Mutex<Weak<Signal>>,
    /// `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s on-disk cache, resolved from the same block and once for
    /// the same reason — or [`None`] for a host that consults none.
    ///
    /// It sits **behind** the two maps above rather than beside them: a path
    /// this compiler has already resolved is answered out of `units` without a
    /// file being opened at all, and this is what the compile that map misses
    /// asks before it walks Cranelift. The two do not overlap, and the disk one
    /// is the only one that survives the process.
    ///
    /// **What it spends:** one open of a content-addressed path per compile
    /// this cache misses, against a whole backend on every one it answers.
    ///
    /// Behind a lock for [`Self::env`]'s reason: the disk cache keys and heads
    /// every artifact with the same digest, so [`Self::rekey`] moves it too. A
    /// compile clones it out, which is one path per compile.
    cache: RwLock<Option<crate::cache::Cache>>,
    /// How many times [`Self::compile`] has run on this cache — the counter
    /// `docs/plan/m7.md`'s acceptance paragraph asks the "compiles it exactly
    /// once" claim to be asserted against, and the only number a caller could
    /// state it as: every other observable — the table's length, what a
    /// resolve hands back — is equal for a unit compiled once and one
    /// compiled a thousand times.
    ///
    /// It counts **compiles and not cores**: one per cache rather than one per
    /// worker, so a fleet sharing a cache sums into it and content compiled
    /// once reads as one however many cores asked for it. That is the whole
    /// point of sharing the cache, and the number that says the sharing works.
    ///
    /// **What it spends:** one word per cache, and a relaxed increment on the
    /// one step that already costs a front end and a backend — relaxed for
    /// `nvs_server::admit`'s reason, that nothing orders anything else against
    /// this and every increment only has to land. Deliberately not
    /// `#[cfg(test)]`: a field that exists in one profile makes the release
    /// build a different struct, and this is the number an `nvs info` would
    /// report if it ever reported one.
    compiles: AtomicU64,
}

impl Default for Compiler {
    /// The compiler of a host with **no configuration file anywhere**, which is
    /// [`nvs_config::Snapshot::default`]'s own state: the default revalidation
    /// policy, and the environment digest of a host with no `[[extension]]`.
    ///
    /// This is what a caller with no snapshot in hand holds, and it is a
    /// correct answer rather than a placeholder: the digest separates
    /// environments, and a run that read no configuration has exactly this one.
    /// Every subcommand that installs a resolver holds one — `nvs test`
    /// resolves the tree above the suite's compile, because `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s artifact
    /// key is half configuration — so what is left here is this crate's own
    /// tests.
    fn default() -> Self {
        Self::new(&Config::default())
    }
}

impl Compiler {
    /// The compiler for a process running under `config`: its environment
    /// digest, the `[opcache]` block's answer to when a resolve looks at a file
    /// it has already compiled, and the artifact cache that same block places.
    pub(crate) fn new(config: &Config) -> Self {
        Self {
            paths: RwLock::new(HashMap::new()),
            units: RwLock::new(HashMap::new()),
            traces: RwLock::new(HashMap::new()),
            env: RwLock::new(env_hash(config)),
            revalidation: RwLock::new(Revalidation::from_config(config)),
            watcher: Mutex::new(Weak::new()),
            cache: RwLock::new(crate::cache::from_config(config)),
            compiles: AtomicU64::new(0),
        }
    }

    /// The environment every key this cache holds was built under.
    fn env(&self) -> EnvHash {
        *shared(&self.env)
    }

    /// The `[opcache]` policy in force now.
    fn revalidation(&self) -> Revalidation {
        *shared(&self.revalidation)
    }

    /// `[opcache] settle`: how long a changed tree must be quiet before it is
    /// read. The mount table's background expansion waits for it too
    /// (`crate::serve`'s `mounts`).
    pub(crate) fn settle(&self) -> Duration {
        self.revalidation().settle
    }

    /// Moves `[opcache] validate`, `revalidate_freq` and `settle` to what
    /// `config` writes, which is how a reload reaches them
    /// (`rule:config/reloadability-is-its-own-field`). The next check reads the
    /// new policy.
    ///
    /// A policy that moved wakes [`watch`]'s thread, which runs a pass and then
    /// waits the new `revalidate_freq`. Without the wake, a server that booted
    /// with a long interval would wait that interval out once more before the
    /// shorter one applied.
    pub(crate) fn reconfigure(&self, config: &Config) {
        let next = Revalidation::from_config(config);
        {
            let mut held = exclusive(&self.revalidation);
            if *held == next {
                return;
            }
            *held = next;
        }
        let watcher = self
            .watcher
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .upgrade();
        if let Some(signal) = watcher {
            let (state, wakes) = &*signal;
            state.lock().unwrap_or_else(PoisonError::into_inner).retimed = true;
            wakes.notify_all();
        }
    }

    /// How many compiled units this cache holds right now.
    ///
    /// The number `rule:config/a-reload-names-what-it-could-not-apply`'s report
    /// carries as its recompile wave, which is why it counts the unit table and
    /// not the path map: two paths holding one content are one unit, and one
    /// path whose content changed is one entry and two.
    pub(crate) fn held(&self) -> usize {
        shared(&self.units).len()
    }

    /// Moves this cache and the disk cache behind it onto the environment `env`
    /// names, dropping every unit keyed under the one it leaves — and answering
    /// how many that was. The disk cache keeps its artifacts: the next compile
    /// looks for one under the new digest, and misses.
    ///
    /// `0` for a digest that has not moved, which is every reload that did not
    /// touch `[[extension]]`: the units stay, and the cheapest reload stays the
    /// common one.
    ///
    /// The units are **dropped** rather than left to age out, because a key is
    /// half environment (`rule:config/the-extension-set-is-in-every-unit-key`)
    /// and a unit under the old digest can never be hit again — keeping it would
    /// hold a compiled program for the life of the process in exchange for
    /// nothing. The path map goes with them: its entries point at content
    /// hashes whose units are gone, so a resolve that hit one would take step 1
    /// down to a compile anyway.
    pub(crate) fn rekey(&self, env: EnvHash) -> usize {
        let mut keyed = exclusive(&self.env);
        if *keyed == env {
            return 0;
        }
        *keyed = env;
        if let Some(cache) = exclusive(&self.cache).as_mut() {
            cache.rekey(env);
        }
        let mut units = exclusive(&self.units);
        let dropped = units.len();
        units.clear();
        exclusive(&self.traces).clear();
        exclusive(&self.paths).clear();
        dropped
    }

    /// The program over `path`'s unit **and** that unit's route table, which is
    /// what a server needs and what [`Resolver::resolve`]'s own signature has
    /// nowhere to put.
    ///
    /// Step 1 of `rule:config/an-edit-reaches-the-next-request-without-a-restart`:
    /// a path this cache has resolved before is answered from what its pointer
    /// names, which is one map lookup and no file-system call. Moving the
    /// pointer is [`Self::revalidate`]'s job, off the request path.
    ///
    /// A path with no pointer yet goes through [`Self::check`], the steps the
    /// background check runs, with step 3's single flight: one caller compiles
    /// a content and every other waits on it. So does a path whose unit is no
    /// longer in the table, which a reload that moved the environment leaves.
    ///
    /// # Errors
    ///
    /// The one-line summary `resolve` reports, for the same two failures: a
    /// program the front end refused, and one the backend could not compile.
    ///
    pub(crate) fn compiled(
        &self,
        path: &str,
    ) -> Result<(Program, Arc<nvs_runtime::routes::Routes>), String> {
        let written = PathBuf::from(path);
        let known = shared(&self.paths).get(&written).copied();
        if let Some(entry) = known
            && let Some(answer) = self.answer_at(&written, entry.serving())
        {
            return answer;
        }
        match self.look(path, &written, known) {
            Ok(observed) => self.take(path, &written, known, &observed, false),
            Err(answer) => answer,
        }
    }

    /// The background check: every path this cache holds a pointer for, looked
    /// at again, and a change compiled and swapped in once the program has been
    /// quiet for `[opcache] settle`. [`watch`] runs it once per
    /// `revalidate_freq`.
    ///
    /// Answers how long until the soonest change it held back is quiet, and
    /// `None` where it held none back, so that the next pass can be timed to
    /// take it then.
    ///
    /// **What it costs:** [`Self::revalidate_trace`]'s `stat`s for every path
    /// held, per call, and one more `stat` per file of a program whose content
    /// changed. None of it is on a request's path.
    pub(crate) fn revalidate(&self) -> Option<Duration> {
        let held: Vec<(PathBuf, PathEntry)> = shared(&self.paths)
            .iter()
            .map(|(path, entry)| (path.clone(), *entry))
            .collect();
        let mut soonest: Option<Duration> = None;
        for (written, entry) in held {
            let path = written.to_string_lossy();
            let Ok(observed) = self.look(&path, &written, Some(entry)) else {
                continue;
            };
            if self.generation(&written, observed.content_hash) != entry.serving()
                && let Some(wait) = self.unsettled(&written, entry, &observed)
            {
                soonest = Some(soonest.map_or(wait, |soonest| soonest.min(wait)));
                continue;
            }
            let _ = self.take(&path, &written, Some(entry), &observed, true);
        }
        soonest
    }

    /// Step 2: what the file behind `written` holds now, with every path its
    /// program's last compile read, missed or probed checked again
    /// ([`Self::revalidate_trace`]). An edit to any of those is one the entry
    /// file's digest does not move.
    ///
    /// A file the system will not answer for keeps whatever it last resolved
    /// to — step 5's reading, for the same reason: the pointer still names the
    /// last content that compiled, and a path being replaced by a rename is
    /// momentarily absent. That is the `Err`, and so is a path with no pointer,
    /// reported here in the shape `front_end` would have reported it.
    fn look(
        &self,
        path: &str,
        written: &Path,
        known: Option<PathEntry>,
    ) -> Result<Observed, Answer> {
        let observed = match observe(written, self.revalidation().validate, known) {
            Ok(observed) => observed,
            Err(error) => {
                if let Some(answer) = known.and_then(|e| self.answer_at(written, e.serving())) {
                    return Err(answer);
                }
                eprintln!("error: could not read {}: {error}", written.display());
                return Err(Err(format!(
                    "`{path}` could not be compiled; see the errors above"
                )));
            }
        };
        self.revalidate_trace(written, observed.content_hash);
        Ok(observed)
    }

    /// Steps 3 to 5 for what [`Self::look`] observed: answer it from the table,
    /// or compile it, then move the pointer on success and record the failure
    /// on one. `background` is the check [`Self::revalidate`] runs, which
    /// discards a compile the tree moved under ([`Self::moved`]).
    fn take(
        &self,
        path: &str,
        written: &Path,
        known: Option<PathEntry>,
        observed: &Observed,
        background: bool,
    ) -> Answer {
        // What step 4 below compares against: the pointer as the caller found
        // it, read once so that everything after it — the `stat`, the hash and
        // the compile — happens outside the map.
        let since = known.map(|entry| entry.unit.content);

        // An observation that did not move addresses the unit the pointer
        // names, and one that did may still name a program this process
        // compiled before — a reverted edit, or a broken one being observed
        // again. Either way it is answered without reaching a flight at all.
        if let Some((generation, answer)) = self.answer(written, observed.content_hash) {
            self.settle_pointer(written, observed, generation, since, answer.is_ok());
            return answer;
        }

        // 3. The compile itself, which is the only step that costs anything —
        //    and which the fleet pays for once. The caller that claims this
        //    content's key is the one that runs it; a caller that arrives while
        //    it runs waits behind the same flight, so a cold path stormed by
        //    every core at once costs one front end rather than one per core.
        //    The key claimed here is the one a lookup can spell, which is the
        //    entry file's digest and no probes the first time. What the compile
        //    publishes is that key with the whole-program digest and the probe
        //    digest its own trace gives, which only a finished front end knows.
        let key = self.key(written, observed.content_hash);
        let flight = Arc::new(Flight::default());
        let claimed = match self.claim(&key, &flight) {
            Claim::Mine => true,
            Claim::Behind(ahead) => {
                ahead.wait();
                false
            }
            Claim::Landed => false,
        };
        // Every caller but that one reads what the compile published. Finding
        // nothing there takes a panic beneath it — [`Landing`] wakes a waiter
        // either way rather than leaving it here — or a [`Self::record`] for
        // another content of this path sweeping the entry out in between. Both
        // fall through and compile on this caller's own account, which is what
        // every caller did before there was a flight to wait behind.
        if !claimed && let Some((generation, answer)) = self.answer(written, observed.content_hash)
        {
            self.settle_pointer(written, observed, generation, since, answer.is_ok());
            return answer;
        }
        let landing = Landing(&flight);
        // A failure is published under its own trace too, which is the key the
        // next check of this content spells while nothing it read has moved,
        // and is how it is answered rather than recompiled.
        let (outcome, trace) = self.compile(path, written, observed.content_hash);
        // A tree that moved while the compile ran may have been read half old
        // and half new, which is a program nobody wrote. The background check
        // throws such a compile away and records nothing, so its next pass
        // looks again and compiles the tree once it is quiet. A request only
        // compiles a path with nothing else to serve, so it keeps what it read.
        if background && self.moved(written, observed.content_hash, &trace) {
            self.release(&key, &flight);
            drop(landing);
            return Err(format!("`{path}` changed while it was compiled"));
        }
        let state = match outcome {
            Ok(compiled) => CompileState::Ready(compiled),
            Err(message) => CompileState::Failed(message),
        };
        // 4 and 5: the pointer moves only on success. What the table keeps for
        // this path is the unit in force and the one it replaced, plus at most
        // the failure the next resolve of this content is owed. A success
        // becomes the unit in force, so the one in force now is the one it
        // replaced, and the one before that goes.
        let ready = matches!(state, CompileState::Ready(_));
        let generation = Generation {
            content: observed.content_hash,
            program: trace.program,
            probes: trace.probes,
        };
        let keep: Vec<Generation> = known
            .map(|entry| {
                let replaced = entry
                    .replaced
                    .filter(|_| !ready || entry.unit == generation);
                [Some(entry.unit), replaced].into_iter().flatten().collect()
            })
            .unwrap_or_default();
        let published = key.with_program(trace.program).with_probes(trace.probes);
        // Taken from the state before it goes into the table, so a record for
        // another content of this path, landing in between, cannot take this
        // caller's answer with it.
        let reply = reply(&state).expect("a finished compile is an answer");
        self.record(published, observed.content_hash, state, &keep, trace);
        // The waiters, released once the answer is in the table and not before.
        // The explicit drop is the ordering; the guard is for the path where
        // the line above never ran at all.
        drop(landing);
        self.settle_pointer(written, observed, generation, since, ready);
        reply
    }

    /// What the table holds for `path` at `content`, and `None` where it holds
    /// nothing. The [`Generation`] beside it is the one the answer was read
    /// under.
    fn answer(&self, path: &Path, content: Digest) -> Option<(Generation, Answer)> {
        let generation = self.generation(path, content);
        Some((generation, self.answer_at(path, generation)?))
    }

    /// What the table holds for `path` at `generation` — the one place a
    /// [`CompileState`] becomes a caller's answer, and the whole of a request
    /// for a path that has a pointer.
    ///
    /// The key is spelled before `units` is read, because spelling it reads
    /// `env`, and no lock here is ever held across another ([`Self::key`]).
    fn answer_at(&self, path: &Path, generation: Generation) -> Option<Answer> {
        let key = self.key_of(path, generation);
        reply(shared(&self.units).get(&key)?)
    }

    /// How long until the program behind `path` has been quiet for `[opcache]
    /// settle`, and `None` where it already has: the newest stamp among the
    /// entry file, every file the traces of both contents read, and every
    /// directory they listed, against the settle time.
    ///
    /// A stamp in the future is left out. The clock that wrote it is not this
    /// one, and holding a change back until that moment could hold it for as
    /// long as the two clocks disagree.
    fn unsettled(&self, path: &Path, entry: PathEntry, observed: &Observed) -> Option<Duration> {
        let watched: Vec<PathBuf> = {
            let traces = shared(&self.traces);
            [entry.unit.content, observed.content_hash]
                .iter()
                .filter_map(|content| traces.get(&(path.to_path_buf(), *content)))
                .flat_map(|traced| &traced.traces)
                .flat_map(|trace| {
                    let files = trace.files.iter().map(|read| read.path.clone());
                    files.chain(trace.listed.iter().map(|listed| listed.dir.clone()))
                })
                .collect()
        };
        let now = SystemTime::now();
        let newest = watched
            .iter()
            .filter_map(|path| stamp_of(path))
            .chain(observed.stamp)
            .map(|stamp| stamp.modified)
            .filter(|modified| *modified <= now)
            .max()?;
        (newest + self.revalidation().settle)
            .duration_since(now)
            .ok()
            .filter(|wait| !wait.is_zero())
    }

    /// The program `path` at `content` is addressed by right now: the
    /// [`Traced::current`] trace [`Self::traces`] holds for that content — its
    /// whole-program digest and its probe digest. Where there is none, it is
    /// the entry file's digest and [`ProbeHash::unrecorded`], which is the key
    /// a compile of the content claims under.
    fn generation(&self, path: &Path, content: Digest) -> Generation {
        let (program, probes) = shared(&self.traces)
            .get(&(path.to_path_buf(), content))
            .and_then(|traced| traced.current)
            .unwrap_or((content, ProbeHash::unrecorded()));
        Generation {
            content,
            program,
            probes,
        }
    }

    /// The key `path` at `content` is addressed by right now: its
    /// [`Self::generation`] under the environment this process is serving.
    ///
    /// Every map is read and released before the caller touches `units`, so
    /// no lock in this resolver is ever held across another: a reload taking
    /// the write half of either cannot meet a reader holding the other.
    fn key(&self, path: &Path, content: Digest) -> UnitKey {
        self.key_of(path, self.generation(path, content))
    }

    /// `generation` of `path` as a key, under the environment this process is
    /// serving.
    fn key_of(&self, path: &Path, generation: Generation) -> UnitKey {
        UnitKey::new(path, generation.program, generation.probes, self.env())
    }

    /// Step 2's other half: every path a compile of this content read, missed
    /// or probed, checked again under the gate the entry file's `stat` rides
    /// rather than one of their own.
    ///
    /// The current trace is checked first. Where a path it names answers
    /// differently now, the next compile would read a different program, so
    /// the trace is no longer this content's, and the other traces kept for
    /// this content are checked newest first. The first one that still
    /// describes the disk becomes current, which is how undoing an edit to a
    /// `require`d file finds the unit compiled before it. Where none does,
    /// nothing is current: the check behind this call spells the unrecorded
    /// key, misses, and compiles. Where the trace that stays current had a
    /// stamp refreshed, the refreshed stamps are written back, so a file re-read
    /// once is not re-read on every later check.
    ///
    /// No trace and no unit is removed here. A trace that moved may describe
    /// the disk again after a revert, a unit is still the right answer for the
    /// key it is under, and [`Self::record`] is the one place that takes either
    /// out of its map.
    ///
    /// **What it costs:** one resolution of the entry path through its links,
    /// one `stat` per file the program read or missed and per
    /// directory a discovery scan listed, one `exists` per probed path, and a
    /// read or a listing only for a path whose stamp moved (or every one, under
    /// `validate = "hash"`) — per check, and
    /// once more for each other trace kept for this content when the current
    /// one moved.
    fn revalidate_trace(&self, path: &Path, content: Digest) {
        let entry = (path.to_path_buf(), content);
        let validate = self.revalidation().validate;
        let checked = SystemTime::now();
        let (was, found) = {
            let traces = shared(&self.traces);
            let Some(traced) = traces.get(&entry) else {
                return;
            };
            let id = |trace: &Trace| (trace.program, trace.probes);
            let current = traced
                .traces
                .iter()
                .filter(|trace| Some(id(trace)) == traced.current);
            let others = traced
                .traces
                .iter()
                .rev()
                .filter(|trace| Some(id(trace)) != traced.current);
            let found = current.chain(others).find_map(|trace| {
                describes_the_disk(path, trace, validate, checked).map(|stamps| (id(trace), stamps))
            });
            (traced.current, found)
        };
        if matches!(found, Some((id, None)) if Some(id) == was)
            || (found.is_none() && was.is_none())
        {
            return;
        }
        let mut traces = exclusive(&self.traces);
        // Only onto the state that was checked: a compile that landed in
        // between recorded a current trace, and stamps, of its own.
        let Some(traced) = traces.get_mut(&entry) else {
            return;
        };
        if traced.current != was {
            return;
        }
        traced.current = None;
        let Some((id, stamps)) = found else {
            return;
        };
        let Some(trace) = traced
            .traces
            .iter_mut()
            .find(|trace| (trace.program, trace.probes) == id)
        else {
            return;
        };
        traced.current = Some(id);
        if let Some((files, dirs)) = stamps {
            for (read, stamp) in trace.files.iter_mut().zip(files) {
                read.stamp = stamp;
            }
            for (listed, stamp) in trace.listed.iter_mut().zip(dirs) {
                listed.stamp = stamp;
            }
        }
    }

    /// Whether the tree behind a compile of `written` at `content` moved while
    /// that compile ran: the entry file no longer holds `content`, the entry
    /// path resolves through its links to another file, or a file, probe or
    /// listed directory in `trace` no longer holds what the compile found there
    /// ([`describes_the_disk`]).
    ///
    /// **What it costs:** one read of the entry file, one resolution of its
    /// path, and the `stat`s of one check of the trace, per background compile.
    fn moved(&self, written: &Path, content: Digest, trace: &Trace) -> bool {
        let entry_holds =
            std::fs::read(written).is_ok_and(|source| content_hash(&source) == content);
        !entry_holds
            || describes_the_disk(
                written,
                trace,
                self.revalidation().validate,
                SystemTime::now(),
            )
            .is_none()
    }

    /// Takes `flight`'s placeholder out of the table, where it is still there,
    /// for a compile that publishes nothing. A caller waiting on it then finds
    /// nothing under the key and compiles the content itself.
    fn release(&self, key: &UnitKey, flight: &Arc<Flight>) {
        let mut units = exclusive(&self.units);
        if matches!(units.get(key), Some(CompileState::Compiling(held)) if Arc::ptr_eq(held, flight))
        {
            units.remove(key);
        }
    }

    /// Who compiles `key`, decided under the write guard so that exactly one
    /// caller can be told to: this one, which found the table holding nothing
    /// for it and left `flight` there for whoever arrives next, or the caller
    /// already running it, or nobody because it has already landed.
    ///
    /// The two answers that are not [`Claim::Mine`] hand back no unit, and the
    /// caller re-reads the table through [`Self::answer`] instead. That keeps
    /// one place where a [`CompileState`] becomes a caller's answer, and it is
    /// also the honest shape: what a waiter wants is what the table holds when
    /// it wakes, which is not what it held when it went to sleep.
    fn claim(&self, key: &UnitKey, flight: &Arc<Flight>) -> Claim {
        let mut units = exclusive(&self.units);
        match units.get(key) {
            Some(CompileState::Compiling(ahead)) => Claim::Behind(Arc::clone(ahead)),
            Some(CompileState::Ready(_) | CompileState::Failed(_)) => Claim::Landed,
            None => {
                units.insert(key.clone(), CompileState::Compiling(Arc::clone(flight)));
                Claim::Mine
            }
        }
    }

    /// Steps 4 and 5 for one check that reached `generation`: the pointer moves
    /// to it where it compiled ([`Self::advance`]), and where it did not, the
    /// pointer stays and names it as the failure its requests are answered
    /// with ([`Self::fail`]).
    fn settle_pointer(
        &self,
        path: &Path,
        observed: &Observed,
        generation: Generation,
        since: Option<Digest>,
        ready: bool,
    ) {
        if ready {
            self.advance(path, observed, generation, since);
        } else {
            self.fail(path, generation, since);
        }
    }

    /// Step 5's pointer write: the unit in force stays, and
    /// [`PathEntry::failed`] names `generation`, so every request of this path
    /// is answered with that failure until a check reaches a program that
    /// compiles. Only where nobody moved the pointer since, as in
    /// [`Self::advance`], and nothing at all for a path with no pointer: a path
    /// that has never compiled is looked at by each request that names it.
    ///
    /// The stamp stays the one the unit in force was observed with. [`observe`]
    /// trusts a matching stamp to mean that unit's content, so the stamp of the
    /// broken file must never stand beside it.
    fn fail(&self, path: &Path, generation: Generation, since: Option<Digest>) {
        let mut paths = exclusive(&self.paths);
        if let Some(entry) = paths.get_mut(path)
            && Some(entry.unit.content) == since
        {
            entry.failed = Some(generation);
        }
    }

    /// Step 4's pointer write: what this path resolves to now — published
    /// **only if nobody moved the pointer since**, which is the compare that
    /// rule's step 4 states rather than an assignment.
    ///
    /// `since` is the digest [`Self::take`] copied out of the map on its
    /// way in, and [`None`] — a path this resolve found nothing for — is one
    /// of its values rather than a case beside it, so two cold resolves of one
    /// path race on the same terms as two revalidations of it. A resolve that
    /// observed the file earlier therefore cannot roll back one that observed
    /// it later, however much longer its own compile took.
    ///
    /// Nothing is retried on a loss, and the caller is not told: the unit
    /// table is keyed by content and already holds what this resolve
    /// compiled, so it still answers with its own unit — what it has lost is
    /// only being the content the *next* resolve of this path starts from.
    /// [`bool`] is here for the tests that order two revalidations by hand.
    ///
    /// `generation` is the unit now in force. Where it differs from the one the
    /// pointer named, that one becomes [`PathEntry::replaced`]; where it is the
    /// same, the pointer keeps the one it had.
    fn advance(
        &self,
        path: &Path,
        observed: &Observed,
        generation: Generation,
        since: Option<Digest>,
    ) -> bool {
        let mut paths = exclusive(&self.paths);
        let known = paths.get(path).copied();
        if known.map(|entry| entry.unit.content) != since {
            return false;
        }
        let replaced = match known {
            Some(entry) if entry.unit == generation => entry.replaced,
            Some(entry) => Some(entry.unit),
            None => None,
        };
        paths.insert(
            path.to_path_buf(),
            PathEntry {
                unit: generation,
                replaced,
                failed: None,
                stamp: observed.stamp,
            },
        );
        true
    }

    /// `state` under `key`, and the entries this path is then allowed to keep:
    /// the program just reached, and the generations in `keep`.
    ///
    /// `keep` is what [`Self::compiled`] decides the path still owes: the unit
    /// in force and the one it replaced ([`PathEntry`]). After a success the
    /// unit in force is about to become the replaced one, and the one before it
    /// is not kept. After a failure both stay beside it. So a path holds at
    /// most three units, and at most one of them is a failure.
    ///
    /// The sweep is what keeps the table O(paths): every other generation of
    /// this path goes, and a unit a running [`Program`] still holds stays
    /// mapped through that program's own `Arc` rather than through this map.
    /// The traces go with their units, and a [`Traced::current`] whose trace
    /// went is cleared.
    ///
    /// A generation is the whole key, so the placeholder a compile claimed
    /// under — the entry file's digest and [`ProbeHash::unrecorded`] — goes the
    /// moment the trace that replaces it lands. [`Self::traces`] is then told
    /// the trace, keyed by `reached`, the entry file's digest, and made current,
    /// after the unit is in the table and before the waiters are released, so
    /// every caller woken by the landing spells the new key and finds the unit
    /// already under it. A caller arriving cold *between* the two writes spells
    /// the old key, finds the placeholder gone and compiles on its own account
    /// — the same answer the map already gives a waiter whose flight died, and
    /// it converges on the same entry rather than on a second one.
    fn record(
        &self,
        key: UnitKey,
        reached: Digest,
        state: CompileState,
        keep: &[Generation],
        trace: Trace,
    ) {
        let path = key.path();
        let published = Generation {
            content: reached,
            program: trace.program,
            probes: trace.probes,
        };
        // Spelled before either guard is taken, because a key reads `env`.
        let kept_keys: Vec<UnitKey> = keep.iter().map(|g| self.key_of(path, *g)).collect();
        {
            let mut units = exclusive(&self.units);
            units.retain(|other, _| {
                other.path() != path || other == &key || kept_keys.contains(other)
            });
            units.insert(key.clone(), state);
        }
        let mut traces = exclusive(&self.traces);
        traces.retain(|(traced_path, content), traced| {
            if traced_path != path {
                return true;
            }
            traced.traces.retain(|kept| {
                let generation = Generation {
                    content: *content,
                    program: kept.program,
                    probes: kept.probes,
                };
                generation == published || keep.contains(&generation)
            });
            if let Some(current) = traced.current
                && !traced
                    .traces
                    .iter()
                    .any(|kept| (kept.program, kept.probes) == current)
            {
                traced.current = None;
            }
            !traced.traces.is_empty()
        });
        let traced = traces.entry((path.to_path_buf(), reached)).or_default();
        traced
            .traces
            .retain(|kept| (kept.program, kept.probes) != (trace.program, trace.probes));
        traced.current = Some((trace.program, trace.probes));
        traced.traces.push(trace);
    }

    /// The front end and the backend, over one path, with this process's own
    /// table holding nothing for it: the whole of what step 3 costs.
    ///
    /// The outcome comes back beside the [`Trace`] of what the front end read,
    /// missed and probed, failure or not, because the caller's key is not
    /// complete until it has one and nothing short of this function can produce
    /// it. `content` is the entry file's digest, which [`failed_program`] folds
    /// in where there is no program digest to take.
    ///
    /// The backend half may still come off disk — [`Self::cache`] is asked here
    /// and nowhere else — and that is why the counter below keeps counting a
    /// warm hit as a compile. `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header` is explicit that a hit skips
    /// codegen and not the front end, so the front end really did run; what a
    /// hit saves is the Cranelift walk, which this counter never claimed to
    /// measure.
    ///
    fn compile(
        &self,
        path: &str,
        written: &Path,
        content: Digest,
    ) -> (Result<Arc<Compiled>, String>, Trace) {
        // Counted here rather than at the call site, and before the front end
        // rather than after it: a compile that *failed* is still a compile
        // this cache paid for, and the claim being counted is about how many
        // times the file was put through the front end at all.
        self.compiles.fetch_add(1, Ordering::Relaxed);
        // Resolved once, and the whole program is read below it: a link
        // switched while the front end runs cannot hand it one file from each
        // release. [`Self::moved`] resolves it again afterwards.
        let real = real_path(written);
        // Taken before the front end reads anything: a file whose stamp is
        // not older than this may have changed after it was read ([`vouching`]).
        let started = SystemTime::now();
        let mut looked = crate::Looked::default();
        let checked = crate::front_end_looking(&real, &mut looked);
        let answers: Vec<(PathBuf, bool)> = looked
            .probed
            .iter()
            .map(|probed| (probed.clone(), probed.exists()))
            .collect();
        let reads: Vec<Read> = looked
            .read
            .iter()
            .filter(|(read, _)| *read != real)
            .map(|(read, digest)| Read {
                path: read.clone(),
                stamp: vouching(stamp_of(read), started),
                digest: Some(*digest),
            })
            .chain(looked.missed.iter().map(|missed| Read {
                path: missed.clone(),
                stamp: None,
                digest: None,
            }))
            .collect();
        let listed: Vec<Listed> = looked
            .listed
            .into_iter()
            .map(|listing| Listed {
                stamp: vouching(stamp_of(&listing.dir), started),
                dir: listing.dir,
                names: listing.names,
            })
            .collect();
        let probes = discovery_hash(
            &looked.probed,
            &listed
                .iter()
                .map(|listed| (listed.dir.clone(), listed.names.clone()))
                .collect::<Vec<_>>(),
        );
        let Ok(checked) = checked else {
            let program = failed_program(content, &reads);
            return (
                Err(format!(
                    "`{path}` could not be compiled; see the errors above"
                )),
                Trace {
                    program,
                    probes,
                    answers,
                    files: reads,
                    listed,
                    real,
                },
            );
        };
        // Held across the lowering and the key alike: § 1's digest is over
        // every file the `require`/`autoload` graph reached, which is the same
        // list that was lowered and not the entry file alone.
        let files = checked.program_files();
        let program = crate::cache::program_digest(&files);
        let lowered = nvs_ir::lower::lower_program(
            nvs_ir::lower::ENTRY_SCRIPT_LABEL,
            &files,
            &checked.exprs,
            &checked.interner,
            &checked.enums,
            &checked.layouts,
        );
        let cache = shared(&self.cache).clone();
        let outcome = crate::cache::unit_for(&lowered, program, cache.as_ref())
            .map(|(unit, _)| {
                Arc::new(Compiled {
                    unit: Arc::new(unit),
                    routes: Arc::new(crate::runtime_routes(&checked.exprs)),
                })
            })
            .map_err(|error| format!("`{path}`: {error}"));
        (
            outcome,
            Trace {
                program,
                probes,
                answers,
                files: reads,
                listed,
                real,
            },
        )
    }
}

/// The shortest time between two passes of [`watch`]'s thread. A written
/// `revalidate_freq` of zero checks this often, and does not spin a core.
const FLOOR: Duration = Duration::from_millis(10);

/// What wakes [`watch`]'s thread before its interval is over.
#[derive(Debug, Default)]
struct Wake {
    /// The [`Watch`] was dropped, and the thread ends.
    stopped: bool,
    /// [`Compiler::reconfigure`] moved the policy, and the thread runs a pass
    /// now and then waits the new interval.
    retimed: bool,
}

/// [`Wake`] and the condition variable its thread waits on.
type Signal = (Mutex<Wake>, Condvar);

/// The thread [`watch`] starts. Dropping this stops the thread and waits for
/// it, so no check outlives the run that started it.
#[derive(Debug)]
pub(crate) struct Watch {
    stop: Arc<Signal>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        let (state, wakes) = &*self.stop;
        state.lock().unwrap_or_else(PoisonError::into_inner).stopped = true;
        wakes.notify_all();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A second check [`watch`]'s thread runs after each [`Compiler::revalidate`],
/// answering the same way: how long until a change it held back is quiet, or
/// `None`.
pub(crate) type Also = Box<dyn FnMut(&Compiler) -> Option<Duration> + Send>;

/// Starts the background check of
/// `rule:config/an-edit-reaches-the-next-request-without-a-restart`: one
/// thread that runs [`Compiler::revalidate`], and then `also` where there is
/// one, once per `revalidate_freq`, or sooner where a change either held back
/// becomes quiet before that. `nvs serve` passes its mount table's expansion
/// as `also`.
///
/// The thread holds the compiler weakly, so it stops by itself when the last
/// owner drops the compiler. A pass that panics beneath the front end is
/// caught, and the next pass runs as usual. A thread that cannot be started
/// is reported once, and then no edit reaches this process.
///
/// **What it spends:** one thread per process, asleep between passes.
pub(crate) fn watch(compiler: &Arc<Compiler>, mut also: Option<Also>) -> Watch {
    let stop: Arc<Signal> = Arc::new((Mutex::new(Wake::default()), Condvar::new()));
    *compiler
        .watcher
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Arc::downgrade(&stop);
    let weak = Arc::downgrade(compiler);
    let stopping = Arc::clone(&stop);
    let mut next = compiler.revalidation().freq.max(FLOOR);
    let spawned = std::thread::Builder::new()
        .name("nvs-revalidate".to_owned())
        .spawn(move || {
            let (state, wakes) = &*stopping;
            loop {
                let guard = state.lock().unwrap_or_else(PoisonError::into_inner);
                let (mut guard, _) = wakes
                    .wait_timeout_while(guard, next, |wake| !wake.stopped && !wake.retimed)
                    .unwrap_or_else(PoisonError::into_inner);
                if guard.stopped {
                    return;
                }
                guard.retimed = false;
                drop(guard);
                let Some(compiler) = weak.upgrade() else {
                    return;
                };
                let held = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    compiler.revalidate()
                }));
                let also_held = also.as_mut().and_then(|also| {
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| also(&compiler)))
                        .ok()
                        .flatten()
                });
                // Read again each pass, so a reload's interval is the one this
                // thread waits next.
                let tick = compiler.revalidation().freq.max(FLOOR);
                next = held
                    .ok()
                    .flatten()
                    .into_iter()
                    .chain(also_held)
                    .min()
                    .map_or(tick, |wait| wait.min(tick))
                    .max(FLOOR);
            }
        });
    let thread = match spawned {
        Ok(thread) => Some(thread),
        Err(error) => {
            eprintln!("warning: source files will not be checked for changes: {error}");
            None
        }
    };
    Watch { stop, thread }
}

/// What the table holds under one key, as a caller's answer — and `None` for a
/// compile still in flight, which is not an answer.
///
/// Saying so is what sends a step-1 hit on that content down to step 3 to wait
/// for the compile rather than reporting that the cache holds nothing.
fn reply(
    state: &CompileState,
) -> Option<Result<(Program, Arc<nvs_runtime::routes::Routes>), String>> {
    match state {
        CompileState::Compiling(_) => None,
        CompileState::Ready(compiled) => Some(Ok((
            program_over(Arc::clone(compiled)),
            Arc::clone(&compiled.routes),
        ))),
        CompileState::Failed(message) => Some(Err(message.clone())),
    }
}

/// The digest a failed compile is published under, in place of the program
/// digest only a compile that reached the backend has: the entry file's digest,
/// then each path the failed run read or missed and what it found there.
///
/// Two failures of one entry content that read different files are two keys,
/// so the one a later resolve spells is the one its trace names.
fn failed_program(content: Digest, files: &[Read]) -> Digest {
    let mut bytes = content.as_bytes().to_vec();
    for read in files {
        let path = read.path.to_string_lossy();
        bytes.extend_from_slice(&u64::try_from(path.len()).unwrap_or(u64::MAX).to_le_bytes());
        bytes.extend_from_slice(path.as_bytes());
        match read.digest {
            Some(digest) => {
                bytes.push(1);
                bytes.extend_from_slice(digest.as_bytes());
            }
            None => bytes.push(0),
        }
    }
    content_hash(&bytes)
}

/// The `mtime`/size pair of the file at `path`, or `None` where the file system
/// answers with neither.
fn stamp_of(path: &Path) -> Option<Stamp> {
    let meta = std::fs::metadata(path).ok()?;
    Some(Stamp {
        modified: meta.modified().ok()?,
        len: meta.len(),
    })
}

/// `written` through every symlink and junction: the real path a compile reads
/// the program through, so that every file it names comes from one release
/// however a `current` link moves (`rule:config/an-edit-reaches-the-next-request-without-a-restart`
/// § *An atomic deploy is atomic*). A path that does not resolve is its own
/// answer, and the front end then reports the file missing.
///
/// `nvs_config::trust::canonical`, so that a Windows path keeps the spelling
/// every other path in the program has and not the verbatim `\\?\` one.
fn real_path(written: &Path) -> PathBuf {
    nvs_config::trust::canonical(written).unwrap_or_else(|_| written.to_path_buf())
}

/// `stamp`, if it can vouch for a read that began at `started`.
///
/// A file modified at or after that moment may have changed after it was read,
/// with a stamp that then matches forever under `mtime`. So such a stamp is not
/// kept, and the next check re-reads the file instead. A stamp older than the
/// read is safe to trust.
fn vouching(stamp: Option<Stamp>, started: SystemTime) -> Option<Stamp> {
    stamp.filter(|stamp| stamp.modified < started)
}

/// Whether the file behind `read` still holds what the compile read — and if it
/// does, the stamp to keep for it. `None` means it moved.
///
/// Under `mtime` a stamp that matches the recorded one answers without a read.
/// Otherwise the file is read and hashed, which is what makes the digest, and
/// never the stamp, the thing that decides. A path that was absent moves the
/// moment anything is there.
fn unmoved(read: &Read, validate: Validate, checked: SystemTime) -> Option<Option<Stamp>> {
    let Some(digest) = read.digest else {
        return (!read.path.exists()).then_some(None);
    };
    let stamp = stamp_of(&read.path);
    if validate == Validate::Mtime && stamp.is_some() && stamp == read.stamp {
        return Some(stamp);
    }
    let source = std::fs::read(&read.path).ok()?;
    (content_hash(&source) == digest).then(|| vouching(stamp, checked))
}

/// [`unmoved`] for a directory a discovery scan listed: under `mtime` a stamp
/// that matches answers without a listing, and otherwise the directory is
/// listed again through the scan's own filter. Only the names decide.
fn unlisted(listed: &Listed, validate: Validate, checked: SystemTime) -> Option<Option<Stamp>> {
    let stamp = stamp_of(&listed.dir);
    if validate == Validate::Mtime && stamp.is_some() && stamp == listed.stamp {
        return Some(stamp);
    }
    (nvs_hir::autoload::listed_names(&listed.dir) == listed.names).then(|| vouching(stamp, checked))
}

/// Whether every path `trace` names still answers the way the compile saw it:
/// `None` where the entry path `written` resolves to another real path, or a
/// probe, a file or a listed directory moved, and otherwise the stamps to write
/// back — `None` inside where none of them was refreshed.
fn describes_the_disk(
    written: &Path,
    trace: &Trace,
    validate: Validate,
    checked: SystemTime,
) -> Option<Option<(Stamps, Stamps)>> {
    if real_path(written) != trace.real {
        return None;
    }
    if trace
        .answers
        .iter()
        .any(|(probed, existed)| probed.exists() != *existed)
    {
        return None;
    }
    let files = trace
        .files
        .iter()
        .map(|read| unmoved(read, validate, checked))
        .collect::<Option<Stamps>>()?;
    let dirs = trace
        .listed
        .iter()
        .map(|listed| unlisted(listed, validate, checked))
        .collect::<Option<Stamps>>()?;
    let changed = trace
        .files
        .iter()
        .map(|read| read.stamp)
        .zip(&files)
        .chain(trace.listed.iter().map(|listed| listed.stamp).zip(&dirs))
        .any(|(was, now)| was != *now);
    Some(changed.then_some((files, dirs)))
}

/// A read guard on one of [`Compiler`]'s two maps: the hit path, and the one
/// every resolve after the first takes.
///
/// Poison is stepped over rather than reported, which is `nvs_host`'s rule for
/// the same reason it is this cache's: nothing under either guard can leave a
/// map half-written — a `get`, an `insert`, a `retain` and an [`Arc`] clone —
/// so a thread that panicked elsewhere has not made anything in here untrue,
/// and refusing to read it would cost a working process its whole cache.
fn shared<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

/// A write guard on one of the same two maps, held for one `insert` and never
/// across a compile — the module doc's *A reader never waits behind a compile*.
fn exclusive<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}

/// Step 2: what the file behind `path` holds now.
///
/// The stamp is read first and the source only where it cannot answer, which is
/// the whole of the `mtime` policy — under `hash` the source is read every time
/// a check happens at all, which is what a file rewritten twice inside one
/// timestamp tick needs.
///
/// **The source is read once more than the compile alone would need**: this
/// read hashes it, and the front end opens it again through its own
/// `SourceMap`. One extra read of one file per compile, and per check that
/// observes a change, against a compile — the ADR's own accounting makes the
/// hash the thing that is trusted, and there is no `SourceMap` to hand here.
///
/// # Errors
///
/// The read's own error, for a path that is absent or unreadable. A `stat` that
/// fails is not an error by itself: it leaves the stamp unknown and the read
/// below reports whatever is really wrong.
fn observe(path: &Path, validate: Validate, known: Option<PathEntry>) -> std::io::Result<Observed> {
    let stamp = stamp_of(path);
    if validate == Validate::Mtime
        && let (Some(stamp), Some(known)) = (stamp, known)
        && known.stamp == Some(stamp)
    {
        return Ok(Observed {
            content_hash: known.unit.content,
            stamp: Some(stamp),
        });
    }
    let source = std::fs::read(path)?;
    Ok(Observed {
        content_hash: content_hash(&source),
        stamp,
    })
}

impl Resolver for Compiler {
    /// Compiles `path` if it has not been compiled already, and hands back a
    /// program over its unit.
    ///
    /// The front end **renders its own diagnostics** to this process's standard
    /// error, exactly as `nvs check` does, and the message returned here is the
    /// one-line summary the parent sees as its failure value. Those are two
    /// audiences rather than one: a person fixing the child wants the spans, and
    /// the parent program wants a string it can print. `rule:security/isolate-shares-nothing`'s
    /// failure-is-a-value rule is about the second and says nothing that
    /// forbids the first.
    fn resolve(&self, path: &str) -> Result<Program, String> {
        // A spawned isolate is not answering a request, so the table
        // [`Self::compiled`] hands back is the half this seam has nothing to
        // do with — it still travels *into* the child, because the program
        // installs it on its own context.
        self.compiled(path).map(|(program, _)| program)
    }
}

/// The [`Program`] over one compiled unit: arm the isolate's context with that
/// unit's own tables, take the argument, and call the script frame.
///
/// The `Arc` is what keeps the code mapped for as long as the program can run
/// — a `Unit` owns its pages (`nvs_codegen::Unit`) — and it is a clone of the
/// cache's, so a second isolate over the same path, on this core or on
/// another, shares them rather than compiling again.
fn program_over(compiled: Arc<Compiled>) -> Program {
    Box::new(move |ctx: &mut Ctx, args: Value| -> Value {
        // The child unit's statics and its error class, which
        // `nvs_runtime::script::Program` requires before any of its code runs
        // and `Ctx::isolate` deliberately left empty.
        compiled.unit.install_in(ctx);
        // And `rule:routing/matched-once-before-the-handler`'s table, on the same terms and for the same reason
        // `nvs run` installs one: a member that reads it is reading a compile
        // product of *this* unit, and an isolate shares nothing else.
        if !compiled.routes.rows().is_empty() {
            ctx.set_routes(Arc::clone(&compiled.routes));
        }
        // Ownership discharged into the isolate's own root; the seam's type
        // doc owns why this and not a release here.
        ctx.set_isolate_argument(args);

        let Some(entry) = compiled.unit.script() else {
            // Not reachable for a unit that compiled — every program has a
            // script frame — but it is a failure value rather than a panic,
            // because a child may not be able to end its parent.
            ctx.set_pending("the child's script frame was not compiled");
            return Value::null();
        };
        // A throw leaves its object on the context, which is where
        // `nvs_host::Isolate`'s own `finish` reads it from — but an
        // `nvs_runtime::EXITED` leaves nothing there at all, and a `Program`
        // answers a `Value` and no status. So the status is recorded on the
        // context beside the answer, and `Ctx::set_ending` owns why that is the
        // only way an `exit` reaches the classifier.
        match entry.call(ctx) {
            Ok(answer) => answer,
            Err(status) => {
                ctx.set_ending(status);
                Value::null()
            }
        }
    })
}

/// A context granting `script.spawn` for everything, and nothing else.
///
/// Every test in this binary that runs a fixture which spawns needs one: ADR
/// 0118 § 1 denies by default, so a bare `Ctx` refuses at
/// [`nvs_runtime::script::resolve`]'s door and the test then asserts the denial
/// instead of whatever it was about. It lives here, beside the resolver, so
/// that the grant has one spelling in this crate rather than one per test
/// module — and it is deliberately not a production constructor, because
/// nothing outside a test should be able to hand itself a capability.
#[cfg(test)]
pub(crate) fn granting_ctx() -> nvs_runtime::Ctx {
    let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Buffer(Vec::new()));
    ctx.set_config(std::sync::Arc::new(granting_snapshot()));
    ctx
}

/// [`granting_ctx`]'s grant as the snapshot itself, for a test that arms
/// something which builds its own contexts.
///
/// `crate::worker::start` gives every job the configuration in force when it
/// is claimed — that module's *What a job's grants are* section is why — so a
/// test arming a queue worker has no context to hand it and needs the snapshot
/// underneath one. The same grant either way, which is the whole reason the
/// pair is here rather than a second capability literal in that test module.
#[cfg(test)]
pub(crate) fn granting_snapshot() -> nvs_config::Snapshot {
    let mut snapshot = nvs_config::Snapshot::default();
    snapshot.config.capabilities = Some(nvs_config::tree::Capabilities {
        script: Some(nvs_config::tree::CapScript {
            spawn: Some(nvs_config::tree::Setting::Bool(true)),
        }),
        // And `rule:http-server/allow-url-pins-the-address`'s outbound pair, for the fixture that reaches `rule:testing/in-process-request`'s ephemeral listener over the wire. Both halves are needed and
        // that is the rule rather than an inconvenience: `connect` names the
        // host, and `internal` is the operator's written exception for § 3's
        // denied loopback range — a `#[Test(server: true)]` in a real program
        // grants exactly this pair to reach its own listener, which is why the
        // helper grants it rather than the runner carving a hole for itself.
        net: Some(nvs_config::tree::CapNet {
            connect: Some(nvs_config::tree::Setting::List(vec!["127.0.0.1".into()])),
            internal: Some(nvs_config::tree::Setting::List(vec!["127.0.0.1".into()])),
            // None of the other grants is one `connect` implies, and none is asked here
            // (`rule:security/net-listen-is-a-separate-grant-from-net-connect`): the
            // ephemeral listener is opened by the runner rather than by the program under
            // test, so nothing in the fixture binds an endpoint, a TCP listener has no
            // socket path either way, the fixture reaches an address it wrote itself
            // rather than one it named with `connectTo`, and it follows no redirect at all
            // — least of all one down to plaintext.
            listen: None,
            local: None,
            connect_to: None,
            downgrade: None,
        }),
        ..nvs_config::tree::Capabilities::default()
    });
    snapshot
}

#[cfg(test)]
mod tests {
    use super::{CompileState, Compiler, ProbeHash, granting_ctx as granting, shared};
    use nvs_runtime::script::{Program, ResolveError, Resolver, resolve, scoped};
    use nvs_runtime::{Ctx, OutputSink, Value};
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    /// The repository root, which is what a written path is anchored at — and
    /// which `cargo test` does not run in, hence the manifest directory.
    fn from_root(relative: &str) -> String {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(relative);
        root.to_string_lossy().into_owned()
    }

    /// Runs a resolved program inside a real isolate, which is the only way to
    /// read the two things the program actually produces: what the child wrote,
    /// and what its top-level `return` answered.
    fn run_child(path: &str) -> nvs_host::Completion {
        let compiler = Compiler::default();
        run_program(compiler.resolve(path).expect("the child compiles"))
    }

    /// The same, over a program resolved earlier — which is what a holder of
    /// one does with it after the cache has moved on.
    fn run_program(program: Program) -> nvs_host::Completion {
        run_under(program, Ctx::new(OutputSink::Buffer(Vec::new())))
    }

    /// The same over a parent that may `spawn script`, on a scheduler and a
    /// reactor of its own — which is the whole of what a program reaching this
    /// resolver from inside its own run needs, and is what `main`'s run
    /// installs for exactly the same reason. The capability is `rule:security/capability-question-is-grant-and-scope`'s,
    /// denied by default, and [`granting_ctx`] is why the grant has one
    /// spelling in this crate.
    fn run_serving(program: Program) -> nvs_host::Completion {
        let mut sched = nvs_host::Scheduler::new();
        let completion = std::rc::Rc::new(std::cell::RefCell::new(None));
        sched.spawn(granting(), nvs_runtime::TaskRoot::Request, {
            let completion = std::rc::Rc::clone(&completion);
            move |ctx| {
                *completion.borrow_mut() = Some(
                    nvs_host::Isolate::new(program, Value::null(), nvs_host::Output::Capture)
                        .run(ctx)
                        .expect("a null argument crosses"),
                );
            }
        });
        let installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("a reactor starts"));
        nvs_host::run_until_idle(&mut sched).expect("the scheduler finishes");
        drop(installed);
        let taken = completion.borrow_mut().take();
        taken.expect("the task ran")
    }

    fn run_under(program: Program, mut parent: Ctx) -> nvs_host::Completion {
        nvs_host::Isolate::new(program, Value::null(), nvs_host::Output::Capture)
            .run(&mut parent)
            .expect("a null argument crosses")
    }

    /// A `.nvs` file this test owns, whose whole body echoes `said`. Called
    /// again with the same `name`, it is the edit.
    fn a_file_saying(name: &str, said: &str) -> std::path::PathBuf {
        a_file_running(name, &format!("echo \"{said}\", \"\\n\";"))
    }

    /// The same file with a body of its own, for a case whose statement is not
    /// an `echo`. The path is handed back with forward slashes available from
    /// [`written`], because a Windows temp path inside a source literal is a
    /// run of escapes rather than a path.
    fn a_file_running(name: &str, body: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("nvs-swap-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory to write the case in");
        let path = dir.join("entry.nvs");
        std::fs::write(&path, format!("<?nvs\n{body}\n")).expect("the case is writable");
        path
    }

    /// One of those paths as a source literal can carry it.
    fn written(path: &std::path::Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    /// A compiler whose check reads the content itself, and takes a change the
    /// moment it sees one. A test runs the check by hand, with
    /// [`Compiler::revalidate`], where a server runs it on [`watch`]'s thread.
    ///
    /// No value is the production default, and all three are spelled in
    /// `[opcache]` on purpose: `mtime` answers from a stamp that two writes
    /// inside one filesystem tick share, and the default one-second `settle`
    /// would hold back an edit a test made microseconds ago. What is being
    /// asserted below is the swap, not the timing —
    /// `nvs_config::cache::Revalidation` is where all three are decided.
    fn revalidating() -> Compiler {
        checking("hash", "0s")
    }

    /// The same, with `validate` and `revalidate_freq` chosen by the test.
    fn checking(validate: &str, freq: &str) -> Compiler {
        use nvs_config::tree::{Config, Opcache, Setting};
        Compiler::new(&Config {
            opcache: Some(Opcache {
                validate: Some(Setting::Text(validate.to_owned())),
                revalidate_freq: Some(Setting::Text(freq.to_owned())),
                settle: Some(Setting::Text("0s".to_owned())),
                ..Opcache::default()
            }),
            ..Config::default()
        })
    }

    /// What a resolved program prints — which is how a test reads *which* unit
    /// a resolve handed back, there being nothing else to compare two
    /// [`Program`]s by.
    fn said(program: Program) -> String {
        let completion = run_program(program);
        assert!(completion.ok, "error: {:?}", completion.error);
        String::from_utf8_lossy(&completion.output).into_owned()
    }

    #[test]
    fn a_path_becomes_a_program_that_runs_as_an_isolate() {
        let completion = run_child(&from_root("examples/isolate/hello.nvs"));
        assert!(completion.ok, "error: {:?}", completion.error);
        assert_eq!(
            String::from_utf8_lossy(&completion.output),
            "child said hello\n"
        );
    }

    #[test]
    fn a_childs_uncaught_throw_arrives_as_a_failure_value_rather_than_an_err() {
        // The other half of the boundary's asymmetry, reached through a real
        // compiled child rather than a hand-built closure: the program ran, so
        // this is the child's fault and not the argument's.
        let completion = run_child(&from_root("examples/isolate/throws.nvs"));
        assert!(!completion.ok);
        let error = completion.error.expect("a failure carries one");
        assert_eq!(error.class, "RuntimeError");
        assert_eq!(error.message, "child could not finish");
    }

    #[test]
    fn one_written_path_is_compiled_once_however_many_isolates_run_it() {
        // `rule:security/isolate-shares-nothing`'s "shares immutable compiled code", which is this cache and
        // nothing else — the module doc says so, and this is what says it is
        // true.
        let compiler = Compiler::default();
        let path = from_root("examples/isolate/capture.nvs");
        let _first = compiler.resolve(&path).expect("the child compiles");
        let _second = compiler.resolve(&path).expect("and again, from the cache");
        assert_eq!(shared(&compiler.units).len(), 1);
    }

    /// A program whose `autoload` resolution really probes: `Framework\Core` is
    /// declared under the *second* root, so the trace it records is a miss
    /// under `./src` and a hit under `./vendor` — which is
    /// `rule:packaging/autoload-probes-fold-into-the-cache-key`'s own example.
    /// The directory is rebuilt from nothing, because a `src/Core.nvs` left by
    /// an earlier run is the very thing the cases below create themselves.
    fn an_autoloading_program(name: &str) -> (std::path::PathBuf, String) {
        let dir = std::env::temp_dir().join(format!("nvs-probe-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).expect("the root the probe misses");
        std::fs::create_dir_all(dir.join("vendor")).expect("the root it hits");
        std::fs::write(
            dir.join("Bootstrap.nvs"),
            "<?nvs\nautoload 'Framework' from './src', './vendor';\n",
        )
        .expect("the file declaring the map");
        std::fs::write(dir.join("vendor").join("Core.nvs"), declaring("vendor"))
            .expect("the autoloaded class, under the second root");
        std::fs::write(
            dir.join("entry.nvs"),
            "<?nvs\nrequire './Bootstrap.nvs';\nvar $app = new Framework\\Core();\necho $app->say(), \"\\n\";\n",
        )
        .expect("the entry point");
        let written = dir.join("entry.nvs").to_string_lossy().into_owned();
        (dir, written)
    }

    /// `Framework\Core`, saying which root it was found under — the one thing
    /// two copies of it under two roots differ by.
    fn declaring(root: &str) -> String {
        format!(
            "<?nvs\nnamespace Framework;\nclass Core {{ public function say(): string {{ return '{root}'; }} }}\n"
        )
    }

    #[test]
    fn a_unit_is_published_under_the_trace_its_autoload_resolution_probed() {
        // `rule:packaging/autoload-probes-fold-into-the-cache-key` from the
        // side that has to *spell* a key: the trace is produced by the very
        // compile the key exists to spare, so a resolve addresses the table
        // once with what it knows and the compile re-keys its result with what
        // it probed. A program whose trace is not empty is the only case where
        // those two keys differ, and what would break here is a second resolve
        // spelling a key nothing is under — which is a permanent recompile
        // rather than a wrong answer, hence the counter.
        let (_dir, written) = an_autoloading_program("published");

        let compiler = Compiler::default();
        let (_program, _routes) = compiler.compiled(&written).expect("the entry compiles");
        let probed = shared(&compiler.traces)
            .values()
            .next()
            .and_then(|traced| traced.current)
            .expect("one current trace")
            .1;
        assert_ne!(
            probed,
            ProbeHash::unrecorded(),
            "the miss under `./src` and the hit under `./vendor` reached no key",
        );
        let units = shared(&compiler.units);
        assert_eq!(units.len(), 1, "the claimed key outlived the published one");
        assert_eq!(
            units.keys().next().expect("one unit").probes(),
            probed,
            "the unit is not under the trace the map in front of it names",
        );
        drop(units);

        let (_again, _routes) = compiler
            .compiled(&written)
            .expect("and again, from the cache");
        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            1,
            "the second resolve could not spell the key the first one published",
        );
        assert_eq!(shared(&compiler.units).len(), 1);
    }

    #[test]
    fn a_file_created_where_a_probe_missed_recompiles_the_unit() {
        // `rule:packaging/autoload-probes-fold-into-the-cache-key`'s claim end
        // to end. Between the two resolves below, every file the first one
        // compiled is byte-for-byte what it was and the entry point's own
        // digest has not moved — so a table keyed on content alone would serve
        // the vendor unit forever, and the recorded miss under `./src` is the
        // only thing that says the answer was ever a question.
        // `crates/nvs-hir/src/requires.rs` pins the trace this rests on; what
        // is asked here is the cache in front of it.
        let (dir, written) = an_autoloading_program("shadowing");
        let compiler = revalidating();

        let (before, _routes) = compiler.compiled(&written).expect("the entry compiles");
        assert_eq!(said(before), "vendor\n");

        std::fs::write(dir.join("src").join("Core.nvs"), declaring("src"))
            .expect("the shadowing class, under the first root");
        compiler.revalidate();
        let (after, _routes) = compiler
            .compiled(&written)
            .expect("and again, with the shadow in place");
        assert_eq!(
            said(after),
            "src\n",
            "the unit compiled before `src/Core.nvs` existed is still being served",
        );
        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            2,
            "the shadow was noticed by something other than a compile",
        );
        assert_eq!(
            shared(&compiler.units).len(),
            2,
            "the table holds more than the unit in force and the one it replaced",
        );
    }

    #[test]
    fn an_edit_to_a_required_file_recompiles_the_unit_and_a_deleted_one_fails_it() {
        // `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s
        // whole-program key. The entry file never changes below, so every
        // answer that moves was noticed through the trace of the files the
        // compile read — and the restore is noticed through the path the
        // failed compile looked for and did not find.
        let dir = std::env::temp_dir().join(format!("nvs-required-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the program's directory");
        let lib = dir.join("lib.nvs");
        std::fs::write(&lib, "<?nvs\necho \"one\\n\";\n").expect("the required file");
        std::fs::write(dir.join("entry.nvs"), "<?nvs\nrequire './lib.nvs';\n")
            .expect("the entry point");
        let written = dir.join("entry.nvs").to_string_lossy().into_owned();
        let compiler = revalidating();

        let (first, _routes) = compiler.compiled(&written).expect("the entry compiles");
        assert_eq!(said(first), "one\n");

        std::fs::write(&lib, "<?nvs\necho \"two\\n\";\n").expect("the edit");
        compiler.revalidate();
        let (edited, _routes) = compiler.compiled(&written).expect("the edit compiles");
        assert_eq!(
            said(edited),
            "two\n",
            "the required file's edit was not seen"
        );

        std::fs::remove_file(&lib).expect("the deletion");
        compiler.revalidate();
        assert!(
            compiler.compiled(&written).is_err(),
            "a program whose required file is gone still compiled",
        );

        std::fs::write(&lib, "<?nvs\necho \"three\\n\";\n").expect("the restore");
        compiler.revalidate();
        let (restored, _routes) = compiler.compiled(&written).expect("the restore compiles");
        assert_eq!(said(restored), "three\n", "the restored file was not seen");
        assert_eq!(
            shared(&compiler.units).len(),
            2,
            "the table holds more than the unit in force and the one it replaced",
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_reverted_edit_to_a_required_file_is_answered_without_a_compile() {
        // `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s
        // reverted edit, where the entry file's digest never moves: both
        // programs have a trace under the same entry content, and the check
        // finds the older one describing the disk again.
        let dir = std::env::temp_dir().join(format!("nvs-reverted-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the program's directory");
        let lib = dir.join("lib.nvs");
        std::fs::write(&lib, "<?nvs\necho \"one\\n\";\n").expect("the required file");
        std::fs::write(dir.join("entry.nvs"), "<?nvs\nrequire './lib.nvs';\n")
            .expect("the entry point");
        let written = dir.join("entry.nvs").to_string_lossy().into_owned();
        let compiler = revalidating();

        let (first, _routes) = compiler.compiled(&written).expect("the entry compiles");
        assert_eq!(said(first), "one\n");
        std::fs::write(&lib, "<?nvs\necho \"two, edited\\n\";\n").expect("the edit");
        compiler.revalidate();
        let (edited, _routes) = compiler.compiled(&written).expect("the edit compiles");
        assert_eq!(said(edited), "two, edited\n");

        std::fs::write(&lib, "<?nvs\necho \"one\\n\";\n").expect("the revert");
        compiler.revalidate();
        let (reverted, _routes) = compiler.compiled(&written).expect("the revert resolves");
        assert_eq!(said(reverted), "one\n", "the revert was not seen");
        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            2,
            "the reverted program was compiled again",
        );

        // And the edit it undid is now the unit it replaced, so redoing it is a
        // swap back.
        std::fs::write(&lib, "<?nvs\necho \"two, edited\\n\";\n").expect("the redo");
        compiler.revalidate();
        let (redone, _routes) = compiler.compiled(&written).expect("the redo resolves");
        assert_eq!(said(redone), "two, edited\n", "the redo was not seen");
        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            2,
            "the redone program was compiled again",
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ten_thousand_concurrent_cold_requests_compile_the_file_exactly_once() {
        // `docs/plan/m7.md`'s core requirement, asserted at the only place a
        // compile happens: ten thousand requests for one cold file, every one
        // of them created before any of them runs — `Scheduler::spawn`'s own
        // doc says nothing runs until `run` is called, so this is the widest
        // concurrency one core admits — and one compile between them.
        //
        // `[opcache]` is written at `hash`/`0s` rather than left at the
        // default, because the default answers requests 2..N from step 1
        // without looking at the file and would make this a test of the rate
        // cap. Here every request after the first re-observes the file, hashes
        // it, and is still answered out of the unit table, because on one
        // accepting core no second resolve of the path runs between an
        // observation and the write that follows it. This is the number that
        // says so, and the module doc's second known gap is what has to land
        // before it says the same thing on four cores.
        const REQUESTS: usize = 10_000;

        let entry = a_file_saying("cold", "served");
        let path = entry.to_string_lossy().into_owned();
        let compiler = std::rc::Rc::new(revalidating());
        let answered = std::rc::Rc::new(std::cell::Cell::new(0_usize));

        let mut sched = nvs_host::Scheduler::new();
        for _ in 0..REQUESTS {
            let compiler = std::rc::Rc::clone(&compiler);
            let answered = std::rc::Rc::clone(&answered);
            let path = path.clone();
            // One task per request, as `serve::run` spawns one per connection,
            // reaching the same `Rc<Compiler>` that command builds per core.
            sched.spawn(
                Ctx::new(OutputSink::Buffer(Vec::new())),
                nvs_runtime::TaskRoot::Request,
                move |_ctx| {
                    let (_program, _routes) = compiler.compiled(&path).expect("the entry compiles");
                    answered.set(answered.get() + 1);
                },
            );
        }
        let report = sched.run();

        assert_eq!(report.finished, REQUESTS, "a request never reached its end");
        assert_eq!(
            answered.get(),
            REQUESTS,
            "a request was answered with no unit"
        );
        assert_eq!(
            compiler.compiles.load(std::sync::atomic::Ordering::Relaxed),
            1,
            "the file was put through the front end more than once"
        );
        // And the table did not grow an entry per request either, which is the
        // same claim stated as what the cache holds afterwards.
        assert_eq!(shared(&compiler.units).len(), 1);
    }

    #[test]
    fn ten_thousand_concurrent_cold_requests_for_one_file_compile_it_exactly_once() {
        // The test above's claim, made about the fleet — which is the shape
        // `docs/plan/m7.md` states it in and the only shape a served request
        // meets. One core collapses the race by construction: no second resolve
        // of a path runs between an observation and the write that follows it.
        // Here the ten thousand requests are spread over four workers sharing
        // one `Arc<Compiler>`, as `serve::run` hands every core a clone of one,
        // and every request is cold — so what holds the count at one is step
        // 3's flight rather than anything about the shape of the test.
        //
        // A worker is an OS thread driving a `Scheduler` of its own, because
        // that is what a worker is: a `Scheduler` is `!Send`, and every request
        // on it reaches the cache from that thread. The barrier is what makes
        // the requests concurrent rather than merely numerous — each worker has
        // its whole queue spawned before any worker takes a turn of one, so the
        // fleet arrives at a cold cache together.
        //
        // `[opcache]` is `hash`/`0s` for the reason the test above names: the
        // default would answer requests 2..N without looking at the file, which
        // would make this a test of the rate cap.
        const WORKERS: usize = 4;
        const PER_WORKER: usize = 2_500;

        let entry = a_file_saying("fleet-cold", "served");
        let path = entry.to_string_lossy().into_owned();
        let compiler = Arc::new(revalidating());
        let answered = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let ready = std::sync::Barrier::new(WORKERS);

        let finished: usize = std::thread::scope(|fleet| {
            let workers: Vec<_> = (0..WORKERS)
                .map(|_| {
                    fleet.spawn(|| {
                        let mut sched = nvs_host::Scheduler::new();
                        for _ in 0..PER_WORKER {
                            let compiler = Arc::clone(&compiler);
                            let answered = Arc::clone(&answered);
                            let path = path.clone();
                            sched.spawn(
                                Ctx::new(OutputSink::Buffer(Vec::new())),
                                nvs_runtime::TaskRoot::Request,
                                move |_ctx| {
                                    let (_program, _routes) =
                                        compiler.compiled(&path).expect("the entry compiles");
                                    answered.fetch_add(1, Ordering::Relaxed);
                                },
                            );
                        }
                        ready.wait();
                        sched.run().finished
                    })
                })
                .collect();
            workers
                .into_iter()
                .map(|worker| worker.join().expect("a worker ran its queue"))
                .sum()
        });

        assert_eq!(
            finished,
            WORKERS * PER_WORKER,
            "a request never reached its end"
        );
        assert_eq!(
            answered.load(Ordering::Relaxed),
            WORKERS * PER_WORKER,
            "a request was answered with no unit"
        );
        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            1,
            "a worker put the file through the front end for itself"
        );
        // And what the fleet holds afterwards is one entry rather than one per
        // worker: the flight they waited behind was replaced by the unit every
        // one of them was answered with.
        assert_eq!(shared(&compiler.units).len(), 1);
    }

    #[test]
    fn a_compiled_unit_is_read_by_every_core_through_one_arc() {
        // The same "compile exactly once" claim as the test above, stated
        // about the fleet rather than about one core, and the half a
        // scheduler cannot state: the readers here are real OS threads, and
        // what each of them is handed is a clone of the *one* published
        // `Arc<Compiled>` rather than a unit of its own.
        //
        // The compile is warmed on this thread first, deliberately. Four cold
        // resolves racing would be a test of the flight step 3 puts them behind
        // — which is the test above — rather than of the publishing this one is
        // about.
        const CORES: usize = 4;

        let entry = a_file_saying("one-arc", "served");
        let path = entry.to_string_lossy().into_owned();
        let compiler = Arc::new(revalidating());
        let (warm, _routes) = compiler.compiled(&path).expect("the entry compiles");
        // Dropped so that every reference counted below belongs to a core.
        drop(warm);

        // Two barriers rather than a join, because the count being asserted
        // is only true while the readers are still holding what they read: a
        // thread that has already returned has dropped its clone.
        let holding = std::sync::Barrier::new(CORES + 1);
        let releasing = std::sync::Barrier::new(CORES + 1);
        std::thread::scope(|cores| {
            for _ in 0..CORES {
                cores.spawn(|| {
                    let (program, _routes) = compiler.compiled(&path).expect("the entry compiles");
                    holding.wait();
                    releasing.wait();
                    drop(program);
                });
            }

            holding.wait();
            let units = shared(&compiler.units);
            let CompileState::Ready(published) = units.values().next().expect("one entry") else {
                panic!("the published entry is a failure");
            };
            assert_eq!(
                Arc::strong_count(published),
                CORES + 1,
                "a core is holding something other than the published entry"
            );
            // And the unit inside it is one allocation, referenced by that
            // one entry: this is what "through one arc" means, and it is the
            // number that would be `CORES` if each core had compiled its own.
            assert_eq!(
                Arc::strong_count(&published.unit),
                1,
                "the published unit exists more than once"
            );
            drop(units);
            releasing.wait();
        });

        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            1,
            "a core put the file through the front end for itself"
        );
    }

    #[test]
    fn a_reader_holding_the_old_unit_keeps_answering_until_it_drops_it() {
        // `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s
        // step 4 seen from a core that resolved *before* two swaps. The table
        // keeps the unit in force and the one it replaced, so [`record`]'s
        // sweep takes the first generation out when the third is published,
        // and the only thing keeping that reader's pages mapped is the `Arc`
        // its own [`Program`] carries — and the counts below are that sentence
        // as a number, rather than an argument from the program having run.
        let path = a_file_saying("outlives", "one");
        let written = path.to_string_lossy().into_owned();
        let compiler = Arc::new(revalidating());

        // The published entry, cloned out while it is still reachable: once
        // the edit lands there is no route back to it through the cache, which
        // is the whole of what is being asserted.
        let (warm, _routes) = compiler.compiled(&written).expect("the entry compiles");
        let old = {
            let units = shared(&compiler.units);
            let CompileState::Ready(published) = units.values().next().expect("one entry") else {
                panic!("the published entry is a failure");
            };
            Arc::clone(published)
        };
        // Dropped so that every reference counted below belongs to a reader.
        drop(warm);

        // Two barriers for the reason the test above has two: the reader has
        // to be holding what it resolved while the edit is published, and it
        // has to still be holding it afterwards.
        let resolved = std::sync::Barrier::new(2);
        let swapped = std::sync::Barrier::new(2);
        std::thread::scope(|cores| {
            cores.spawn(|| {
                let (program, _routes) = compiler.compiled(&written).expect("the entry compiles");
                resolved.wait();
                swapped.wait();
                assert_eq!(
                    said(program),
                    "one\n",
                    "a reader was overtaken by an edit it never resolved"
                );
            });

            resolved.wait();
            let _ = a_file_saying("outlives", "two");
            compiler.revalidate();
            let (between, _routes) = compiler
                .compiled(&written)
                .expect("the edited entry compiles");
            drop(between);
            let _ = a_file_saying("outlives", "three");
            compiler.revalidate();
            let (after, _routes) = compiler
                .compiled(&written)
                .expect("the edited entry compiles again");
            let held = shared(&compiler.units).len();
            let holders = Arc::strong_count(&old);
            // Released before asserting, so a failure below cannot leave the
            // reader waiting on the barrier for ever.
            swapped.wait();
            drop(after);
            assert_eq!(
                held, 2,
                "the table holds more than the unit in force and the one it replaced"
            );
            assert_eq!(
                holders, 2,
                "the old unit is held by something other than its reader and this test"
            );
        });

        // The reader has returned, so its clone is gone and the last hold on
        // the old unit is this test's own: nothing in the cache outlived it.
        assert_eq!(
            Arc::strong_count(&old),
            1,
            "the old unit is still held after its last reader dropped it"
        );
    }

    #[test]
    fn units_held_stay_bounded_after_ten_thousand_edits() {
        // `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s
        // last paragraph: the table keeps, per path, the unit in force and the
        // one it replaced, so what it holds is in proportion to entry files and
        // never to edits. Every edit below is a content the table has not seen,
        // so each one is compiled and published, and each publish is a sweep.
        // The artifact cache is off, so the run leaves no file behind it.
        use nvs_config::tree::{Config, Opcache, Setting};
        const EDITS: u64 = 10_000;
        let dir = std::env::temp_dir().join(format!("nvs-bounded-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the program's directory");
        let lib = dir.join("lib.nvs");
        std::fs::write(&lib, "<?nvs\necho \"0\";\n").expect("the required file");
        std::fs::write(dir.join("entry.nvs"), "<?nvs\nrequire './lib.nvs';\n")
            .expect("the entry point");
        let written = dir.join("entry.nvs").to_string_lossy().into_owned();
        let compiler = Compiler::new(&Config {
            opcache: Some(Opcache {
                validate: Some(Setting::Text("hash".to_owned())),
                revalidate_freq: Some(Setting::Text("0s".to_owned())),
                settle: Some(Setting::Text("0s".to_owned())),
                file_cache: Some(false),
                ..Opcache::default()
            }),
            ..Config::default()
        });

        let (first, _routes) = compiler.compiled(&written).expect("the entry compiles");
        for edit in 1..=EDITS {
            std::fs::write(&lib, format!("<?nvs\necho \"{edit}\";\n")).expect("the edit");
            compiler.revalidate();
            assert!(
                compiler.held() <= 2,
                "after edit {edit} the table holds {} units",
                compiler.held()
            );
        }
        let (last, _routes) = compiler.compiled(&written).expect("the last edit compiles");
        assert_eq!(
            said(last),
            EDITS.to_string(),
            "the last edit was not swapped in"
        );
        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            EDITS + 1,
            "an edit was answered without its own compile"
        );
        assert_eq!(
            shared(&compiler.paths).len(),
            1,
            "one path holds more than one pointer"
        );
        let traces: usize = shared(&compiler.traces)
            .values()
            .map(|traced| traced.traces.len())
            .sum();
        assert!(traces <= 2, "the trace table holds {traces} traces");
        // A request that resolved before the first edit still holds its unit,
        // which the table let go of long ago.
        assert_eq!(
            said(first),
            "0",
            "an old unit did not outlive the table's hold on it"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_revalidation_that_wins_publishes_and_readers_never_block_on_a_compile() {
        // The two halves of `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s
        // step 4 that only a fleet has a spelling for: the revalidation that
        // wins the compare moves the pointer, and the cores serving requests
        // meanwhile are answered *throughout* it.
        // `no_request_stalls_while_the_file_is_compiled` states the second half
        // on one core, where a stall is a resume; here it is a thread, and what
        // says nobody waited is that every reader was answered many times
        // inside the window a single compile occupies.
        //
        // The readers resolve a path nobody edits, deliberately. A reader
        // landing on the edited one would wait behind step 3's flight for the
        // revalidation's own compile, and that is a test of the wait rather
        // than of the publish.
        const READERS: usize = 3;

        let read = a_file_saying("winner-read", "one");
        let reading = read.to_string_lossy().into_owned();
        let swap = a_file_saying("winner-swap", "one");
        let swapping = swap.to_string_lossy().into_owned();
        let compiler = Arc::new(revalidating());

        // Both warm first, so that what races below is a revalidation against
        // readers rather than two cold compiles against each other.
        let (warm_read, _routes) = compiler
            .compiled(&reading)
            .expect("the read entry compiles");
        drop(warm_read);
        let (warm_swap, _routes) = compiler
            .compiled(&swapping)
            .expect("the swapped entry compiles");
        drop(warm_swap);
        let before = shared(&compiler.paths)[&swap].unit.content;

        let started = std::sync::Barrier::new(READERS + 1);
        let published = std::sync::atomic::AtomicBool::new(false);
        let answered = std::thread::scope(|cores| {
            let readers: Vec<_> = (0..READERS)
                .map(|_| {
                    cores.spawn(|| {
                        started.wait();
                        let mut answers = 0_usize;
                        while !published.load(Ordering::Relaxed) {
                            let (program, _routes) = compiler
                                .compiled(&reading)
                                .expect("the read entry compiles");
                            answers += 1;
                            drop(program);
                        }
                        answers
                    })
                })
                .collect();

            // The edit, published while every reader is already looping.
            started.wait();
            let _ = a_file_saying("winner-swap", "two");
            compiler.revalidate();
            let (after, _routes) = compiler
                .compiled(&swapping)
                .expect("the edited entry compiles");
            published.store(true, Ordering::Relaxed);

            let answered: Vec<_> = readers
                .into_iter()
                .map(|reader| reader.join().expect("a reader finished"))
                .collect();
            assert_eq!(
                said(after),
                "two\n",
                "the revalidation that won published something else"
            );
            answered
        });

        assert_ne!(
            shared(&compiler.paths)[&swap].unit.content,
            before,
            "the revalidation won the compare and did not publish"
        );
        assert_eq!(
            shared(&compiler.units).len(),
            3,
            "the read path's unit, and the swapped path's unit in force and the one it replaced"
        );
        // Three compiles: the two warm-ups and the edit. A reader that had been
        // made to wait for the compile would have gone back to the table
        // afterwards; one that had gone through the front end for itself would
        // be counted here.
        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            3,
            "a reader put something through the front end of its own"
        );
        for answers in answered {
            assert!(
                answers > 1,
                "a reader was answered once and then waited the compile out: {answers}"
            );
        }
        // And the path they were reading is untouched by the swap beside it.
        let (still, _routes) = compiler
            .compiled(&reading)
            .expect("the read entry compiles");
        assert_eq!(said(still), "one\n");
    }

    #[test]
    fn a_stale_revalidation_does_not_overwrite_a_fresher_published_one() {
        // Step 4's compare, driven at [`Compiler::advance`] because that is
        // the one place two revalidations of a path can be *ordered* rather
        // than raced: a resolve that observed the file before the edit is
        // exactly a caller arriving with a `since` the pointer has moved past,
        // and threads could only reproduce that by winning a coin toss.
        let path = a_file_saying("stale", "one");
        let written = path.to_string_lossy().into_owned();
        let compiler = revalidating();
        let (first, _routes) = compiler.compiled(&written).expect("the entry compiles");
        drop(first);
        let stale_unit = shared(&compiler.paths)[&path].unit;
        let stale = stale_unit.content;

        let _ = a_file_saying("stale", "two");
        compiler.revalidate();
        let (second, _routes) = compiler
            .compiled(&written)
            .expect("the edited entry compiles");
        drop(second);
        let fresher = shared(&compiler.paths)[&path].unit.content;
        assert_ne!(stale, fresher, "the edit never reached the pointer");

        // The slower revalidation, landing after the one that overtook it. It
        // compiled content this table already holds, so it is answered either
        // way; what it must not do is name that content as the path's current
        // one a second time.
        let observed = super::Observed {
            content_hash: stale,
            stamp: None,
        };
        assert!(
            !compiler.advance(&path, &observed, stale_unit, Some(stale)),
            "a revalidation that observed the file first won step 4 by finishing last"
        );
        assert_eq!(
            shared(&compiler.paths)[&path].unit.content,
            fresher,
            "the published content was rolled back to what a slower resolve saw"
        );

        // And the next resolve starts from the fresher pointer, which is the
        // whole of what step 4 is for.
        let (after, _routes) = compiler.compiled(&written).expect("the entry compiles");
        assert_eq!(said(after), "two\n");
    }

    #[test]
    fn a_revalidation_that_fails_to_compile_fails_only_the_requests_that_resolve_it_afterwards() {
        // `rule:config/a-broken-edit-fails-the-requests-that-resolve-it`'s
        // first paragraph, over the three things it decides at once: the
        // caller that resolves the broken content is handed the failure as
        // ordinary checked-return data, the path's pointer is left naming the
        // content that compiled, and the request already holding the last good
        // unit runs to completion regardless
        // (`rule:config/a-request-keeps-the-unit-it-resolved`).
        //
        // A resolve answers from the pointer without looking at the file, so
        // the check that reaches the edit is [`Compiler::revalidate`], run by
        // hand where a server runs it on [`watch`]'s thread.
        let path = a_file_saying("broken-edit", "one");
        let written = path.to_string_lossy().into_owned();
        let compiler = revalidating();

        let (running, _routes) = compiler.compiled(&written).expect("the entry compiles");
        let good = shared(&compiler.paths)[&path].unit.content;

        // The edit that does not parse, and the resolve that reaches it. What
        // comes back is a message rather than a panic or a stale unit, which
        // is the rule's "fail loudly".
        let _ = a_file_running("broken-edit", "echo \"one\" \"two\";");
        compiler.revalidate();
        let Err(refusal) = compiler.compiled(&written) else {
            panic!("a file that does not parse was handed back as a program");
        };
        assert!(
            refusal.contains("could not be compiled"),
            "unhelpful refusal: {refusal}"
        );
        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            2,
            "the broken content never reached the front end"
        );

        // Step 4 never ran, so the path still resolves to the content that
        // compiled — and [`Compiler::record`]'s `keep` is why that unit is
        // still in the table beside the failure rather than swept by it.
        assert_eq!(
            shared(&compiler.paths)[&path].unit.content,
            good,
            "a broken edit moved the pointer"
        );
        assert_eq!(
            shared(&compiler.units).len(),
            2,
            "the unit still in force, and the failure the next resolve of that content is owed"
        );

        // The request that resolved before the edit is unaffected by it: it
        // holds its own unit and the file behind it is not read again.
        assert_eq!(said(running), "one\n");

        // And a repair reaches the next resolve on the same terms the break
        // did, nothing about the failure being sticky past its own content key.
        let _ = a_file_saying("broken-edit", "two");
        compiler.revalidate();
        let (repaired, _routes) = compiler
            .compiled(&written)
            .expect("the repaired entry compiles");
        assert_eq!(said(repaired), "two\n");
        assert_ne!(
            shared(&compiler.paths)[&path].unit.content,
            good,
            "the repair never reached the pointer"
        );
    }

    #[test]
    fn a_storm_against_a_broken_file_costs_one_compile_and_one_rendering_of_its_spans() {
        // The same rule's cost claim, in the shape only a fleet has a spelling
        // for: every request after the first lands on the same `UnitKey` and
        // is answered from the table, so the break is compiled — and its spans
        // rendered — once however many requests arrive against it.
        // [`Compiler::compiles`] is what says so, because it counts a compile
        // that failed and the front end it counts is the half that renders.
        const WORKERS: usize = 4;
        const PER_WORKER: usize = 500;

        let entry = a_file_saying("broken-storm", "served");
        let path = entry.to_string_lossy().into_owned();
        let compiler = Arc::new(revalidating());

        // Warmed first, so what the storm meets is a break in a file this
        // cache already serves rather than a cold path that never compiled.
        let (warm, _routes) = compiler.compiled(&path).expect("the entry compiles");
        let good = shared(&compiler.paths)[&entry].unit.content;
        drop(warm);
        let _ = a_file_running("broken-storm", "echo \"one\" \"two\";");
        compiler.revalidate();

        let refused = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let ready = std::sync::Barrier::new(WORKERS);
        let finished: usize = std::thread::scope(|fleet| {
            let workers: Vec<_> = (0..WORKERS)
                .map(|_| {
                    fleet.spawn(|| {
                        let mut sched = nvs_host::Scheduler::new();
                        for _ in 0..PER_WORKER {
                            let compiler = Arc::clone(&compiler);
                            let refused = Arc::clone(&refused);
                            let path = path.clone();
                            sched.spawn(
                                Ctx::new(OutputSink::Buffer(Vec::new())),
                                nvs_runtime::TaskRoot::Request,
                                move |_ctx| {
                                    if compiler.compiled(&path).is_err() {
                                        refused.fetch_add(1, Ordering::Relaxed);
                                    }
                                },
                            );
                        }
                        ready.wait();
                        sched.run().finished
                    })
                })
                .collect();
            workers
                .into_iter()
                .map(|worker| worker.join().expect("a worker ran its queue"))
                .sum()
        });

        assert_eq!(
            finished,
            WORKERS * PER_WORKER,
            "a request never reached its end"
        );
        assert_eq!(
            refused.load(Ordering::Relaxed),
            WORKERS * PER_WORKER,
            "a request resolving the broken content was answered with a unit"
        );
        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            2,
            "the storm put the same break through the front end more than once"
        );
        assert_eq!(
            shared(&compiler.paths)[&entry].unit.content,
            good,
            "the storm moved the pointer off the content that compiled"
        );
    }

    #[test]
    fn the_compile_counter_counts_compiles_and_not_cores() {
        // [`Compiler::compiles`]'s own claim, which is what `docs/plan/m7.md`'s
        // "compiles it exactly once" is asserted against: one counter per
        // cache rather than one per worker. Two contents move it twice; any
        // number of cores reading either of them does not move it at all.
        const CORES: usize = 4;

        let path = a_file_saying("counted", "one");
        let written = path.to_string_lossy().into_owned();
        let compiler = Arc::new(revalidating());

        let (warm, _routes) = compiler.compiled(&written).expect("the entry compiles");
        drop(warm);
        assert_eq!(compiler.compiles.load(Ordering::Relaxed), 1);

        // The cores resolve warm on purpose: four *cold* resolves racing would
        // be counting the flight they wait behind, which has its own test,
        // rather than this claim about what moves the counter at all.
        let read_by_every_core = || {
            std::thread::scope(|cores| {
                for _ in 0..CORES {
                    cores.spawn(|| {
                        let (_program, _routes) =
                            compiler.compiled(&written).expect("the entry compiles");
                    });
                }
            });
        };

        read_by_every_core();
        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            1,
            "a core reading the published unit was counted as a compile"
        );

        // A second content is a second compile: the number moves with what was
        // put through the front end, and it stays where it is however many
        // cores then read the result.
        let _ = a_file_saying("counted", "two");
        compiler.revalidate();
        let (edited, _routes) = compiler
            .compiled(&written)
            .expect("the edited entry compiles");
        drop(edited);
        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            2,
            "a new content reached the front end without being counted"
        );

        read_by_every_core();
        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            2,
            "a core reading the swapped-in unit was counted as a compile"
        );
    }

    #[test]
    fn no_request_stalls_while_the_file_is_compiled() {
        // The other half of `docs/plan/m7.md`'s core requirement, and the half
        // the number above cannot state: compiling once is worth nothing if
        // the other requests paid for it by waiting.
        //
        // On one core a stall has exactly one shape — a suspension. A request
        // made to wait for a compile in flight would have to give the core
        // back and be resumed once the unit existed, and `RunReport::resumes`
        // counts precisely that: a task that runs from its first turn to its
        // end without ever yielding costs one resume, and every wait costs
        // another. So `resumes == REQUESTS` is "nobody waited", asserted rather
        // than argued from what step 3 does.
        //
        // It stays true now that step 3 has a flight to wait behind, and this
        // is why: on one accepting core no second resolve of a path runs
        // between an observation and the write that follows it, so the flight
        // is never contended and the wait it exists for is one only a second
        // worker can reach. That wait is a thread block rather than a
        // suspension in any case, so it could never show up in this count —
        // `ten_thousand_concurrent_cold_requests_for_one_file_compile_it_exactly_once`
        // is where it is asserted instead.
        const REQUESTS: usize = 64;

        let entry = a_file_saying("unstalled", "served");
        let path = entry.to_string_lossy().into_owned();
        let compiler = std::rc::Rc::new(revalidating());
        let held: std::rc::Rc<std::cell::RefCell<Vec<Program>>> =
            std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));

        let mut sched = nvs_host::Scheduler::new();
        for _ in 0..REQUESTS {
            let compiler = std::rc::Rc::clone(&compiler);
            let held = std::rc::Rc::clone(&held);
            let path = path.clone();
            sched.spawn(
                Ctx::new(OutputSink::Buffer(Vec::new())),
                nvs_runtime::TaskRoot::Request,
                move |_ctx| {
                    let (program, _routes) = compiler.compiled(&path).expect("the entry compiles");
                    held.borrow_mut().push(program);
                },
            );
        }
        let report = sched.run();

        assert_eq!(report.finished, REQUESTS, "a request never reached its end");
        assert_eq!(report.parked, 0, "a request was left parked on the compile");
        assert_eq!(
            report.resumes, REQUESTS,
            "a request gave the core back and was resumed, which is the stall"
        );
        assert_eq!(
            compiler.compiles.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        // And what the requests that did not compile were handed is the unit,
        // not a placeholder waiting to be filled in: every one of them runs.
        assert_eq!(held.borrow().len(), REQUESTS);
        for program in held.borrow_mut().drain(..) {
            assert_eq!(said(program), "served\n");
        }
    }

    #[test]
    fn a_path_that_is_not_a_program_is_refused_with_a_message_rather_than_a_panic() {
        let compiler = Compiler::default();
        let Err(refusal) =
            compiler.resolve(&from_root("examples/isolate/there-is-no-such-file.nvs"))
        else {
            panic!("nothing to compile, so nothing to hand back");
        };
        assert!(
            refusal.contains("could not be compiled"),
            "unhelpful refusal: {refusal}"
        );
    }

    #[test]
    fn an_open_connection_keeps_its_compiled_unit_across_an_edit() {
        // `rule:concurrency/connection-bounds-are-finite`'s second bullet, first half — asserted at the cache the
        // bullet is a statement about. A connection isolate's hold on its code
        // *is* the [`Program`] a resolve handed it (`nvs_host::Isolate` runs
        // one), and `nvs serve` resolves per request through this compiler, so
        // an edit reaches a connection only if it reaches this table. What
        // cannot be driven from here is the socket: `serve::run` is an
        // `ExitCode` entry point over one accept loop, so the second
        // connection the bullet compares against has no spelling in a unit
        // test of this crate.
        let path = a_file_saying("keeps", "one");
        let compiler = revalidating();
        let (open, _) = compiler
            .compiled(&path.to_string_lossy())
            .expect("the entry compiles");

        // The edit, and a resolve after it — a new connection's, which is what
        // makes this a test of the swap rather than of a cache nobody touched.
        let _ = a_file_saying("keeps", "two");
        compiler.revalidate();
        let (_swapped, _) = compiler
            .compiled(&path.to_string_lossy())
            .expect("the edited entry compiles");

        let completion = run_program(open);
        assert!(completion.ok, "error: {:?}", completion.error);
        assert_eq!(String::from_utf8_lossy(&completion.output), "one\n");
    }

    #[test]
    fn a_connection_opened_after_the_swap_runs_the_new_unit() {
        // The other half of the bullet, over the same fixture: what a resolve
        // taken after the edits hands back is the *new* unit. The length
        // assertion is `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s
        // own accounting — the table keeps the unit in force and the one it
        // replaced, rather than growing an entry per edit — and it holds while
        // the program resolved before the edits is still alive, because that
        // one's pages are kept by its own `Arc` (`script`'s module doc).
        let path = a_file_saying("swaps", "one");
        let compiler = revalidating();
        let (before, _) = compiler
            .compiled(&path.to_string_lossy())
            .expect("the entry compiles");

        let _ = a_file_saying("swaps", "two");
        compiler.revalidate();
        let (between, _) = compiler
            .compiled(&path.to_string_lossy())
            .expect("the edited entry compiles");
        drop(between);
        let _ = a_file_saying("swaps", "three");
        compiler.revalidate();
        let (after, _) = compiler
            .compiled(&path.to_string_lossy())
            .expect("the edited entry compiles again");

        let completion = run_program(after);
        assert!(completion.ok, "error: {:?}", completion.error);
        assert_eq!(String::from_utf8_lossy(&completion.output), "three\n");
        assert_eq!(shared(&compiler.units).len(), 2);
        drop(before);
    }

    #[test]
    fn a_swap_never_blocks_a_request_serving_core() {
        // `rule:config/an-edit-reaches-the-next-request-without-a-restart`: the
        // swap is the background check's, and a serving core only reads the
        // pointer it moved. The caller that proves the core is free is a
        // program already in flight, because its `spawn script` re-enters this
        // resolver from inside the very run a held guard would have to span.
        let child = a_file_saying("in-flight", "one");
        let compiler = revalidating();
        let (holding, _) = compiler
            .compiled(&child.to_string_lossy())
            .expect("the child compiles");
        let before = shared(&compiler.paths)[&child].unit.content;

        // The edit, the check that swaps it in, and the request after it: this
        // parent resolves the edited path mid-run, through the seam `spawn
        // script` lowers to.
        let _ = a_file_saying("in-flight", "two");
        compiler.revalidate();
        let parent = a_file_running(
            "serving",
            &format!(
                "var $swapped = spawn script '{}';\necho \"served\", \"\\n\";",
                written(&child)
            ),
        );
        let (running, _) = compiler
            .compiled(&parent.to_string_lossy())
            .expect("the parent compiles");
        let completion = scoped(&compiler, || run_serving(running));

        // The core served its own request through the swap, the swap published
        // — step 4's pointer write happened underneath a running program — and
        // the unit that program was handed before the edit is untouched, which
        // is the same paragraph's first half.
        assert!(completion.ok, "error: {:?}", completion.error);
        assert_eq!(String::from_utf8_lossy(&completion.output), "served\n");
        assert_ne!(
            shared(&compiler.paths)[&child].unit.content,
            before,
            "the swap did not publish"
        );
        assert_eq!(said(holding), "one\n");
    }

    #[test]
    fn revalidation_is_lazy_and_rate_capped() {
        // `rule:config/an-edit-reaches-the-next-request-without-a-restart`: no
        // request looks at the file, and the background check is what does.
        // What a resolve hands back is what the last look observed, so an edit
        // between two resolves says whether the second one looked. Nothing
        // here asserts a syscall count directly, because a count would pin the
        // implementation rather than the rule.
        //
        // `revalidate_freq` is zero, which is the setting that made every
        // resolve look while the check ran inside it.
        let path = a_file_saying("lazy", "one");
        let compiler = revalidating();
        let (before, _) = compiler
            .compiled(&path.to_string_lossy())
            .expect("the entry compiles");
        let _ = a_file_saying("lazy", "two");
        let (unlooked, _) = compiler
            .compiled(&path.to_string_lossy())
            .expect("the entry resolves again");
        assert_eq!(said(before), "one\n");
        assert_eq!(said(unlooked), "one\n", "a resolve read the file");
        assert_eq!(compiler.compiles.load(Ordering::Relaxed), 1);

        // The check sees the edit, and compiles it before any request asks.
        compiler.revalidate();
        assert_eq!(
            compiler.compiles.load(Ordering::Relaxed),
            2,
            "the check did not compile the edit"
        );
        let (checked, _) = compiler
            .compiled(&path.to_string_lossy())
            .expect("the edited entry resolves");
        assert_eq!(said(checked), "two\n", "the check did not swap the edit in");
        assert_eq!(compiler.compiles.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn a_change_is_held_back_until_the_program_is_quiet_for_settle() {
        // `[opcache] settle`: an edit younger than the settle time is left
        // alone by the check, and taken by the first check after it.
        use nvs_config::tree::{Config, Opcache, Setting};
        let path = a_file_saying("settling", "one");
        let compiler = Compiler::new(&Config {
            opcache: Some(Opcache {
                validate: Some(Setting::Text("hash".to_owned())),
                settle: Some(Setting::Text("300ms".to_owned())),
                ..Opcache::default()
            }),
            ..Config::default()
        });
        let (_, _) = compiler
            .compiled(&path.to_string_lossy())
            .expect("the entry compiles");
        let _ = a_file_saying("settling", "two");
        let wait = compiler.revalidate().expect("the edit is held back");
        assert!(wait <= std::time::Duration::from_millis(300), "{wait:?}");
        assert_eq!(compiler.compiles.load(Ordering::Relaxed), 1);

        std::thread::sleep(wait + std::time::Duration::from_millis(20));
        assert_eq!(compiler.revalidate(), None);
        assert_eq!(compiler.compiles.load(Ordering::Relaxed), 2);
        let (after, _) = compiler
            .compiled(&path.to_string_lossy())
            .expect("the edited entry resolves");
        assert_eq!(said(after), "two\n");
    }

    #[test]
    fn the_seam_reaches_this_resolver_once_it_is_installed() {
        // The route itself, end to end: what `nvs run` installs is what a
        // lowered `spawn script` will call, and neither names the other.
        let mut ctx = granting();
        assert_eq!(
            resolve(&ctx, "examples/isolate/hello.nvs").err(),
            Some(ResolveError::NoResolver)
        );
        let compiler = Compiler::default();
        scoped(&compiler, || {
            let program = resolve(&ctx, &from_root("examples/isolate/hello.nvs"))
                .expect("the installed resolver answers");
            let _ = program(&mut ctx, Value::null());
        });
        // And gone again the moment the call returned, which is the half a
        // leaked resolver could not have.
        assert_eq!(
            resolve(&ctx, "examples/isolate/hello.nvs").err(),
            Some(ResolveError::NoResolver)
        );
        assert_eq!(
            String::from_utf8_lossy(&ctx.take_buffered_output().unwrap_or_default()),
            "child said hello\n"
        );
    }

    /// A context granting every write and naming `root` as `rule:core-classes/temporary-dir-sweep`'s owned
    /// root, written the way an operator writes both — the grant because
    /// `Core\IO::temporaryDir` asks `fs.write` for the path it is about to
    /// create (`rule:security/capability-check-at-the-door`), and the root because the default is the platform
    /// one and a case asserting a root is empty must own that root outright.
    ///
    /// Not [`granting_ctx`]: that one is the `script.spawn` grant a fixture
    /// which spawns needs, and this case spawns nothing.
    fn rooted_at(root: &std::path::Path) -> Ctx {
        use nvs_config::tree::{CapFs, Capabilities, Io, Setting};

        let mut snapshot = nvs_config::Snapshot::default();
        snapshot.config.capabilities = Some(Capabilities {
            fs: Some(CapFs {
                write: Some(Setting::Bool(true)),
                ..CapFs::default()
            }),
            ..Capabilities::default()
        });
        snapshot.config.io = Some(Io {
            temp_root: Some(root.to_string_lossy().into_owned()),
        });
        let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        ctx.set_config(std::sync::Arc::new(snapshot));
        ctx
    }

    /// `rule:core-classes/temporary-dir-sweep` read end to end: a whole script asks for a temporary
    /// directory, fills it, ends — and the owned root holds nothing.
    ///
    /// `-p nvs-runtime` already pins the sweep per context, which is the same
    /// claim one layer down; what only this layer can say is that a *program*
    /// compiled from a file and run as an isolate reaches it, because the
    /// context that owns the tracked path is the one `nvs_host::Isolate::run`
    /// makes and drops, and nothing in this crate arranges that on purpose.
    ///
    /// Three assertions and each is load-bearing. The directory landing under
    /// the configured root is what stops the emptiness below from being vacuous
    /// — a member ignoring `[io] temp_root` leaves a root that was empty all
    /// along. The file written inside it is what makes the removal recursive
    /// rather than an `rmdir` that happened to succeed on an empty directory.
    /// And the root *surviving* is § 2's: the sweep takes what § 1 handed out,
    /// never the root it created under, which is the one directory the next
    /// script on this host still needs.
    #[test]
    fn a_finished_scripts_temporary_dir_is_gone_from_the_owned_root() {
        let root = std::env::temp_dir().join(format!("nvs-cli-swept-{}", std::process::id()));
        // A pid outlives one `cargo test`, so a case that panicked in an earlier
        // run under this number would otherwise leave an entry behind and fail
        // this one for it.
        let _ = std::fs::remove_dir_all(&root);
        let entry = a_file_running(
            "temporary-dir",
            r#"string $dir = Core\IO::temporaryDir();
Core\IO::write($dir . "/note.txt", "written while the script ran");
echo $dir, "\n";"#,
        );

        let compiler = Compiler::default();
        let program = compiler
            .resolve(&entry.to_string_lossy())
            .expect("the entry compiles");
        let completion = run_under(program, rooted_at(&root));

        assert!(completion.ok, "error: {:?}", completion.error);
        let made = std::path::PathBuf::from(String::from_utf8_lossy(&completion.output).trim_end());
        assert!(
            made.starts_with(&root),
            "the script's directory was made under the configured root: {} is not under {}",
            made.display(),
            root.display()
        );
        assert!(
            !made.exists(),
            "and the script ending took it away, note and all: {} is still there",
            made.display()
        );
        assert!(
            root.is_dir()
                && root
                    .read_dir()
                    .is_ok_and(|mut entries| entries.next().is_none()),
            "leaving the owned root itself standing and empty"
        );

        std::fs::remove_dir_all(&root).expect("the case removes what it made");
    }
}
