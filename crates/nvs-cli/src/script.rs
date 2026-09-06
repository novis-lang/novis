//! `nvs-cli`'s half of [`nvs_runtime::script`]: the path a `spawn script`
//! wrote, compiled into a [`Program`] an isolate can run.
//!
//! The seam's own module doc owns why the compiler is reached this way round
//! rather than from `nvs-host`. This file owns the two things only an
//! implementor can decide.
//!
//! # Decision: a relative path is anchored at the working directory
//!
//! Not at the entry file.
//! [ADR 0104](/docs/adr/0104-an-application-is-an-entry-file-path.md)
//! makes an application an entry-file path and says nothing about a child, so
//! there was a choice to make, and the working directory is the one a reader of
//! the program can already predict: it is what every other path a CLI program
//! writes is relative to — `require`'s spelling excepted, which ADR 0021
//! resolves against the requiring file because a library has to move as a unit.
//! A `spawn script` target is not a library; it is a second program, named the
//! way the shell that started this one would name it. `examples/isolate.nvs`
//! is written against exactly this and says so in its own comment.
//!
//! # Decision: one unit per written path, swapped when its content moves
//!
//! ADR 0006's "an isolate shares immutable compiled code" is a property of this
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
//! [ADR 0004](/docs/adr/0004-memory-for-simplicity.md) — and freed with
//! the resolver, which is a local of `nvs run` published through
//! [`nvs_runtime::script::scoped`] rather than leaked.
//!
//! # Decision: [ADR 0017]'s five steps, and what one core collapses
//!
//! [ADR 0017]'s § *Decision* is implemented here whole, because this is the
//! tree's only in-memory unit table: a [`PathEntry`] holding the digest and the
//! stamp the last check observed, in front of a table keyed by
//! [`UnitKey`]`{ path, content_hash, env_hash }`. A resolve walks its five
//! steps — reuse the known digest under `[opcache] validate = "never"` or
//! inside `revalidate_freq`; otherwise `stat`, and re-read the source only
//! where the stamp cannot answer; compile only content this table has not seen;
//! write the digest back on success; leave it alone on failure, and answer that
//! caller with the failure the new content is now keyed to.
//!
//! **A single core collapses the concurrent half of it.** That ADR is written
//! against `DashMap`s reached from many request-serving cores, and specifies a
//! compile pool, a `Compiling`/`Ready`/`Failed` broadcast every racing caller
//! single-flights on, and a step 4 that writes a new digest back *only if a
//! fresher revalidation has not won*. This cache is a [`RefCell`] reached from
//! one coroutine on one core: there is no second resolve of a path between an
//! observation and the write that follows it, so the compare in step 4 is
//! **unreachable rather than relaxed**, and single-flighting is a property of
//! the borrow rather than machinery. `Compiling` has no representation for the
//! same reason — nothing can observe this cache while a compile is running in
//! it. What survives is [`CompileState`]'s other two states, which are
//! observable: a second resolve landing on content that already failed is
//! answered from the table rather than compiled again.
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
//! [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md) § 7's
//! second bullet is a statement about. The swap is a write to the *table*: a
//! [`Program`] already handed out owns its unit's pages through its own `Rc`,
//! so a connection isolate runs to completion on the code it began with while
//! the next resolve of that path hands the new unit to whoever asks next.
//!
//! [ADR 0017]: /docs/adr/0017-hot-reload-without-restart.md

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use nvs_config::cache::{Digest, EnvHash, Revalidation, UnitKey, Validate, content_hash, env_hash};
use nvs_config::tree::Config;
use nvs_runtime::script::{Program, Resolver};
use nvs_runtime::{Ctx, Value};

/// One compiled unit, and the one compile product a *caller* of this cache
/// still needs beside it.
///
/// ADR 0102 § 1's table is not something the unit's code can be asked for: it
/// is matched against **before** any of that code runs, by the door, so it has
/// to be reachable without running the program. Holding it here is what makes
/// "the compiled unit's route table" a thing the server can have — the cache
/// that already answers "which unit serves this file" is the one place both
/// halves of that answer exist.
#[derive(Debug)]
pub(crate) struct Compiled {
    /// The unit itself, whose `Rc` is what keeps its pages mapped.
    unit: Rc<nvs_codegen::Unit>,
    /// The routes it declared, already crossed into the runtime's own shape.
    /// Empty for a program with no `#[Route]`, which is ADR 0077 § 5's opt-in
    /// rule and is one case rather than an `Option`'s two.
    routes: Arc<nvs_runtime::routes::Routes>,
}

