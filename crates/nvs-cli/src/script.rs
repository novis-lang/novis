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

use std::cell::RefCell;
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
}

impl Default for Compiler {
    /// The compiler of a host with **no configuration file anywhere**, which is
    /// [`nvs_config::Snapshot::default`]'s own state: the default revalidation
    /// policy, and the environment digest of a host with no `[[extension]]`.
    ///
    /// This is what a caller with no snapshot in hand holds — `nvs test` builds
    /// its context before any tree is resolved — and it is a correct answer
    /// there rather than a placeholder: the digest separates environments, and
    /// a run that read no configuration has exactly this one.
    fn default() -> Self {
        Self::new(&Config::default())
    }
}

impl Compiler {
    /// The compiler for a process running under `config`: its environment
    /// digest, and the `[opcache]` block's answer to when a resolve looks at a
    /// file it has already compiled.
    pub(crate) fn new(config: &Config) -> Self {
        Self {
            paths: RefCell::new(HashMap::new()),
            units: RefCell::new(HashMap::new()),
            env: env_hash(config),
            revalidation: Revalidation::from_config(config),
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

    /// The front end and the backend, over one path, with nothing cached: the
    /// whole of what step 3 costs.
    fn compile(&self, path: &str, written: &Path) -> Result<Rc<Compiled>, String> {
        let checked = crate::front_end(written)
            .map_err(|_| format!("`{path}` could not be compiled; see the errors above"))?;
        let lowered = nvs_ir::lower::lower_program(
            crate::SCRIPT,
            &checked.program_files(),
            &checked.exprs,
            &checked.interner,
            &checked.enums,
            &checked.layouts,
        );
        Ok(Rc::new(Compiled {
            unit: Rc::new(
                nvs_codegen::compile(&lowered).map_err(|error| format!("`{path}`: {error}"))?,
            ),
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

        let Some(entry) = compiled.unit.function(crate::SCRIPT) else {
            // Not reachable for a unit that compiled — every program has a
            // script frame — but it is a failure value rather than a panic,
            // because a child may not be able to end its parent.
            ctx.set_pending("the child's script frame was not compiled");
            return Value::null();
        };
        // An error status leaves the throw on the context, which is where
        // `nvs_host::Isolate`'s own `finish` reads it from; there is nothing
        // to carry back by hand.
        nvs_runtime::call(entry, ctx, &[]).unwrap_or_else(|_| Value::null())
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
        let mut parent = Ctx::new(OutputSink::Buffer(Vec::new()));
        nvs_host::Isolate::new(program, Value::null(), nvs_host::Output::Capture)
            .run(&mut parent)
            .expect("a null argument crosses")
    }

    /// A `.nvs` file this test owns, whose whole body echoes `said`. Called
    /// again with the same `name`, it is the edit.
    fn a_file_saying(name: &str, said: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("nvs-swap-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory to write the case in");
        let path = dir.join("entry.nvs");
        std::fs::write(&path, format!("<?nvs\necho \"{said}\", \"\\n\";\n"))
            .expect("the case is writable");
        path
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
        use nvs_config::tree::{Config, Opcache, Setting};
        Compiler::new(&Config {
            opcache: Some(Opcache {
                validate: Some(Setting::Text("hash".to_owned())),
                revalidate_freq: Some(Setting::Text("0s".to_owned())),
                ..Opcache::default()
            }),
            ..Config::default()
        })
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
