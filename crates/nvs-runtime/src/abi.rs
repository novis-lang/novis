//! `rule:errors/propagation`'s calling
//! convention, and the macro that makes it impossible to write a helper
//! without it.
//!
//! ```text
//! extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32
//! //            ^ request  ^ arguments   ^ result     ^ OK | THROWN | FATAL
//! ```
//!
//! Nothing unwinds through a JIT frame, because `cranelift-jit` registers no
//! unwind tables with the OS on any platform — that ADR's *Context* has the
//! measurement. So a failure travels in the return value and the caller
//! branches on it, and every helper wraps its body in `catch_unwind`, which is
//! what contains a runtime panic to one request instead of the process.
//!
//! Both halves are supplied by [`nvs_helper!`] rather than left to a
//! convention, for the reason that ADR's *Corollary* gives: `extern "C"`
//! rather than `extern "C-unwind"` and the `catch_unwind` wrapper are each a
//! silent, per-helper correctness cliff, and a macro cannot forget either.
//!
//! # Containment has two boundaries, and this module holds both
//!
//! The helper wrapper is the **inner** one, and it only ever sees a fault
//! raised beneath a JIT frame.
//! `rule:http-server/containment-does-not-end-at-the-helper`
//! puts the **outer** one at the root of every task, because the code a
//! worker runs with no request beneath it — the accept loop, the HTTP reader,
//! the compiled-unit cache index — has no helper frame to be contained by.
//! [`run_task`] is that boundary and [`TaskRoot`] is the whole of what
//! distinguishes its two outcomes; its doc comment carries the two rules that
//! travel with it, which are about teardown rather than about wrapping.

use std::panic::{self, AssertUnwindSafe};

use crate::ctx::{Ctx, SafepointFlags};
use crate::value::Value;

/// Success. The result has been written through the `out` pointer.
pub const OK: i32 = 0;

/// An Novis-level exception is pending in [`Ctx`]. A `catch` may handle it.
pub const THROWN: i32 = 1;

/// Unrecoverable: a resource limit, or an internal error caught at a helper
/// boundary. Propagates to the request boundary, and Novis code cannot catch it
/// (`rule:errors/escalation-ladder`).
pub const FATAL: i32 = 2;

/// The program stopped itself: `exit` or `exit(...)` ran, and the status it
/// named is on the context ([`Ctx::exit_code`]).
///
/// It propagates exactly the way [`FATAL`] does — no `catch` sees it, and no
/// `finally` runs, which is PHP's own `exit` — but it is **not** a failure:
/// `exit(0)` is the most ordinary end a program has, so the request boundary
/// reports the code rather than an error. `docs/adr/README.md`
/// § *Decisions taken at project start* owns why this is a fourth status
/// rather than a `FATAL` carrying a code.
pub const EXITED: i32 = 3;

/// A compiled Novis function.
///
/// `unsafe` because the three pointers carry a contract the type cannot
/// express: each must be non-null, aligned and valid for the duration of the
/// call, `args` must point at every slot the callee reads, and `out` must be
/// writable. Compiled code satisfies this by construction; hand callers go
/// through [`call`].
///
/// **`args` is not the argument list — it is the argument list behind an
/// implicit receiver.** A compiled *method* reads slot 0 as its receiver and
/// its first declared parameter at slot **1**, so the array is `1 + arity`
/// values long. What slot 0 holds depends on the method: `$this` for an
/// instance method, and for a `static` one the **called class descriptor**,
/// which is late static binding's whole mechanism and the reason it costs no
/// second parameter and no second calling convention (`nvs_ir::lower`, which
/// owns the numbering). A *script frame* is the one compiled function with no
/// receiver at all, so it takes an empty array.
///
/// Every compiled call site writes that leading slot, so a hand caller that
/// passes only the declared arguments does not get a diagnostic — it reads one
/// `Value` past the end of its own slice for **every** parameter, and answers
/// with whatever was next in memory. [`crate::dispatch`], [`crate::callable`]
/// and `nvs_codegen::Unit`'s named entry points — `script`, `call_static`,
/// `call_on_new_instance`, `build_fixture` — are the constructors that get it
/// right, and that type's own docs are the table of which one is which; prefer
/// one of them to building the array by hand.
pub type NvsFn = unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32;

/// A runtime helper. Identical to [`NvsFn`] — that identity is the point of
/// having one convention.
pub type HelperFn = NvsFn;