/// What one written path resolved to last, and when that was checked — [ADR
/// 0017]'s `PathEntry`, the pointer an edit swaps.
///
/// Copied out of the map rather than borrowed across the `stat` and the compile
/// below it, which is why every field is [`Copy`]: holding the borrow over a
/// front-end run would make the map unreachable from the `spawn script` that
/// run may itself perform.
///
/// [ADR 0017]: /docs/adr/0017-hot-reload-without-restart.md
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

/// The `mtime`/size pair [ADR 0017] § *Investigation* calls the cheap
/// pre-filter: enough to say a file did *not* change, never enough to say what
/// it now holds.
///
/// [ADR 0017]: /docs/adr/0017-hot-reload-without-restart.md
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Stamp {
    modified: SystemTime,
    len: u64,
}

/// [ADR 0017] § *Decision* step 3's state machine, less the state one core
/// cannot be in — the module doc owns why `Compiling` has no spelling here.
///
/// [ADR 0017]: /docs/adr/0017-hot-reload-without-restart.md
#[derive(Debug)]
enum CompileState {
    /// The unit, and the route table beside it.
    Ready(Rc<Compiled>),
    /// The one-line summary this content failed with, kept so that every later
    /// resolve landing on the same [`UnitKey`] is answered rather than
    /// recompiled.
    Failed(String),
}

/// What one resolve saw of the file behind a path.
struct Observed {
    content_hash: Digest,
    stamp: Option<Stamp>,
}

/// The one implementor: the front end and the backend `nvs run` already
/// carries, plus [ADR 0017]'s two maps in front of them.
///
/// [ADR 0017]: /docs/adr/0017-hot-reload-without-restart.md
#[derive(Debug)]
pub(crate) struct Compiler {
    /// Written path to what the last check of it observed. `RefCell` because
    /// the seam borrows a resolver shared, and a cache that could not be
    /// written on a hit would not be one.
    paths: RefCell<HashMap<PathBuf, PathEntry>>,
    /// The unit table proper, addressed by content rather than by path, so that
    /// two paths holding the same source compile once and a reverted edit is a
    /// hit rather than a recompile.
    units: RefCell<HashMap<UnitKey, CompileState>>,
    /// The environment half of every key here — [ADR 0078] § 4's digest, taken
    /// once from the configuration this process booted, because it is constant
    /// for the life of a snapshot.
    ///
    /// [ADR 0078]: /docs/adr/0078-config-reload-and-control-socket.md
    env: EnvHash,
    /// `[opcache] validate` and `revalidate_freq`, read once for the same
    /// reason: both are `System`-class, so no request can move them.
    revalidation: Revalidation,
    /// [ADR 0042]'s on-disk cache, resolved from the same block and once for
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
    /// [ADR 0042]: /docs/adr/0042-on-disk-artifact-cache-format.md
    cache: Option<crate::cache::Cache>,
    /// How many times [`Self::compile`] has run on this cache — the counter
    /// `docs/plan/m7.md`'s acceptance paragraph asks the "compiles it exactly
    /// once" claim to be asserted against, and the only number a caller could
    /// state it as: every other observable — the table's length, what a
    /// resolve hands back — is equal for a unit compiled once and one
    /// compiled a thousand times.
    ///
    /// **What it spends:** one word per compiler, which is one per core, and
    /// an increment on the one step that already costs a front end and a
    /// backend. Deliberately not `#[cfg(test)]`: a field that exists in one
    /// profile makes the release build a different struct, and this is the
    /// number an `nvs info` would report if it ever reported one.
    compiles: Cell<u64>,
}

