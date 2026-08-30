//! The seam a `spawn script` path becomes runnable code through — one
//! thread-local, one trait, and one operation nothing below the compiler can
//! perform for itself.
//!
//! [ADR 0006](../../../docs/adr/0006-isolated-script-execution.md)'s isolate
//! runs *another file*, and `nvs_host::Isolate` is that boundary in code. What
//! it deliberately cannot do is turn the written path into something to call:
//! resolving a path means the front end, `nvs-ir`, `nvs-codegen` and the unit
//! cache in front of them, and an edge from the host to any of those would link
//! a scheduler, a reactor and a JIT into every `nvs check`. That argument is
//! [`crate::host`]'s, made once for the scheduler and applying here unchanged.
//!
//! So this module is the same inversion on the other axis. The trait is
//! declared in the crate both sides already depend on, whoever *can* compile
//! implements it — `nvs-cli` today, the server at M7 — and the lowered
//! `spawn script` reaches it without naming either.
//!
//! # Why this is a second seam rather than a third `Host` method
//!
//! A host is **per core** and its subject is a task: run this group, give the
//! core back, wake me. A resolver is **per program** and its subject is code:
//! turn this path into something callable, once, and share the result. Neither
//! is reachable from the other — a `nvs check` has a resolver's whole toolchain
//! and no scheduler, a worker thread with nothing installed has a scheduler and
//! no compiler — so folding them into one trait would oblige every implementor
//! of either to answer for the other.
//!
//! # What crosses, and who owns it afterwards
//!
//! A [`Program`] is a boxed closure, so the unit behind it is the resolver's to
//! keep alive and its own to share: ADR 0006's "an isolate shares immutable
//! compiled code" is a property of *that* cache, not of this seam. What the
//! seam fixes is only the shape both ends agree on, and the argument's
//! ownership, which [`Program`]'s own doc states in full.
//!
//! # What it spends
//!
//! One machine word pair per thread — a null-checked wide pointer in a
//! thread-local, `const`-initialized and holding no `Drop` type, which is what
//! this crate's `alloc` module requires of every one in it. It is
//! O(threads) and does not grow with isolates spawned, per
//! [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md).

use std::cell::Cell;

use crate::ctx::Ctx;
use crate::value::Value;

/// An isolate's code, prepared by whoever could compile it.
///
/// Called once, on the isolate's own stack, with the isolate's own context and
/// the argument value that has already crossed. It owes that context what a
/// request's entry point owes one: arming the child unit's statics through
/// [`Ctx::install_statics`] before running any of its code — which is
/// `nvs_codegen::Unit::install_in`'s job rather than a builder's — and
/// answering with the value the script's top-level `return` produced, or
/// [`Value::null`] for a script that returned nothing.
///
/// The argument is **transferred**: the program is handed one reference and has
/// to put it somewhere the isolate's wholesale release will reach, which is
/// what [`Ctx::set_isolate_argument`] is for. A program that drops it on the
/// floor leaks it, and a program that releases it itself has to be sure nothing
/// else will read it — so there is one right answer and it is that method.
///
/// A boxed closure rather than a path, and rather than a compiled artifact type
/// this crate would have to name: the module docs own why.
pub type Program = Box<dyn FnOnce(&mut Ctx, Value) -> Value>;

/// Why a path did not become a [`Program`].
///
/// Two variants because the two mean different things to the isolate that
/// asked. Neither is a [`crate::Thrown`]: ADR 0006's failure-is-a-value rule
/// starts at the boundary, and this is one step before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// Nothing on this thread can compile anything. Not a fault of the path —
    /// it is what an embedder that installed no resolver gets, and it is a bug
    /// in that embedder rather than in the program.
    NoResolver,
    /// The resolver tried and could not: an unreadable path, or a file that
    /// does not compile. Already rendered for a person to read.
    Refused(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoResolver => f.write_str("no script resolver is installed on this thread"),
            Self::Refused(message) => f.write_str(message),
        }
    }
}