/// How a helper body reports a failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum Fault {
    /// An Novis exception a `catch` may handle; becomes [`THROWN`].
    ///
    /// The class is which of spec § 10's tree the promoted object is built
    /// from, so `Core\Json::decode("{oops}")` is caught by
    /// `catch (ParseError $e)` and not only by `catch (Throwable $e)`.
    /// [`Fault::thrown`] means [`crate::ThrownClass::Runtime`], which is what a
    /// failure with nothing more specific to say is.
    Thrown(crate::ThrownClass, std::borrow::Cow<'static, str>),
    /// [`Self::Thrown`], plus one value for a property the thrown class
    /// declares beyond `Throwable`'s four — named by its slot index, so every
    /// such property reaches its slot through this one variant.
    ///
    /// The classes that fill it, any further one owing a caller rather than a
    /// variant: `rule:core-classes/derive-reports-every-field`'s `issues` on
    /// `ParseError`, so a member that found *several* things wrong with one
    /// input tells a form about every bad field rather than the first, and
    /// `rule:core-classes/db-error`'s `kind` on
    /// `Core\Db\DbError`, so a `catch` branches on the condition the server
    /// named rather than on the wording of the message. A sibling variant per
    /// property was the alternative and was rejected: each one costs an arm in
    /// [`record_fault`], another in `nvs_stdlib::task`'s `call_child` and one in
    /// every reader added later, all to say the same thing with a different
    /// constant in it. The slot index is passed rather than derived from the
    /// class because deriving it would bake "one own property per class" into
    /// the ABI, which is true of those classes today and of neither ADR.
    ///
    /// This variant is the one place a [`Fault`] carries a reference at all: the
    /// values are transferred into those slots the moment the fault is recorded,
    /// and released where the class is too narrow to have one. Building it
    /// eagerly is what keeps [`Ctx`]'s pending state free of a reference it
    /// would have to release on every replacement path.
    ///
    /// The pairs are boxed rather than held inline, which is
    /// `rule:core-classes/db-error`'s measurement: a
    /// `Box<[_]>` is two words where one `(slot, value)` pair is three, so this
    /// carries any number of properties in **less** width than it carried one,
    /// and every helper's `Result` is narrower for it. The allocation is paid
    /// only by a throw that fills a slot at all.
    ThrownWithSlots(
        crate::ThrownClass,
        std::borrow::Cow<'static, str>,
        Box<[(usize, crate::Value)]>,
    ),
    /// Unrecoverable; becomes [`FATAL`].
    Fatal(std::borrow::Cow<'static, str>),
    /// A callee this helper invoked already failed and already recorded what
    /// failed in [`Ctx`] — return its status unchanged.
    ///
    /// The one variant carrying no message, and deliberately: a
    /// [`Self::Thrown`] built here would call [`Ctx::set_pending`] a second
    /// time and replace the exception object the callee raised with a bare
    /// string. Reached from [`crate::call_callable`], which is the one place a
    /// helper calls compiled Novis code, and from `nvs_exit`, whose *success* is
    /// [`EXITED`] and which has already recorded the status on the context.
    Pending(i32),
}

impl Fault {
    /// A [`Fault::Thrown`] with a message, as spec § 10's `RuntimeError`.
    #[must_use]
    pub fn thrown(message: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        Self::Thrown(crate::ThrownClass::Runtime, message.into())
    }

    /// A [`Fault::Thrown`] with a message, as a *named* class of spec § 10's
    /// tree — what a member whose own signature promises one throws.
    #[must_use]
    pub fn thrown_as(
        class: crate::ThrownClass,
        message: impl Into<std::borrow::Cow<'static, str>>,
    ) -> Self {
        Self::Thrown(class, message.into())
    }

    /// A [`Fault::ThrownWithSlots`] filling `slot` on the object the throw
    /// builds — the one route from a native member to a property below
    /// `Throwable`'s four.
    ///
    /// Takes over `value`'s reference; see that variant for where it goes.
    #[must_use]
    pub fn thrown_with_slot(
        class: crate::ThrownClass,
        message: impl Into<std::borrow::Cow<'static, str>>,
        slot: usize,
        value: crate::Value,
    ) -> Self {
        Self::ThrownWithSlots(class, message.into(), Box::new([(slot, value)]))
    }

    /// [`Self::thrown_with_slot`] for a class filling more than one of its own
    /// properties — `Core\Db\DbError`'s normalised `kind` beside the raw
    /// `sqlState` the driver read it off
    /// (`rule:core-classes/db-error`).
    ///
    /// Takes over every value's reference.
    #[must_use]
    pub fn thrown_with_slots(
        class: crate::ThrownClass,
        message: impl Into<std::borrow::Cow<'static, str>>,
        slots: Vec<(usize, crate::Value)>,
    ) -> Self {
        Self::ThrownWithSlots(class, message.into(), slots.into_boxed_slice())
    }

    /// [`Self::thrown_with_slot`] at [`crate::ISSUES_SLOT`] — `rule:core-classes/derive-reports-every-field`'s
    /// "report every bad field at once". Spelled once here rather than at each
    /// of `nvs_stdlib::json`'s throw sites, none of which should have to name a
    /// slot index to throw a `ParseError`.
    ///
    /// Takes over `issues`' reference.
    #[must_use]
    pub fn thrown_with_issues(
        class: crate::ThrownClass,
        message: impl Into<std::borrow::Cow<'static, str>>,
        issues: crate::Value,
    ) -> Self {
        Self::thrown_with_slot(class, message, crate::ISSUES_SLOT, issues)
    }

    /// A [`Fault::Fatal`] with a message.
    #[must_use]
    pub fn fatal(message: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        Self::Fatal(message.into())
    }
}

/// A size that is about to become an allocation, refused before it is
/// attempted — **the one place every count-shaped argument is checked**.
///
/// `member` is the fully qualified name for the message, e.g.
/// `"Core\\Arr::fill"`.
///
/// # Why this is a function and not a check per call site
///
/// A guard that is written per call site is a guard the next call site
/// forgets, and the members with the largest appetite of all —
/// `Core\Arr::fill`/`padStart`/`padEnd` through `append_copies`, and
/// `Core\Str::padStart`/`padEnd` through `padding_run` — are exactly the ones
/// such a convention reaches last. That is the shape of PHP's own history
/// here: its `memory_limit` is enforced in the allocator precisely because
/// per-function checks did not hold.
///
/// # What it refuses, in two tiers
///
/// A size past `isize::MAX`, or a computation that already overflowed to
/// `None`, is an allocation **no process** could make. Nothing about the
/// request is wrong, so that is a throw.
///
/// A size the running request cannot afford is
/// `rule:errors/on-limit`'s resource `FATAL`.
/// [`crate::budget::affords`] is asked against `[limits] memory`'s remaining
/// balance, and a `false` from it has already recorded the breach: the request
/// is over from that moment whatever this caller does next, so handing it back
/// a `catch` to carry on from would put a resource limit below the tier
/// `rule:errors/escalation-ladder` gives it.
///
/// Asking *in front of* the allocation is what bounds **one** operation rather
/// than a loop of them — [`crate::budget::add`] compares once the block is
/// already held, so an input-sized ask that reaches the allocator first is a
/// request holding twice its ceiling before anything polls it.
/// `rule:programs/memory-priority` settles that a per-request ceiling is
/// enforceable, and this is the seam it attaches to for every count-shaped
/// argument — one place to change rather than one per call site.
///
/// An uncapped request is refused nothing here but the unallocatable: `affords`
/// answers on its sentinel without reading a balance, which is what every
/// context with no configuration — every test's — costs.
///
/// # Errors
///
/// A [`Fault::Thrown`] for the unallocatable size, so a program can catch it.
/// The alternative is the allocator's own behaviour, which is an abort for the
/// raw paths and a panic for the `Vec` ones — contained to the request by
/// [`run_helper`], but not catchable, and a refusal of that shape is exactly
/// the kind a caller may want to handle.
///
/// A [`Fault::Fatal`] for the unaffordable one, which no `catch` sees.
/// [`run_helper`]'s failure arm re-reads
/// [`Ctx::memory_breach`](crate::Ctx::memory_breach) and reports the breach in
/// place of whatever the member said, so a refused `Core` member names the
/// ceiling and the reading; the message built here is what a direct caller
/// reads.
pub fn affordable(bytes: Option<usize>, member: &str) -> Result<usize, Fault> {
    let size = bytes
        .filter(|size| isize::try_from(*size).is_ok())
        .ok_or_else(|| {
            Fault::thrown(format!(
                "{member}: the requested allocation is larger than any this process could hold"
            ))
        })?;
    if !crate::budget::affords(size) {
        return Err(Fault::fatal(format!(
            "{member}: the request cannot afford an allocation of {size} bytes — it is past its memory limit"
        )));
    }
    Ok(size)
}

/// How many iterations of a [`bounded_loop`] pass between two deadline polls.
///
/// `rule:http-server/time-is-bounded-inside-a-helper`
/// requires the *amortised* poll to stay under the stack check's own
/// per-call cost, and this constant is the only thing that number depends on:
/// a poll is one relaxed load and a compare against a line
/// [`crate::nvs_stack_check`] has already brought in, so dividing it by 256
/// leaves it a fraction of a check that itself costs a load and a compare.
///
/// Small enough to matter in the other direction too: at 256 iterations a
/// deadline that fires is observed within a few hundred nanoseconds of work,
/// not within the whole loop.
pub const DEADLINE_POLL_BATCH: usize = 256;

/// Runs a helper's O(input) loop and polls the request's deadline *for* it.
///
/// `rule:http-server/time-is-bounded-inside-a-helper`
/// 's first constraint: **the poll is supplied by a bounded-loop
/// combinator, not remembered per helper.** The safepoint bounds Novis code
/// because it sits between calls, and a helper is one call — so a member whose
/// runtime scales with its input is exactly the shape a deadline cannot
/// otherwise reach inside. This is that shape, and it rests on the same
/// argument [`crate::nvs_helper!`] does: a member adopts the shape and the
/// obligation arrives with it, so the next member cannot omit a poll it never
/// had to know about.
///
/// The poll is amortised over [`DEADLINE_POLL_BATCH`] iterations. Per iteration
/// that is a register decrement and a not-taken branch; once per batch it is
/// the relaxed load [`Ctx::deadline_expired`] makes. `Ctx`'s module doc
/// § *The request's deadline* owns why the flag is read rather than a clock.
///
/// `body` is handed the context back, so a member that needs it inside the loop
/// does not have to choose between the poll and its own work.
///
/// **A member with no consistent point to abandon at does not reach for this.**
/// A sort cannot hand back a half-permuted array; where the operation has no
/// such point the bound belongs on the *input* instead, which is `rule:http-server/time-is-bounded-inside-a-helper`'s
/// second constraint and what
/// `rule:core-classes/regex-two-tiers` already did for
/// patterns. Between two iterations of *this* loop is such a point by
/// construction, because `body` has returned.
///
/// # Errors
///
/// Whatever `body` returns, or a [`Fault::Fatal`] naming `member` and the
/// ceiling that stopped it when the deadline has passed. Fatal rather than
/// thrown because a deadline is a cancellation, and
/// `rule:concurrency/cancellation-runs-no-user-code`
/// settles that cancellation is not a `Throwable` and runs no user code — the
/// same standing [`FATAL`] already gives a resource limit, and the same one
/// [`crate::nvs_safepoint`] gives a cancelled context.
pub fn bounded_loop<I, F>(ctx: &mut Ctx, member: &str, items: I, mut body: F) -> Result<(), Fault>
where
    I: IntoIterator,
    F: FnMut(&mut Ctx, I::Item) -> Result<(), Fault>,
{
    let mut until_poll = DEADLINE_POLL_BATCH;
    for item in items {
        until_poll -= 1;
        if until_poll == 0 {
            if ctx.deadline_expired() {
                return Err(deadline_passed(ctx, member));
            }
            until_poll = DEADLINE_POLL_BATCH;
        }
        body(ctx, item)?;
    }
    Ok(())
}

/// What a fired poll reports, in one place so every site says the same thing.
///
/// One word carries two ceilings into this poll: `[limits] wall_time` expires
/// the deadline, and `rule:errors/on-limit`'s CPU ceiling expires it beside
/// raising [`SafepointFlags::CPU_LIMIT`], because a member part-way through one
/// long call reaches no other poll (`nvs_host::watchdog` is the raiser, and
/// [`crate::SafepointView::expire_deadline`] owns that argument). So which
/// ceiling stopped the walk is read from the flags — on a path where the
/// request is already over, so a second relaxed load buys the right message for
/// nothing.
fn deadline_passed(ctx: &Ctx, member: &str) -> Fault {
    if ctx.safepoint_flags().contains(SafepointFlags::CPU_LIMIT) {
        return Fault::fatal(format!("{member}: the request exceeded its CPU-time limit"));
    }
    Fault::fatal(format!("{member}: the request's deadline passed"))
}

/// What a helper body returns: the result value, or a [`Fault`].
///
/// A helper invoked purely for its effect — `nvs_ir::Helper::EchoStr` and its
/// carrier-aware twin `EchoValue` are the ones so far — returns
/// [`Value::null`].
pub type HelperResult = Result<Value, Fault>;

/// The body of every helper, shared so [`crate::nvs_helper!`] expands to one line.
///
/// Kept public because the macro's expansion names it, not because callers
/// should reach for it directly.
///
/// # Safety
///
/// `ctx` and `out` must be non-null, aligned and valid for the duration of the
/// call, and `out` must be writable. `args` must point at `arity` initialized
/// values, unless `arity` is zero, in which case it is ignored.
#[expect(
    unsafe_code,
    reason = "this is the one place the helper ABI's pointer contract is \
              discharged; every helper reaches it through `nvs_helper!`"
)]
pub unsafe fn run_helper<F>(
    ctx: *mut Ctx,
    args: *const Value,
    arity: usize,
    out: *mut Value,
    body: F,
) -> i32
where
    F: FnOnce(&mut Ctx, &[Value]) -> HelperResult,
{
    // Before anything else and dropped after everything else: from here to the
    // return, a task standing on this stack is one a host may not force-unwind.
    // See [`HelperFrame`].
    let _frame = HelperFrame::enter();
    // `rule:errors/on-limit`'s memory limit, asked *before* the body: a member
    // that has already run holds a `Value` this frame would then have to
    // release on a path nothing else takes, and refusing in front of the
    // allocation is what [`affordable`]'s own doc comment says this seam is
    // for. The `Ok(Err(..))` arm below asks it a second time, on the one way
    // out of a body that leaves no such `Value` behind.
    //
    // Not the only place it is asked at all: [`crate::budget`] arms the
    // allocator with this request's ceiling, so an allocation that crosses it
    // — or is refused in front of it — raises
    // [`SafepointFlags::MEMORY_LIMIT`] and the program is stopped at its next
    // back edge. What this seam still answers alone is growth that reaches no
    // back edge, and what it answers in every case is that no member's body
    // runs past the ceiling.
    //
    // What it costs an uncapped request — every context with no configuration,
    // which is every test's — is one compare against a zero field: see
    // [`Ctx::over_memory_limit`].
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `ctx` is valid for this call; this \
                  borrow ends before the closure below takes its own"
    )]
    let breach = unsafe { &*ctx }.memory_breach();
    if let Some(fault) = breach {
        #[expect(unsafe_code, reason = "same contract, and the borrow above has ended")]
        let ctx = unsafe { &mut *ctx };
        // The ceiling's second chance, and it is asked only here — on the one
        // path that was about to stop the request — so a member call under the
        // ceiling pays nothing for the collector's existence. What was asked
        // for is a walk of this context's cyclic garbage
        // (`crate::object::collect`), and the counter is read again after it:
        // bytes a dead cycle was holding are the request's again, and the
        // member's body runs. `Ctx::collect_if_asked` owns why the ask can
        // only have come from a crossing of this very ceiling.
        let breach = if ctx.collect_if_asked() {
            ctx.memory_breach()
        } else {
            Some(fault)
        };
        if let Some(fault) = breach {
            return report_memory_breach(ctx, fault);
        }
    }
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        #[expect(
            unsafe_code,
            reason = "the caller guarantees `ctx` is valid for this call and \
                      `args` points at `arity` values; the zero-arity case is \
                      split out because `from_raw_parts` rejects a null \
                      pointer even for an empty slice"
        )]
        let (ctx, args) = unsafe {
            (
                &mut *ctx,
                if arity == 0 {
                    &[][..]
                } else {
                    std::slice::from_raw_parts(args, arity)
                },
            )
        };
        // Inside `catch_unwind`, so an injected fault is contained exactly the
        // way a real helper bug would be — which is the only thing this hook
        // exists to demonstrate. See `crate::ctx::FaultSite`.
        assert!(
            !ctx.take_armed_helper_panic(),
            "internal error: injected helper panic (--fault-inject=helper-panic)"
        );
        body(ctx, args)
    }));

    #[expect(
        unsafe_code,
        reason = "same contract as above; the borrow taken inside the closure \
                  has ended, so this one cannot alias it"
    )]
    let ctx = unsafe { &mut *ctx };

    match outcome {
        Ok(Ok(value)) => {
            #[expect(
                unsafe_code,
                reason = "the caller guarantees `out` is writable for one Value"
            )]
            unsafe {
                out.write(value);
            }
            OK
        }
        Ok(Err(fault)) => match ctx.memory_breach() {
            // A member that failed *while* the request crossed its ceiling
            // reports the ceiling and not its own error. A refused allocation
            // is what a fallible producer reads as "no room", and one that
            // words that as a throw would otherwise hand the program a `catch`
            // to carry on from — which is the whole of what
            // `rule:errors/escalation-ladder` puts a resource limit above. The
            // question is asked on this arm and not on the `Ok` one for the
            // reason the comment ahead of the body gives: there is no result
            // `Value` to release here.
            Some(breach) => report_memory_breach(ctx, breach),
            None => record_fault(ctx, fault),
        },
        Err(payload) => {
            if Teardown::in_progress() {
                // Not a helper bug: this thread is tearing a task's stack down
                // and the unwind belongs to it. A helper is on that stack
                // whenever a member *parks* — `Core\Time::sleep` through
                // [`crate::host::Host::sleep`] is one — and a cancelled
                // task is resumed into a forced unwind rather than into its
                // body, so the unwind passes through this frame on its way out.
                // Containing it here would leave the coroutine's own runtime
                // with an unwind it started and never got back, which is a
                // process abort. See [`Teardown`], and `run_task` below, which
                // makes the same test for the same reason.
                panic::resume_unwind(payload);
            }
            ctx.set_pending(panic_message(&*payload));
            FATAL
        }
    }
}

