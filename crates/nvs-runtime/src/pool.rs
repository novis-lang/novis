//! [ADR 0067](../../../docs/adr/0067-core-db.md) § 13's connection pool: where a
//! request's database connection goes at teardown instead of being dropped.
//!
//! **Per core, and never shared between cores** — § 13's first bullet, and the
//! reason this is a `thread_local!` rather than anything with a lock in it. The
//! acquire path is a scan of one `Vec` the calling core owns outright, so a
//! connection reaches the next request on that core for the price of a string
//! comparison, and a core that has never opened a database holds an empty `Vec`
//! and pays nothing at all.
//!
//! **The key is the caller's, and this module never parses one.** § 13 keys a
//! pool on every credential — the block's name for `Core\Db::connect`, a hash of
//! every settings field for `open` — which is the key § 2 already computes to
//! memoize a connection *within* a request. The crate that computes it hands it
//! over in a [`Ticket`], so two config blocks are two pools and two database
//! users never share a connection, without this module knowing what a
//! credential is.
//!
//! A block's name identifies its credentials only within one configuration, so
//! the key [`Ticket::for_block`] builds is scoped to the generation the name was
//! read from as well — see that function for why a reload would otherwise be
//! the one way two database users *could* share a connection.
//!
//! # Where the reset is
//!
//! § 13 makes the reset a security boundary: a connection that cannot be proven
//! clean is closed rather than reused. That proof is taken **at acquire**, by
//! the request about to use the connection, and not here at release.
//!
//! The reason is structural. Release happens inside [`Ctx`](crate::Ctx)'s own
//! [`Drop`], where a driver cannot do I/O: the parking stream needs a core to
//! hand back and a deadline to wait against, and a `Ctx` is also dropped by a
//! CLI and by a test that has neither. Acquire has both. It is also where the
//! property has to hold anyway — an acquiring request that cannot reset what it
//! took destroys it and opens a fresh connection, so no request ever observes
//! another's session state, which is the whole of what § 13 asks.
//!
//! What that costs is stated rather than hidden: a connection waits in the pool
//! carrying whatever session state its last request left behind. Never an open
//! transaction or a wire mid-message — [`HeldConnection::is_poolable`] refuses
//! both at release — but possibly an advisory lock or a temporary table, until
//! the next acquire clears them. A deployment that cannot accept that has
//! § 13's `pool = false`.
//!
//! # The two bounds this module does not read yet
//!
//! [`PoolBounds`] carries four numbers and this module honours `idle` and
//! `lifetime`. `max` is a ceiling on connections a core holds *including the
//! ones in use*, and `acquire` is how long a request waits at that ceiling —
//! together they are a wait, and a wait needs the core's scheduler rather than
//! this `Vec`. Until they land, a core opens as many connections as its
//! requests ask for, exactly as it did before there was a pool; what is bounded
//! is the number kept *idle*, which is the memory this module spends.
//!
//! # What it spends
//!
//! Up to `idle` connections per key per core — a socket, a TLS session and a
//! statement cache each — held open between the requests that use them. That is
//! the footprint § 13 spends to buy priority 3, and it is O(cores × keys)
//! rather than O(requests served): a release past `idle` closes the connection
//! instead of queueing it, so nothing accumulates with traffic.

use std::cell::RefCell;
use std::sync::Arc;
use std::time::Instant;

use nvs_config::db::PoolBounds;
use nvs_config::snapshot::Snapshot;

use crate::ctx::HeldConnection;

/// What a held connection needs to rejoin the pool: the key § 13 pools it
/// under, and the bounds that key resolved to at boot.
///
/// Carried beside the connection for the request's whole life rather than
/// re-derived at teardown, because the crate that knows a block's name is
/// `nvs-stdlib` and the crate that learns when a request ends is this one.
/// [`PoolBounds`] is `Copy` and five words, so a ticket is one allocation — the
/// key — per connection a request opens.
#[derive(Clone, Debug)]
pub struct Ticket {
    /// § 2's key, generation-scoped: see [`Ticket::key_for`]. Two requests share
    /// a pooled connection exactly when this string matches, so it must carry
    /// every credential.
    pub key: String,
    /// The bounds `nvs_config::db::pool_for` resolved for that key at boot.
    pub bounds: PoolBounds,
    /// The configuration generation [`key`](Ticket::key) names, held for as
    /// long as a connection opened under it may still be pooled.
    ///
    /// Nothing reads it. It is here so that the generation's address, which is
    /// what makes the key unique, cannot be reused by a later generation while
    /// a connection is still keyed on it — the alias
    /// [`Ticket::key_for`] would otherwise have.
    #[expect(
        dead_code,
        reason = "held for its address, which the key already carries — a read \
                  would be the bug, not the silence"
    )]
    generation: Arc<Snapshot>,
}

