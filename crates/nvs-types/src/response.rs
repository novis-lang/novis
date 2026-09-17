//! `rule:security/response-body-is-one-typed-member`
//! 's sixth row: one response has one body writer.
//!
//! § 4 replaces `Core\Response::write` with five typed members, each owning one
//! body shape and setting its own `Content-Type`, and then makes "`echo` and a
//! typed writer on the same response" a compile error — "they disagree about
//! the body's type and its `Content-Type`, and silently letting the last one
//! win is how a JSON endpoint acquires an HTML prelude". This module is that
//! error, and it is the reason nothing at run time adjudicates between two
//! declarations: `nvs_stdlib::response`'s own doc says a body member declares
//! and then writes, with no arbitration, because the disagreement cannot reach
//! it.
//!
//! **Checked where a response is statically certain, which is a `#[Route]`
//! handler and nothing else.** § 3 binds `echo` by *context*, not by syntax:
//! the same method body writes a response body under an HTTP request and a
//! terminal sink under `nvs run`, and which one a given body runs as is a
//! run-time fact for every body but one. `rule:routing/matched-once-before-the-handler` matches a request to a
//! `#[Route]` handler and to nothing else, so a handler is a response body by
//! declaration and the rule has something to be about. Everywhere else is left
//! alone rather than refused on suspicion: a `#[Command]` method, a `#[Test]`
//! method or a `.nvst` case that writes both is a CLI program, where a body
//! member's declaration is inert (`nvs_stdlib::response`'s gap 1) and there is
//! no response for two writers to disagree over. Refusing those would refuse
//! programs that have no response to be wrong about, and the corpus already
//! pins the shared-output behaviour they rely on
//! (`tests/conformance/core/a-response-body-member-and-echo-share-one-output.nvst`).
//!
//! **Known gap: a mount's entry script.** `rule:http-server/a-request-resolves-in-five-steps`'s steps 4 and 5 run a
//! `.nvs` file's *top-level frame* as the request body, so an entry that echoes
//! a page and also calls a body member is § 4's sixth row and is not refused
//! here. The same file is one compiled unit whether the server ran it or
//! `nvs run` did, so a static refusal there would refuse the CLI use of every
//! such file — the missing fact is a declaration that a file is an entry, which
//! the language does not have. Until it does, the entry-script half of § 4's
//! row is answered by § 3's default alone: `echo` means `text/html`, and a body
//! member written beside it wins the `Content-Type` it declared last.
//! Decided: Keep the run-time default (`echo` is text/html, the last body member wins) — No new
//! surface, and the mixed case stays legal with defined behaviour.
//! — owner: decided-closures
//!
//! **The reach inside a handler is that handler's own body**, closures written
//! in it included — an `fn` literal writes the same response, and
//! [`crate::expr::calls::check_fn_literal`] checks its body inline, so the two
//! writers meet here with nothing added. A helper *method* the handler calls is
//! not seen: which bodies reach a handler is a whole-program question and this
//! is a body-local rule, deliberately, for [`crate::links`]'s opposite reason —
//! a rule that needs the call graph cannot be answered where it is written.
//!
//! **Two typed members in one handler are refused here too, and one member
//! called twice is not.** What the rule says is that a body has one writer, so
//! the disagreement needs two *different* writers to exist: `text` then `json`
//! declares the body's type twice over, while `text` then `text` declares one
//! thing twice and is a handler writing its whole body one way. The corpus
//! depends on that second half — a body member written in a loop is an
//! ordinary program — so the roster is compared by the writer's name and never
//! by the call site.
//!
//! **The roster spans two classes.** A body written over time has a spelling
//! on each of `rule:concurrency/two-doors-one-isolate`'s doors, and
//! `Core\Response::stream` and `Core\Sse::stream` open the same cell and
//! declare a content type each — so `echo` beside either is the same
//! disagreement, and the two of them together are two writers of one body.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::MethodMember;

use crate::{Ctx, Env};

/// Every member that writes the body, as the `Core` class it is spelled on and
/// the name a call spells.
///
/// `rule:security/response-body-is-one-typed-member`'s table first, in its own
/// order, and every row of it is a registered member of
/// `nvs_stdlib::registry`'s `Core\Response`. A row here costs nothing while a
/// member is being landed either way: an unregistered name is reported as
/// `E0405` *beside* this refusal rather than instead of it.
///
/// The two `stream` rows are not that table's: a body written over time is
/// `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
/// subject, and it has one spelling on each of
/// `rule:concurrency/two-doors-one-isolate`'s doors. Both belong here for this
/// rule's own reason — each writes the body and declares a content type, so
/// `echo` beside either is the same disagreement about what the response
/// carries, reached through the machinery already here rather than through a
/// second rule that would have to agree with this one. The class is carried
/// beside the name because of them: every other row is a `Core\Response`
/// member, and a name list could not tell `Core\Sse::stream` from an
/// unrelated `stream` on some other `Core` class.
const BODY_MEMBERS: [(&str, &str); 7] = [
    ("Response", "html"),
    ("Response", "json"),
    ("Response", "text"),
    ("Response", "bytes"),
    ("Response", "sendFile"),
    ("Response", "stream"),
    ("Sse", "stream"),
];

