//! A block's SMTP sessions, kept open between sends on the core that opened them.
//!
//! Each message over TLS pays a connection, a handshake, two `EHLO`s and `AUTH` before its
//! envelope. A kept session pays one `RSET` instead, which is countable less work
//! (`rule:programs/less-work-is-taken`). The store is [`nvs_runtime::pool`], the one `[db]` and
//! `[ldap]` connections use: per core, keyed by the block's name in the configuration generation
//! that declared it, so a session never serves another block or a reloaded one.
//!
//! **A kept session starts each message with `RSET`.** A server that closed it in the meantime
//! fails the `RSET`, and the send opens one new session in its place. If that one fails too, its
//! error is the send's. Nothing of the message was sent over the closed one, so the replacement
//! cannot deliver it twice.
//!
//! **A session unused for [`IDLE_TIME`] is closed, not reused.** Relays close a quiet client after
//! a while, and many do it sooner than RFC 5321's five minutes. Reusing one that the server is
//! about to close costs a failed `RSET`, so the pool closes it first. The time is fixed, not a
//! setting: `[mail.<name>.pool] idle` is the count of kept sessions, as it is for `[db]`.
//!
//! A session that failed in any way is closed. Only a session whose last message the server
//! accepted goes back to the pool.
//!
//! **What it spends:** at most `[mail.<name>.pool] idle` open sessions per block per core between
//! sends, each a socket, a TLS session and its read buffer, and at most `max` while sending. Both
//! are bounded by the configuration, never by traffic. `pool = false` opens and closes a session
//! for every message.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use nvs_runtime::Fault;
use nvs_runtime::pool::Lease;

use super::Endpoint;
use super::smtp::{self, Envelope, Session};

/// How long a kept session may wait unused before the next send closes it and opens a new one.
pub(super) const IDLE_TIME: Duration = Duration::from_secs(30);

/// A session in the pool, and when its last message ended.
#[derive(Debug)]
struct Kept {
    session: Session,
    used: Instant,
}

impl nvs_runtime::HeldConnection for Kept {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }

    /// Only a session whose last message was accepted is released into the pool, so every one
    /// that reaches it may stay.
    fn is_poolable(&self) -> bool {
        true
    }
}

/// Sends one message over a kept session of `lease`'s block, or over a new one, and keeps the
/// session for the next send when the pool is on. `now` is the instant the send starts, and every
/// wait ends at `now` plus the block's `timeout`.
///
/// # Errors
///
/// [`smtp::open`]'s and [`smtp::transact`]'s. A kept session that fails its `RSET` is replaced
/// once, and the new session's error is the one returned.
pub(super) fn deliver(
    lease: Lease,
    now: Instant,
    address: SocketAddr,
    endpoint: &Endpoint,
    envelope: &Envelope<'_>,
    message: &str,
    member: &str,
) -> Result<(), Fault> {
    let until = now + endpoint.timeout;
    if !lease.bounds().enabled {
        return smtp::deliver(address, endpoint, until, envelope, message, member);
    }
    let mut session = match warm(&lease, now, until, member) {
        Some(session) => session,
        None => smtp::open(address, endpoint, until, member)?,
    };
    match smtp::transact(&mut session, endpoint, envelope, message, member) {
        Ok(()) => {
            let used = now.max(Instant::now());
            nvs_runtime::pool::release(lease, used, Box::new(Kept { session, used }));
            Ok(())
        }
        Err(fault) => {
            session.quit();
            Err(fault)
        }
    }
}

/// A kept session of `lease`'s block, reset for a new message, or `None` when there is none, when
/// it was unused past [`IDLE_TIME`], or when the server closed it.
fn warm(lease: &Lease, now: Instant, until: Instant, member: &str) -> Option<Session> {
    let kept = nvs_runtime::pool::take(lease, now)?
        .into_any()
        .downcast::<Kept>()
        .ok()?;
    let Kept { mut session, used } = *kept;
    if now.saturating_duration_since(used) > IDLE_TIME {
        session.quit();
        return None;
    }
    smtp::reset(&mut session, until, member).ok()?;
    Some(session)
}

#[cfg(test)]
mod tests {
    use nvs_config::db::PoolBounds;
    use nvs_config::mail::Tls;
    use nvs_runtime::pool::{Ticket, admit};

    use super::super::smtp::tests::{Fake, MESSAGE, ada, block, serve_many};
    use super::*;

    const MEMBER: &str = "Core\\Mail::send";

    /// The ticket of one block, under `bounds`.
    fn ticket(block: &str, bounds: PoolBounds) -> Ticket {
        Ticket::for_settings(format!("mail-test:{block}"), bounds)
    }

