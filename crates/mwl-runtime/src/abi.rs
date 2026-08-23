//! [ADR 0002](../../../docs/adr/0002-error-propagation.md)'s calling
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
//! Both halves are supplied by [`mwl_helper!`] rather than left to a
//! convention, for the reason that ADR's *Corollary* gives: `extern "C"`
//! rather than `extern "C-unwind"` and the `catch_unwind` wrapper are each a
//! silent, per-helper correctness cliff, and a macro cannot forget either.

use std::panic::{self, AssertUnwindSafe};

use crate::ctx::Ctx;
use crate::value::Value;

/// Success. The result has been written through the `out` pointer.
pub const OK: i32 = 0;

/// An MWL-level exception is pending in [`Ctx`]. A `catch` may handle it.
pub const THROWN: i32 = 1;

/// Unrecoverable: a resource limit, or an internal error caught at a helper
/// boundary. Propagates to the request boundary, and MWL code cannot catch it
/// ([ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)).
pub const FATAL: i32 = 2;

/// A compiled MWL function.
///
/// `unsafe` because the three pointers carry a contract the type cannot
/// express: each must be non-null, aligned and valid for the duration of the
/// call, `args` must point at as many values as the callee's arity, and `out`
/// must be writable. Compiled code satisfies this by construction; hand
/// callers go through [`call`].
pub type MwlFn = unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32;

/// A runtime helper. Identical to [`MwlFn`] — that identity is the point of
/// having one convention.
pub type HelperFn = MwlFn;

/// How a helper body reports a failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum Fault {
    /// An MWL exception a `catch` may handle; becomes [`THROWN`].
    Thrown(std::borrow::Cow<'static, str>),
    /// Unrecoverable; becomes [`FATAL`].
    Fatal(std::borrow::Cow<'static, str>),
    /// A callee this helper invoked already failed and already recorded what
    /// failed in [`Ctx`] — return its status unchanged.
    ///
    /// The one variant carrying no message, and deliberately: a
    /// [`Self::Thrown`] built here would call [`Ctx::set_pending`] a second
    /// time and replace the exception object the callee raised with a bare
    /// string. Reached today only from [`crate::call_closure`], which is the
    /// one place a helper calls compiled MWL code.
    Pending(i32),
}

impl Fault {
    /// A [`Fault::Thrown`] with a message.
    #[must_use]
    pub fn thrown(message: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        Self::Thrown(message.into())
    }

    /// A [`Fault::Fatal`] with a message.
    #[must_use]
    pub fn fatal(message: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        Self::Fatal(message.into())
    }
}

/// What a helper body returns: the result value, or a [`Fault`].
///
/// A helper invoked purely for its effect — `mwl_ir::Helper::EchoStr` is the
/// only one so far — returns [`Value::null`].
pub type HelperResult = Result<Value, Fault>;

/// The body of every helper, shared so [`mwl_helper!`] expands to one line.
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
              discharged; every helper reaches it through `mwl_helper!`"
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
        Ok(Err(Fault::Thrown(message))) => {
            ctx.set_pending(message);
            THROWN
        }
        Ok(Err(Fault::Fatal(message))) => {
            ctx.set_pending(message);
            FATAL
        }
        // Nothing to record: the callee that failed already did, and this
        // frame has nothing of its own to add — see `Fault::Pending`.
        Ok(Err(Fault::Pending(status))) => status,
        Err(payload) => {
            ctx.set_pending(panic_message(&*payload));
            FATAL
        }
    }
}

/// Recovers a panic's message, so a contained bug says what it was.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> std::borrow::Cow<'static, str> {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        std::borrow::Cow::Borrowed(*message)
    } else if let Some(message) = payload.downcast_ref::<String>() {
        std::borrow::Cow::Owned(message.clone())
    } else {
        std::borrow::Cow::Borrowed("internal error: a runtime helper panicked")
    }
}

/// Defines a runtime helper with MWL's calling convention.
///
/// The body is written against safe references — `&mut Ctx` and a `&[Value]`
/// of exactly the declared arity — and returns a [`HelperResult`]. Everything
/// else (the `extern "C"` signature, the `catch_unwind` wrapper, writing the
/// result through `out`, recording a message in [`Ctx`]) is supplied.
///
/// ```
/// use mwl_runtime::{Ctx, Fault, Value, call, mwl_helper};
///
/// mwl_helper! {
///     /// Doubles an `int`.
///     fn mwl_double(_ctx, args: [1]) {
///         let n = args[0].as_int().ok_or_else(|| Fault::fatal("not an int"))?;
///         Ok(Value::int(n * 2))
///     }
/// }
///
/// let mut ctx = Ctx::buffered();
/// assert_eq!(call(mwl_double, &mut ctx, &[Value::int(21)]).unwrap().as_int(), Some(42));
/// ```
#[macro_export]
macro_rules! mwl_helper {
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
        /// this helper's arity; and `out` must be writable. Compiled MWL code
        /// satisfies this by construction — hand callers go through
        /// [`call`](crate::call).
        #[allow(
            unsafe_code,
            reason = "a runtime helper's signature is fixed by ADR 0002; the \
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
/// The safe wrapper hand callers — this crate's tests, `mwl-codegen`'s tests,
/// and eventually `mwl run` itself — use instead of building the three
/// pointers by hand.
///
/// # Errors
///
/// The [`THROWN`] or [`FATAL`] status, with the message left in `ctx` for the
/// caller to [`Ctx::take_pending`].
pub fn call(function: MwlFn, ctx: &mut Ctx, args: &[Value]) -> Result<Value, i32> {
    let mut out = Value::null();
    #[expect(
        unsafe_code,
        reason = "every pointer here is derived from a live Rust reference, and \
                  `args.len()` is by construction the arity the callee is being \
                  asked for"
    )]
    let status = unsafe { function(&raw mut *ctx, args.as_ptr(), &raw mut out) };
    if status == OK { Ok(out) } else { Err(status) }
}

