//! The process environment, as the reads a `Core` member is allowed to make of
//! it.
//!
//! A module beside [`crate::capability`] rather than a member of it, for
//! exactly the reason [`crate::terminal`]'s module doc gives about the terminal:
//! **a door that asks nothing is not a door**, and filing one as a capability
//! would make the roster of real doors harder to read.
//! `rule:security/request-state-throws-in-an-isolate` is where that is
//! decided — environment variables are "process-wide facts already governed by
//! the existing capability/config-overlay machinery", not per-request secrets —
//! and `nvs_stdlib::env`'s module doc carries the rest of the argument, since
//! the class is that crate's.
//!
//! # Why it exists at all, if it adds nothing
//!
//! `nvs-stdlib`'s `nvs_stdlib_reaches_the_os_only_through_the_gate` forbids
//! `std::env::var` in that crate's sources outright, and it is right to: a
//! member that reaches the operating system through its own spelling is a
//! member that could have forgotten to ask. So the effect lives here, where
//! `rule:security/capability-check-at-the-door`
//! puts every other one, and the fact that this particular door opens for
//! everyone is a property of *this file* rather than a judgement each caller
//! re-makes.
//!
//! # Bytes, not text
//!
//! Every function here answers in [`OsString`], because an environment
//! variable is bytes on every platform and `rule:types/bytes`
//! makes a `string` UTF-8. What to do about one that is not text is a library
//! contract and not a runtime one — `Core\Env::get` throws and `Core\Env::all`
//! omits, for the reasons `nvs_stdlib::env` states — so this module decides
//! nothing and drops nothing.
//!
//! # Read-only
//!
//! There is no `set`, and there will not be one, because a process-global
//! mutation is unsound across cores. Nothing in this runtime writes the
//! environment after start, which is what makes an unsynchronised read from a
//! core thread sound.

use std::ffi::OsString;

/// One environment variable's raw value, or `None` where nothing set it.
///
/// A name the platform cannot hold — empty, or carrying `=` or a NUL — is
/// `None` as well, which is `std::env::var_os`'s own answer and the honest one:
/// such a variable cannot have been set either.
#[must_use]
pub fn var(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

/// Every environment variable, in the platform's own order.
///
/// Collected rather than borrowed lazily, so a caller holds a snapshot of one
/// moment instead of an iterator over a table it does not own.
#[must_use]
pub fn vars() -> Vec<(OsString, OsString)> {
    std::env::vars_os().collect()
}
