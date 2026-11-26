//! `Core\Imap` — reads a mailbox an operator named in an `[imap.<name>]` block.
//!
//! `rule:core-api/tier-roster` places the class Native by test 3, as `Core\Mail`'s transport is,
//! because it waits on the outside world. A program names the block and nothing else, and
//! `[capabilities.imap] connect` grants it by name, so no value a request carries can choose a
//! host, a port or a credential.
//!
//! [`session`] is the protocol: the connection, TLS, the sign-in and each command, written and
//! read by `imap-codec`. The class rows that open a block and return its folders and messages
//! are the next part of this module.

#![expect(
    dead_code,
    reason = "the `Core\\Imap` class rows and the session's tests call it"
)]

pub(crate) mod session;