#[cfg(test)]
mod tests {
    // `mwl_helper!` emits `pub` entry points; inside this private test module
    // they are deliberately unreachable from outside the crate.
    #![allow(unreachable_pub)]

    use super::*;
    use crate::value::Tag;

    mwl_helper! {
        /// Returns its one argument unchanged.
        fn mwl_test_identity(_ctx, args: [1]) {
            Ok(args[0])
        }
    }

    mwl_helper! {
        /// Reports whether it was handed an empty argument slice.
        fn mwl_test_arity_zero(_ctx, args: [0]) {
            Ok(Value::bool(args.is_empty()))
        }
    }

    mwl_helper! {
        /// Always throws.
        fn mwl_test_throws(_ctx, _args: [0]) {
            Err(Fault::thrown("thrown from a helper"))
        }
    }

    mwl_helper! {
        /// Always fatal.
        fn mwl_test_fatal(_ctx, _args: [0]) {
            Err(Fault::fatal("fatal from a helper"))
        }
    }

    mwl_helper! {
        /// Panics with a `&'static str`.
        fn mwl_test_panics_static(_ctx, _args: [0]) {
            panic!("a static panic message")
        }
    }

    mwl_helper! {
        /// Panics with an owned `String`.
        fn mwl_test_panics_owned(_ctx, _args: [0]) {
            panic!("an owned panic message: {}", 7)
        }
    }

    mwl_helper! {
        /// Writes to the request's output, proving the context is reachable.
        fn mwl_test_writes(ctx, _args: [0]) {
            ctx.write_output(b"written").map_err(|e| Fault::fatal(e.to_string()))?;
            Ok(Value::null())
        }
    }

    #[test]
    fn a_helper_returns_its_result_through_out() {
        let mut ctx = Ctx::buffered();
        let value = call(mwl_test_identity, &mut ctx, &[Value::int(5)]).unwrap();
        assert_eq!(value.as_int(), Some(5));
        assert_eq!(ctx.pending(), None);
    }

    #[test]
    fn a_zero_arity_helper_gets_an_empty_slice_not_a_null_deref() {
        let mut ctx = Ctx::buffered();
        let value = call(mwl_test_arity_zero, &mut ctx, &[]).unwrap();
        assert_eq!(value.as_bool(), Some(true));
    }

    #[test]
    fn a_thrown_fault_becomes_thrown_with_its_message() {
        let mut ctx = Ctx::buffered();
        assert_eq!(call(mwl_test_throws, &mut ctx, &[]).unwrap_err(), THROWN);
        assert_eq!(ctx.take_pending().as_deref(), Some("thrown from a helper"));
    }

    #[test]
    fn a_fatal_fault_becomes_fatal_with_its_message() {
        let mut ctx = Ctx::buffered();
        assert_eq!(call(mwl_test_fatal, &mut ctx, &[]).unwrap_err(), FATAL);
        assert_eq!(ctx.take_pending().as_deref(), Some("fatal from a helper"));
    }

    #[test]
    fn a_panic_is_contained_as_fatal_and_keeps_its_message() {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));

        let mut ctx = Ctx::buffered();
        assert_eq!(
            call(mwl_test_panics_static, &mut ctx, &[]).unwrap_err(),
            FATAL
        );
        assert_eq!(
            ctx.take_pending().as_deref(),
            Some("a static panic message")
        );

        assert_eq!(
            call(mwl_test_panics_owned, &mut ctx, &[]).unwrap_err(),
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
            call(mwl_test_panics_static, &mut ctx, &[]).unwrap_err(),
            FATAL
        );
        let _ = ctx.take_pending();

        panic::set_hook(previous);

        let value = call(mwl_test_identity, &mut ctx, &[Value::bool(true)]).unwrap();
        assert_eq!(value.tag(), Some(Tag::Bool));
    }

    #[test]
    fn a_helper_reaches_the_requests_own_output() {
        let mut ctx = Ctx::buffered();
        call(mwl_test_writes, &mut ctx, &[]).unwrap();
        assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b"written"[..]));
    }
}
