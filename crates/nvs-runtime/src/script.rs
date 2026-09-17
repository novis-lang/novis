//! The seam a `spawn script` path becomes runnable code through — one
//! thread-local, one trait, and one operation nothing below the compiler can
//! perform for itself.
//!
//! `rule:security/isolate-shares-nothing`'s isolate
//! runs *another file*, and `nvs_host::Isolate` is that boundary in code. What
//! it deliberately cannot do is turn the written path into something to call:
//! resolving a path means the front end, `nvs-ir`, `nvs-codegen` and the unit
//! cache in front of them, and an edge from the host to any of those would link
//! a scheduler, a reactor and a JIT into every `nvs check`. That argument is
//! [`crate::host`]'s, made once for the scheduler and applying here unchanged.
//!
//! So this module is the same inversion on the other axis. The trait is
//! declared in the crate both sides already depend on, whoever *can* compile
//! implements it — `nvs-cli`, and the server at M7 — and the lowered
//! `spawn script` reaches it without naming either.
//!
//! # Why this is a second seam rather than another `Host` method
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
//! keep alive and its own to share: `rule:security/isolate-shares-nothing`'s "an isolate shares immutable
//! compiled code" is a property of *that* cache, not of this seam. What the
//! seam fixes is only the shape both ends agree on, and the argument's
//! ownership, which [`Program`]'s own doc states in full.
//!
//! # Why `rule:security/isolate-shares-nothing`'s *other* entry form asks no resolver
//!
//! [ADR 0006](/docs/decisions/0006.md) § *Decision* gives
//! `spawn script` two operands: a path, and a `Class::method(...)` reference.
//! Only the first one is a question for a resolver, and the difference is not a
//! convenience — it is the whole of what a resolver is for. A path is a name
//! nothing in this process has compiled yet, so answering it means the front
//! end, `nvs-ir`, `nvs-codegen` and a unit cache. **A method is code the
//! parent's own unit already contains**: `nvs-types` resolved it at compile
//! time, and `nvs-codegen` emitted it under the label `"{class}::{method}"`,
//! reachable through the class descriptor's method table by
//! [`crate::call_static`]. There is nothing left to compile, so there is
//! nothing to ask.
//!
//! That leaves one real question — *which* unit — and the answer is that the
//! child never has to name one. Two things reach it from the parent's context
//! and they are the two halves of "an isolate shares nothing but compiled
//! code":
//!
//! - **The class table**, which [`Ctx::isolate`] already clones as the
//!   [`crate::ErrorClass`] handle. That is what makes `call_static` find both
//!   the class and the method's address on the child's own context.
//! - **The unit's static-property recipes**, which [`Ctx::install_statics`]
//!   keeps when it arms a context, so a child can be armed against the same
//!   slot numbering with no `Unit` in hand. [`Ctx::method_isolate`] is that one
//!   call, and its doc owns why the slots are re-materialized rather than
//!   aliased.
//!
//! So the method form's program is built right here by [`method_program`],
//! out of the class-and-method label and the parameter names the spawn site
//! wrote, and it asks nothing of a `Resolver`. Both forms are prepared in this
//! module because both are prepared by whichever core is about to *run* the
//! child — `rule:concurrency/on-worker-runs-the-child-on-another-core` is why
//! that is the core rather than the parent's.
//!
//! A `Resolver` keyed by path stays keyed by path, and gains no notion of a
//! *running* unit — which it could not hold anyway, being per program where a
//! unit is per spawn. The alternative considered was widening this trait with a
//! second operation answering "the unit this thread is running"; it was refused
//! because it makes every implementor answer for a fact it does not have, and
//! because the fact already travels on the context that is going to run the
//! code.
//!
//! **The capability question is asked, and it is asked unscoped.**
//! [`resolve`] below is `rule:security/capability-check-at-the-door`'s door for a path, and `script.spawn`'s
//! grant names filesystem roots ([ADR 0006](/docs/decisions/0006.md)
//! § *Executing code is its own capability*). A method has no path to
//! canonicalise and prefix-check, but the grant still decides whether this
//! program spawns isolates at all, so the method form asks `Cap::ScriptSpawn`
//! with `Scope::Unscoped`: `script.spawn = false` refuses both forms, and a
//! list of roots grants the method form the way it grants any unscoped ask. No
//! new code becomes executable either way — the class is in the unit already
//! running.
//!
//! # Reaching a core that starts later
//!
//! [`install`] and [`scoped`] both publish onto *this* thread, and a thread
//! `nvs-host` starts for a worker placement was handed neither: a
//! `&'static dyn Resolver` is not `Send`, and `scoped`'s borrow lives on the
//! installing core's own stack. So a path entry placed `on: "worker"` would ask
//! a core that answers [`ResolveError::NoResolver`], while
//! `rule:concurrency/on-worker-runs-the-child-on-another-core` asks the core
//! that *starts* a child to be the one that prepares it.
//!
//! [`SharedResolver`] is the third form, and it is a **handle rather than a
//! second cache**. The implementor behind it is `Send + Sync` and shared by
//! `Arc`, which is what `nvs-cli`'s compiler already is: its path map and its
//! unit map sit behind `RwLock`s so that a source compiles once for the process
//! rather than once per core. A core publishes one with [`publish`], and a core
//! starting later takes its own clone through [`published`] and installs it for
//! the length of its own run with [`SharedResolver::scoped`]. What is per
//! thread stays the seam; what is per process is the table, exactly as before.
//!
//! What crosses is therefore the handle and never a [`Program`], which is a
//! boxed closure this crate never asked to be `Send` and is built on the core
//! about to run it. [`resolve`]'s answer and [`ResolveError`] are unchanged by
//! any of this: a started core holding the published handle answers a path the
//! way the booting core does, and one holding nothing answers
//! [`ResolveError::NoResolver`] the way it always has.
//!
//! # What it spends
//!
//! One machine word pair per thread — a null-checked wide pointer in a
//! thread-local, `const`-initialized and holding no `Drop` type, which is what
//! this crate's `alloc` module requires of every one in it. It is
//! O(threads) and does not grow with isolates spawned, per
//! `rule:programs/memory-priority`.
//!
//! Plus **one handle for the whole process**: a `Mutex<Option<SharedResolver>>`
//! in a `static`, `const`-initialized, holding one `Arc` clone of a resolver the
//! process built anyway. A `static` is not a thread-local and so is never
//! dropped at all, which is the stricter end of the rule the paragraph above
//! states. O(1), whatever the core count and whatever a program places.