/// Runs `rule:errors/on-limit`'s handler for `breach` and answers the status
/// the breach becomes.
///
/// Two seams ask the same question and owe the same answer: [`run_helper`]
/// ahead of a member's body, and the way out of a body that failed. The handler
/// is what the program gets instead of the member the call was for, and it runs
/// before the breach becomes the status the caller sees, so a fault of its own
/// is overwritten by [`record_fault`] rather than reported in place of the
/// limit. `Ctx::run_limit_handler` owns the zero-retry rule that keeps the
/// handler's own first helper call from arriving back here and calling it a
/// second time.
fn report_memory_breach(ctx: &mut Ctx, breach: Fault) -> i32 {
    ctx.run_limit_handler(crate::Limit::Memory);
    record_fault(ctx, breach)
}

/// The status a write refused by `rule:errors/on-limit`'s memory ceiling
/// becomes, for a primitive whose signature already carries one.
///
/// The same two lines [`run_helper`] takes ahead of a member's body — the
/// handler, then the breach as a `FATAL` no `catch` sees — reached from a
/// helper that holds a `ctx` and a status but has no [`Fault`] of its own to
/// hand back. `crate::array::nvs_array_append` and `nvs_array_spread` are the
/// callers: the array writes that do not have to answer a degenerate value,
/// because they can say what happened.
pub(crate) fn report_refusal(ctx: &mut Ctx) -> i32 {
    match ctx.memory_breach() {
        Some(breach) => report_memory_breach(ctx, breach),
        // The refusal has already been taken from the request — `take_refusal`'s
        // two callers are the context that ends and the handler lent a slice to
        // report with — so there is nothing left to record and the only thing
        // the caller owes is the write it did not make.
        None => OK,
    }
}