/// Whatever can turn a written path into runnable code on this thread.
///
/// Exactly one implementor is expected per embedder, and it holds the unit
/// cache: `resolve` is called once per `spawn script` *evaluation*, so a loop
/// spawning the same child a thousand times compiles it once or a thousand
/// times entirely according to what the implementor keeps.
///
/// `Debug` is a supertrait so that [`Installed`] can derive it.
pub trait Resolver: std::fmt::Debug {
    /// Prepares the script `path` names, or says why it could not.
    ///
    /// `path` is exactly what the program wrote. How a relative one is
    /// anchored is the implementor's — nothing about it is decidable here, and
    /// [ADR 0104](../../../docs/adr/0104-an-application-is-an-entry-file-path.md)
    /// settles the entry file rather than this.
    ///
    /// # Errors
    ///
    /// The rendered reason, which reaches the parent as an ordinary failure
    /// value rather than as a throw.
    fn resolve(&self, path: &str) -> Result<Program, String>;
}

thread_local! {
    /// The resolver installed on this thread, or `None` when there is none.
    ///
    /// A `Cell` of a `Copy` wide pointer so it is `const`-initialized and
    /// carries no destructor — the module docs' *What it spends*, and
    /// this crate's `alloc` module for why that is a rule here rather than a
    /// preference.
    static CURRENT: Cell<Option<&'static dyn Resolver>> = const { Cell::new(None) };
}

/// Restores whatever resolver was installed before, when dropped.
///
/// Held on the installer's own stack, which is why the `Drop` here does not
/// break the thread-local's no-destructor rule: the slot holds a pointer, this
/// guard holds the previous one.
#[derive(Debug)]
#[must_use = "the resolver is uninstalled the moment this guard is dropped"]
pub struct Installed {
    previous: Option<&'static dyn Resolver>,
}

impl Drop for Installed {
    fn drop(&mut self) {
        CURRENT.with(|slot| slot.set(self.previous.take()));
    }
}

/// Publishes `resolver` as this thread's resolver for as long as the guard
/// lives.
///
/// Nesting is restoration, not replacement, exactly as [`crate::host::install`]
/// is: an embedder driven from inside another one installs over it and puts the
/// outer one back on the way out.
pub fn install(resolver: &'static dyn Resolver) -> Installed {
    let previous = CURRENT.with(|slot| slot.replace(Some(resolver)));
    Installed { previous }
}

/// Publishes `resolver` for the duration of `run`, and takes it back down
/// however `run` ends.
///
/// This is the shape an embedder whose resolver is **not** a `static` wants, and
/// it exists because the only alternative is `Box::leak`. [`install`] takes a
/// `&'static` so that an [`Installed`]'s remembered previous can never dangle
/// whatever order a nest of guards is dropped in, and `nvs_host`'s host
/// satisfies that by being a
/// unit struct in a `static` — a resolver cannot, because it holds the unit
/// cache and a compiled unit is `Rc`-shared, so the whole type is `!Sync`.
/// Leaking one per process is 56 bytes and well inside
/// [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md)'s bound, but it
/// is a *definite* loss to a leak checker, and `tools/loop.py`'s valgrind sweep
/// is worth more than the 56 bytes: a sweep with one known-red fixture is a
/// sweep nobody reads.
///
/// `run`'s own panic unwinds straight through — the guard is a local here, so
/// the previous resolver is restored on the way past.
pub fn scoped<R>(resolver: &(dyn Resolver + 'static), run: impl FnOnce() -> R) -> R {
    #[expect(
        unsafe_code,
        reason = "the widened reference is published into `CURRENT` and nowhere \
                  else, and `installed`'s `Drop` clears it before this function \
                  returns on either edge -- so it is never readable after \
                  `resolver`'s own borrow ends. Nothing reachable from `run` can \
                  carry it back out either: `resolve` hands back a `'static` \
                  `Program` that borrows nothing from the resolver, and \
                  `is_installed` answers with a `bool`. A nested `install` inside \
                  `run` is sound however its guard is dropped or forgotten, \
                  because putting *this* resolver back is what `installed` does \
                  regardless"
    )]
    let widened: &'static dyn Resolver = unsafe { &*std::ptr::from_ref(resolver) };
    let installed = install(widened);
    let answer = run();
    drop(installed);
    answer
}

