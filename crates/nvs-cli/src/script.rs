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
//! [ADR 0104](../../../docs/adr/0104-an-application-is-an-entry-file-path.md)
//! makes an application an entry-file path and says nothing about a child, so
//! there was a choice to make, and the working directory is the one a reader of
//! the program can already predict: it is what every other path a CLI program
//! writes is relative to — `require`'s spelling excepted, which ADR 0021
//! resolves against the requiring file because a library has to move as a unit.
//! A `spawn script` target is not a library; it is a second program, named the
//! way the shell that started this one would name it. `examples/isolate.nvs`
//! is written against exactly this and says so in its own comment.
//!
//! # Decision: one unit per written path, kept for the process
//!
//! ADR 0006's "an isolate shares immutable compiled code" is a property of this
//! cache and of nothing else — the seam hands over a closure and has no opinion
//! about what is behind it. So a path is compiled once and every later isolate
//! over it runs the same pages, which is what makes spawning a child cheap
//! enough to be worth doing.
//!
//! It is keyed by the path **as written**, so two spellings of one file compile
//! twice. Canonicalizing would buy the sharing back at the cost of a syscall on
//! every spawn and of a failure mode before the front end has run — and what a
//! program controls is its own text, which is the thing this key already is.
//!
//! **What it spends:** one compiled unit per distinct `spawn script` path in
//! the program, held for as long as the run is. O(the program's text), never
//! O(isolates spawned), per
//! [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md) — and freed with
//! the resolver, which is a local of `nvs run` published through
//! [`nvs_runtime::script::scoped`] rather than leaked.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

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

/// The one implementor: the front end and the backend `nvs run` already
/// carries, plus the cache in front of them.
#[derive(Debug, Default)]
pub(crate) struct Compiler {
    /// Written path to what was compiled from it. `RefCell` because the seam
    /// borrows a resolver shared, and a cache that could not be written on a
    /// hit would not be one.
    cache: RefCell<HashMap<PathBuf, Rc<Compiled>>>,
}

impl Compiler {
    /// The program over `path`'s unit **and** that unit's route table, which is
    /// what a server needs and what [`Resolver::resolve`]'s own signature has
    /// nowhere to put.
    ///
    /// Compiles on the first ask and hits the cache afterwards, exactly as
    /// `resolve` does — it *is* what `resolve` does, with the second half kept
    /// rather than dropped.
    ///
    /// # Errors
    ///
    /// The one-line summary `resolve` reports, for the same two failures: a
    /// program the front end refused, and one the backend could not compile.
    pub(crate) fn compiled(
        &self,
        path: &str,
    ) -> Result<(Program, Arc<nvs_runtime::routes::Routes>), String> {
        let key = PathBuf::from(path);
        if let Some(compiled) = self.cache.borrow().get(&key) {
            return Ok((
                program_over(Rc::clone(compiled)),
                Arc::clone(&compiled.routes),
            ));
        }

        let checked = crate::front_end(&key)
            .map_err(|_| format!("`{path}` could not be compiled; see the errors above"))?;
        let lowered = nvs_ir::lower::lower_program(
            crate::SCRIPT,
            &checked.program_files(),
            &checked.exprs,
            &checked.interner,
            &checked.enums,
            &checked.layouts,
        );
        let compiled = Rc::new(Compiled {
            unit: Rc::new(
                nvs_codegen::compile(&lowered).map_err(|error| format!("`{path}`: {error}"))?,
            ),
            routes: Arc::new(crate::runtime_routes(checked.exprs.routes())),
        });
        self.cache.borrow_mut().insert(key, Rc::clone(&compiled));
        let routes = Arc::clone(&compiled.routes);
        Ok((program_over(compiled), routes))
    }
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
    use nvs_runtime::script::{ResolveError, Resolver, resolve, scoped};
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
        let program = compiler.resolve(path).expect("the child compiles");
        let mut parent = Ctx::new(OutputSink::Buffer(Vec::new()));
        nvs_host::Isolate::new(program, Value::null(), nvs_host::Output::Capture)
            .run(&mut parent)
            .expect("a null argument crosses")
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
        assert_eq!(compiler.cache.borrow().len(), 1);
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
