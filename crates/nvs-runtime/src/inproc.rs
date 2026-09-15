//! `rule:testing/in-process-request`'s
//! in-process request: the seam `Core\Test::request` reaches the unit under
//! test through.
//!
//! § 18 asks for a request that runs through
//! `rule:routing/routes-are-compiled-not-registered`'s compiled table and the
//! real middleware chain with **no socket and no port**. Everything that needs
//! is on the two sides of this seam and on neither one alone: the carrier, the
//! table and the response are all `nvs-runtime`'s, while the *program* a
//! synthetic request is answered by is a compile product only the binary
//! holding the front end has — the same asymmetry [`crate::script`] exists for,
//! and this module is that module's shape applied to a second question.
//!
//! # What crosses, and what deliberately does not
//!
//! The implementor is handed one [`Inbound`] and answers with one
//! [`crate::host::Completion`] — the same value a `spawn script` is collected
//! as, because an answered request *is* an isolate that ended, and inventing a
//! second result type would be inventing a second reading of what a child that
//! threw said. What does not cross is the unit: a `Core` member may not hold a
//! `nvs_codegen::Unit`, that crate being above this one, so the implementor
//! runs the isolate on its own side and hands back only the answer.
//!
//! **The door refuses re-entry**, and that is a rule rather than a guard
//! against a test that loops. What answers a synthetic request is the entry the
//! application is served by, whose top-level statements are exactly what a
//! served request runs; a program whose top level calls `Core\Test::request`
//! would therefore answer its own request with another one, forever. `rule:errors/on-limit`'s script-depth ceiling would eventually stop it, but a depth breach
//! reports as an engine limit rather than as the mistake it is, so
//! [`answer`] refuses the second one at the door and says so. The same refusal
//! is what keeps a *served* request from making one: a request already being
//! answered has its own response, and a second one inside it would have nowhere
//! to go.
//!
//! **What it spends:** one pointer in a thread-local for the length of a run,
//! and one `Inbound` per call, released with the child's context. Nothing is
//! held between two calls.

use std::cell::Cell;

use crate::host::Completion;
use crate::{Ctx, Inbound};

/// Whatever holds the unit a synthetic request is answered by.
///
/// Exactly one implementor is expected per binary that can compile a program —
/// `nvs-cli`'s test runner — and there is none in a served process, which is
/// [`answer`]'s [`AnswerError::NoUnit`] and is the fail-closed direction: a
/// member that could dispatch inside a live server would be a second door onto
/// the application, reachable from application code.
///
/// `Debug` is a supertrait for [`Installed`]'s derive, exactly as
/// [`crate::script::Resolver`]'s is.
pub trait Answering: std::fmt::Debug {
    /// Runs one request through the unit under test and answers with what
    /// crossed back.
    ///
    /// `ctx` is the **caller's**, borrowed for the length of the call: the
    /// child's own context is built on the other side of this seam and is
    /// nothing the caller can reach, exactly as
    /// [`crate::host::Host::start_isolate`]'s is. The call suspends the calling
    /// task while the child runs, so control does not leave it with a request
    /// still being answered.
    ///
    /// **`rule:routing/matched-once-before-the-handler`
    /// 's match is the implementor's**, taken on `inbound` before the child
    /// starts. It is on this side because the table is a compile product of the
    /// unit the implementor holds, and the *caller's* context need not have one
    /// at all: a `#[Test]` method runs in an isolate of its own, which shares
    /// compiled code with the unit under test and nothing else — so a member
    /// matching against its own context's table would match against nothing and
    /// hand every request a `null` route. § 1's "once, before any application
    /// code runs" still holds, and this is the one side that can honour it.
    ///
    /// # Errors
    ///
    /// The message a refusal is worded with — a program that would not compile,
    /// a child that could not be started. A child that *ran* and threw is not
    /// an error here: it is a [`Completion`] with `ok` false, which is
    /// `rule:security/isolate-shares-nothing`'s failure-is-a-value
    /// and is what a test asserting a handler's own error path reads.
    fn answer(&self, ctx: &mut Ctx, inbound: Box<Inbound>) -> Result<Completion, String>;
}

/// Why [`answer`] has no completion to hand back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnswerError {
    /// Nothing is installed on this thread — the caller is not inside a run
    /// that holds a unit under test.
    NoUnit,
    /// The caller is already answering a request, which the module doc's
    /// *the door refuses re-entry* paragraph is the whole of.
    Reentrant,
    /// The implementor refused, carrying its own message.
    Refused(String),
}

impl std::fmt::Display for AnswerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoUnit => formatter.write_str(
                "there is no unit under test here — an in-process request is answered by the \
                 program a `nvs test` or `nvs run` invocation compiled, and this run has none",
            ),
            Self::Reentrant => formatter.write_str(
                "an in-process request may not be made from inside one — the program answering \
                 it is the same program that asked, so a second request would answer itself",
            ),
            Self::Refused(message) => formatter.write_str(message),
        }
    }
}

thread_local! {
    /// The unit under test on this thread, or `None` when there is none.
    ///
    /// A `Cell` of a `Copy` wide pointer for [`crate::host`]'s reason: it is
    /// `const`-initialized and carries no destructor, which is this crate's
    /// rule for a thread-local rather than a preference.
    static CURRENT: Cell<Option<&'static dyn Answering>> = const { Cell::new(None) };
}

/// Restores whatever was installed before, when dropped.
#[derive(Debug)]
#[must_use = "the unit under test is uninstalled the moment this guard is dropped"]
pub struct Installed {
    previous: Option<&'static dyn Answering>,
}

impl Drop for Installed {
    fn drop(&mut self) {
        CURRENT.with(|slot| slot.set(self.previous.take()));
    }
}

