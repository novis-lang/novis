//! `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`'s
//! store: where a connection goes once the reply on it has been framed to its
//! end, instead of being closed.
//!
//! **Per core, and never shared between cores** — the rule's first paragraph,
//! and the reason this is a `thread_local!` rather than anything with a lock in
//! it. A core that has made no outbound call holds an empty `Vec` and pays
//! nothing; one that has scans a handful of entries and compares a string,
//! which beats hashing a key that is already in hand. It is the same shape
//! [`nvs_runtime::pool`] holds a database connection in, for the same reason.
//!
//! **The key is the caller's, and nothing here parses one.** [`super::transport`]
//! builds it out of what the door approved — the pinned address, the port, the
//! scheme and the server name — so no reading of a URL in this module can widen
//! reuse past what the check ahead of the call allowed. What the key *means* is
//! the rule's second paragraph; what it *is* here is an opaque string.
//!
//! **What it spends:** at most [`Caps::idle`] entries per core, each one a
//! socket, a TLS session where the call had one, and the key it is filed under.
//! Held between requests and charged to the core, which is
//! O(cores × `pool_idle`) and never O(requests served) — the accounting
//! `rule:security/db-pool-reset-is-a-boundary` states for the other pool this
//! process keeps.

use std::cell::RefCell;
use std::time::{Duration, Instant};

use super::transport::Connection;

/// The two `System`-class bounds on one core's store — `[http.client]
/// pool_idle` and `pool_idle_timeout`, resolved by the caller before the call
/// began.
///
/// `Copy` and owning nothing, so carrying it beside every connection costs the
/// two numbers and no allocation. They are `System` because they bound a core's
/// memory rather than a request's (`rule:config/three-changeability-classes`),
/// which is why nothing in a call's own options bag can move them.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Caps {
    /// How many idle connections one core may hold. Zero keeps none, which is
    /// connect-per-request exactly.
    pub(crate) idle: usize,
    /// How long one may sit idle before it is closed unused.
    pub(crate) timeout: Duration,
}

/// One connection waiting for the next call on this core.
struct Idle {
    /// What it was filed under, and the only thing a draw compares.
    key: String,
    /// The connection itself, with the reply that was on it read to its end.
    connection: Box<dyn Connection>,
    /// When it is closed unused: the release instant plus [`Caps::timeout`],
    /// stored as the deadline so that a scan compares rather than adds.
    expires: Instant,
}

thread_local! {
    /// This core's idle connections, most recently released last.
    ///
    /// A `Vec` and not a map, for [`Idle`]'s handful of entries: `pool_idle`
    /// ships at `16` and a deployment talks to a handful of upstreams, so a
    /// linear scan beats hashing a key the caller is already holding. It holds
    /// a `Drop` type, which is what closes a core's connections when its thread
    /// ends.
    static IDLE: RefCell<Vec<Idle>> = const { RefCell::new(Vec::new()) };
}

/// The connection this core holds under `key`, or `None` where it holds none.
///
/// Every entry past its own timeout is closed on the way, and not only the ones
/// under `key`: a store that retired only what it was asked for would hold a
/// connection to a host nobody calls again for as long as the process runs. The
/// most recently released match is the one handed back, since it is the one
/// whose far end has had least time to give up on it.
pub(crate) fn take(key: &str, now: Instant) -> Option<Box<dyn Connection>> {
    IDLE.with_borrow_mut(|store| {
        store.retain(|held| held.expires > now);
        let at = store.iter().rposition(|held| held.key == key)?;
        Some(store.remove(at).connection)
    })
}

/// Files `connection` under `key` until the next call that matches it.
///
/// The oldest entry goes when the store is full rather than this one being
/// turned away: either keeps `pool_idle`, and the connection that has sat idle
/// longest is the one nearest to being closed by the far end anyway.
pub(crate) fn release(key: String, mut connection: Box<dyn Connection>, caps: Caps, now: Instant) {
    if caps.idle == 0 {
        return;
    }
    // Nothing waits on an idle connection, so the deadline the call bounded its
    // own reads with comes off as it is filed: the next call sets its own
    // before it writes a byte.
    connection.bound_by(None);
    IDLE.with_borrow_mut(|store| {
        store.retain(|held| held.expires > now);
        store.push(Idle {
            key,
            connection,
            expires: now + caps.timeout,
        });
        let over = store.len().saturating_sub(caps.idle);
        store.drain(..over);
    });
}

#[cfg(test)]
mod tests {
    use super::{Caps, IDLE, release, take};
    use std::io::{Read, Write};
    use std::time::{Duration, Instant};

    /// A connection that carries nothing and answers nothing: what is under
    /// test here is the store's arithmetic, and a socket would only add a
    /// listener to every case.
    struct Nothing;

    impl Read for Nothing {
        fn read(&mut self, _into: &mut [u8]) -> std::io::Result<usize> {
            Ok(0)
        }
    }

    impl Write for Nothing {
        fn write(&mut self, from: &[u8]) -> std::io::Result<usize> {
            Ok(from.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl super::Connection for Nothing {
        fn bound_by(&mut self, _at: Option<Instant>) {}
    }

    /// How many this core is holding.
    fn held() -> usize {
        IDLE.with_borrow(Vec::len)
    }

    /// The rule's two caps: a core holds at most `pool_idle` idle connections,
    /// and one that has sat past `pool_idle_timeout` is closed rather than
    /// handed to the next call.
    #[test]
    fn pool_holds_no_more_than_its_idle_cap_and_closes_past_its_idle_timeout() {
        let caps = Caps {
            idle: 2,
            timeout: Duration::from_secs(30),
        };
        let now = Instant::now();

        for key in ["a", "b", "c"] {
            release(key.to_owned(), Box::new(Nothing), caps, now);
        }
        assert_eq!(held(), 2, "the cap is a cap on the whole store");
        assert!(
            take("a", now).is_none(),
            "the oldest is the one the cap drops"
        );
        assert!(take("c", now).is_some(), "the newest is still there");
        assert!(take("b", now).is_some());
        assert_eq!(held(), 0);

        release("d".to_owned(), Box::new(Nothing), caps, now);
        assert!(
            take("d", now + caps.timeout + Duration::from_secs(1)).is_none(),
            "one past its timeout is closed rather than drawn"
        );
        assert_eq!(held(), 0, "and the scan that passed it took it out");

        // `pool_idle = 0` is connect-per-request exactly, which is the shape a
        // deployment turning the pool off is asking for.
        let none = Caps {
            idle: 0,
            timeout: Duration::from_secs(30),
        };
        release("e".to_owned(), Box::new(Nothing), none, now);
        assert_eq!(held(), 0);
    }

    /// A draw compares the whole key and nothing less: two keys that differ
    /// anywhere are two connections, which is what keeps the pin true through
    /// reuse.
    #[test]
    fn a_draw_matches_the_whole_key() {
        let caps = Caps {
            idle: 4,
            timeout: Duration::from_secs(30),
        };
        let now = Instant::now();
        release(
            "https|api.example|127.0.0.1:443".to_owned(),
            Box::new(Nothing),
            caps,
            now,
        );
        assert!(take("https|api.example|127.0.0.1:8443", now).is_none());
        assert!(take("http|api.example|127.0.0.1:443", now).is_none());
        assert!(take("https|other.example|127.0.0.1:443", now).is_none());
        assert!(take("https|api.example|127.0.0.1:443", now).is_some());
    }
}