/// What has written the body of the body being checked, so far.
///
/// One of these is installed per method body by [`entering_body`] and put back
/// afterwards by [`crate::check`]. `armed` is the module doc's scope decision
/// held as a field rather than asked at each site: whether a body is a response
/// is a question about the *declaration*, and asking it once at the top is what
/// keeps the two note sites free of it.
#[derive(Default)]
pub(crate) struct BodyWriters {
    /// Whether this body is a `#[Route]` handler's, and so has a response.
    armed: bool,
    /// The first `echo` statement in it.
    echo: Option<Span>,
    /// The first body-writing member call in it, and the label naming it —
    /// the label rather than the bare name, because two writers are told apart
    /// by which member they are and the roster spans two classes.
    member: Option<(Span, String)>,
    /// Whether the conflict has already been reported for this body. One report
    /// per body: a handler that echoes in a loop and declares a body once has
    /// made one mistake, and a diagnostic per `echo` describes it no better.
    reported: bool,
}

/// The state to install for `m`'s body — armed for a `#[Route]` handler, inert
/// for every other method.
///
/// Reads the attribute through [`crate::testing::attribute_named`], which is
/// the nominal match [`crate::routes`] uses for the same question, so a
/// userland `#[Route]` that does not resolve to `Core\Route` arms nothing here
/// for the same reason it contributes no route there.
pub(crate) fn entering_body(m: &MethodMember, ctx: &Ctx<'_>, env: &Env<'_>) -> BodyWriters {
    BodyWriters {
        armed: crate::testing::attribute_named(&m.attributes, crate::derive::ROUTE, ctx, env)
            .is_some(),
        ..BodyWriters::default()
    }
}

/// An `echo` statement in the body being checked.
///
/// Called from [`crate::locals::check_stmt`]'s `Echo` arm with the statement's
/// own span, which is what the label points at: the operand is where `rule:security/secret-sinks-refuse`
/// 's secret refusal points, because there the *value* is the mistake, and
/// here the writer is.
pub(crate) fn note_echo(span: Span, env: &mut Env<'_>) {
    if !env.body_writers.armed || env.body_writers.reported {
        return;
    }
    if let Some((member_span, member)) = env.body_writers.member.clone() {
        report(span, "`echo`", member_span, &member, env);
        return;
    }
    env.body_writers.echo.get_or_insert(span);
}

/// A resolved `Core\Class::member(...)` call in the body being checked.
///
/// Called from [`crate::expr::calls::infer_static_call`] for every static call
/// whose class side resolved, rather than only for `Core` ones: the class test
/// is one comparison and keeping it here is what makes `rule:security/response-body-is-one-typed-member`'s roster
/// readable in one place.
pub(crate) fn note_body_member(qname: &QName, member: &str, span: Span, env: &mut Env<'_>) {
    if !env.body_writers.armed || env.body_writers.reported {
        return;
    }
    let Some(label) = body_member_label(qname, member) else {
        return;
    };
    if let Some(echo) = env.body_writers.echo {
        report(span, &label, echo, "`echo`", env);
        return;
    }
    if let Some((first, first_label)) = env.body_writers.member.clone() {
        // Two *different* writers of one body, which the module doc separates
        // from one writer called twice: the same member declaring the same
        // content type again has said nothing new, and a handler writing its
        // body in a loop is an ordinary program.
        if first_label != label {
            report(span, &label, first, &first_label, env);
        }
        return;
    }
    env.body_writers.member = Some((span, label));
}

/// `` `Core\Response::json` ``, quoted for a message — `None` where
/// `qname::member` is not one of [`BODY_MEMBERS`].
fn body_member_label(qname: &QName, member: &str) -> Option<String> {
    if !qname.is_core() || qname.segments().len() != 2 {
        return None;
    }
    let class = qname.short_name();
    BODY_MEMBERS
        .iter()
        .any(|(owner, name)| *owner == class && *name == member)
        .then(|| format!("`Core\\{class}::{member}`"))
}

/// The refusal, pointing at the writer that made the body ambiguous and at the
/// one that was already there.
///
/// The *second* writer takes the primary span because it is the one whose
/// removal leaves a body with a single type — the first is a complete program
/// on its own — and both are named in the message so a reader who sees only the
/// first line knows which two writers are meant.
fn report(second: Span, second_label: &str, first: Span, first_label: &str, env: &mut Env<'_>) {
    env.body_writers.reported = true;
    env.diags.report(
        Diagnostic::error(
            code::E_ECHO_BESIDE_A_BODY_MEMBER,
            format!(
                "a response has one body, and this handler writes it with both {first_label} \
                 and {second_label}"
            ),
        )
        .with_primary(second, format!("{second_label} writes the body here"))
        .with_secondary(first, format!("{first_label} already wrote it here"))
        .with_help(
            "write the whole body one way: `echo` alone is the inline-HTML page and means \
             `text/html`, and a body member sets its own `Content-Type` for everything the \
             response carries",
        ),
    );
}