impl Default for Compiler {
    /// The compiler of a host with **no configuration file anywhere**, which is
    /// [`nvs_config::Snapshot::default`]'s own state: the default revalidation
    /// policy, and the environment digest of a host with no `[[extension]]`.
    ///
    /// This is what a caller with no snapshot in hand holds, and it is a
    /// correct answer rather than a placeholder: the digest separates
    /// environments, and a run that read no configuration has exactly this one.
    /// Every subcommand that installs a resolver now holds one — `nvs test`
    /// resolves the tree above the suite's compile, because ADR 0042's artifact
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
            paths: RefCell::new(HashMap::new()),
            units: RefCell::new(HashMap::new()),
            env: env_hash(config),
            revalidation: Revalidation::from_config(config),
            cache: crate::cache::from_config(config),
            compiles: Cell::new(0),
        }
    }

    /// The program over `path`'s unit **and** that unit's route table, which is
    /// what a server needs and what [`Resolver::resolve`]'s own signature has
    /// nowhere to put.
    ///
    /// [ADR 0017] § *Decision*'s five steps, in order, with the module doc's
    /// note about what a single core collapses.
    ///
    /// # Errors
    ///
    /// The one-line summary `resolve` reports, for the same two failures: a
    /// program the front end refused, and one the backend could not compile.
    ///
    /// [ADR 0017]: /docs/adr/0017-hot-reload-without-restart.md
    pub(crate) fn compiled(
        &self,
        path: &str,
    ) -> Result<(Program, Arc<nvs_runtime::routes::Routes>), String> {
        let written = PathBuf::from(path);
        let known = self.paths.borrow().get(&written).copied();

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

        // Step 2's second half and step 3's single-flight in one lookup: an
        // observation that did not move addresses the entry the last one wrote,
        // and one that did may still name content this process compiled before
        // — a reverted edit, or a broken one being re-observed.
        if let Some(answer) = self.answer(&written, observed.content_hash) {
            if answer.is_ok() {
                self.advance(&written, &observed);
            }
            return answer;
        }

        // 3. The compile itself, which is the only step that costs anything.
        let key = UnitKey::new(&written, observed.content_hash, self.env);
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
        if ready {
            self.advance(&written, &observed);
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
        match self
            .units
            .borrow()
            .get(&UnitKey::new(path, content, self.env))?
        {
            CompileState::Ready(compiled) => Some(Ok((
                program_over(Rc::clone(compiled)),
                Arc::clone(&compiled.routes),
            ))),
            CompileState::Failed(message) => Some(Err(message.clone())),
        }
    }

    /// Step 4's pointer write: what this path resolves to now, and the moment
    /// the cap is measured from.
    fn advance(&self, path: &Path, observed: &Observed) {
        self.paths.borrow_mut().insert(
            path.to_path_buf(),
            PathEntry {
                content_hash: observed.content_hash,
                stamp: observed.stamp,
                last_checked: Instant::now(),
            },
        );
    }

    /// `state` under `key`, and the two entries this path is then allowed to
    /// keep: the content just reached, and `keep` where a failure leaves an
    /// older unit still in force.
    ///
    /// The sweep is what keeps the table O(paths): every earlier generation of
    /// this path goes, and a unit a running [`Program`] still holds stays
    /// mapped through that program's own `Rc` rather than through this map.
    fn record(&self, key: UnitKey, state: CompileState, keep: Option<Digest>) {
        let mut units = self.units.borrow_mut();
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
    /// warm hit as a compile. [ADR 0042] § 2 is explicit that a hit skips
    /// codegen and not the front end, so the front end really did run; what a
    /// hit saves is the Cranelift walk, which this counter never claimed to
    /// measure.
    ///
    /// [ADR 0042]: /docs/adr/0042-on-disk-artifact-cache-format.md
    fn compile(&self, path: &str, written: &Path) -> Result<Rc<Compiled>, String> {
        // Counted here rather than at the call site, and before the front end
        // rather than after it: a compile that *failed* is still a compile
        // this cache paid for, and the claim being counted is about how many
        // times the file was put through the front end at all.
        self.compiles.set(self.compiles.get() + 1);
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
        Ok(Rc::new(Compiled {
            unit: Rc::new(unit),
            routes: Arc::new(crate::runtime_routes(checked.exprs.routes())),
        }))
    }
}

/// Step 2: what the file behind `path` holds now.
///
/// The stamp is read first and the source only where it cannot answer, which is
/// the whole of the `mtime` policy — under `hash` the source is read every time
/// a check happens at all, which is what a file rewritten twice inside one
/// timestamp tick needs.
///
/// **The source is read once more than it was before this cache revalidated**:
/// this read hashes it, and the front end opens it again through its own
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
    /// the parent program wants a string it can print. ADR 0006's
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
/// The `Rc` is what keeps the code mapped for as long as the program can run —
/// a `Unit` owns its pages (`nvs_codegen::Unit`) — and it is a clone of the
/// cache's, so a second isolate over the same path shares them rather than
/// compiling again.
fn program_over(compiled: Rc<Compiled>) -> Program {
    Box::new(move |ctx: &mut Ctx, args: Value| -> Value {
        // The child unit's statics and its error class, which
        // `nvs_runtime::script::Program` requires before any of its code runs
        // and `Ctx::isolate` deliberately left empty.
        compiled.unit.install_in(ctx);
        // And ADR 0102 § 1's table, on the same terms and for the same reason
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
        // An error status leaves the throw on the context, which is where
        // `nvs_host::Isolate`'s own `finish` reads it from; there is nothing
        // to carry back by hand.
        entry.call(ctx).unwrap_or_else(|_| Value::null())
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
    let mut snapshot = nvs_config::Snapshot::default();
    snapshot.config.capabilities = Some(nvs_config::tree::Capabilities {
        script: Some(nvs_config::tree::CapScript {
            spawn: Some(nvs_config::tree::Setting::Bool(true)),
        }),
        // And ADR 0058's outbound pair, for the fixture that reaches ADR 0079
        // § 18's ephemeral listener over the wire. Both halves are needed and
        // that is the rule rather than an inconvenience: `connect` names the
        // host, and `internal` is the operator's written exception for § 3's
        // denied loopback range — a `#[Test(server: true)]` in a real program
        // grants exactly this pair to reach its own listener, which is why the
        // helper grants it rather than the runner carving a hole for itself.
        net: Some(nvs_config::tree::CapNet {
            connect: Some(nvs_config::tree::Setting::List(vec!["127.0.0.1".into()])),
            internal: Some(nvs_config::tree::Setting::List(vec!["127.0.0.1".into()])),
        }),
        ..nvs_config::tree::Capabilities::default()
    });
    let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Buffer(Vec::new()));
    ctx.set_config(std::sync::Arc::new(snapshot));
    ctx
}

#[cfg(test)]
mod tests {
    use super::{Compiler, granting_ctx as granting};
    use nvs_runtime::script::{Program, ResolveError, Resolver, resolve, scoped};
    use nvs_runtime::{Ctx, OutputSink, Value};

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
    /// installs for exactly the same reason. The capability is ADR 0118 § 1's,
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
        // ADR 0006's "shares immutable compiled code", which is this cache and
        // nothing else — the module doc says so, and this is what says it is
        // true.
        let compiler = Compiler::default();
        let path = from_root("examples/isolate/capture.nvs");
        let _first = compiler.resolve(&path).expect("the child compiles");
        let _second = compiler.resolve(&path).expect("and again, from the cache");
        assert_eq!(compiler.units.borrow().len(), 1);
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
        // it, and is still answered out of the unit table: the single-flight
        // is the module doc's "property of the borrow", and this is the number
        // that says so.
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
            compiler.compiles.get(),
            1,
            "the file was put through the front end more than once"
        );
        // And the table did not grow an entry per request either, which is the
        // same claim stated as what the cache holds afterwards.
        assert_eq!(compiler.units.borrow().len(), 1);
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
        // another. So `resumes == REQUESTS` is "nobody waited", asserted
        // rather than argued from the absence of a `Compiling` state.
        //
        // This is the ADR's single-flight seen from the other side. ADR 0017
        // gives racing callers a broadcast to wait on because its cache is
        // reached from many cores; the module doc's § *what one core
        // collapses* says why there is nothing to wait on here, and a resume
        // count is what turns that paragraph into a test.
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
        assert_eq!(compiler.compiles.get(), 1);
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
        // ADR 0083 § 7's second bullet, first half — asserted at the cache the
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
        // assertion is ADR 0017's own accounting — the pointer moved rather
        // than the table growing an entry per edit — and it holds while the
        // program resolved before the edit is still alive, because that one's
        // pages are kept by its own `Rc` (`script`'s module doc).
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
        assert_eq!(compiler.units.borrow().len(), 1);
        drop(before);
    }

    #[test]
    fn a_swap_never_blocks_a_request_serving_core() {
        // ADR 0017 § *Decision*'s paragraph after the five steps, in the
        // spelling one core has for it. The ADR keeps a *thread* free by
        // running step 3 on the compile pool; what keeps this core free is the
        // property [`PathEntry`]'s own doc states — neither table is borrowed
        // across the stat or the compile — and the caller that proves it is a
        // program already in flight, because its `spawn script` re-enters this
        // resolver from inside the very run a held borrow would have to span.
        let child = a_file_saying("in-flight", "one");
        let compiler = revalidating();
        let (holding, _) = compiler
            .compiled(&child.to_string_lossy())
            .expect("the child compiles");
        let before = compiler.paths.borrow()[&child].content_hash;

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
            compiler.paths.borrow()[&child].content_hash,
            before,
            "the swap did not publish"
        );
        assert_eq!(said(holding), "one\n");
    }

    #[test]
    fn revalidation_is_lazy_and_rate_capped() {
        // ADR 0017 § *Decision* step 1, both halves. A `stat` is counted the
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
}
