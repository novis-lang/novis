//! [ADR 0097](../../../docs/adr/0097-development-server-and-proxied-origin.md)
//! § 5's drain: the one bit a health probe and an application both read.
//!
//! The bit lives here rather than beside the accept loop because it has two
//! readers on opposite sides of the crate graph. `nvs_server::Draining` is the
//! loop's handle over it and owns the rule that the loop is its only writer;
//! `Core\Server::isDraining()` is the other reader, and `nvs-stdlib` does not
//! depend on `nvs-server` — nor should it, since a `Core` member that needed
//! the HTTP server linked in to answer a question about this process would put
//! `hyper` in the graph of every CLI program. This crate is what both rest on,
//! so this is where a fact both of them state has to sit.
//!
//! # One relaxed atomic
//!
//! Read once per probe, written once per process. There is no ordering to
//! establish because the bit is the whole of the message: a reader that sees
//! `false` an instant before the drain begins is indistinguishable from one
//! that asked an instant earlier, and no second fact has to be visible with it.
//! This is not on the value path [`crate::string`]'s non-atomic refcounts
//! protect — it is one relaxed load per health request, and a health request is
//! not the request path.
//!
//! # Process-wide, and one handle that is not
//!
//! [`Drain::process`] is the bit every real server and every reader of the
//! probe takes, for [`Drain`]'s own stated reason: a shutdown drains the
//! process, so a bit held per core would answer differently depending on which
//! core happened to take the connection.
//!
//! [`Drain::detached`] is the other case, and it is a server rather than a
//! test-only escape hatch: an accept loop run inside a process that is not
//! *only* that server — a test binary, or a second listener a harness ends on
//! its own — stops a **server**, and reporting the process as draining because
//! one of them stopped would be the fail-*closed* direction of the same error
//! `Drain::process` exists to prevent.

use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// This process's own bit, allocated on the first ask and never again.
///
/// A `OnceLock` rather than a plain `static AtomicBool` so that a handle is one
/// type whichever bit it points at: [`Drain::detached`] owns its bit, so the
/// shared one has to be ownable too, and an `Arc` is what makes both spellings
/// the same three-word struct.
fn process_bit() -> &'static Arc<AtomicBool> {
    static PROCESS: OnceLock<Arc<AtomicBool>> = OnceLock::new();
    PROCESS.get_or_init(|| Arc::new(AtomicBool::new(false)))
}

/// A handle on a server's drain bit — whether it has stopped accepting.
///
/// Cloning a handle shares the bit; that is the whole of what a handle is for.
/// Which bit it is comes from the constructor and from nowhere else, so a
/// caller that wants the process's answer has said so.
#[derive(Clone, Debug)]
pub struct Drain(Arc<AtomicBool>);

impl Drain {
    /// The process's drain — what a health probe reports and what
    /// `Core\Server::isDraining()` answers.
    #[must_use]
    pub fn process() -> Self {
        Self(Arc::clone(process_bit()))
    }

    /// A drain nothing else in this process can see.
    ///
    /// For a server whose stopping is not this process stopping; the module
    /// docs above own why that is a real case and not a test affordance.
    #[must_use]
    pub fn detached() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Stop accepting: from here the probe answers `503`.
    ///
    /// Idempotent, because a drain that has begun cannot begin again and a
    /// second core reaching this is the same shutdown, not a new one.
    pub fn begin(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    /// Whether the drain this handle names has begun.
    #[must_use]
    pub fn is_draining(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Whether *this process* has begun draining, with no handle to ask through.
///
/// The same answer as [`Drain::process`]'s, for a caller that has no server to
/// have taken a handle from — a `Core` member reached from a request, which is
/// the only reader that is neither the accept loop nor its own probe.
#[must_use]
pub fn is_draining() -> bool {
    process_bit().load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::{Drain, is_draining};

    /// A detached drain is one server's, and beginning it says nothing about
    /// the process or about any other server in it.
    ///
    /// This is the property an in-process accept loop rests on, so it is
    /// asserted rather than assumed from the constructor's name.
    #[test]
    fn a_detached_drain_is_one_servers_alone() {
        let first = Drain::detached();
        let second = Drain::detached();
        first.begin();
        assert!(first.is_draining(), "a drain that began reported accepting");
        assert!(
            !second.is_draining(),
            "one server's drain was reported by another's handle"
        );
        assert!(
            !Drain::process().is_draining(),
            "a detached drain reported this process as draining"
        );
    }

    /// A clone shares the bit — the whole of what makes this a handle rather
    /// than a value a caller could copy and then disagree with.
    #[test]
    fn a_cloned_handle_shares_the_bit() {
        let held = Drain::detached();
        let copy = held.clone();
        held.begin();
        assert!(
            copy.is_draining(),
            "a cloned handle answered from a bit of its own"
        );
    }

    /// The free function and the process handle are one bit, whatever that bit
    /// currently says.
    ///
    /// Asserted as *agreement* rather than against `false`: this test binary's
    /// other tests share the process, and a test that pinned the value would be
    /// pinning the order they run in.
    #[test]
    fn the_free_reader_and_the_process_handle_agree() {
        assert_eq!(
            is_draining(),
            Drain::process().is_draining(),
            "the reader with no handle answered from a different bit"
        );
    }
}