/// Records `fault` on `ctx` and answers the status it becomes.
///
/// [`run_helper`]'s own translation, extracted because a second caller reaches
/// it from outside the helper ABI: [`crate::dispatch::construct_and_call`],
/// which runs compiled code with no helper frame around it and still owes the
/// context the same record. Two copies of this match would be two answers to
/// "what did a `Fault::Fatal` leave behind".
pub(crate) fn record_fault(ctx: &mut Ctx, fault: Fault) -> i32 {
    match fault {
        Fault::Thrown(class, message) => {
            ctx.set_pending_as(class, message);
            THROWN
        }
        Fault::ThrownWithSlots(class, message, slots) => {
            #[expect(
                unsafe_code,
                reason = "the helper body transferred these references, and \
                          `raise_with_slots` transfers each on into the \
                          exception object's slot or releases it"
            )]
            unsafe {
                ctx.raise_with_slots(class, &message, &slots);
            }
            THROWN
        }
        Fault::Fatal(message) => {
            ctx.set_pending(message);
            FATAL
        }
        // Nothing to record: the callee that failed already did, and this
        // frame has nothing of its own to add — see `Fault::Pending`.
        Fault::Pending(status) => status,
    }
}

/// Recovers a panic's message, so a contained bug says what it was.
///
/// `pub(crate)` for [`crate::floor::install_panic_hook`], which reads the same
/// payload one moment earlier: a hook and this boundary reporting one panic
/// under two different messages is the divergence `crate::floor`'s module doc
/// exists to prevent, applied to the payload rather than to the record.
pub(crate) fn panic_message(
    payload: &(dyn std::any::Any + Send),
) -> std::borrow::Cow<'static, str> {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        std::borrow::Cow::Borrowed(*message)
    } else if let Some(message) = payload.downcast_ref::<String>() {
        std::borrow::Cow::Owned(message.clone())
    } else {
        std::borrow::Cow::Borrowed("internal error: a runtime helper panicked")
    }
}

