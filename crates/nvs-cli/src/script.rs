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
//! **What it spends:** at most two compiled units per distinct written path —
//! the one in force and, while an edit does not compile, the failure the next
//! resolve of that same content is answered with. O(the program's text), never
//! O(isolates spawned) and never O(edits), per
//! `rule:programs/memory-priority` — and freed with
//! the resolver, which is a local of `nvs run` published through
//! [`nvs_runtime::script::scoped`] rather than leaked.
//!
//! # Decision: `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s five steps, and what a shared cache still owes
//!
//! `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s § *Decision* is implemented here whole, because this is the
//! tree's only in-memory unit table: a [`PathEntry`] holding the digest and the
//! stamp the last check observed, in front of a table keyed by
//! [`UnitKey`]`{ path, content_hash, env_hash }`. A resolve walks its five
//! steps — reuse the known digest under `[opcache] validate = "never"` or
//! inside `revalidate_freq`; otherwise `stat`, and re-read the source only
//! where the stamp cannot answer; compile only content this table has not seen;
//! write the digest back on success; leave it alone on failure, and answer that
//! caller with the failure the new content is now keyed to.
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
use std::sync::{Arc, Condvar, Mutex, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Instant, SystemTime};

use nvs_config::cache::{Digest, EnvHash, Revalidation, UnitKey, Validate, content_hash, env_hash};
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
    /// The digest of the content this path last *compiled* to, which is the
    /// half of its [`UnitKey`] that moves.
    content_hash: Digest,
    /// What `validate = "mtime"` compares against, and `None` where the file
    /// system answered with neither — a path whose stamp cannot be read is
    /// re-hashed rather than trusted.
    stamp: Option<Stamp>,
    /// When the last check happened. `revalidate_freq` gates the next one
    /// against this, which is what makes the cost `N ⁄ freq` rather than `N`.
    last_checked: Instant,
}

