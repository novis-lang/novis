//! [ADR 0118] § 2's check, and § 5's refusal: the one function a door to the operating system calls
//! before it opens.
//!
//! [`require`] is deliberately the only *decision* here. The decision procedure is
//! [`nvs_config::capability`] and is pure; this is the half that knows about a request — where the
//! snapshot comes from, and what a denial looks like to the program that hit it. [`open_read`] and
//! [`write()`] are the first two of § 2's doors and the rest (`connect`, `exec`) arrive with the first
//! `Core` member that needs one; each of them calls [`require`] before it names a spelling that
//! performs the effect, which is what makes § 2's claim structural rather than a convention: a member
//! reaches the OS through a door or not at all, and every door has already asked.
//!
//! **A context with no configuration grants nothing.** That is not a special case for tests — it is the
//! same deny-by-default the absent block gets, and a request path that reached a capability check
//! without a snapshot has a bug that should fail closed rather than quietly succeed.
//!
//! [ADR 0118]: ../../../docs/adr/0118-a-capability-is-checked-at-the-door-to-the-effect.md

use std::fs::File;
use std::path::Path;

use nvs_config::capability::{Cap, Scope};

use crate::ctx::Ctx;
use crate::{Fault, ThrownClass};

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
    match refusal(ctx, cap, scope, member) {
        Some(message) => Err(Fault::thrown(message)),
        None => Ok(()),
    }
}

/// [`require`]'s answer as data: `None` when the capability covers `scope`, and § 5's message
/// otherwise.
///
/// For the one door that cannot hand back a [`Fault`] — [`crate::script::resolve`] owns an error
/// type of its own, because a spawn's two other ways of failing are not capability questions. It
/// asks this rather than re-deriving the sentence, so the message a denial prints has exactly one
/// author whichever door produced it.
pub(crate) fn refusal(ctx: &Ctx, cap: Cap, scope: Scope<'_>, member: &str) -> Option<String> {
    let granted = ctx.config().is_some_and(|config| {
        config
            .snapshot()
            .config
            .capabilities
            .as_ref()
            .is_some_and(|caps| caps.allows(cap, scope, &nvs_config::resolve::Disk))
    });
    if granted {
        return None;
    }
    Some(denial(cap, scope, member))
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

/// § 2's read door: the file at `path`, open for reading, once [`Cap::FsRead`] has been shown to
/// cover it.
///
/// The open handle rather than the bytes, because one door has to serve a whole-file read and an
/// incremental one alike, and the capability question belongs to the *handle*: a descriptor already
/// open is a descriptor already checked, so nothing downstream of this call has to ask again.
///
/// `member` is what a refusal names — see [`require`].
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`, or [`io_failure`]'s `IOError` when the open itself fails. The order is the point: a path
/// outside the grant is refused as a capability whether or not it exists, so a program cannot use
/// the difference between the two messages to probe a directory it was never allowed to read.
pub fn open_read(ctx: &Ctx, path: &Path, member: &str) -> Result<File, Fault> {
    require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    File::open(path).map_err(|err| io_failure(member, path, &err))
}

/// § 2's write door: `bytes` become the whole content of `path`, once [`Cap::FsWrite`] has been
/// shown to cover it.
///
/// The finished effect rather than an open handle, unlike [`open_read`], because a write *creates*
/// the path it names: `nvs_config::capability` § 4's argument side resolves the deepest existing
/// ancestor precisely so this check can happen before anything is created, and handing back a
/// handle would put the create on the caller's side of the door.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// `path`, or [`io_failure`]'s `IOError` when the write itself fails.
pub fn write(ctx: &Ctx, path: &Path, bytes: &[u8], member: &str) -> Result<(), Fault> {
    require(ctx, Cap::FsWrite, Scope::Path(path), member)?;
    std::fs::write(path, bytes).map_err(|err| io_failure(member, path, &err))
}

/// What a door reports when the operating system refuses something the capability allowed: an
/// `IOError`, catchable, naming the member, the path and what the OS said.
///
/// Public because [`open_read`] hands back a handle rather than a result, so the caller that reads
/// from it owes the same message for the same kind of failure; one function is how the two agree
/// rather than drifting into two spellings of "could not read".
#[must_use]
pub fn io_failure(member: &str, path: &Path, err: &std::io::Error) -> Fault {
    Fault::thrown_as(
        ThrownClass::Io,
        format!("{member} failed on {}: {err}", path.display()),
    )
}