/// Defines a runtime helper with Novis's calling convention.
///
/// The body is written against safe references — `&mut Ctx` and a `&[Value]`
/// of exactly the declared arity — and returns a [`HelperResult`]. Everything
/// else (the `extern "C"` signature, the `catch_unwind` wrapper, writing the
/// result through `out`, recording a message in [`Ctx`]) is supplied.
///
/// ```
/// use nvs_runtime::{Ctx, Fault, Value, call, nvs_helper};
///
/// nvs_helper! {
///     /// Doubles an `int`.
///     fn nvs_double(_ctx, args: [1]) {
///         let n = args[0].as_int().ok_or_else(|| Fault::fatal("not an int"))?;
///         Ok(Value::int(n * 2))
///     }
/// }
///
/// let mut ctx = Ctx::buffered();
/// assert_eq!(call(nvs_double, &mut ctx, &[Value::int(21)]).unwrap().as_int(), Some(42));
/// ```
#[macro_export]
macro_rules! nvs_helper {
    (
        $(#[$meta:meta])*
        fn $name:ident($ctx:ident, $args:ident: [$arity:expr]) $body:block
    ) => {
        $(#[$meta])*
        ///
        /// # Safety
        ///
        /// `ctx`, `args` and `out` must each be non-null, aligned and valid for
        /// the duration of the call; `args` must point at as many values as
        /// this helper's arity; and `out` must be writable. Compiled Novis code
        /// satisfies this by construction — hand callers go through
        /// [`call`](crate::call).
        #[allow(
            unsafe_code,
            reason = "a runtime helper's signature is fixed by `rule:errors/propagation`; the \
                      pointer contract cannot be expressed in the type, so the \
                      function is honestly marked unsafe"
        )]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            ctx: *mut $crate::Ctx,
            args: *const $crate::Value,
            out: *mut $crate::Value,
        ) -> i32 {
            // The arity and the body are bound outside the `unsafe` block, so
            // nothing a caller of this macro wrote is ever *inside* one — the
            // block covers exactly `run_helper`'s pointer contract and nothing
            // else.
            let arity: usize = $arity;
            let body = |$ctx: &mut $crate::Ctx, $args: &[$crate::Value]| $body;
            #[allow(
                unsafe_code,
                reason = "the caller's contract is exactly `run_helper`'s"
            )]
            unsafe {
                $crate::run_helper(ctx, args, arity, out, body)
            }
        }
    };
}

/// Calls a compiled function or a helper from Rust.
///
/// The safe wrapper hand callers — this crate's tests, `nvs-codegen`'s tests,
/// and eventually `nvs run` itself — use instead of building the three
/// pointers by hand.
///
/// `args` is passed through **exactly as given**: it is the callee's ABI slot
/// array, not its argument list, and the two differ for a compiled method by
/// the implicit receiver [`NvsFn`] describes. A helper's slots are its
/// arguments and nothing else; a method's slot 0 is its receiver and its
/// parameters start at slot 1. Nothing here can tell the two apart, which is
/// why the shape is the caller's to get right.
///
/// # Errors
///
/// The [`THROWN`] or [`FATAL`] status, with the message left in `ctx` for the
/// caller to [`Ctx::take_pending`].
pub fn call(function: NvsFn, ctx: &mut Ctx, args: &[Value]) -> Result<Value, i32> {
    let mut out = Value::null();
    // What the release path reaches a context through, since a decrement
    // carries none — `crate::ctx::CurrentCtx` owns the argument.
    let _current = crate::ctx::CurrentCtx::install(ctx);
    #[expect(
        unsafe_code,
        reason = "every pointer here is derived from a live Rust reference, and \
                  `args.len()` is by construction the arity the callee is being \
                  asked for"
    )]
    let status = unsafe { function(&raw mut *ctx, args.as_ptr(), &raw mut out) };
    if status == OK { Ok(out) } else { Err(status) }
}

/// What a task's root was running, and therefore who owns a panic that
/// reaches it.
///
/// `rule:http-server/containment-does-not-end-at-the-helper`
/// splits containment's two outcomes on exactly this question and on
/// nothing else, so it is the whole of what [`run_task`] has to be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskRoot {
    /// A request. Everything the fault touched belongs to that request and is
    /// released wholesale with its arena, so the request fails through
    /// `rule:errors/escalation-ladder`'s ladder
    /// as an internal panic and the worker's other in-flight requests are
    /// untouched.
    Request,
    /// Worker-owned work with no request beneath it — the accept loop, the
    /// HTTP reader, the compiled-unit cache index. There is nothing to charge
    /// the fault to and the state that faulted is shared, so the worker is
    /// **retired**: it stops accepting, its in-flight requests finish under the
    /// existing drain, and a replacement is started.
    Worker,
}

/// A panic that reached a task's root and was contained there.
#[derive(Debug, Clone)]
pub struct TaskPanic {
    message: std::borrow::Cow<'static, str>,
    root: TaskRoot,
}

impl TaskPanic {
    /// The payload's own text, or a generic message for a payload that is
    /// neither `&'static str` nor `String` — the same recovery the helper
    /// boundary makes.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// What the task's root was running.
    #[must_use]
    pub const fn root(&self) -> TaskRoot {
        self.root
    }

    /// Whether this fault retires the worker, which is `rule:http-server/containment-does-not-end-at-the-helper`'s split
    /// and the only decision a caller has to make from one of these.
    #[must_use]
    pub const fn retires_worker(&self) -> bool {
        matches!(self.root, TaskRoot::Worker)
    }
}

