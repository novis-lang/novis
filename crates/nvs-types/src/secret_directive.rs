//! `Core\Config::get`'s one decision made while compiling: a literal name that
//! is a secret directive types the call `?secret string`
//! (`rule:config/a-secret-is-a-file-whose-content-is-the-value`).
//!
//! `get` declares `?string`, and `secret` is a static qualifier the run time
//! never carries, so the value a `password_file` holds would otherwise reach
//! `echo`, a log line or a response as plain text. Which names are secret is
//! [`nvs_config::secret::is_secret_key`], the table the boot materializes
//! secrets from, so this pass holds no copy of a name.
//!
//! **Only a written name is typed.** A name the program computed cannot be
//! typed by its value, and a plain `?string` for it would be the leak. So the
//! call is also filed as [`PreparedFact::SecretDirective`], which reaches the
//! member as its argument 0, and the member throws `LogicError` for a secret
//! directive that arrives without it. A method reference reaches the member
//! with no word too, so it throws the same way.

use nvs_diagnostics::Span;
use nvs_hir::QName;
use nvs_syntax::ast::CallArgs;

use crate::Env;
use crate::defaults::ConstArg;
use crate::expr_table::{Prepared, PreparedFact};
use crate::ty::Ty;

/// The class whose `get` this is.
const OWNER: &str = "Core\\Config";

/// Whether the call is `Core\Config::get` with a literal secret directive for
/// its name, filing the fact the member reads when it is.
///
/// The hook `crate::expr::calls` reaches for a static call after the target
/// has resolved. The caller types the call `?secret string` on `true`.
pub(crate) fn types_secret(
    owner: &QName,
    member: &str,
    call_span: Span,
    args: &CallArgs,
    env: &mut Env<'_>,
) -> bool {
    if member != "get" || owner.to_string() != OWNER {
        return false;
    }
    let CallArgs::List(list) = args else {
        return false;
    };
    // `get` has one parameter, so a `name:` argument fills it as a positional
    // one does. A `...` spread is not a written name.
    let Some(arg) = list.iter().find(|arg| !arg.spread) else {
        return false;
    };
    let declared = env.interner.intern(Ty::String);
    let Some(ConstArg::Str(name)) = crate::defaults::literal_default(&arg.value, declared, env)
    else {
        return false;
    };
    if !nvs_config::secret::is_secret_key(&name) {
        return false;
    }
    env.exprs.record_prepared(
        call_span,
        Prepared {
            literal: arg.value.span,
            fact: PreparedFact::SecretDirective,
        },
    );
    true
}