    /// One message to `ada@example.com` through `ticket`'s pool, starting at `now`.
    fn send(ticket: &Ticket, address: SocketAddr, now: Instant) -> Result<(), Fault> {
        let lease = admit(ticket.clone()).expect("a free slot");
        let recipients = ada();
        deliver(
            lease,
            now,
            address,
            &block(Tls::None),
            &Envelope {
                sender: "noreply@example.com",
                recipients: &recipients,
            },
            MESSAGE,
            MEMBER,
        )
    }

    /// Closes every session the pool keeps under `ticket`, so the fake server's threads end.
    fn close(ticket: &Ticket) {
        let lease = admit(ticket.clone()).expect("a free slot");
        while let Some(kept) = nvs_runtime::pool::take(&lease, Instant::now()) {
            drop(kept);
        }
    }

    fn messages(session: &[String]) -> usize {
        session.iter().filter(|line| *line == "<body>").count()
    }

    /// Two sends to one block open one connection. The second starts with `RSET` and sends no
    /// second `EHLO`.
    // covers: Core\Mail::send
    #[test]
    fn a_second_send_reuses_the_connection() {
        let server = serve_many(Fake::default());
        let pool = ticket("reuse", nvs_config::mail::POOL_DEFAULTS);
        let now = Instant::now();
        send(&pool, server.address, now).expect("the first message");
        send(&pool, server.address, now).expect("the second message");
        close(&pool);
        let sessions = server.stop();
        assert_eq!(sessions.len(), 1, "{sessions:?}");
        assert_eq!(messages(&sessions[0]), 2, "{sessions:?}");
        assert_eq!(
            sessions[0]
                .iter()
                .filter(|line| line.starts_with("EHLO"))
                .count(),
            1,
            "{sessions:?}"
        );
        assert!(sessions[0].contains(&"RSET".to_owned()), "{sessions:?}");
    }

    /// A kept connection the server closed fails its `RSET`, and the send opens one new
    /// connection and sends the message over it.
    // covers: Core\Mail::send
    #[test]
    fn a_connection_the_server_closed_is_replaced_once() {
        let server = serve_many(Fake {
            close_after_message: true,
            ..Fake::default()
        });
        let pool = ticket("closed", nvs_config::mail::POOL_DEFAULTS);
        let now = Instant::now();
        send(&pool, server.address, now).expect("the first message");
        send(&pool, server.address, now).expect("the message over a new connection");
        close(&pool);
        let sessions = server.stop();
        assert_eq!(sessions.len(), 2, "{sessions:?}");
        assert!(
            sessions.iter().all(|session| messages(session) == 1),
            "{sessions:?}"
        );
    }

    /// A connection kept longer than [`IDLE_TIME`] is closed with `QUIT` when the next send
    /// starts, and the send opens a new one.
    // covers: Core\Mail::send
    #[test]
    fn an_idle_connection_is_closed_after_its_idle_time() {
        let server = serve_many(Fake::default());
        let pool = ticket("idle", nvs_config::mail::POOL_DEFAULTS);
        let now = Instant::now();
        send(&pool, server.address, now).expect("the first message");
        let later = now + IDLE_TIME + Duration::from_secs(1);
        send(&pool, server.address, later).expect("the second message");
        close(&pool);
        let sessions = server.stop();
        assert_eq!(sessions.len(), 2, "{sessions:?}");
        assert_eq!(
            sessions[0].last().map(String::as_str),
            Some("QUIT"),
            "{sessions:?}"
        );
    }

    /// `pool = false` opens a connection for every message and ends each with `QUIT`.
    // covers: Core\Mail::send
    #[test]
    fn pool_false_opens_one_connection_per_send() {
        let server = serve_many(Fake::default());
        let off = PoolBounds {
            enabled: false,
            ..nvs_config::mail::POOL_DEFAULTS
        };
        let pool = ticket("off", off);
        let now = Instant::now();
        send(&pool, server.address, now).expect("the first message");
        send(&pool, server.address, now).expect("the second message");
        let sessions = server.stop();
        assert_eq!(sessions.len(), 2, "{sessions:?}");
        assert!(
            sessions
                .iter()
                .all(|session| session.last().map(String::as_str) == Some("QUIT")),
            "{sessions:?}"
        );
    }

    /// A connection kept for one block is never used for another, even to the same server.
    // covers: Core\Mail::send
    #[test]
    fn a_pooled_connection_never_serves_another_block() {
        let server = serve_many(Fake::default());
        let first = ticket("first", nvs_config::mail::POOL_DEFAULTS);
        let second = ticket("second", nvs_config::mail::POOL_DEFAULTS);
        let now = Instant::now();
        send(&first, server.address, now).expect("the first block's message");
        send(&second, server.address, now).expect("the second block's message");
        close(&first);
        close(&second);
        let sessions = server.stop();
        assert_eq!(sessions.len(), 2, "{sessions:?}");
        assert!(
            sessions.iter().all(|session| messages(session) == 1),
            "{sessions:?}"
        );
    }
}