/// The `mtime`/size pair `rule:config/an-edit-reaches-the-next-request-without-a-restart` calls the cheap
/// pre-filter: enough to say a file did *not* change, never enough to say what
/// it now holds.
///
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Stamp {
    modified: SystemTime,
    len: u64,
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
    /// The environment half of every key here — `rule:config/the-extension-set-is-in-every-unit-key`'s digest, taken
    /// once from the configuration this process booted, because it is constant
    /// for the life of a snapshot.
    ///
    env: EnvHash,
    /// `[opcache] validate` and `revalidate_freq`, read once for the same
    /// reason: both are `System`-class, so no request can move them.
    revalidation: Revalidation,
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
    cache: Option<crate::cache::Cache>,
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
            env: env_hash(config),
            revalidation: Revalidation::from_config(config),
            cache: crate::cache::from_config(config),
            compiles: AtomicU64::new(0),
        }
    }

    /// The program over `path`'s unit **and** that unit's route table, which is
    /// what a server needs and what [`Resolver::resolve`]'s own signature has
    /// nowhere to put.
    ///
    /// `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s five steps, in order, with step 3's
    /// single flight: one caller compiles a content and every other waits on it.
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
        // What step 4 below compares against: the pointer as this resolve
        // found it, read once here so that everything after it — the `stat`,
        // the hash and the compile — happens outside the map.
        let since = known.map(|entry| entry.content_hash);

        // 1. The syscall this resolve does not make: `validate = "never"` is
        //    production's answer for every resolve, and the rate cap is the
        //    same answer for the requests arriving inside one window.
        if let Some(entry) = known
            && (self.revalidation.validate == Validate::Never
                || entry.last_checked.elapsed() < self.revalidation.freq)
            && let Some(answer) = self.answer(&written, entry.content_hash)
        {
            return answer;
        }

        // 2. Otherwise look. A file the system will not answer for keeps
        //    whatever it last resolved to — step 5's reading, for the same
        //    reason: the entry still names the last content that compiled, and
        //    a path being replaced by a rename is momentarily absent. One with
        //    no entry has nothing to fall back on, so it is reported here in
        //    the shape `front_end` would have reported it.
        let observed = match observe(&written, self.revalidation.validate, known) {
            Ok(observed) => observed,
            Err(error) => {
                if let Some(answer) = known.and_then(|e| self.answer(&written, e.content_hash)) {
                    return answer;
                }
                eprintln!("error: could not read {}: {error}", written.display());
                return Err(format!(
                    "`{path}` could not be compiled; see the errors above"
                ));
            }
        };

        // Step 2's second half and step 3's content key in one lookup: an
        // observation that did not move addresses the entry the last one wrote,
        // and one that did may still name content this process compiled before
        // — a reverted edit, or a broken one being re-observed. Either way this
        // resolve is answered without reaching a flight at all.
        if let Some(answer) = self.answer(&written, observed.content_hash) {
            if answer.is_ok() {
                self.advance(&written, &observed, since);
            }
            return answer;
        }

        // 3. The compile itself, which is the only step that costs anything —
        //    and which the fleet pays for once. The caller that claims this
        //    content's key is the one that runs it; a caller that arrives while
        //    it runs waits behind the same flight, so a cold path stormed by
        //    every core at once costs one front end rather than one per core.
        let key = UnitKey::new(&written, observed.content_hash, self.env);
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
        if !claimed && let Some(answer) = self.answer(&written, observed.content_hash) {
            if answer.is_ok() {
                self.advance(&written, &observed, since);
            }
            return answer;
        }
        let landing = Landing(&flight);
        let state = match self.compile(path, &written) {
            Ok(compiled) => CompileState::Ready(compiled),
            Err(message) => CompileState::Failed(message),
        };
        // 4 and 5: the pointer moves only on success, and what the table keeps
        // for this path is the entry in force plus, at most, the failure the
        // next resolve of this content is owed.
        let ready = matches!(state, CompileState::Ready(_));
        let keep = if ready {
            None
        } else {
            known.map(|entry| entry.content_hash)
        };
        self.record(key, state, keep);
        // The waiters, released once the answer is in the table and not before.
        // The explicit drop is the ordering; the guard is for the path where
        // the line above never ran at all.
        drop(landing);
        if ready {
            self.advance(&written, &observed, since);
        }
        self.answer(&written, observed.content_hash)
            .expect("the state just written is in the table")
    }

    /// What the table holds for `path` at `content`, and `None` where it holds
    /// nothing — the one place a [`CompileState`] becomes a caller's answer.
    fn answer(
        &self,
        path: &Path,
        content: Digest,
    ) -> Option<Result<(Program, Arc<nvs_runtime::routes::Routes>), String>> {
        match shared(&self.units).get(&UnitKey::new(path, content, self.env))? {
            // A compile in flight is not an answer, and saying so here is what
            // sends a step-1 hit on this content down to step 3 to wait for it
            // rather than reporting that the cache holds nothing.
            CompileState::Compiling(_) => None,
            CompileState::Ready(compiled) => Some(Ok((
                program_over(Arc::clone(compiled)),
                Arc::clone(&compiled.routes),
            ))),
            CompileState::Failed(message) => Some(Err(message.clone())),
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

    /// Step 4's pointer write: what this path resolves to now, and the moment
    /// the cap is measured from — published **only if nobody moved the pointer
    /// since**, which is the compare that rule's step 4 states rather than an
    /// assignment.
    ///
    /// `since` is the digest [`Self::compiled`] copied out of the map on its
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
    fn advance(&self, path: &Path, observed: &Observed, since: Option<Digest>) -> bool {
        let mut paths = exclusive(&self.paths);
        if paths.get(path).map(|entry| entry.content_hash) != since {
            return false;
        }
        paths.insert(
            path.to_path_buf(),
            PathEntry {
                content_hash: observed.content_hash,
                stamp: observed.stamp,
                last_checked: Instant::now(),
            },
        );
        true
    }

    /// `state` under `key`, and the two entries this path is then allowed to
    /// keep: the content just reached, and `keep` where a failure leaves an
    /// older unit still in force.
    ///
    /// The sweep is what keeps the table O(paths): every earlier generation of
    /// this path goes, and a unit a running [`Program`] still holds stays
    /// mapped through that program's own `Arc` rather than through this map.
    fn record(&self, key: UnitKey, state: CompileState, keep: Option<Digest>) {
        let mut units = exclusive(&self.units);
        let reached = key.content_hash();
        units.retain(|other, _| {
            other.path() != key.path()
                || other.content_hash() == reached
                || Some(other.content_hash()) == keep
        });
        units.insert(key, state);
    }

    /// The front end and the backend, over one path, with this process's own
    /// table holding nothing for it: the whole of what step 3 costs.
    ///
    /// The backend half may still come off disk — [`Self::cache`] is asked here
    /// and nowhere else — and that is why the counter below keeps counting a
    /// warm hit as a compile. `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header` is explicit that a hit skips
    /// codegen and not the front end, so the front end really did run; what a
    /// hit saves is the Cranelift walk, which this counter never claimed to
    /// measure.
    ///
    fn compile(&self, path: &str, written: &Path) -> Result<Arc<Compiled>, String> {
        // Counted here rather than at the call site, and before the front end
        // rather than after it: a compile that *failed* is still a compile
        // this cache paid for, and the claim being counted is about how many
        // times the file was put through the front end at all.
        self.compiles.fetch_add(1, Ordering::Relaxed);
        let checked = crate::front_end(written)
            .map_err(|_| format!("`{path}` could not be compiled; see the errors above"))?;
        // Held across the lowering and the key alike: § 1's digest is over
        // every file the `require`/`autoload` graph reached, which is the same
        // list that was lowered and not the entry file alone.
        let files = checked.program_files();
        let lowered = nvs_ir::lower::lower_program(
            nvs_ir::lower::ENTRY_SCRIPT_LABEL,
            &files,
            &checked.exprs,
            &checked.interner,
            &checked.enums,
            &checked.layouts,
        );
        let unit = crate::cache::unit_for(
            &lowered,
            crate::cache::program_digest(&files),
            self.cache.as_ref(),
        )
        .map(|(unit, _)| unit)
        .map_err(|error| format!("`{path}`: {error}"))?;
        Ok(Arc::new(Compiled {
            unit: Arc::new(unit),
            routes: Arc::new(crate::runtime_routes(checked.exprs.routes())),
        }))
    }
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
    let stamp = std::fs::metadata(path).ok().and_then(|meta| {
        Some(Stamp {
            modified: meta.modified().ok()?,
            len: meta.len(),
        })
    });
    if validate == Validate::Mtime
        && let (Some(stamp), Some(known)) = (stamp, known)
        && known.stamp == Some(stamp)
    {
        return Ok(Observed {
            content_hash: known.content_hash,
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
/// `crate::worker::start` hands every worker the run's own configuration —
/// that module's *Why the grants are the run's own* section is why — so a test
/// arming a queue worker has no context to hand it and needs the snapshot
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
    use super::{CompileState, Compiler, granting_ctx as granting, shared};
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

    /// A compiler that checks the content itself, on every resolve.
    ///
    /// Neither half is the production default and both are spelled in
    /// `[opcache]` on purpose: `mtime` answers from a stamp that two writes
    /// inside one filesystem tick share, and the default two-second cap puts
    /// the second write of a test that takes microseconds inside the first
    /// check's window. What is being asserted below is the swap, not the rate
    /// cap — `nvs_config::cache::Revalidation` is where both are decided.
    fn revalidating() -> Compiler {
        checking("hash", "0s")
    }

    /// The same, with both directives written out — the shape a test that is
    /// *about* `[opcache]` reaches for, since either value alone decides
    /// whether a resolve looks at the file.
    fn checking(validate: &str, freq: &str) -> Compiler {
        use nvs_config::tree::{Config, Opcache, Setting};
        Compiler::new(&Config {
            opcache: Some(Opcache {
                validate: Some(Setting::Text(validate.to_owned())),
                revalidate_freq: Some(Setting::Text(freq.to_owned())),
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
        // step 4 seen from a core that resolved *before* the swap. [`record`]'s
        // sweep takes the old generation out of the table the moment the new
        // content is published, so the only thing keeping that reader's pages
        // mapped is the `Arc` its own [`Program`] carries — and the counts
        // below are that sentence as a number, rather than an argument from
        // the program having run.
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
            let (after, _routes) = compiler
                .compiled(&written)
                .expect("the edited entry compiles");
            assert_eq!(
                shared(&compiler.units).len(),
                1,
                "the swept generation is still in the table"
            );
            assert_eq!(
                Arc::strong_count(&old),
                2,
                "the old unit is held by something other than its reader and this test"
            );
            swapped.wait();
            drop(after);
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
        let before = shared(&compiler.paths)[&swap].content_hash;

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
            shared(&compiler.paths)[&swap].content_hash,
            before,
            "the revalidation won the compare and did not publish"
        );
        assert_eq!(
            shared(&compiler.units).len(),
            2,
            "one path per generation, so a swept generation is still in the table"
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
        let stale = shared(&compiler.paths)[&path].content_hash;

        let _ = a_file_saying("stale", "two");
        let (second, _routes) = compiler
            .compiled(&written)
            .expect("the edited entry compiles");
        drop(second);
        let fresher = shared(&compiler.paths)[&path].content_hash;
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
            !compiler.advance(&path, &observed, Some(stale)),
            "a revalidation that observed the file first won step 4 by finishing last"
        );
        assert_eq!(
            shared(&compiler.paths)[&path].content_hash,
            fresher,
            "the published content was rolled back to what a slower resolve saw"
        );

        // And the next resolve starts from the fresher pointer, which is the
        // whole of what step 4 is for.
        let (after, _routes) = compiler.compiled(&written).expect("the entry compiles");
        assert_eq!(said(after), "two\n");
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
        // taken after the edit hands back is the *new* unit. The length
        // assertion is `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s own accounting — the pointer moved rather
        // than the table growing an entry per edit — and it holds while the
        // program resolved before the edit is still alive, because that one's
        // pages are kept by its own `Arc` (`script`'s module doc).
        let path = a_file_saying("swaps", "one");
        let compiler = revalidating();
        let (before, _) = compiler
            .compiled(&path.to_string_lossy())
            .expect("the entry compiles");

        let _ = a_file_saying("swaps", "two");
        let (after, _) = compiler
            .compiled(&path.to_string_lossy())
            .expect("the edited entry compiles");

        let completion = run_program(after);
        assert!(completion.ok, "error: {:?}", completion.error);
        assert_eq!(String::from_utf8_lossy(&completion.output), "two\n");
        assert_eq!(shared(&compiler.units).len(), 1);
        drop(before);
    }

    #[test]
    fn a_swap_never_blocks_a_request_serving_core() {
        // `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s paragraph after the five steps, in the
        // spelling one core has for it. The ADR keeps a *thread* free by
        // running step 3 on the compile pool; what keeps this core free is the
        // property [`PathEntry`]'s own doc states — neither map's guard is held
        // across the stat or the compile — and the caller that proves it is a
        // program already in flight, because its `spawn script` re-enters this
        // resolver from inside the very run a held guard would have to span.
        let child = a_file_saying("in-flight", "one");
        let compiler = revalidating();
        let (holding, _) = compiler
            .compiled(&child.to_string_lossy())
            .expect("the child compiles");
        let before = shared(&compiler.paths)[&child].content_hash;

        // The edit a serving core is about to find, and the request that finds
        // it: this parent resolves the edited path mid-run, through the seam
        // `spawn script` lowers to.
        let _ = a_file_saying("in-flight", "two");
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
            shared(&compiler.paths)[&child].content_hash,
            before,
            "the swap did not publish"
        );
        assert_eq!(said(holding), "one\n");
    }

    #[test]
    fn revalidation_is_lazy_and_rate_capped() {
        // `rule:config/an-edit-reaches-the-next-request-without-a-restart` step 1, both halves. A `stat` is counted the
        // only way a unit test can count one: `observe` is the single place
        // this module makes one, and what a resolve hands back is what it
        // observed — so an edit between two resolves says whether the second
        // one looked at all. Nothing here asserts a syscall count directly,
        // because a count would pin the implementation rather than the rule.

        // `validate = "never"` is production's answer for *every* resolve, and
        // it is not the rate cap wearing a longer window: the cap below is
        // written at zero here, so a resolve that consulted the clock at all
        // would look, and this one still does not.
        let never = a_file_saying("never", "one");
        let compiler = checking("never", "0s");
        let (first, _) = compiler
            .compiled(&never.to_string_lossy())
            .expect("the entry compiles");
        let _ = a_file_saying("never", "two");
        let (again, _) = compiler
            .compiled(&never.to_string_lossy())
            .expect("the entry resolves again");
        assert_eq!(said(first), "one\n");
        assert_eq!(said(again), "one\n", "a `never` resolve read the file");

        // The cap, on both sides of one window, since a resolve that stopped
        // one edit early reads plausibly against either half alone. Inside a
        // 60-second window the second resolve is answered from the entry the
        // first one wrote — one check for the two of them.
        let capped = a_file_saying("capped", "one");
        let compiler = checking("hash", "60s");
        let (before, _) = compiler
            .compiled(&capped.to_string_lossy())
            .expect("the entry compiles");
        let _ = a_file_saying("capped", "two");
        let (inside, _) = compiler
            .compiled(&capped.to_string_lossy())
            .expect("the entry resolves again");
        assert_eq!(said(before), "one\n");
        assert_eq!(said(inside), "one\n", "a capped resolve read the file");

        // Past the window — the fixture writes it at zero — the same pair of
        // resolves makes two checks, and the second one sees the edit.
        let past = a_file_saying("uncapped", "one");
        let compiler = revalidating();
        let (old, _) = compiler
            .compiled(&past.to_string_lossy())
            .expect("the entry compiles");
        let _ = a_file_saying("uncapped", "two");
        let (new, _) = compiler
            .compiled(&past.to_string_lossy())
            .expect("the edited entry compiles");
        assert_eq!(said(old), "one\n");
        assert_eq!(said(new), "two\n", "an uncapped resolve did not look");
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