use std::cell::Cell;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use nvs_config::capability::{Cap, Scope};

use crate::ctx::Ctx;
use crate::value::Value;
use crate::{Fault, NvsArray, call_static_bound};

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
/// Separate variants because they mean different things to the isolate that
/// asked. Neither is a [`crate::Thrown`]: `rule:security/isolate-shares-nothing`'s failure-is-a-value rule
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
    /// The configuration does not grant `script.spawn` for this path —
    /// `rule:security/denial-is-a-runtime-error`
    /// 's message, written by `crate::capability` and carried out through
    /// here rather than re-worded, so every denial reads the same whichever
    /// door produced it.
    ///
    /// A variant rather than a `Fault` in the error type: this enum is plain
    /// data that `nvs-cli`'s tests match on, and the caller that turns one into
    /// an exception is the one that already decides which of the other
    /// variants throws and which is fatal.
    Denied(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoResolver => f.write_str("no script resolver is installed on this thread"),
            Self::Refused(message) | Self::Denied(message) => f.write_str(message),
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
    /// `rule:config/an-application-is-its-entry-file-path`
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
/// unit struct in a `static` — a resolver cannot, because it is built out of the
/// configuration this process booted under and owns the unit cache it goes on
/// filling, so it is a value with a lifetime rather than a constant.
/// Leaking one per process is well inside
/// `rule:programs/memory-priority`'s bound, but it
/// is a *definite* loss to a leak checker, and `tools/loop.py`'s valgrind sweep
/// is worth more than those bytes: a sweep with one known-red fixture is a
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

/// A resolver a core that has not started yet can be handed.
///
/// [`install`] and [`scoped`] both publish onto the calling thread, so neither
/// reaches a thread somebody else starts afterwards. This is the form that
/// does: a `Send + Sync` handle, cheap to clone, which the starting core
/// installs on its own thread for as long as it runs. The module doc's
/// *Reaching a core that starts later* owns why this is a handle to the one
/// table rather than a second one.
///
/// It is deliberately not a [`Resolver`] itself. That trait's subject is a path
/// and its answer is a [`Program`] built for the core about to run it; this
/// type's subject is *reaching* an implementor from another thread. Folding the
/// two would oblige every resolver to be `Send + Sync`, including the ones a
/// `nvs check` and a single-core run install, which never cross anything.
#[derive(Clone, Debug)]
pub struct SharedResolver(Arc<dyn Resolver + Send + Sync>);

impl SharedResolver {
    /// Takes a handle to `resolver`.
    ///
    /// An `Arc` the caller already holds rather than a value to wrap, because
    /// the whole point is that the placing core goes on using the same one: a
    /// serving fleet's cores share a single compiler so that a source compiles
    /// once for the process.
    #[must_use]
    pub fn new<R: Resolver + Send + Sync + 'static>(resolver: Arc<R>) -> Self {
        Self(resolver)
    }

    /// Publishes this handle's resolver on **this** thread for the duration of
    /// `run`, and takes it back down however `run` ends.
    ///
    /// [`scoped`] with the handle's own `Arc` keeping the implementor alive, so
    /// a started core wraps its whole run in one call and every `spawn script`
    /// underneath reaches [`resolve`] as it would on the core that published.
    pub fn scoped<R>(&self, run: impl FnOnce() -> R) -> R {
        scoped(&*self.0, run)
    }
}

