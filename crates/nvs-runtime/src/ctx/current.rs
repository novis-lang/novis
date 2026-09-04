//! The context whose compiled frames are running on this thread.
//!
//! A thread-local raw pointer, armed for the length of a run by [`CurrentCtx`]
//! and null between runs. It exists for the paths that cannot be handed a
//! context: the allocator's live list, which is reached from `Drop` and from
//! sweep code that has no argument to carry one in.
//!
//! [`with_current`] is the only way to reach it, and it answers `None` rather
//! than dereferencing a null — so a caller off the request path gets a miss
//! instead of a crash.

use super::*;

thread_local! {
    /// The context whose compiled frames are running on this thread, or null
    /// between runs — see [`CurrentCtx`].
    ///
    /// A raw pointer and a `Cell` so it is `const`-initialized and carries no
    /// destructor, which is what `crate::alloc`'s own thread-local requires of
    /// every one in this crate and costs nothing here.
    static CURRENT: std::cell::Cell<*mut Ctx> = const { std::cell::Cell::new(std::ptr::null_mut()) };

    /// The live list of that same context, or null — see [`CurrentCtx`].
    ///
    /// A second word rather than a hop through [`CURRENT`], because the two
    /// readers want different things: the release path wants the context, and
    /// [`crate::object::NvsObj::alloc`] wants a list it may write to while a
    /// helper above it holds `&mut Ctx`. Reaching the list *through* the
    /// context would make every object allocation write into an allocation
    /// that reference claims exclusively; the `Rc` it names does not.
    static CURRENT_LIVE: std::cell::Cell<*const crate::object::LiveList> =
        const { std::cell::Cell::new(std::ptr::null()) };
}

/// The live list of this thread's current context, or null when no compiled
/// frame is running — [`crate::object::NvsObj::alloc`]'s one reader.
pub(crate) fn current_live_list() -> *const crate::object::LiveList {
    CURRENT_LIVE.get()
}

/// Installs a context as this thread's current one for as long as the guard
/// lives, restoring whatever was there before when it drops.
///
/// # Decision: the release path reaches its context through here
///
/// [`crate::abi::call`] is the one door from Rust into compiled code, so this
/// is set there and nowhere else. Its one reader is
/// [`crate::object::dismantle`], which has to call a dying generator's unwind
/// entry point (`nvs_ir::lower::generator`'s transform) and reaches it from a
/// `nvs_object_release` whose `extern "C"` signature is one pointer wide:
/// threading a context through every release primitive would put a parameter
/// on the hot path of every decrement in the language to serve the one release
/// in ten thousand that frees a suspended generator, which AGENTS.md's
/// priority 3 rules out.
///
/// **A release performs no other context access**, which is what makes this
/// sound: the `&mut Ctx` frames above a release are dormant for the length of
/// it, so the reborrow [`with_current`] hands out is the only live one.
/// **Cost:** two thread-local word stores per Rust-to-compiled call boundary —
/// not per compiled call, which passes the context in a register.
pub(crate) struct CurrentCtx(*mut Ctx, *const crate::object::LiveList);

impl CurrentCtx {
    /// Makes `ctx` this thread's current context until the guard drops.
    pub(crate) fn install(ctx: &mut Ctx) -> Self {
        let live = std::rc::Rc::as_ptr(&ctx.live);
        Self(CURRENT.replace(&raw mut *ctx), CURRENT_LIVE.replace(live))
    }
}

impl Drop for CurrentCtx {
    fn drop(&mut self) {
        CURRENT_LIVE.set(self.1);
        CURRENT.set(self.0);
    }
}

/// Runs `body` against this thread's current context, or answers `None` when
/// no compiled frame is running — see [`CurrentCtx`].
pub(crate) fn with_current<R>(body: impl FnOnce(&mut Ctx) -> R) -> Option<R> {
    let ptr = CURRENT.get();
    if ptr.is_null() {
        return None;
    }
    #[expect(
        unsafe_code,
        reason = "the pointer was installed by `CurrentCtx::install` from a \
                  live `&mut Ctx` whose guard is still on this thread's stack, \
                  and every frame holding one is dormant for the length of a \
                  release"
    )]
    Some(body(unsafe { &mut *ptr }))
}