impl Ticket {
    /// The ticket for `[db.<name>]` as `generation` declares it.
    #[must_use]
    pub fn for_block(generation: &Arc<Snapshot>, name: &str, bounds: PoolBounds) -> Ticket {
        Ticket {
            key: Ticket::key_for(generation, name),
            bounds,
            generation: Arc::clone(generation),
        }
    }

    /// The pool key `[db.<name>]` has in `generation` — what
    /// [`take`] is asked for before a ticket exists.
    ///
    /// **A block's name identifies its credentials only within one
    /// configuration.** § 13 keys a pool on every credential and names the
    /// block's name as that key for `connect`, which holds because the block is
    /// where the credentials are written — until
    /// [ADR 0078](../../../docs/adr/0078-config-reload-and-control-socket.md)
    /// § 1's reload publishes a `[db.main]` naming a different database user
    /// under the same name. A pool keyed on the name alone would then hand the
    /// new generation's request a connection authenticated as the old one's
    /// user, which is exactly the sharing § 13 forbids. Scoping the key to the
    /// generation makes a reload two pools rather than one, and the old
    /// generation's connections retire against their own `lifetime` with
    /// nothing left to hand them to.
    ///
    /// **What it spends:** the superseded [`Snapshot`] stays alive while any of
    /// its connections is still pooled — one allocation per generation, not per
    /// connection, and freed when the last of them retires.
    #[must_use]
    pub fn key_for(generation: &Arc<Snapshot>, name: &str) -> String {
        format!("{:p}:{name}", Arc::as_ptr(generation))
    }
}

/// One connection waiting for the next request on this core.
#[derive(Debug)]
struct Idle {
    /// The [`Ticket::key`] it was released under.
    key: String,
    /// The connection itself, still holding its statement cache — the thing
    /// § 13's reset is written *not* to throw away.
    connection: Box<dyn HeldConnection>,
    /// When it is retired regardless of health: the release instant plus
    /// [`PoolBounds::lifetime`], stored as the deadline so that a scan compares
    /// rather than adds.
    retire: Instant,
}

thread_local! {
    /// This core's idle connections, most recently released last.
    ///
    /// A `Vec` and not a map: `idle` defaults to 2 and a deployment names a
    /// handful of blocks, so the whole store is a few entries and a linear scan
    /// beats hashing a key that is already in hand. It holds a `Drop` type, so
    /// unlike [`crate::alloc`]'s cache this one has a destructor — which is
    /// what closes a core's connections when its thread ends.
    static IDLE: RefCell<Vec<Idle>> = const { RefCell::new(Vec::new()) };
}

/// Releases `connection` to this core's pool, or closes it here.
///
/// It is closed rather than pooled when § 13 says it must be: the pool is off
/// for that key (`pool = false`), the driver cannot prove the wire clean
/// ([`HeldConnection::is_poolable`]), or the key already holds
/// [`PoolBounds::idle`] connections doing nothing. All three drop the box
/// before this returns, which is the same close a request without a pool
/// already performed.
///
/// `now` is passed in rather than read here so that the one clock read at a
/// request's teardown covers every connection it held — and so that a test can
/// name an instant the future has already passed.
pub fn release(ticket: &Ticket, now: Instant, connection: Box<dyn HeldConnection>) {
    if !ticket.bounds.enabled || !connection.is_poolable() {
        return;
    }
    // A lifetime that cannot be added to `now` is not a pool a connection can
    // be retired from, so it is not one it may enter.
    let Some(retire) = now.checked_add(ticket.bounds.lifetime) else {
        return;
    };
    let bound = usize::try_from(ticket.bounds.idle).unwrap_or(usize::MAX);
    // Dropped after the borrow ends: a connection's own `Drop` closes a socket
    // and cannot reach this module, but a store that hands a `Drop` type out
    // from inside its own borrow is one edit away from a panic.
    let closed = IDLE.with_borrow_mut(|idle| {
        if idle.iter().filter(|entry| entry.key == ticket.key).count() >= bound {
            return Some(connection);
        }
        idle.push(Idle {
            key: ticket.key.clone(),
            connection,
            retire,
        });
        None
    });
    drop(closed);
}

