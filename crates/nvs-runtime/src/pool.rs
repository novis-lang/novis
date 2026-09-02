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
//! # What `max` counts
//!
//! § 13's `max` bounds the connections one core holds under one key, and what
//! this module counts is the ones it has **handed out**: a [`Lease`] per live
//! connection, taken by [`admit`] before the connection is opened and given
//! back when it is released or closed.
//!
//! The idle ones are deliberately not counted a second time, and they do not
//! need to be. An idle entry is only ever a connection that was live under a
//! lease, and [`take`] hands one back only *against* a lease — so `live + idle`
//! under a key can no more exceed `max` than `live` can, and a fresh handshake
//! happens only where the idle store had nothing to offer the slot. That is
//! what makes § 13's `cores × max` the number an operator sizes the server's
//! own connection limit against, rather than a number the idle bound quietly
//! adds to.
//!
//! A lease is the core's that took it, like everything else here, and is
//! dropped there: the slot it gives back is that core's.
//!
//! # The bound this module does not read yet
//!
//! [`PoolBounds::acquire`] is how long a request may wait at that ceiling
//! before it throws, and a wait needs the core's scheduler rather than this
//! `Vec`. Until it lands, [`admit`] answers `None` the moment a key is full —
//! which is exactly what `acquire = 0` means, a value § 13 makes legal and
//! defines as refusing rather than queueing. What is missing is the waiting,
//! not the refusal.
//!
//! # What it spends
//!
//! Up to `idle` connections per key per core — a socket, a TLS session and a
//! statement cache each — held open between the requests that use them. That is
//! the footprint § 13 spends to buy priority 3, and it is O(cores × keys)
//! rather than O(requests served): a release past `idle` closes the connection
//! instead of queueing it, so nothing accumulates with traffic.
//!
//! The count above adds one entry per key a core has a live connection under —
//! a string and a number, dropped when the last of them goes home, so it is
//! O(keys in use) and not O(connections) or O(generations seen).

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

    /// The pool key `[db.<name>]` has in `generation` — what [`Ticket::key`]
    /// holds, named on its own for a caller that has a name and no ticket yet.
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

/// How many connections this core has live under one key — what
/// [`PoolBounds::max`] is the ceiling on.
#[derive(Debug)]
struct Live {
    /// The [`Ticket::key`] they were admitted under.
    key: String,
    /// How many are out under it. Never `0`: the entry is removed instead, so
    /// a core that finished with a key holds no trace of it.
    count: u32,
}

thread_local! {
    /// This core's live connections, counted per key.
    ///
    /// A `Vec` for the same reason the idle store is one — a deployment names
    /// a handful of blocks, and the key is already in hand at every call, so a
    /// linear scan beats hashing it. Unlike that store this one holds no
    /// connection: a slot is a number, and the connection it stands for is the
    /// request's until the request ends.
    static LIVE: RefCell<Vec<Live>> = const { RefCell::new(Vec::new()) };
}

/// One live connection's slot under a key: § 13's `max`, held.
///
/// A lease is the *right* to have a connection open on this core under
/// [`Ticket::key`], taken before the connection exists — by [`admit`], which
/// is where the ceiling is enforced — and given back when the connection is
/// released or closed. [`take`] draws against one, so every connection under a
/// key is counted and not only the ones the pool itself supplied; the module
/// doc's *What `max` counts* owns why that is what makes `cores × max` true.
///
/// It is carried beside the connection for the request's whole life — a
/// [`Ctx`](crate::Ctx) files it with the connection and [`release`] consumes it
/// — so a request that fails between admission and the handshake gives its slot
/// back through this type's [`Drop`] rather than through a path someone has to
/// remember to write. That is also why it is not [`Clone`]: a second copy would
/// be a second slot nobody paid for.
#[derive(Debug)]
pub struct Lease {
    /// The key, the bounds and the generation this slot is held under.
    ticket: Ticket,
}

