//! [ADR 0118] § 2's check, and § 5's refusal: the one function a door to the operating system calls
//! before it opens.
//!
//! [`require`] is deliberately the *only* thing here. The decision procedure is
//! [`nvs_config::capability`] and is pure; this is the half that knows about a request — where the
//! snapshot comes from, and what a denial looks like to the program that hit it. The doors themselves
//! (`open_read`, `connect`, `exec`) arrive with the first `Core` member that needs one; each of them
//! calls this first, which is what makes § 2's claim structural rather than a convention: a member
//! reaches the OS through a door or not at all, and every door has already asked.
//!
//! **A context with no configuration grants nothing.** That is not a special case for tests — it is the
//! same deny-by-default the absent block gets, and a request path that reached a capability check
//! without a snapshot has a bug that should fail closed rather than quietly succeed.
//!
//! [ADR 0118]: ../../../docs/adr/0118-a-capability-is-checked-at-the-door-to-the-effect.md

use nvs_config::capability::{Cap, Scope};

use crate::Fault;
use crate::ctx::Ctx;

/// § 1's question, asked of `ctx`'s own snapshot, and § 5's `RuntimeError` when the answer is no.
///
/// `member` is what the message names as the thing that wanted the capability — `Core\File::write`,
/// or `spawn script` for the language construct, which is not a `Core` member at all and is checked
/// through the same function for exactly that reason.
///
/// # Errors
///
/// [`Fault::thrown`] — a `RuntimeError`, catchable, naming the capability in the spelling `nvs.toml`
/// grants it under and, for a scoped check, the argument that fell outside the grant. It is never a
/// `FATAL`: a denial is known before any work is done and leaves nothing behind, so a program that
/// degrades when a capability is missing is a reasonable program (ADR 0118 § 5).
pub fn require(ctx: &Ctx, cap: Cap, scope: Scope<'_>, member: &str) -> Result<(), Fault> {
    let granted = ctx.config().is_some_and(|config| {
        config
            .snapshot()
            .config
            .capabilities
            .as_ref()
            .is_some_and(|caps| caps.allows(cap, scope, &nvs_config::resolve::Disk))
    });
    if granted {
        return Ok(());
    }
    Err(Fault::thrown(denial(cap, scope, member)))
}

/// § 5's message. The capability's name comes first after the member because the reader is usually
/// the operator, and that string is what they are about to paste into a configuration file.
fn denial(cap: Cap, scope: Scope<'_>, member: &str) -> String {
    let name = cap.name();
    match scope {
        Scope::Unscoped => format!("{member} needs the capability `{name}`, which is not granted"),
        Scope::Path(path) => format!(
            "{member} needs the capability `{name}` for {}, which is not granted",
            path.display()
        ),
        Scope::Host(host) | Scope::Name(host) => {
            format!("{member} needs the capability `{name}` for {host}, which is not granted")
        }
    }
}