/// The connection this core last released under `key`, or `None` for a key it
/// holds none of.
///
/// **What comes back has not been reset**, and the caller owes § 13's reset
/// before it runs a statement over it — the module doc's *Where the reset is*
/// owns why that is the acquiring side's job. A caller that cannot reset what
/// it took must close it and open a fresh connection.
///
/// The most recently released connection under the key comes back first: it is
/// the warmest, and it leaves the older ones to age out against their own
/// [`PoolBounds::lifetime`] rather than being kept alive forever by a rotation.
/// Entries whose lifetime `now` has passed are closed as the scan walks them,
/// which is the only place a pooled connection is retired.
#[must_use]
pub fn take(key: &str, now: Instant) -> Option<Box<dyn HeldConnection>> {
    let mut retired = Vec::new();
    let taken = IDLE.with_borrow_mut(|idle| {
        let mut index = idle.len();
        while index > 0 {
            index -= 1;
            if idle[index].retire <= now {
                retired.push(idle.remove(index));
                continue;
            }
            if idle[index].key == key {
                return Some(idle.remove(index).connection);
            }
        }
        None
    });
    drop(retired);
    taken
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use nvs_config::snapshot::Snapshot;

    use super::{Ticket, release, take};
    use crate::ctx::HeldConnection;

    /// A connection that is nothing but an identity and an answer to the
    /// release gate — everything this module does to a connection it does
    /// through the trait, so nothing else is needed to test it.
    #[derive(Debug)]
    struct Fake {
        id: u32,
        poolable: bool,
    }

    impl HeldConnection for Fake {
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
            self
        }

        fn is_poolable(&self) -> bool {
            self.poolable
        }
    }

    /// A poolable connection, boxed as the trait object the pool stores.
    fn fake(id: u32) -> Box<dyn HeldConnection> {
        Box::new(Fake { id, poolable: true })
    }

    /// Which connection came back, or `None`.
    fn id_of(taken: Option<Box<dyn HeldConnection>>) -> Option<u32> {
        let mut taken = taken?;
        Some(taken.as_any_mut().downcast_mut::<Fake>()?.id)
    }

    /// One configuration generation, which is all a key needs of one.
    fn generation() -> Arc<Snapshot> {
        Arc::new(Snapshot::default())
    }

    /// A ticket on `[db.<name>]` in `generation`, with the default bounds and
    /// `idle` overridden.
    fn ticket(generation: &Arc<Snapshot>, name: &str, idle: u32) -> Ticket {
        Ticket::for_block(
            generation,
            name,
            nvs_config::db::PoolBounds {
                idle,
                ..nvs_config::db::PoolBounds::DEFAULT
            },
        )
    }

    // Every case below runs on its own test thread and so gets its own pool,
    // which is the same isolation a core gets and the reason none of them has
    // to clean up after itself.

    #[test]
    fn a_released_connection_comes_back_under_its_key() {
        let now = std::time::Instant::now();
        let generation = generation();
        release(&ticket(&generation, "main", 2), now, fake(1));

        assert_eq!(
            id_of(take(&Ticket::key_for(&generation, "other"), now)),
            None
        );
        assert_eq!(
            id_of(take(&Ticket::key_for(&generation, "main"), now)),
            Some(1)
        );
        assert_eq!(
            id_of(take(&Ticket::key_for(&generation, "main"), now)),
            None
        );
    }

    #[test]
    fn the_warmest_connection_under_a_key_comes_back_first() {
        let now = std::time::Instant::now();
        let generation = generation();
        let ticket = ticket(&generation, "main", 4);
        release(&ticket, now, fake(1));
        release(&ticket, now, fake(2));

        assert_eq!(id_of(take(&ticket.key, now)), Some(2));
        assert_eq!(id_of(take(&ticket.key, now)), Some(1));
    }

    #[test]
    fn a_release_past_the_idle_bound_closes_the_connection() {
        let now = std::time::Instant::now();
        let generation = generation();
        let ticket = ticket(&generation, "main", 1);
        release(&ticket, now, fake(1));
        release(&ticket, now, fake(2));

        assert_eq!(id_of(take(&ticket.key, now)), Some(1));
        assert_eq!(id_of(take(&ticket.key, now)), None);
    }

    #[test]
    fn the_idle_bound_is_per_key_and_not_per_core() {
        let now = std::time::Instant::now();
        let generation = generation();
        let main = ticket(&generation, "main", 1);
        let reports = ticket(&generation, "reports", 1);
        release(&main, now, fake(1));
        release(&reports, now, fake(2));

        assert_eq!(id_of(take(&main.key, now)), Some(1));
        assert_eq!(id_of(take(&reports.key, now)), Some(2));
    }

    #[test]
    fn two_configuration_generations_are_two_pools() {
        let now = std::time::Instant::now();
        let before = generation();
        let after = generation();
        release(&ticket(&before, "main", 2), now, fake(1));

        // ADR 0078's reload can put a different database user behind the same
        // block name, so the name alone must not reach the old connection.
        assert_eq!(id_of(take(&Ticket::key_for(&after, "main"), now)), None);
        assert_eq!(id_of(take(&Ticket::key_for(&before, "main"), now)), Some(1));
    }

    #[test]
    fn pool_false_stores_nothing() {
        let now = std::time::Instant::now();
        let generation = generation();
        let off = Ticket::for_block(&generation, "main", nvs_config::db::PoolBounds::OFF);
        release(&off, now, fake(1));

        assert_eq!(id_of(take(&off.key, now)), None);
    }

    #[test]
    fn a_connection_the_driver_cannot_prove_clean_is_closed() {
        let now = std::time::Instant::now();
        let generation = generation();
        let ticket = ticket(&generation, "main", 2);
        release(
            &ticket,
            now,
            Box::new(Fake {
                id: 1,
                poolable: false,
            }),
        );

        assert_eq!(id_of(take(&ticket.key, now)), None);
    }

    #[test]
    fn a_connection_past_its_lifetime_is_never_handed_out() {
        let now = std::time::Instant::now();
        let generation = generation();
        let ticket = ticket(&generation, "main", 2);
        release(&ticket, now, fake(1));
        let expired = now + ticket.bounds.lifetime + Duration::from_secs(1);

        assert_eq!(id_of(take(&ticket.key, expired)), None);
        // Retired by that scan rather than left for the next one to trip over.
        assert_eq!(id_of(take(&ticket.key, now)), None);
    }

    #[test]
    fn a_request_teardown_releases_what_it_held() {
        let now = std::time::Instant::now();
        let generation = generation();
        let ticket = ticket(&generation, "main", 2);
        let key = ticket.key.clone();

        let mut ctx = crate::Ctx::stdout();
        ctx.hold_open_connection(Some("main".to_owned()), Some(ticket), fake(1));
        // Still the request's while the request is running: § 13 releases at
        // teardown and never before.
        assert_eq!(id_of(take(&key, now)), None);
        drop(ctx);

        assert_eq!(id_of(take(&key, now)), Some(1));
    }

    #[test]
    fn a_connection_filed_with_no_ticket_is_closed_with_the_request() {
        let now = std::time::Instant::now();
        let generation = generation();
        let key = Ticket::key_for(&generation, "main");

        let mut ctx = crate::Ctx::stdout();
        ctx.hold_open_connection(None, None, fake(1));
        drop(ctx);

        assert_eq!(id_of(take(&key, now)), None);
    }

    #[test]
    fn a_pool_is_per_core_and_shared_with_no_other() {
        let now = std::time::Instant::now();
        let generation = generation();
        let ticket = ticket(&generation, "main", 2);
        release(&ticket, now, fake(1));

        let key = ticket.key.clone();
        let elsewhere = std::thread::spawn(move || id_of(take(&key, now)))
            .join()
            .expect("the probe thread does not panic");

        assert_eq!(elsewhere, None);
        assert_eq!(id_of(take(&ticket.key, now)), Some(1));
    }
}