/// The process's published handle, or `None` when nothing has published one.
///
/// A `static` rather than a thread-local, which is the whole of why it exists:
/// it is read by a core that has not started yet and so has no thread-local of
/// the publishing core's to read. One handle per process, per
/// `rule:programs/memory-priority`'s *say what you spend*.
static PUBLISHED: Mutex<Option<SharedResolver>> = Mutex::new(None);

/// Withdraws the published handle and restores whatever was published before,
/// when dropped.
#[derive(Debug)]
#[must_use = "the handle is withdrawn the moment this guard is dropped"]
pub struct Published {
    previous: Option<SharedResolver>,
}

impl Drop for Published {
    fn drop(&mut self) {
        *published_slot() = self.previous.take();
    }
}

/// Publishes `resolver` for every core that starts while the guard lives.
///
/// Nesting is restoration, not replacement, exactly as [`install`] is — a
/// process driven from inside another one publishes over it and puts the outer
/// one back on the way out.
///
/// A core that has already started does not see this; it holds whatever it
/// installed on its own thread. What [`publish`] decides is what the *next*
/// core reads out of [`published`] as it starts, which is the one moment a core
/// has no resolver of its own and no stack to borrow one from.
pub fn publish(resolver: SharedResolver) -> Published {
    let previous = published_slot().replace(resolver);
    Published { previous }
}

/// The published handle, for a core that is starting and has none of its own.
///
/// A clone, so the caller owns its share of the implementor for as long as it
/// runs and no [`Published`] guard dropped meanwhile can pull the table out from
/// under a core mid-placement.
#[must_use]
pub fn published() -> Option<SharedResolver> {
    published_slot().clone()
}

/// The slot, with a poisoned lock read through rather than around.
///
/// Nothing here can leave a broken invariant behind a panic: the value is one
/// `Option` that is replaced whole, so the state a poisoned lock guards is
/// exactly as valid as an unpoisoned one's.
fn published_slot() -> MutexGuard<'static, Option<SharedResolver>> {
    PUBLISHED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Turns `path` into a [`Program`] through this thread's resolver, once
/// `script.spawn` has been shown to cover it.
///
/// One call rather than a `with_current` the caller then has to unwrap twice:
/// there is exactly one thing anybody does with a resolver, and every way of
/// not getting a program is a [`ResolveError`] variant.
///
/// **This is `rule:security/capability-check-at-the-door`
/// 's spawn door**, and it takes a `ctx` for no other reason. The check is
/// here rather than in the lowered helper that calls it because this function
/// *is* the effect: a `Program` is the thing a spawn was after, and there is no
/// second way to obtain one, so a caller that skipped the check would have
/// nothing to run. `Cap::ScriptSpawn` is asked with the path as its scope, so
/// § 4's canonicalise-then-prefix rule applies to a spawn target exactly as it
/// does to a read — being allowed to read a file has never been permission to
/// run it, which is why this is not implied by `fs.read`.
///
/// # Errors
///
/// [`ResolveError::Denied`] when the configuration does not grant
/// `script.spawn` for `path`, [`ResolveError::NoResolver`] when nothing is
/// installed here, and [`ResolveError::Refused`] carrying the implementor's own
/// message otherwise. The capability is asked **first**: a path outside the
/// grant is refused whether or not it names a file that compiles.
pub fn resolve(ctx: &Ctx, path: &str) -> Result<Program, ResolveError> {
    if let Some(message) = crate::capability::refusal(
        ctx,
        Cap::ScriptSpawn,
        Scope::Path(std::path::Path::new(path)),
        "`spawn script`",
    ) {
        return Err(ResolveError::Denied(message));
    }
    let Some(resolver) = CURRENT.with(Cell::get) else {
        return Err(ResolveError::NoResolver);
    };
    resolver.resolve(path).map_err(ResolveError::Refused)
}