/// Publishes `unit` as this thread's unit under test for as long as the guard
/// lives.
///
/// Nesting is restoration, not replacement — [`crate::script::install`]'s rule
/// and for its reason.
pub fn install(unit: &'static dyn Answering) -> Installed {
    let previous = CURRENT.with(|slot| slot.replace(Some(unit)));
    Installed { previous }
}

/// Publishes `unit` for the duration of `run`, and takes it back down however
/// `run` ends.
///
/// The shape a runner whose unit is **not** a `static` wants, and it exists for
/// [`crate::script::scoped`]'s reason: an implementor holds an `Rc`-shared
/// compiled unit, so it is `!Sync` and cannot live in a `static`, and leaking
/// one per process is a definite loss a valgrind sweep then reports.
pub fn scoped<R>(unit: &(dyn Answering + 'static), run: impl FnOnce() -> R) -> R {
    #[expect(
        unsafe_code,
        reason = "the widened reference is published into `CURRENT` and nowhere \
                  else, and `installed`'s `Drop` clears it before this function \
                  returns on either edge -- so it is never readable after \
                  `unit`'s own borrow ends. Nothing reachable from `run` can \
                  carry it back out: `answer` hands back a `Completion`, which \
                  owns its bytes and borrows nothing. A nested `install` inside \
                  `run` is sound however its guard is dropped or forgotten, \
                  because putting *this* one back is what `installed` does \
                  regardless"
    )]
    // SAFETY: as the reason above states.
    let widened: &'static dyn Answering = unsafe { &*std::ptr::from_ref(unit) };
    let installed = install(widened);
    let answer = run();
    drop(installed);
    answer
}

/// Runs `inbound` through this thread's unit under test.
///
/// One call rather than a `with_current` every caller then unwraps twice: there
/// is exactly one thing anybody does with a unit under test, and every way of
/// not getting a completion is an [`AnswerError`] variant.
///
/// **This is where re-entry is refused**, ahead of asking whether anything is
/// installed at all, because the two answers would otherwise arrive in the
/// wrong order for the caller that hits both: a served request has no unit
/// under test *and* is already answering one, and the second is the fact that
/// explains it.
///
/// # Errors
///
/// [`AnswerError::Reentrant`] when `ctx` is already answering a request,
/// [`AnswerError::NoUnit`] when nothing is installed here, and
/// [`AnswerError::Refused`] carrying the implementor's own message otherwise.
pub fn answer(ctx: &mut Ctx, inbound: Box<Inbound>) -> Result<Completion, AnswerError> {
    if ctx.inbound().is_some() {
        return Err(AnswerError::Reentrant);
    }
    let Some(unit) = CURRENT.with(Cell::get) else {
        return Err(AnswerError::NoUnit);
    };
    unit.answer(ctx, inbound).map_err(AnswerError::Refused)
}

/// Whether this thread has a unit under test, without running anything through
/// it.
#[must_use]
pub fn is_installed() -> bool {
    CURRENT.with(|slot| slot.get().is_some())
}

#[cfg(test)]
mod tests {
    use super::{AnswerError, Answering, Completion, Ctx, Inbound, answer, is_installed, scoped};

    /// An implementor that answers every request with the same completion, so
    /// the tests below are about the seam and never about a program.
    #[derive(Debug)]
    struct Fixed(u16);

    impl Answering for Fixed {
        fn answer(&self, _ctx: &mut Ctx, inbound: Box<Inbound>) -> Result<Completion, String> {
            Ok(Completion {
                ok: true,
                value: crate::Value::null(),
                output: inbound.path().as_bytes().to_vec(),
                content_type: None,
                file_body: None,
                status: Some(self.0),
                headers: Vec::new(),
                error: None,
                wall: None,
                trace: Vec::new(),
            })
        }
    }

    #[test]
    fn nothing_is_installed_until_a_scope_opens_one() {
        assert!(
            !is_installed(),
            "a bare thread already had a unit under test"
        );
        scoped(&Fixed(200), || {
            assert!(is_installed(), "a scope did not publish its unit");
        });
        assert!(!is_installed(), "a closed scope left its unit installed");
    }

    #[test]
    fn a_request_reaches_the_installed_unit_and_its_answer_comes_back() {
        let mut ctx = Ctx::new(crate::OutputSink::Sink);
        let completion = scoped(&Fixed(201), || {
            answer(&mut ctx, Box::new(Inbound::new("GET", "/users/1", "")))
        })
        .expect("the installed unit refused a request");
        assert_eq!(
            completion.status,
            Some(201),
            "the unit's status did not cross"
        );
        assert_eq!(
            completion.output, b"/users/1",
            "the unit was handed a different path from the one asked for"
        );
    }

    #[test]
    fn a_run_with_no_unit_under_test_is_told_so_rather_than_panicking() {
        let mut ctx = Ctx::new(crate::OutputSink::Sink);
        let refused = answer(&mut ctx, Box::new(Inbound::new("GET", "/", "")));
        assert_eq!(refused.unwrap_err(), AnswerError::NoUnit);
    }

    #[test]
    fn a_context_already_answering_a_request_is_refused_before_the_unit_is_asked() {
        let mut ctx = Ctx::new(crate::OutputSink::Sink);
        ctx.set_inbound(Inbound::new("GET", "/served", ""));
        let refused = scoped(&Fixed(200), || {
            answer(&mut ctx, Box::new(Inbound::new("GET", "/again", "")))
        });
        assert_eq!(
            refused.unwrap_err(),
            AnswerError::Reentrant,
            "a request inside a request was answered instead of refused"
        );
    }
}
