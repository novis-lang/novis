//! `rule:core-classes/html-to-source`'s second sentence: the `Core` members whose written **reason** has to be
//! in the source, and the hook that refuses one that is not.
//!
//! The sibling of [`crate::intrinsics`], [`crate::links`], [`crate::retrieval`]
//! and [`crate::program`] — a `Core` call the compiler knows by name — reached
//! from the same two places in [`crate::expr::calls`] and matched the same way,
//! nominally against the *declaring* class rather than against what the site
//! spelled.
//!
//! # What is decided here
//!
//! * **It is the one pass that refuses an argument for being dynamic**, which
//!   is exactly what [`crate::intrinsics`] § 4 forbids itself. The two look
//!   alike and are opposites: an intrinsic reads a literal where there is one
//!   and says nothing where there is not, because the runtime would still have
//!   accepted it. A reason is the other case — the runtime accepts *any*
//!   string, and the whole content of § 3 is that a human wrote this one — so
//!   the rule cannot be one the runtime also makes, and lives here rather than
//!   on that roster.
//! * **A `const` is a source literal.** The test is that the argument *folds*,
//!   through the decoders every other compile-time read of a string uses, so
//!   `const string WHY = "…";` passes and `$why` does not — see [`folded_str`],
//!   which is the one place the two are joined. That is the shape the hatch
//!   wants: a long justification
//!   pulled out to a named constant is still greppable and still at the site
//!   in every way a reader cares about, which is `Core\Secret::reveal`'s own
//!   position on the same question.
//! * **An empty reason is deliberately not this pass's.** `rule:core-classes/html-to-source` refuses
//!   one and the body already throws on it at run time (`nvs_stdlib::html`),
//!   which `tests/conformance/core/to-source-refuses-an-empty-reason-and-takes-any-written-one.nvst`
//!   pins *as a throw*. Refusing it here would make that throw unreachable for
//!   every reason this pass can read — a corpus case traded for an earlier
//!   answer to a question nobody is waiting on. This pass is about where a
//!   reason **is**, not about what it says.
//! * **The roster is closed and each row owes an ADR section.** One row today.
//!   `Core\Taint::assertTrusted` and `Core\Secret::reveal` are the obvious
//!   neighbours and are deliberately *not* added on the strength of the
//!   resemblance: each is its own ADR's decision to make, and widening this to
//!   "every parameter named `$reason`" is the open extension point `rule:expressions/intrinsic-list-is-closed`
//!   refuses for the sibling table.
//! * **The reason is found through the parameter it fills**, which
//!   [`crate::expr::args::check_args_typed`] already worked out, so `reason:`
//!   is read wherever it was written and the roster carries no second spelling
//!   of a name the registry publishes
//!   (`rule:core-api/parameters-are-callable-by-name`). A
//!   `Core\Html::toSource(...$pair)` fills the parameter with nothing this
//!   pass can see — and needs no answer here, because a `...` into a member
//!   that declares no variadic tail is refused where it is mapped.

use nvs_diagnostics::{Diagnostic, code};
use nvs_hir::QName;
use nvs_syntax::ast::{CallArgs, Expr};

use crate::defaults::ConstArg;
use crate::expr::args::argument_filling;
use crate::expr_table::ArgSlot;
use crate::{Ctx, Env};

/// One row: a member, and which of its parameters is the written reason.
struct Reason {
    /// The *declaring* class, fully qualified.
    owner: &'static str,
    /// The member's own name.
    member: &'static str,
    /// The parameter holding the reason — a receiver is not one, exactly as
    /// [`crate::intrinsics`]' `at` does not count one. The call's own mapping
    /// turns it into the written argument, so a `reason:` written anywhere
    /// fills it.
    at: usize,
}

/// `rule:core-classes/html-to-source`'s one member, and every future row that earns its own section.
static REASONS: &[Reason] = &[Reason {
    owner: r"Core\Html",
    member: "toSource",
    at: 1,
}];

/// Refuses a written reason that is not a source literal — the hook both call
/// sites in [`crate::expr::calls`] reach beside [`crate::intrinsics::check_call`],
/// after the target has resolved.
pub(crate) fn check_call(
    owner: &QName,
    member: &str,
    args: &CallArgs,
    slots: &[ArgSlot],
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    // The member name first, and the owner only after it matches. This runs at
    // every resolved call in the program and `QName` has no borrowed spelling
    // to compare against, so ordering the test this way is the difference
    // between one allocation per call site and one per `toSource`.
    if !REASONS.iter().any(|row| row.member == member) {
        return;
    }
    let owner = owner.to_string();
    let Some(row) = REASONS
        .iter()
        .find(|row| row.owner == owner && row.member == member)
    else {
        return;
    };
    let CallArgs::List(list) = args else {
        return;
    };
    let Some(written) = argument_filling(row.at, list, slots).map(|arg| &arg.value) else {
        // Nothing fills the reason's parameter: a call too short to have
        // written it, which is the arity check's refusal and already made, or
        // a `...` this member has no variadic tail for, refused where it was
        // mapped.
        return;
    };
    let span = written.span;
    if matches!(folded_str(written, ctx, env), Some(ConstArg::Str(_))) {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_REASON_NOT_A_SOURCE_LITERAL,
            format!("`{owner}::{member}`'s reason must be a source literal"),
        )
        .with_primary(span, "written here")
        .with_help(
            "`rule:core-classes/html-to-source`: this is an escape hatch, and what makes it safe is that it is rare, \
             greppable and carries a justification a reader can see — a reason that can be \
             computed is a reason nobody wrote. A `const` holding the text is a source literal \
             and compiles",
        ),
    );
}

/// One expression folded as a `string`: the literal decoder
/// [`crate::intrinsics`] and [`crate::links`] both reach, **and** the class
/// constant they do not.
///
/// The second half is the whole of what "a `const` is a source literal" means,
/// and it is [`crate::defaults::fold_const_reference`] rather than the
/// signature-collection sibling: this runs in the checking pass, so the
/// constant is in hand with the value a *read* of it would inline, which is
/// what makes the reason a reader greps for and the reason that compiles the
/// same text.
fn folded_str(expr: &Expr, ctx: &Ctx<'_>, env: &mut Env<'_>) -> Option<ConstArg> {
    let declared = env.interner.intern(crate::ty::Ty::String);
    crate::defaults::literal_default(expr, declared, env)
        .or_else(|| crate::defaults::fold_const_reference(expr, ctx, env))
}