/// The program a `Class::method` entry runs, built from what the spawn site
/// wrote and nothing else.
///
/// The counterpart to [`resolve`] for [ADR 0006](/docs/decisions/0006.md)'s
/// second form, and it asks no resolver for the reason the module doc gives:
/// the method is code the parent's unit already holds, reached through
/// [`crate::call_static_bound`] on the child's own context. What that context
/// carries — the class table and the static-property recipes — is
/// [`Ctx::method_isolate`]'s, arranged before this ever runs.
///
/// `label` is `"{class}::{method}"`, the label `nvs-codegen` emitted the method
/// under. `names` is the entry's parameter names in declaration order, which is
/// what [`bound_arguments`] reads the `args:` map by; the spawn site has
/// already judged that the map names exactly those parameters.
///
/// It is built **here** rather than at the spawn site so that the seam carries
/// the two names rather than a closure: a [`Program`] is the parent's to run
/// and means nothing on another core, and
/// `rule:concurrency/on-worker-runs-the-child-on-another-core` needs the core
/// that starts a child to be the one that prepares it.
#[must_use]
pub fn method_program(label: String, names: Vec<String>) -> Program {
    Box::new(move |child, argument| {
        // Ownership discharged into the isolate's own root, exactly as a path
        // entry's program does it — [`Program`]'s own doc owns why this and not
        // a release. It happens **before** the binding below, which is what
        // makes every value that binding reads live for the length of the call:
        // the root owns the map, and the map owns them.
        child.set_isolate_argument(argument);
        let mut bound = bound_arguments(&names, child.isolate_argument());
        match call_static_bound(child, &label, &mut bound) {
            Ok(Some(value)) => value,
            // The class table crossed with the context, so a miss here is the
            // child's unit disagreeing with what `nvs_types` resolved. A
            // failure value rather than a panic, because a child may not end
            // its parent.
            Ok(None) => {
                child.set_pending(format!(
                    "`spawn script {label}`: this program declares no such static method"
                ));
                Value::null()
            }
            // The judgement `call_static_bound` makes on this frame's behalf —
            // an argument whose tag the parameter does not admit,
            // `rule:security/isolate-shares-nothing`'s "typed at the boundary".
            // There is no frame above it inside the child, so it is recorded as
            // the isolate's pending throw and reaches the parent as ADR 0006
            // § *Failure is a value*'s `ok = false` rather than as a status
            // nothing wrote.
            Err(Fault::Thrown(class, message)) => {
                child.set_pending_as(class, message);
                Value::null()
            }
            // `call_static_bound`'s remaining `Err` is `Fault::Pending`, whose
            // status is the whole of what the frame said: a throw is already on
            // this context, where `nvs_host::Isolate`'s `finish` reads it from,
            // and an `EXITED` leaves nothing there. Both are recorded the same
            // way, because a `Program` answers a `Value` and no status —
            // [`Ctx::set_ending`].
            Err(Fault::Pending(status)) => {
                child.set_ending(status);
                Value::null()
            }
            Err(_) => Value::null(),
        }
    })
}

/// The `args:` map's entries in the entry's own parameter order — ADR 0006
/// § *Decision*'s binding, which is a positional list by the time a compiled
/// callee sees it.
///
/// Every value is **borrowed** out of the map, which the isolate's ownership
/// root holds for the length of the call; [`crate::call_static_bound`] retains
/// each argument on the way in exactly as every compiled call site does, so
/// nothing here owns anything.
///
/// A name with no entry cannot arrive — the spawn site refuses a map that does
/// not name the entry's parameters — and reads as `null`, which the parameter's
/// own tag then refuses rather than a slot nobody filled.
#[must_use]
pub fn bound_arguments(names: &[String], map: Value) -> Vec<Value> {
    let Some(ptr) = map.array_ptr() else {
        return Vec::new();
    };
    #[expect(
        unsafe_code,
        reason = "a Tag::Array value owns a reference to a live allocation, so \
                  it is live for the length of this call, and the handle is \
                  never dropped"
    )]
    // SAFETY: `array_ptr` answered, so the value is an array and its header is
    // live; `ManuallyDrop` rather than retaining keeps the borrow free of
    // refcount traffic, and nothing here drops the handle.
    let map = std::mem::ManuallyDrop::new(unsafe { NvsArray::from_raw(ptr) });
    names
        .iter()
        .map(|name| map.get(name.as_bytes()).unwrap_or_else(Value::null))
        .collect()
}