impl Lease {
    /// The pool key this slot is held under.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.ticket.key
    }

    /// The bounds `nvs_config::db::pool_for` resolved for that key at boot.
    #[must_use]
    pub fn bounds(&self) -> &PoolBounds {
        &self.ticket.bounds
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        if !self.ticket.bounds.enabled {
            return;
        }
        LIVE.with_borrow_mut(|live| {
            let Some(at) = live.iter().position(|entry| entry.key == self.ticket.key) else {
                return;
            };
            // The entry goes rather than sitting at zero: a key names one
            // configuration generation, and a long-running core reloading its
            // config would otherwise accumulate one dead counter per reload.
            if live[at].count <= 1 {
                live.remove(at);
            } else {
                live[at].count -= 1;
            }
        });
    }
}

/// A slot for one live connection under `ticket`, or `None` because the key
/// already holds [`PoolBounds::max`] of them on this core.
///
/// **Taken before the connection is opened**, so that `max` bounds handshakes
/// and not just the pool's own hand-outs. A caller that has a lease may
/// [`take`] a warm connection under it or open a fresh one, and owes the slot
/// back either way — which costs it nothing to remember, because the [`Lease`]
/// gives it back when it drops.
///
/// `None` is the ceiling reached and nothing else. What a request does about it
/// is the caller's: § 13's [`PoolBounds::acquire`] is how long it may wait for
/// a slot, and until that lands the answer is immediate — the module doc's
/// *The bound this module does not read yet*.
///
/// A key whose pool is off (§ 13's `pool = false`) has no ceiling and is not
/// counted: that switch restores connect-per-request *exactly*, and a limit the
/// old behaviour never had would not be that.
#[must_use]
pub fn admit(ticket: Ticket) -> Option<Lease> {
    if !ticket.bounds.enabled {
        return Some(Lease { ticket });
    }
    let admitted = LIVE.with_borrow_mut(|live| {
        match live.iter_mut().find(|entry| entry.key == ticket.key) {
            Some(entry) if entry.count >= ticket.bounds.max => false,
            Some(entry) => {
                entry.count += 1;
                true
            }
            // `max` is never `0` — `nvs_config::db::pool_for` refuses that at
            // boot and names `pool = false` as what was meant — so the first
            // connection under a key is always admitted.
            None => {
                live.push(Live {
                    key: ticket.key.clone(),
                    count: 1,
                });
                true
            }
        }
    });
    admitted.then(|| Lease { ticket })
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
///
/// The [`Lease`] is consumed on every one of those paths, which is how the
/// slot goes back under `max` whether the connection was pooled or closed. It
/// carries the key and the bounds, so this is also the whole of what the pool
/// needs to be handed.
pub fn release(lease: Lease, now: Instant, connection: Box<dyn HeldConnection>) {
    let ticket = &lease.ticket;
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

/// The connection this core last released under `lease`'s key, or `None` for a
/// key it holds none of.
///
/// **A lease, not a bare key**, because that is what keeps § 13's `max` honest:
/// a connection may leave the idle store only into a slot [`admit`] granted, so
/// the ceiling counts every live connection under a key rather than only the
/// ones that arrived by a handshake. `None` here is the caller's cue to open
/// one instead, on the slot it is already holding.
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
pub fn take(lease: &Lease, now: Instant) -> Option<Box<dyn HeldConnection>> {
    let key = lease.key();
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

    use super::{Lease, PoolBounds, Ticket, admit, release, take};
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
            PoolBounds {
                idle,
                ..PoolBounds::DEFAULT
            },
        )
    }

    /// A ticket whose `max` is named, for the cases about the ceiling. `idle`
    /// follows it, which is the most `nvs_config::db::pool_for` would let a
    /// block write.
    fn capped(generation: &Arc<Snapshot>, name: &str, max: u32) -> Ticket {
        Ticket::for_block(
            generation,
            name,
            PoolBounds {
                max,
                idle: max,
                ..PoolBounds::DEFAULT
            },
        )
    }

    /// The slot a request holds while one connection under `[db.<name>]` is
    /// open — what a release and a take both need, and what the default `max`
    /// of 16 grants every case below without a case having to say so.
    fn lease(generation: &Arc<Snapshot>, name: &str, idle: u32) -> Lease {
        admit(ticket(generation, name, idle))
            .expect("the default `max` admits a case's connections")
    }

    // Every case below runs on its own test thread and so gets its own pool,
    // which is the same isolation a core gets and the reason none of them has
    // to clean up after itself.

    #[test]
    fn a_released_connection_comes_back_under_its_key() {
        let now = std::time::Instant::now();
        let generation = generation();
        release(lease(&generation, "main", 2), now, fake(1));

        assert_eq!(id_of(take(&lease(&generation, "other", 2), now)), None);
        assert_eq!(id_of(take(&lease(&generation, "main", 2), now)), Some(1));
        assert_eq!(id_of(take(&lease(&generation, "main", 2), now)), None);
    }

    #[test]
    fn the_warmest_connection_under_a_key_comes_back_first() {
        let now = std::time::Instant::now();
        let generation = generation();
        release(lease(&generation, "main", 4), now, fake(1));
        release(lease(&generation, "main", 4), now, fake(2));

        assert_eq!(id_of(take(&lease(&generation, "main", 4), now)), Some(2));
        assert_eq!(id_of(take(&lease(&generation, "main", 4), now)), Some(1));
    }

    #[test]
    fn a_release_past_the_idle_bound_closes_the_connection() {
        let now = std::time::Instant::now();
        let generation = generation();
        release(lease(&generation, "main", 1), now, fake(1));
        release(lease(&generation, "main", 1), now, fake(2));

        assert_eq!(id_of(take(&lease(&generation, "main", 1), now)), Some(1));
        assert_eq!(id_of(take(&lease(&generation, "main", 1), now)), None);
    }

    #[test]
    fn the_idle_bound_is_per_key_and_not_per_core() {
        let now = std::time::Instant::now();
        let generation = generation();
        release(lease(&generation, "main", 1), now, fake(1));
        release(lease(&generation, "reports", 1), now, fake(2));

        assert_eq!(id_of(take(&lease(&generation, "main", 1), now)), Some(1));
        assert_eq!(id_of(take(&lease(&generation, "reports", 1), now)), Some(2));
    }

    #[test]
    fn two_configuration_generations_are_two_pools() {
        let now = std::time::Instant::now();
        let before = generation();
        let after = generation();
        release(lease(&before, "main", 2), now, fake(1));

        // ADR 0078's reload can put a different database user behind the same
        // block name, so the name alone must not reach the old connection.
        assert_eq!(id_of(take(&lease(&after, "main", 2), now)), None);
        assert_eq!(id_of(take(&lease(&before, "main", 2), now)), Some(1));
    }

    #[test]
    fn pool_false_stores_nothing() {
        let now = std::time::Instant::now();
        let generation = generation();
        let off = || {
            admit(Ticket::for_block(&generation, "main", PoolBounds::OFF))
                .expect("a pool that is off has no ceiling to refuse at")
        };
        release(off(), now, fake(1));

        assert_eq!(id_of(take(&off(), now)), None);
    }

    #[test]
    fn a_connection_the_driver_cannot_prove_clean_is_closed() {
        let now = std::time::Instant::now();
        let generation = generation();
        release(
            lease(&generation, "main", 2),
            now,
            Box::new(Fake {
                id: 1,
                poolable: false,
            }),
        );

        assert_eq!(id_of(take(&lease(&generation, "main", 2), now)), None);
    }

    #[test]
    fn a_connection_past_its_lifetime_is_never_handed_out() {
        let now = std::time::Instant::now();
        let generation = generation();
        let held = lease(&generation, "main", 2);
        let expired = now + held.bounds().lifetime + Duration::from_secs(1);
        release(held, now, fake(1));

        assert_eq!(id_of(take(&lease(&generation, "main", 2), expired)), None);
        // Retired by that scan rather than left for the next one to trip over.
        assert_eq!(id_of(take(&lease(&generation, "main", 2), now)), None);
    }

    #[test]
    fn a_request_teardown_releases_what_it_held() {
        let now = std::time::Instant::now();
        let generation = generation();

        let mut ctx = crate::Ctx::stdout();
        ctx.hold_open_connection(
            Some("main".to_owned()),
            Some(lease(&generation, "main", 2)),
            fake(1),
        );
        // Still the request's while the request is running: § 13 releases at
        // teardown and never before.
        assert_eq!(id_of(take(&lease(&generation, "main", 2), now)), None);
        drop(ctx);

        assert_eq!(id_of(take(&lease(&generation, "main", 2), now)), Some(1));
    }

    #[test]
    fn a_connection_filed_with_no_lease_is_closed_with_the_request() {
        let now = std::time::Instant::now();
        let generation = generation();

        let mut ctx = crate::Ctx::stdout();
        ctx.hold_open_connection(None, None, fake(1));
        drop(ctx);

        assert_eq!(id_of(take(&lease(&generation, "main", 2), now)), None);
    }

    #[test]
    fn a_pool_is_per_core_and_shared_with_no_other() {
        let now = std::time::Instant::now();
        let generation = generation();
        release(lease(&generation, "main", 2), now, fake(1));

        let elsewhere_generation = Arc::clone(&generation);
        let elsewhere =
            std::thread::spawn(move || id_of(take(&lease(&elsewhere_generation, "main", 2), now)))
                .join()
                .expect("the probe thread does not panic");

        assert_eq!(elsewhere, None);
        assert_eq!(id_of(take(&lease(&generation, "main", 2), now)), Some(1));
    }

    #[test]
    fn a_key_at_its_max_admits_nothing_more() {
        let generation = generation();
        let held: Vec<_> = (0..2)
            .map(|_| admit(capped(&generation, "main", 2)).expect("both are under the ceiling"))
            .collect();

        assert!(admit(capped(&generation, "main", 2)).is_none());
        drop(held);
        // The slots come back with the connections they stood for, so the
        // ceiling bounds what is live and never what a core has served.
        assert!(admit(capped(&generation, "main", 2)).is_some());
    }

    #[test]
    fn the_ceiling_counts_a_warm_connection_and_a_fresh_one_alike() {
        let now = std::time::Instant::now();
        let generation = generation();
        release(
            admit(capped(&generation, "main", 1)).expect("the first is under the ceiling"),
            now,
            fake(1),
        );

        // One connection is idle and no slot is held, so the next request is
        // admitted and draws that connection rather than opening one.
        let first = admit(capped(&generation, "main", 1)).expect("the released slot came back");
        assert_eq!(id_of(take(&first, now)), Some(1));
        // Holding it is what `max` counts: a warm connection is not a free one,
        // which is why `live + idle` can never exceed the bound either.
        assert!(admit(capped(&generation, "main", 1)).is_none());
    }

    #[test]
    fn the_max_bound_is_per_key_and_not_per_core() {
        let generation = generation();
        let main = admit(capped(&generation, "main", 1)).expect("the key's one slot");
        let reports =
            admit(capped(&generation, "reports", 1)).expect("another key has its own ceiling");

        assert!(admit(capped(&generation, "main", 1)).is_none());
        drop((main, reports));
    }

    #[test]
    fn a_slot_is_given_back_by_a_request_that_never_opened_a_connection() {
        let generation = generation();
        // A handshake that failed after the admission: the lease is dropped
        // with the connection it was granted for never existing.
        drop(admit(capped(&generation, "main", 1)).expect("the key's one slot"));

        assert!(admit(capped(&generation, "main", 1)).is_some());
    }

    #[test]
    fn pool_false_has_no_ceiling_at_all() {
        let generation = generation();
        let bounds = PoolBounds {
            max: 1,
            ..PoolBounds::OFF
        };
        // § 13's `pool = false` restores connect-per-request *exactly*, and
        // that behaviour never had a ceiling to arrive at.
        let held: Vec<_> = (0..4)
            .map(|_| {
                admit(Ticket::for_block(&generation, "main", bounds))
                    .expect("a pool that is off refuses nothing")
            })
            .collect();

        assert_eq!(held.len(), 4);
    }
}