/// Turns `path` into a [`Program`] through this thread's resolver.
///
/// One call rather than a `with_current` the caller then has to unwrap twice:
/// there is exactly one thing anybody does with a resolver, and both ways of
/// not getting a program are [`ResolveError`]'s two variants.
///
/// # Errors
///
/// [`ResolveError::NoResolver`] when nothing is installed here, and
/// [`ResolveError::Refused`] carrying the implementor's own message otherwise.
pub fn resolve(path: &str) -> Result<Program, ResolveError> {
    let Some(resolver) = CURRENT.with(Cell::get) else {
        return Err(ResolveError::NoResolver);
    };
    resolver.resolve(path).map_err(ResolveError::Refused)
}

/// Whether this thread has a resolver, without calling it.
#[must_use]
pub fn is_installed() -> bool {
    CURRENT.with(|slot| slot.get().is_some())
}

#[cfg(test)]
mod tests {
    use super::{
        Installed, Program, ResolveError, Resolver, install, is_installed, resolve, scoped,
    };
    use crate::ctx::{Ctx, OutputSink};
    use crate::value::Value;

    /// A resolver that hands back a program answering with the length of the
    /// path it was asked for, which is all the *route* has to be proved
    /// against — there is no compiler on this side of the seam and there is not
    /// meant to be one.
    #[derive(Debug)]
    struct Fixed;

    /// A second one, so nesting has two answers to tell apart.
    #[derive(Debug)]
    struct Refusing;

    impl Resolver for Fixed {
        fn resolve(&self, path: &str) -> Result<Program, String> {
            let len = i64::try_from(path.len()).unwrap_or(-1);
            Ok(Box::new(move |ctx, args| {
                ctx.set_isolate_argument(args);
                Value::int(len)
            }))
        }
    }

    impl Resolver for Refusing {
        fn resolve(&self, path: &str) -> Result<Program, String> {
            Err(format!("`{path}` is not a script"))
        }
    }

    // `install` takes a `&'static dyn Resolver`, so a `static` is the only way
    // to reach it directly; a real implementor holds a unit cache, is `!Sync`
    // for that reason and goes through `scoped` instead.
    static FIXED: Fixed = Fixed;
    static REFUSING: Refusing = Refusing;

    fn install_fixed() -> Installed {
        install(&FIXED)
    }

    #[test]
    fn with_nothing_installed_a_path_is_refused_without_a_resolver_being_invented() {
        assert!(!is_installed());
        assert_eq!(resolve("child.nvs").err(), Some(ResolveError::NoResolver));
    }

    #[test]
    fn an_installed_resolver_answers_and_its_program_runs_on_a_context() {
        let installed = install_fixed();
        assert!(is_installed());
        let program = resolve("abc.nvs").expect("the fixed resolver answers");
        let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        let answer = program(&mut ctx, Value::null());
        assert_eq!(answer.as_int(), Some(7));
        drop(installed);
        assert!(!is_installed());
    }

    #[test]
    fn installing_nests_and_restores_rather_than_replacing() {
        let outer = install_fixed();
        {
            let inner = install(&REFUSING);
            assert!(matches!(
                resolve("child.nvs"),
                Err(ResolveError::Refused(_))
            ));
            drop(inner);
        }
        // The outer one is back, which is what a scheduler driven from inside
        // another embedder's task depends on.
        assert!(resolve("child.nvs").is_ok());
        drop(outer);
    }

    #[test]
    fn a_stack_resolver_is_installed_only_for_the_length_of_the_scoped_call() {
        // The whole point of `scoped`: this resolver is a local, and `install`
        // on its own would oblige a `Box::leak` to widen it to `&'static`.
        let stack = Fixed;
        assert!(!is_installed());
        let answered = scoped(&stack, || {
            assert!(is_installed());
            resolve("abc.nvs").is_ok()
        });
        assert!(answered);
        assert!(!is_installed());
    }

    #[test]
    fn a_scoped_resolver_puts_back_the_one_it_covered() {
        let outer = install(&REFUSING);
        let stack = Fixed;
        scoped(&stack, || assert!(resolve("child.nvs").is_ok()));
        assert!(matches!(
            resolve("child.nvs"),
            Err(ResolveError::Refused(_))
        ));
        drop(outer);
    }

    #[test]
    fn a_refusal_carries_the_implementors_own_message() {
        let installed = install(&REFUSING);
        let Err(error) = resolve("notes.txt") else {
            panic!("the refusing resolver refuses");
        };
        assert_eq!(error.to_string(), "`notes.txt` is not a script");
        drop(installed);
    }
}