/// Whether this thread has a resolver, without calling it.
#[must_use]
pub fn is_installed() -> bool {
    CURRENT.with(|slot| slot.get().is_some())
}

#[cfg(test)]
mod tests {
    use super::{
        Installed, Program, ResolveError, Resolver, SharedResolver, install, is_installed, publish,
        published, resolve, scoped,
    };
    use crate::ctx::{Ctx, OutputSink};
    use crate::host::Entry;
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
    // to reach it directly; a real implementor is built out of a configuration
    // and owns a unit cache, so it is a value on a stack and goes through
    // `scoped` or a `SharedResolver` instead.
    static FIXED: Fixed = Fixed;
    static REFUSING: Refusing = Refusing;

    fn install_fixed() -> Installed {
        install(&FIXED)
    }

    /// A context granting `script.spawn` for everything — an operator's
    /// `[capabilities.script] spawn = true`.
    ///
    /// Every test below except [`a_spawn_target_is_refused_before_a_resolver_is_asked`]
    /// is about the *resolver seam* and not about `rule:security/capability-check-at-the-door`'s check, so each
    /// one clears the door first; a bare `Ctx` grants nothing and would make
    /// them all assert the denial instead of the routing they are for.
    fn granting() -> Ctx {
        let mut snapshot = nvs_config::Snapshot::default();
        snapshot.config.capabilities = Some(nvs_config::tree::Capabilities {
            script: Some(nvs_config::tree::CapScript {
                spawn: Some(nvs_config::tree::Setting::Bool(true)),
            }),
            ..nvs_config::tree::Capabilities::default()
        });
        let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        ctx.set_config(std::sync::Arc::new(snapshot));
        ctx
    }

    #[test]
    fn a_spawn_target_is_refused_before_a_resolver_is_asked() {
        // `rule:security/capability-check-at-the-door`: the check is inside the door, so a context granting
        // nothing is refused with the resolver that *would* have answered
        // installed and untouched — the denial names the capability rather
        // than the path failing to compile.
        let installed = install_fixed();
        let ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
        let Err(ResolveError::Denied(message)) = resolve(&ctx, "child.nvs") else {
            panic!("an unconfigured context grants no `script.spawn`");
        };
        assert!(
            message.contains("script.spawn") && message.contains("child.nvs"),
            "§ 5's message names the capability and the target: {message}"
        );
        // And the grant is what turns it into the resolver's answer, on the
        // same path and the same resolver: nothing but the configuration moved.
        assert!(resolve(&granting(), "child.nvs").is_ok());
        drop(installed);
    }

    #[test]
    fn with_nothing_installed_a_path_is_refused_without_a_resolver_being_invented() {
        assert!(!is_installed());
        assert_eq!(
            resolve(&granting(), "child.nvs").err(),
            Some(ResolveError::NoResolver)
        );
    }

    #[test]
    fn an_installed_resolver_answers_and_its_program_runs_on_a_context() {
        let installed = install_fixed();
        assert!(is_installed());
        let mut ctx = granting();
        let program = resolve(&ctx, "abc.nvs").expect("the fixed resolver answers");
        let answer = program(&mut ctx, Value::null());
        assert_eq!(answer.as_int(), Some(7));
        drop(installed);
        assert!(!is_installed());
    }

    /// The length of the path the two cross-thread cases resolve, which is what
    /// [`Fixed`] answers with and therefore the proof that the *far* thread's
    /// resolver is the one that ran.
    const RESOLVED: Option<i64> = Some(8);

    #[test]
    fn a_shared_handle_crosses_to_a_thread_and_answers_there() {
        // The property the third form exists for: the handle is `Send`, so a
        // core `nvs-host` starts can be handed one, and what it installs on its
        // own thread answers `resolve` exactly as an `install` here would.
        // Nothing of the `Program` crosses — it is built on the far side.
        let mine = SharedResolver::new(std::sync::Arc::new(Fixed));
        let answered = std::thread::spawn(move || {
            assert!(
                !is_installed(),
                "a started thread installs nothing by itself"
            );
            mine.scoped(|| {
                let mut ctx = granting();
                let program = resolve(&ctx, "abcd.nvs").expect("the shared resolver answers");
                program(&mut ctx, Value::null()).as_int()
            })
        })
        .join()
        .expect("the started thread did not panic");
        assert_eq!(answered, RESOLVED);
    }