impl std::fmt::Display for TaskPanic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Runs a task's root under the **outer** containment boundary.
///
/// [`nvs_helper!`]'s wrapper contains a panic raised beneath a JIT frame, and
/// that stays exactly where it is: it is the inner boundary, it reports through
/// [`Ctx`] as [`FATAL`], and a fault it catches never reaches here. This one
/// exists for the code a worker runs with no helper frame under it at all —
/// `rule:http-server/containment-does-not-end-at-the-helper` — and it is applied by whatever spawns the task rather than
/// written per call site, for the same reason the macro exists: a boundary a
/// contributor can forget is not a boundary.
///
/// Two rules travel with it, and neither is discharged here because neither is
/// a wrapper:
///
/// * **Nothing on a teardown path may panic.** A panic raised while a panic is
///   unwinding calls `abort()` before any `catch_unwind` — this one included —
///   is consulted, so a `Drop` that can fail turns a contained fault into an
///   uncontained one. What holds it is that Novis's teardown runs no user code
///   and no fallible operation: [`crate::release`]'s module doc is that rule's
///   home.
/// * **Teardown is iterative, never recursive.** Held by the same worklist, and
///   pinned by `object::tests::a_deep_chain_is_freed_without_recursing` and
///   `array::tests::a_deeply_nested_array_releases_without_recursing`.
///
/// It costs nothing on the path that does not panic, and it is one wrap per
/// *task* rather than one per call.
///
/// # Errors
///
/// The contained panic, carrying its message and the [`TaskRoot`] that decides
/// whether the worker survives it.
pub fn run_task<T, F>(root: TaskRoot, body: F) -> Result<T, TaskPanic>
where
    F: FnOnce() -> T,
{
    panic::catch_unwind(AssertUnwindSafe(body)).map_err(|payload| {
        if Teardown::in_progress() {
            // Not a fault: the host is tearing this task's stack down and the
            // unwind belongs to it. See `Teardown`.
            panic::resume_unwind(payload);
        }
        TaskPanic {
            message: panic_message(&*payload),
            root,
        }
    })
}

