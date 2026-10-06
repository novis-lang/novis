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
/// in ten thousand that frees a suspended generator, which would spend latency
/// on every program for a rare case.
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

/// The two words [`CurrentCtx`] installs, taken off the thread for the length
/// of a stack switch.
///
/// # Why the switch has to carry them
///
/// **Both words describe one *stack*, while the thread-local they live in is
/// shared by every coroutine the core runs.** [`CurrentCtx`] is a save/restore
/// guard, so a task that installs while another task is parked saves *that*
/// task's words and writes them back when its own guard drops. If the parked
/// task finished in between — resumed, returned, and had its context dropped —
/// the restore puts a freed [`Ctx`] and a freed [`crate::object::LiveList`]
/// back on the thread, and the next object allocation links itself onto a list
/// that is gone. That is a use-after-free with no unsafe block written anywhere
/// near it, which is why the carrier is declared here rather than in the
/// scheduler: this module owns what the words mean, so it owns the rule that
/// they never outlive the stack that set them.
///
/// `nvs_host`'s `yield_on` is the one caller, beside [`crate::HelperFrame`]'s
/// count and for exactly its reason: [`take`](CurrentStack::take) on the way
/// out of a stack and [`restore`](CurrentStack::restore) on the way back in. A
/// task that never comes back — cancelled, or unwound through the switch —
/// simply drops this without restoring, and the thread is left carrying
/// nothing, which both readers already answer for: [`with_current`] with `None`
/// and [`current_live_list`] with a null.
///
/// **Cost:** two thread-local word swaps per park and two per resume, against
/// a switch that already costs a stack change.
#[derive(Debug)]
#[must_use = "the words are off the thread until this is restored"]
pub struct CurrentStack(*mut Ctx, *const crate::object::LiveList);

impl CurrentStack {
    /// Takes both words off the thread, answering what they were.
    pub fn take() -> Self {
        Self(
            CURRENT.replace(std::ptr::null_mut()),
            CURRENT_LIVE.replace(std::ptr::null()),
        )
    }

    /// Puts back what [`take`](CurrentStack::take) answered.
    pub fn restore(self) {
        CURRENT.set(self.0);
        CURRENT_LIVE.set(self.1);
    }

    /// Whether the stack this came off was running a compiled frame.
    ///
    /// For the assertion a caller cannot otherwise make: a thread between tasks
    /// carries nothing, and a restore that crossed a switch is what makes it
    /// carry something that is gone.
    #[must_use]
    pub fn is_armed(&self) -> bool {
        !self.0.is_null()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// [`CurrentStack`]'s whole contract, in the order `yield_on` makes the
    /// calls: a stack that parks leaves the thread carrying nothing, and gets
    /// its own words back when it is resumed.
    ///
    /// The middle assertion is the load-bearing one. It is what makes the next
    /// task's [`CurrentCtx::install`] save a null rather than this stack's
    /// context, so the guard that task drops — possibly long after this
    /// context has been dropped — writes a null back instead of a dangling
    /// pointer.
    #[test]
    fn a_stack_switch_takes_the_context_off_the_thread_and_gives_it_back() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let installed = CurrentCtx::install(&mut ctx);
        assert!(!CURRENT.get().is_null(), "a running frame is on the thread");
        assert!(!current_live_list().is_null());

        let parked = CurrentStack::take();
        assert!(parked.is_armed(), "the stack that parked was running one");
        assert!(
            CURRENT.get().is_null() && current_live_list().is_null(),
            "the thread between two stacks carries neither word"
        );
        assert!(
            with_current(|_| ()).is_none(),
            "and no reader finds a context to reach through"
        );

        parked.restore();
        assert!(
            std::ptr::eq(current_live_list(), std::rc::Rc::as_ptr(&ctx.live)),
            "the resumed stack gets its own list back and not another's"
        );

        drop(installed);
        assert!(CURRENT.get().is_null(), "and the guard still clears it");
    }

    /// A stack that was running nothing carries nothing across the switch —
    /// the case every scheduler-owned stack is in, since only
    /// [`crate::abi::call`] ever arms the pair.
    #[test]
    fn a_stack_that_was_running_nothing_restores_nothing() {
        let taken = CurrentStack::take();
        assert!(!taken.is_armed());
        taken.restore();
        assert!(CURRENT.get().is_null() && current_live_list().is_null());
    }
}