    #[test]
    fn a_published_resolver_reaches_a_thread_started_after_it_and_is_withdrawn_with_its_guard() {
        assert!(
            published().is_none(),
            "nothing has published in this process"
        );
        let guard = publish(SharedResolver::new(std::sync::Arc::new(Fixed)));
        // The core is started *after* the publish and reads the slot for
        // itself, which is the one moment it has neither a resolver of its own
        // nor a stack of the publisher's to borrow one from.
        let answered = std::thread::spawn(|| {
            let handle = published().expect("the process published a handle");
            handle.scoped(|| {
                let mut ctx = granting();
                let program = resolve(&ctx, "abcd.nvs").expect("the published resolver answers");
                program(&mut ctx, Value::null()).as_int()
            })
        })
        .join()
        .expect("the started thread did not panic");
        assert_eq!(answered, RESOLVED);
        drop(guard);
        assert!(
            published().is_none(),
            "the guard restores what was published before it, as `install`'s does"
        );
    }

    #[test]
    fn a_named_entry_is_send_and_becomes_a_program_where_the_child_will_run() {
        // The property the seam exists for: what a spawn names crosses to
        // another core, and what it becomes does not. `Entry` is two strings
        // and a list of them, so this compiles; the day one of its arms carries
        // a `Program` again it stops compiling, which is the point.
        const fn crosses_a_thread<T: Send>() {}
        crosses_a_thread::<Entry>();

        let installed = install_fixed();
        let mut ctx = granting();
        let entry = Entry::Path("abc.nvs".to_owned());
        assert!(!entry.is_method());
        let program = entry
            .program(&ctx)
            .expect("the path form asks this thread's resolver");
        assert_eq!(program(&mut ctx, Value::null()).as_int(), Some(7));
        drop(installed);
    }

    #[test]
    fn a_method_entry_becomes_a_program_with_no_resolver_installed() {
        // `rule:security/isolate-shares-nothing`'s second form is code the
        // parent's unit already holds, so the core that starts such a child
        // needs no compiler on it — which is why the entry carries the label
        // and the parameter names rather than a closure over either.
        assert!(!is_installed());
        let entry = Entry::Method {
            label: "Chat::run".to_owned(),
            names: vec!["room".to_owned()],
        };
        assert!(entry.is_method());
        // The program is not *run* here: calling it would want a class table
        // and a unit, which is `nvs_host::Isolate`'s to arrange and not this
        // seam's. What is asserted is that nothing refused it.
        assert!(
            entry
                .program(&Ctx::new(OutputSink::Buffer(Vec::new())))
                .is_ok()
        );
    }

    #[test]
    fn installing_nests_and_restores_rather_than_replacing() {
        let ctx = granting();
        let outer = install_fixed();
        {
            let inner = install(&REFUSING);
            assert!(matches!(
                resolve(&ctx, "child.nvs"),
                Err(ResolveError::Refused(_))
            ));
            drop(inner);
        }
        // The outer one is back, which is what a scheduler driven from inside
        // another embedder's task depends on.
        assert!(resolve(&ctx, "child.nvs").is_ok());
        drop(outer);
    }

    #[test]
    fn a_stack_resolver_is_installed_only_for_the_length_of_the_scoped_call() {
        // The whole point of `scoped`: this resolver is a local, and `install`
        // on its own would oblige a `Box::leak` to widen it to `&'static`.
        let stack = Fixed;
        let ctx = granting();
        assert!(!is_installed());
        let answered = scoped(&stack, || {
            assert!(is_installed());
            resolve(&ctx, "abc.nvs").is_ok()
        });
        assert!(answered);
        assert!(!is_installed());
    }

    #[test]
    fn a_scoped_resolver_puts_back_the_one_it_covered() {
        let ctx = granting();
        let outer = install(&REFUSING);
        let stack = Fixed;
        scoped(&stack, || assert!(resolve(&ctx, "child.nvs").is_ok()));
        assert!(matches!(
            resolve(&ctx, "child.nvs"),
            Err(ResolveError::Refused(_))
        ));
        drop(outer);
    }

    #[test]
    fn a_refusal_carries_the_implementors_own_message() {
        let installed = install(&REFUSING);
        let Err(error) = resolve(&granting(), "notes.txt") else {
            panic!("the refusing resolver refuses");
        };
        assert_eq!(error.to_string(), "`notes.txt` is not a script");
        drop(installed);
    }
}