thread_local! {
    /// How many [`Teardown`] guards this thread is inside. A counter rather
    /// than a flag because tearing down one task's stack can drop a value that
    /// tears down another's.
    static TEARDOWN_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Marks the calling thread as tearing a task's stack down, so that
/// [`run_task`] lets an unwind through instead of containing it.
///
/// **This is the one hole in the outer containment boundary, and it is not
/// optional.** A stackful coroutine that is dropped while suspended is torn
/// down by unwinding its stack, and the mechanism that does it raises a panic
/// carrying a private marker which it expects to see come back out. A
/// `catch_unwind` in the way — [`run_task`]'s, which sits under every task root
/// — swallows that marker, and the coroutine implementation then aborts the
/// process because it cannot tell a swallowed teardown from a corrupted stack.
/// That is a path to `abort()` reached by an ordinary worker shutdown with a
/// request still parked, which
/// `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`
/// does not allow.
///
/// It is *narrow* on purpose: the guard is held only across the drop of a
/// suspended task, by the host that is doing the dropping, on the thread doing
/// it. Nothing script-level runs inside the window —
/// `rule:concurrency/cancellation-runs-no-user-code`'s
/// "cancellation runs no user code" is untouched, because what unwinds is
/// native `Drop` code, which that section already permits and requires.
///
/// A genuine panic raised inside the window is propagated rather than
/// contained. That is the correct answer: it is a panic in teardown, which
/// [`crate::release`]'s module doc rules out by construction, and containing
/// one would hide the very thing that rule exists to prevent.
#[derive(Debug)]
pub struct Teardown(());

impl Teardown {
    /// Enters the window. It ends when the returned guard is dropped.
    #[must_use]
    pub fn enter() -> Self {
        TEARDOWN_DEPTH.with(|depth| depth.set(depth.get() + 1));
        Self(())
    }

    /// Whether this thread is inside one.
    #[must_use]
    pub fn in_progress() -> bool {
        TEARDOWN_DEPTH.with(std::cell::Cell::get) > 0
    }
}

impl Drop for Teardown {
    fn drop(&mut self) {
        TEARDOWN_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

thread_local! {
    /// How many helper frames the stack this thread is *running on* carries.
    ///
    /// Running on, not "has ever entered": [`HelperFrame::take`] moves the count
    /// off the thread at a stack switch and [`HelperFrame::restore`] puts it
    /// back, so a task suspended inside a helper does not make its neighbour
    /// look as if it were inside one.
    static HELPER_FRAMES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// A frame a forced unwind may not cross, counted for as long as the helper
/// runs.
///
/// [`Teardown`] above is how an unwind gets *through* [`run_task`]'s
/// containment boundary. This is the question that has to be asked before one
/// is started at all: **a helper's entry point is `extern "C"`**, and an unwind
/// leaving one aborts the process rather than continuing outwards — as does an
/// unwind through the compiled Novis frames underneath it, which carry no
/// unwind tables. A host that cancels a task standing on such a stack therefore
/// may not unwind it; it has to resume the task and let it die by
/// `rule:errors/propagation`'s return status at
/// its next safepoint, which is
/// `rule:concurrency/cancellation-runs-no-user-code`'s
/// rule reached the only way this stack allows. `nvs-host`'s scheduler module
/// owns that decision and is this type's only consumer.
///
/// **What it spends:** two thread-local word accesses per helper call, on a
/// `Cell<u32>` with a `const` initializer — the same cost class as
/// [`crate::ctx::CurrentCtx`]'s pair, bought for the same reason. The
/// alternative is asking the *caller* to declare what its stack looks like at
/// every park site, which is exactly the kind of invariant AGENTS.md's priority
/// ordering says not to spend.
#[derive(Debug)]
#[must_use = "the frame is counted only while this guard lives"]
pub struct HelperFrame(());

impl HelperFrame {
    /// Counts one helper frame until the guard is dropped.
    pub fn enter() -> Self {
        HELPER_FRAMES.with(|frames| frames.set(frames.get().saturating_add(1)));
        Self(())
    }

    /// Whether the stack running right now carries one.
    #[must_use]
    pub fn on_this_stack() -> bool {
        HELPER_FRAMES.with(std::cell::Cell::get) > 0
    }

    /// Takes the count off the thread, answering what it was.
    ///
    /// For a stack switch and nothing else: the count describes one stack, and
    /// the coroutine that is about to give the core back is taking its stack
    /// with it. The switch's other side owes a [`HelperFrame::restore`] of the
    /// same number — unless it never comes back, in which case the guards on
    /// that stack drop against a count that is already zero, which saturates.
    #[must_use]
    pub fn take() -> u32 {
        HELPER_FRAMES.with(|frames| frames.replace(0))
    }

    /// Puts back what [`HelperFrame::take`] answered.
    pub fn restore(frames: u32) {
        HELPER_FRAMES.with(|slot| slot.set(frames));
    }
}

impl Drop for HelperFrame {
    fn drop(&mut self) {
        HELPER_FRAMES.with(|frames| frames.set(frames.get().saturating_sub(1)));
    }
}

#[cfg(test)]
mod tests {
    // `nvs_helper!` emits `pub` entry points; inside this private test module
    // they are deliberately unreachable from outside the crate.
    #![allow(unreachable_pub)]

    use super::*;
    use crate::value::Tag;

    nvs_helper! {
        /// Returns its one argument unchanged.
        fn nvs_test_identity(_ctx, args: [1]) {
            Ok(args[0])
        }
    }

    nvs_helper! {
        /// Reports whether a helper frame is counted while a helper runs.
        fn nvs_test_counts_its_frame(_ctx, _args: [0]) {
            Ok(Value::bool(HelperFrame::on_this_stack()))
        }
    }

    nvs_helper! {
        /// Reports whether it was handed an empty argument slice.
        fn nvs_test_arity_zero(_ctx, args: [0]) {
            Ok(Value::bool(args.is_empty()))
        }
    }

    nvs_helper! {
        /// Always throws.
        fn nvs_test_throws(_ctx, _args: [0]) {
            Err(Fault::thrown("thrown from a helper"))
        }
    }

    nvs_helper! {
        /// Always fatal.
        fn nvs_test_fatal(_ctx, _args: [0]) {
            Err(Fault::fatal("fatal from a helper"))
        }
    }

    nvs_helper! {
        /// Panics with a `&'static str`.
        fn nvs_test_panics_static(_ctx, _args: [0]) {
            panic!("a static panic message")
        }
    }

    nvs_helper! {
        /// Panics with an owned `String`.
        fn nvs_test_panics_owned(_ctx, _args: [0]) {
            panic!("an owned panic message: {}", 7)
        }
    }

    nvs_helper! {
        /// Writes to the request's output, proving the context is reachable.
        fn nvs_test_writes(ctx, _args: [0]) {
            ctx.write_output(b"written").map_err(|e| Fault::fatal(e.to_string()))?;
            Ok(Value::null())
        }
    }

    #[test]
    fn a_helper_returns_its_result_through_out() {
        let mut ctx = Ctx::buffered();
        let value = call(nvs_test_identity, &mut ctx, &[Value::int(5)]).unwrap();
        assert_eq!(value.as_int(), Some(5));
        assert_eq!(ctx.pending(), None);
    }

    /// One poll, two ceilings: a wall clock expires the deadline, and
    /// `rule:errors/on-limit`'s CPU ceiling expires it beside raising its flag,
    /// so what a fired poll reports is read from the flags rather than assumed.
    #[test]
    fn a_fired_poll_names_the_ceiling_that_stopped_the_walk() {
        let entries = || 0..(DEADLINE_POLL_BATCH * 2);

        let mut wall = Ctx::buffered();
        wall.expire_deadline();
        let stopped = bounded_loop(&mut wall, "Core\\Arr::map", entries(), |_, _| Ok(()));
        let Err(Fault::Fatal(message)) = stopped else {
            panic!("a walk under an expired deadline was not stopped");
        };
        assert!(
            message.contains("Core\\Arr::map") && message.contains("deadline passed"),
            "{message}"
        );

        let mut burned = Ctx::buffered();
        burned.expire_deadline();
        burned.request_safepoint(SafepointFlags::CPU_LIMIT);
        let stopped = bounded_loop(&mut burned, "Core\\Arr::map", entries(), |_, _| Ok(()));
        let Err(Fault::Fatal(message)) = stopped else {
            panic!("a walk under a raised CPU ceiling was not stopped");
        };
        assert!(
            message.contains("Core\\Arr::map") && message.contains("CPU-time limit"),
            "{message}"
        );
    }

    #[test]
    fn a_zero_arity_helper_gets_an_empty_slice_not_a_null_deref() {
        let mut ctx = Ctx::buffered();
        let value = call(nvs_test_arity_zero, &mut ctx, &[]).unwrap();
        assert_eq!(value.as_bool(), Some(true));
    }

    #[test]
    fn a_helper_counts_its_own_frame_and_gives_it_back() {
        // The question a host asks before it force-unwinds a task: is there an
        // `extern "C"` frame on this stack? `HelperFrame`'s doc owns why the
        // answer decides between an unwind and a resume, and `nvs-host`'s
        // scheduler is what asks.
        let mut ctx = Ctx::buffered();
        assert!(
            !HelperFrame::on_this_stack(),
            "a frame was counted outside every helper"
        );
        let value = call(nvs_test_counts_its_frame, &mut ctx, &[]).unwrap();
        assert_eq!(value.as_bool(), Some(true));
        assert!(
            !HelperFrame::on_this_stack(),
            "the frame outlived the call it was entered for"
        );
    }

    #[test]
    fn a_thrown_fault_becomes_thrown_with_its_message() {
        let mut ctx = Ctx::buffered();
        assert_eq!(call(nvs_test_throws, &mut ctx, &[]).unwrap_err(), THROWN);
        assert_eq!(ctx.take_pending().as_deref(), Some("thrown from a helper"));
    }

    #[test]
    fn a_fatal_fault_becomes_fatal_with_its_message() {
        let mut ctx = Ctx::buffered();
        assert_eq!(call(nvs_test_fatal, &mut ctx, &[]).unwrap_err(), FATAL);
        assert_eq!(ctx.take_pending().as_deref(), Some("fatal from a helper"));
    }

    #[test]
    fn a_panic_is_contained_as_fatal_and_keeps_its_message() {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));

        let mut ctx = Ctx::buffered();
        assert_eq!(
            call(nvs_test_panics_static, &mut ctx, &[]).unwrap_err(),
            FATAL
        );
        assert_eq!(
            ctx.take_pending().as_deref(),
            Some("a static panic message")
        );

        assert_eq!(
            call(nvs_test_panics_owned, &mut ctx, &[]).unwrap_err(),
            FATAL
        );
        assert_eq!(
            ctx.take_pending().as_deref(),
            Some("an owned panic message: 7")
        );

        panic::set_hook(previous);
    }

    #[test]
    fn a_contained_panic_leaves_the_context_usable() {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));

        let mut ctx = Ctx::buffered();
        assert_eq!(
            call(nvs_test_panics_static, &mut ctx, &[]).unwrap_err(),
            FATAL
        );
        let _ = ctx.take_pending();

        panic::set_hook(previous);

        let value = call(nvs_test_identity, &mut ctx, &[Value::bool(true)]).unwrap();
        assert_eq!(value.tag(), Some(Tag::Bool));
    }

    #[test]
    fn a_helper_reaches_the_requests_own_output() {
        let mut ctx = Ctx::buffered();
        call(nvs_test_writes, &mut ctx, &[]).unwrap();
        assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b"written"[..]));
    }

    /// Runs `body` with the panic hook silenced, so a deliberately contained
    /// panic does not print a backtrace over the test output.
    fn quietly<T>(body: impl FnOnce() -> T) -> T {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));
        let value = body();
        panic::set_hook(previous);
        value
    }

    #[test]
    fn a_task_that_returns_normally_is_not_a_panic() {
        let outcome = run_task(TaskRoot::Worker, || 6 * 7);
        assert_eq!(outcome.ok(), Some(42));
    }

    #[test]
    fn a_panic_with_no_helper_beneath_it_is_contained_at_the_task_root() {
        // The whole reason this boundary exists: the accept loop and the HTTP
        // reader have no `nvs_helper!` frame under them, so the inner boundary
        // never sees their faults.
        let fault = quietly(|| run_task(TaskRoot::Worker, || panic!("the accept loop faulted")))
            .expect_err("the panic must be contained here");

        assert_eq!(fault.message(), "the accept loop faulted");
        assert_eq!(fault.root(), TaskRoot::Worker);
        assert!(
            fault.retires_worker(),
            "shared state faulted, so `rule:http-server/containment-does-not-end-at-the-helper` retires the worker"
        );
    }

    #[test]
    fn a_request_owned_panic_does_not_retire_the_worker() {
        let fault =
            quietly(|| run_task(TaskRoot::Request, || panic!("a request-owned bug: {}", 7)))
                .expect_err("the panic must be contained here");

        assert_eq!(fault.message(), "a request-owned bug: 7");
        assert!(
            !fault.retires_worker(),
            "the request owns the fault, so the worker keeps its other requests"
        );
    }

    #[test]
    fn the_helper_boundary_is_still_the_inner_one() {
        // A panic under a helper is contained *there* and reported as FATAL, so
        // the outer boundary sees an ordinary return. Nesting the two must not
        // change either answer.
        let mut ctx = Ctx::buffered();
        let status = quietly(|| {
            run_task(TaskRoot::Request, || {
                call(nvs_test_panics_static, &mut ctx, &[]).unwrap_err()
            })
        })
        .expect("the inner boundary contains it, so nothing reaches this one");

        assert_eq!(status, FATAL);
        assert_eq!(
            ctx.take_pending().as_deref(),
            Some("a static panic message")
        );
    }

    #[test]
    fn the_worker_runs_the_next_task_after_a_contained_one() {
        quietly(|| run_task(TaskRoot::Worker, || panic!("first"))).unwrap_err();
        let outcome = run_task(TaskRoot::Worker, || "second");
        assert_eq!(outcome.ok(), Some("second"));
    }

    #[test]
    fn a_bounded_loop_visits_every_item_while_the_deadline_holds() {
        let mut ctx = Ctx::buffered();
        let items = DEADLINE_POLL_BATCH * 4 + 7;
        let mut seen = 0usize;

        // Counting the *live* answers rather than the iterations also pins that
        // the context handed to the body is the request's own.
        bounded_loop(&mut ctx, "Core\\Test::sweep", 0..items, |ctx, _item| {
            seen += usize::from(!ctx.deadline_expired());
            Ok(())
        })
        .expect("no deadline was ever set, so no poll can fire");

        assert_eq!(
            seen, items,
            "a poll site must not swallow the iteration it guards"
        );
    }

    #[test]
    fn an_expired_deadline_stops_a_bounded_loop_at_the_first_batch_boundary() {
        let mut ctx = Ctx::buffered();
        ctx.expire_deadline();
        let mut seen = 0usize;

        let fault = bounded_loop(
            &mut ctx,
            "Core\\Test::sweep",
            0..DEADLINE_POLL_BATCH * 10,
            |_ctx, _item| {
                seen += 1;
                Ok(())
            },
        )
        .expect_err("the flag was already set when the loop began");

        assert_eq!(
            seen,
            DEADLINE_POLL_BATCH - 1,
            "the poll is amortised over one batch, so it fires on the first \
             boundary — not per iteration, and not only at the end"
        );
        assert!(
            matches!(&fault, Fault::Fatal(message)
                if message.contains("Core\\Test::sweep") && message.contains("deadline")),
            "a deadline is a cancellation, so `rule:concurrency/cancellation-runs-no-user-code` makes it FATAL and \
             uncatchable, and it names the member it interrupted: {fault:?}"
        );
    }

    #[test]
    fn a_run_shorter_than_one_batch_finishes_under_an_expired_deadline() {
        // The other side of the same bound, asserted deliberately: amortising
        // the poll means a loop too short to reach a batch boundary never pays
        // for one and never stops. `rule:http-server/time-is-bounded-inside-a-helper` bounds a helper's *runtime*,
        // and a run under one batch is already bounded.
        let mut ctx = Ctx::buffered();
        ctx.expire_deadline();
        let mut seen = 0usize;

        bounded_loop(
            &mut ctx,
            "Core\\Test::sweep",
            0..DEADLINE_POLL_BATCH - 1,
            |_ctx, _item| {
                seen += 1;
                Ok(())
            },
        )
        .expect("a run shorter than a batch reaches no poll site");

        assert_eq!(seen, DEADLINE_POLL_BATCH - 1);
    }

    #[test]
    fn a_fault_from_the_body_propagates_unchanged() {
        let mut ctx = Ctx::buffered();
        let mut seen = 0usize;

        let fault = bounded_loop(&mut ctx, "Core\\Test::sweep", 0..10, |_ctx, item| {
            seen += 1;
            if item == 3 {
                Err(Fault::thrown("element 3 is not an int"))
            } else {
                Ok(())
            }
        })
        .expect_err("the body refused an element");

        assert_eq!(seen, 4, "the loop stops at the element that failed");
        assert!(
            matches!(&fault, Fault::Thrown(_, message) if message == "element 3 is not an int"),
            "the combinator guards the deadline and rewrites nothing else: {fault:?}"
        );
    }
}
