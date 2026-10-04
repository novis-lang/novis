//! `rule:security/db-pool-reset-is-a-boundary`'s connection pool: where a
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
//! # `acquire` is a queue, and a slot is handed over rather than freed
//!
//! [`PoolBounds::acquire`] is how long a request may wait at that ceiling
//! before it throws. [`admit`] on its own is the `acquire = 0` answer — a value
//! § 13 makes legal and defines as refusing rather than queueing — and [`queue`]
//! is every other value: it joins the caller to a per-key line, [`Waiting::slot`]
//! is what a woken task re-asks, and dropping the [`Waiting`] leaves the line.
//!
//! **The parking is the caller's, and deliberately.** Giving a core back needs a
//! scheduler, which this crate does not have and does not grow one for:
//! `nvs-stdlib` holds the loop and parks through [`crate::host`], the same
//! inversion `Core\Channel` already waits on. What lives here is the part that
//! has to be exactly as long as the pool it bounds — the line, and the
//! hand-over.
//!
//! **A [`Lease`] that ends gives its slot to the longest-waiting task under its
//! key rather than to the count.** The count does not dip across that hand-over,
//! so a request arriving between the wake and the woken task's resumption finds
//! the key still full and joins the back of the line instead of taking a slot
//! that was already spoken for. That is what makes `acquire` a queue rather than
//! a scramble: a request that waited its `acquire` out did so because the pool
//! genuinely never had a slot for it, and not because it kept losing a race it
//! was never told it was in. It costs nothing to enforce — [`admit`] is
//! unchanged, and the count it already reads is the whole of the rule.
//!
//! # What it spends
//!
//! Up to `idle` connections per key per core — a socket, a TLS session and a
//! statement cache each — held open between the requests that use them. That is
//! the footprint § 13 spends to buy priority 3, and it is O(cores × keys)
//! rather than O(requests served): a release past `idle` closes the connection
//! instead of queueing it, so nothing accumulates with traffic.
//!
//! **`keys` is the operator's count for `connect` and the program's for
//! `open`**, which is the one place that bound needs reading carefully: a
//! [`Ticket::for_block`] key can only be one of the config's `[db.<name>]`
//! blocks, while a [`Ticket::for_settings`] key is a hash of fields a program
//! chooses, and `database`, `user` and `password` all accept `tainted`. What
//! keeps the second finite is that every [`take`] retires *every* expired entry
//! it walks past and not only the ones under its own key, so the store holds
//! `idle` per key opened within one `lifetime` and not one per key ever seen.
//!
//! The count above adds one entry per key a core has a live connection under —
//! a string and a number, dropped when the last of them goes home, so it is
//! O(keys in use) and not O(connections) or O(generations seen).
//!
//! The line adds one entry per task *currently waiting* — a string, a number and
//! a boxed Rust closure — and nothing per task that is not. It is empty on every
//! deployment whose `max` fits its traffic, which is the shape `max` is sized
//! for, and it is O(in-flight) rather than O(requests served) in the shape that
//! is not.

use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::time::Instant;

use nvs_config::db::PoolBounds;
use nvs_config::snapshot::Snapshot;

use crate::host::Waker;

use crate::ctx::HeldConnection;

/// What a held connection needs to rejoin the pool: the key § 13 pools it
/// under, and the bounds that key resolved to at boot.
///
/// Carried beside the connection for the request's whole life rather than
/// re-derived at teardown, because the crate that knows a block's name is
/// `nvs-stdlib` and the crate that learns when a request ends is this one.
/// [`PoolBounds`] is `Copy` and owns nothing, so a ticket is one allocation —
/// the key — per connection a request opens.
#[derive(Clone, Debug)]
pub struct Ticket {
    /// § 2's key: a block's name scoped to its generation for
    /// [`Ticket::for_block`], and the settings hash itself for
    /// [`Ticket::for_settings`]. Two requests share a pooled connection exactly
    /// when this string matches, so it must carry every credential.
    pub key: String,
    /// The bounds `nvs_config::db::pool_for` resolved for that key at boot.
    pub bounds: PoolBounds,
    /// The configuration generation [`key`](Ticket::key) names, held for as
    /// long as a connection opened under it may still be pooled — and `None`
    /// for a key that names no generation at all.
    ///
    /// Nothing reads it. It is here so that the generation's address, which is
    /// what makes a [`Ticket::for_block`] key unique, cannot be reused by a
    /// later generation while a connection is still keyed on it — the alias
    /// [`Ticket::key_for`] would otherwise have. A settings hash carries no
    /// address, so there is nothing for a [`Ticket::for_settings`] key to hold
    /// alive; that constructor's own doc owns why that is not the same hole.
    #[expect(
        dead_code,
        reason = "held for its address, which the key already carries — a read \
                  would be the bug, not the silence"
    )]
    generation: Option<Arc<Snapshot>>,
}

impl Ticket {
    /// The ticket for `[db.<name>]` as `generation` declares it.
    #[must_use]
    pub fn for_block(generation: &Arc<Snapshot>, name: &str, bounds: PoolBounds) -> Ticket {
        Ticket {
            key: Ticket::key_for(generation, name),
            bounds,
            generation: Some(Arc::clone(generation)),
        }
    }

    /// The ticket for one `Core\Db::open` settings object, under the key § 2
    /// already hashes out of every field of it — `nvs_stdlib::db`'s
    /// `settings_key`, which owns which fields those are and why the hash
    /// cannot collide with a block's name.
    ///
    /// **This key is deliberately not generation-scoped**, where
    /// [`Ticket::for_block`]'s is. The two answer the same question — may these
    /// two requests share one connection? — from opposite sides. A block's name
    /// is an *indirection* an operator can repoint, so § 1's reload can publish
    /// different credentials under an unchanged name and only the generation
    /// tells the two apart; a settings object *is* the credentials, written by
    /// the program, so two settings objects that hash alike name the same endpoint as
    /// the same user whichever generation was live when each was written.
    /// Scoping it anyway would retire warm connections on every reload that are
    /// still exactly what the next request asked for. What a reload genuinely
    /// changes about an `open` — whether `db.open` still grants that host — is
    /// asked again on every call, ahead of the draw, so it is not a question
    /// the key was ever answering.
    #[must_use]
    pub fn for_settings(key: String, bounds: PoolBounds) -> Ticket {
        Ticket {
            key,
            bounds,
            generation: None,
        }
    }

    /// The pool key `[db.<name>]` has in `generation` — what [`Ticket::key`]
    /// holds, named on its own for a caller that has a name and no ticket yet.
    ///
    /// **A block's name identifies its credentials only within one
    /// configuration.** § 13 keys a pool on every credential and names the
    /// block's name as that key for `connect`, which holds because the block is
    /// where the credentials are written — until
    /// `rule:config/the-config-is-an-immutable-snapshot`
    /// 's reload publishes a `[db.main]` naming a different database user
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

/// One task waiting for a slot under a key: § 13's `acquire`, from the inside.
struct Waiter {
    /// The [`Ticket::key`] it is waiting under.
    key: String,
    /// This registration's own number, so a drop removes *its* entry and not
    /// whichever one happens to be at the same index.
    id: u64,
    /// What makes the waiting task runnable again, fired once by the [`Lease`]
    /// that hands this waiter its slot and taken at that same moment — so a
    /// second lease ending cannot find it and hand the slot over twice.
    waker: Option<Waker>,
    /// Whether a lease that ended has already handed this waiter its slot. The
    /// slot stays counted in [`LIVE`] across the hand-over, which is what keeps
    /// a newcomer's [`admit`] from taking it.
    granted: bool,
}

thread_local! {
    /// Every task on this core waiting for a slot, oldest first — the order is
    /// the queue itself, which is why this is a `Vec` and not a map even more
    /// plainly than the stores above: `max` is sized so that this is empty.
    static WAITING: RefCell<Vec<Waiter>> = const { RefCell::new(Vec::new()) };
    /// The next registration's number, never reused within a core's life.
    static TICKETS: Cell<u64> = const { Cell::new(0) };
}

/// Counts one more live connection under `ticket`'s key, or reports that the
/// key already holds [`PoolBounds::max`] of them on this core.
///
/// The whole of the ceiling, in one function because [`admit`] and
/// [`Waiting::slot`] ask the same question from either side of the queue.
fn grant(live: &mut Vec<Live>, ticket: &Ticket) -> bool {
    if !ticket.bounds.enabled {
        return true;
    }
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
}

/// One live connection's slot under a key: § 13's `max`, held.
///
/// A lease is the *right* to have a connection open on this core under
/// [`Ticket::key`], taken before the connection exists — by [`admit`], which
/// is where the ceiling is enforced — and given back when the connection is
/// released or closed, or handed straight to a task waiting under the same key
/// ([`queue`]). [`take`] draws against one, so every connection under a
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
        // The slot goes to the longest-waiting task under this key before it
        // goes back to the count, and the count does not dip while it travels:
        // the module doc's *`acquire` is a queue* owns why handing over rather
        // than freeing-and-waking is what makes the line a line.
        let handed = WAITING.with_borrow_mut(|waiting| {
            let waiter = waiting
                .iter_mut()
                .find(|waiter| waiter.key == self.ticket.key && !waiter.granted)?;
            // A waiter is registered with its wake and loses it only here, so
            // ungranted-and-wakeless is a state it cannot be in; taking the
            // wake first anyway is what keeps that true of the entry as well.
            let wake = waiter.waker.take();
            waiter.granted = wake.is_some();
            wake
        });
        // Fired outside the borrow: a wake queues a task id on the scheduler's
        // tree and reaches nothing here, but a store that calls out from inside
        // its own borrow is one edit away from a panic.
        if let Some(wake) = handed {
            wake();
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
/// `None` is the ceiling reached and nothing else, and it is the whole of
/// § 13's `acquire = 0`. A caller that may wait longer than that joins the line
/// with [`queue`] instead; a caller that may not — or that has no task under it
/// to park — reads this `None` as the refusal.
///
/// **A queued waiter is never barged past**, and that needs no test here: a
/// [`Lease`] hands its slot to the head of the line without the count dipping,
/// so a key with anyone waiting is a key this function still finds full.
///
/// A key whose pool is off (§ 13's `pool = false`) has no ceiling and is not
/// counted: that switch restores connect-per-request *exactly*, and a limit
/// connect-per-request does not have would not be that.
#[must_use]
pub fn admit(ticket: Ticket) -> Option<Lease> {
    if !ticket.bounds.enabled {
        return Some(Lease { ticket });
    }
    LIVE.with_borrow_mut(|live| grant(live, &ticket))
        .then(|| Lease { ticket })
}

/// A place in the line for a slot under one key: § 13's `acquire`, held.
///
/// [`queue`] takes one, [`Waiting::slot`] is what the caller re-asks after every
/// wake, and dropping it leaves the line — including handing on a slot that was
/// granted to a task which has stopped waiting for it, so a deadline that passed
/// at the wrong moment cannot strand one.
///
/// The waiting itself is the caller's, for the reason the module doc gives; what
/// this is, is the registration that makes the wait finite and fair.
#[derive(Debug)]
pub struct Waiting {
    /// This registration's number in [`WAITING`].
    id: u64,
    /// The ticket a granted slot would be held under, taken when it becomes a
    /// [`Lease`] and `None` after that — which is also how [`Drop`] knows there
    /// is nothing left to hand on.
    ticket: Option<Ticket>,
}

/// Joins the line for a slot under `ticket`'s key, firing `waker` when one is
/// handed over.
///
/// **Taken before the caller parks**, which is [`crate::host::Host::waker`]'s
/// own rule read on this queue: a registration made after the last look at the
/// pool could miss the very hand-over it is about to wait for, and that is the
/// one way this becomes a hang rather than a wait.
///
/// It does not itself check whether a slot is free — [`Waiting::slot`] is that
/// question, and asking it *after* joining the line is what closes the gap
/// above. A caller therefore queues first and looks second, every time.
pub fn queue(ticket: Ticket, waker: Waker) -> Waiting {
    let id = TICKETS.with(|next| {
        let id = next.get();
        next.set(id + 1);
        id
    });
    WAITING.with_borrow_mut(|waiting| {
        waiting.push(Waiter {
            key: ticket.key.clone(),
            id,
            waker: Some(waker),
            granted: false,
        });
    });
    Waiting {
        id,
        ticket: Some(ticket),
    }
}

impl Waiting {
    /// The slot this waiter has now, or `None` because the key is still full.
    ///
    /// Two ways to have one, and the caller cannot tell them apart because
    /// nothing turns on it: a lease that ended handed this waiter its slot, or
    /// the key has room and nobody was waiting ahead to be handed it. The second
    /// is what makes the first call to this — the one right after [`queue`] —
    /// the acquire that usually succeeds.
    ///
    /// Answers `Some` at most once: the registration is off the line after it,
    /// and the [`Lease`] is now the only thing holding the slot.
    #[must_use]
    pub fn slot(&mut self) -> Option<Lease> {
        let ticket = self.ticket.as_ref()?;
        let granted = WAITING.with_borrow(|waiting| {
            waiting
                .iter()
                .any(|waiter| waiter.id == self.id && waiter.granted)
        });
        // A granted slot is already counted, so it is taken rather than
        // re-granted; the ceiling would otherwise be off by every hand-over.
        if !granted && !LIVE.with_borrow_mut(|live| grant(live, ticket)) {
            return None;
        }
        self.leave();
        self.ticket.take().map(|ticket| Lease { ticket })
    }

    /// Takes this registration off the line, if it is still on it.
    fn leave(&self) {
        WAITING.with_borrow_mut(|waiting| {
            if let Some(at) = waiting.iter().position(|waiter| waiter.id == self.id) {
                waiting.remove(at);
            }
        });
    }
}

impl Drop for Waiting {
    fn drop(&mut self) {
        let granted = WAITING.with_borrow(|waiting| {
            waiting
                .iter()
                .find(|waiter| waiter.id == self.id)
                .is_some_and(|waiter| waiter.granted)
        });
        self.leave();
        // A slot handed to a task that has stopped waiting — its deadline came
        // up between the hand-over and its resumption, or it was cancelled — is
        // still a slot somebody holds under `max`. Turning it back into a lease
        // and dropping it is how it reaches the next waiter, or the count, by
        // the one path that does either.
        if granted && let Some(ticket) = self.ticket.take() {
            drop(Lease { ticket });
        }
    }
}

/// Releases `connection` to this core's pool, or closes it here.
///
/// It is closed rather than pooled when § 13 says it must be: the pool is off
/// for that key (`pool = false`), the driver cannot prove the wire clean
/// ([`HeldConnection::is_poolable`]), or the key already holds
/// [`PoolBounds::idle`] connections doing nothing. Each of them drops the box
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

    use super::{Lease, PoolBounds, Ticket, Waiting, admit, queue, release, take};
    use crate::ctx::HeldConnection;

    /// Joins the line under `ticket`'s key with a wake that counts its firings.
    ///
    /// A `Waker` is a `Box<dyn FnOnce()>` and nothing more, so a case needs no
    /// scheduler to be a waiting task here — which is the seam doing its job:
    /// the pool's half of `acquire` is the line and the hand-over, and neither
    /// of those is a park.
    fn waiter(ticket: Ticket) -> (Waiting, std::rc::Rc<std::cell::Cell<u32>>) {
        let woken = std::rc::Rc::new(std::cell::Cell::new(0));
        let counter = std::rc::Rc::clone(&woken);
        let waiting = queue(ticket, Box::new(move || counter.set(counter.get() + 1)));
        (waiting, woken)
    }

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
    /// grants every case below without a case having to say so.
    fn lease(generation: &Arc<Snapshot>, name: &str, idle: u32) -> Lease {
        admit(ticket(generation, name, idle))
            .expect("the default `max` admits a case's connections")
    }

    // Every case below runs on its own test thread and so gets its own pool,
    // which is the same isolation a core gets and the reason none of them has
    // to clean up after itself.

    /// § 13's other key: the hash `Core\Db::open` computes over its settings,
    /// carried verbatim.
    ///
    /// Two claims in one case because they are one property — the settings key
    /// space and the block key space do not meet, in either direction. The
    /// block half is a name spelled *as* the settings key, which is the closest
    /// a `[db.<name>]` block can get to one and still be a name; the settings
    /// half is the same hash asked for twice, which is what a request in a
    /// reloaded generation does, since [`Ticket::for_settings`] has no
    /// generation to differ in and the literal it hashed named the same
    /// credentials either way.
    #[test]
    fn a_settings_key_is_its_own_pool_and_survives_a_reload() {
        let now = std::time::Instant::now();
        let generation = generation();
        let key = String::from("\u{0}open:00000000deadbeef");
        let opened = admit(Ticket::for_settings(key.clone(), PoolBounds::DEFAULT)).unwrap();
        release(opened, now, fake(1));

        let block = admit(ticket(&generation, &key, 2)).unwrap();
        assert_eq!(id_of(take(&block, now)), None);
        drop(block);

        let reloaded = admit(Ticket::for_settings(key, PoolBounds::DEFAULT)).unwrap();
        assert_eq!(id_of(take(&reloaded, now)), Some(1));
    }

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

        // `rule:config/the-config-is-an-immutable-snapshot`'s reload can put a different database user behind the same
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
        // connect-per-request has no ceiling to arrive at.
        let held: Vec<_> = (0..4)
            .map(|_| {
                admit(Ticket::for_block(&generation, "main", bounds))
                    .expect("a pool that is off refuses nothing")
            })
            .collect();

        assert_eq!(held.len(), 4);
    }

    #[test]
    fn a_key_with_room_hands_the_first_look_a_slot() {
        let generation = generation();
        let (mut waiting, woken) = waiter(capped(&generation, "main", 1));

        // Queue first, look second — the ordering `queue` requires — and the
        // usual answer to that first look is a slot, because a caller only
        // reaches the line once `admit` has already said the key is full.
        let taken = waiting.slot().expect("nothing was holding the one slot");
        assert_eq!(woken.get(), 0, "nobody had to be woken to grant it");
        assert!(
            admit(capped(&generation, "main", 1)).is_none(),
            "the slot the waiter took counts under `max` like any other"
        );
        drop(taken);
    }

    #[test]
    fn a_lease_that_ends_hands_its_slot_to_the_task_waiting_for_it() {
        let generation = generation();
        let held = admit(capped(&generation, "main", 1)).expect("the first is under the ceiling");
        let (mut waiting, woken) = waiter(capped(&generation, "main", 1));

        assert!(waiting.slot().is_none(), "the one slot is out");
        drop(held);

        assert_eq!(woken.get(), 1, "the lease that ended woke the waiter");
        assert!(waiting.slot().is_some(), "and handed it the slot");
    }

    #[test]
    fn a_newcomer_cannot_take_the_slot_a_waiter_is_owed() {
        let generation = generation();
        let held = admit(capped(&generation, "main", 1)).expect("the first is under the ceiling");
        let (mut waiting, _woken) = waiter(capped(&generation, "main", 1));
        drop(held);

        // The count does not dip across the hand-over, so a request arriving
        // between the wake and the woken task's resumption still finds the key
        // full — `acquire` is a queue and not a scramble.
        assert!(admit(capped(&generation, "main", 1)).is_none());
        assert!(waiting.slot().is_some());
    }

    #[test]
    fn the_line_is_served_oldest_first() {
        let generation = generation();
        let held = admit(capped(&generation, "main", 1)).expect("the first is under the ceiling");
        let (mut first, _first_woken) = waiter(capped(&generation, "main", 1));
        let (mut second, second_woken) = waiter(capped(&generation, "main", 1));
        drop(held);

        assert_eq!(second_woken.get(), 0, "the older waiter is served first");
        let first = first.slot().expect("the head of the line has the slot");
        assert!(second.slot().is_none());

        drop(first);
        assert_eq!(second_woken.get(), 1);
        assert!(second.slot().is_some());
    }

    #[test]
    fn a_waiter_that_stops_waiting_hands_its_slot_on() {
        let generation = generation();
        let held = admit(capped(&generation, "main", 1)).expect("the first is under the ceiling");
        let (first, _first_woken) = waiter(capped(&generation, "main", 1));
        let (mut second, second_woken) = waiter(capped(&generation, "main", 1));
        drop(held);

        // The deadline came up between the hand-over and the resumption, which
        // is the one moment a granted slot has no lease holding it. It is still
        // a slot somebody holds under `max`, so leaving the line passes it on.
        drop(first);

        assert_eq!(second_woken.get(), 1);
        assert!(second.slot().is_some());
    }

    #[test]
    fn a_waiter_that_leaves_before_a_slot_comes_free_is_off_the_line() {
        let generation = generation();
        let held = admit(capped(&generation, "main", 1)).expect("the first is under the ceiling");
        let (waiting, woken) = waiter(capped(&generation, "main", 1));
        drop(waiting);
        drop(held);

        assert_eq!(woken.get(), 0, "nothing wakes a task that stopped waiting");
        // With nobody left in the line the slot goes back to the count, which
        // is the path a pool with no waiting on it takes every time.
        assert!(admit(capped(&generation, "main", 1)).is_some());
    }

    #[test]
    fn a_line_is_per_key_and_not_per_core() {
        let generation = generation();
        let held = admit(capped(&generation, "main", 1)).expect("the first is under the ceiling");
        let (mut waiting, woken) = waiter(capped(&generation, "main", 1));

        // A lease ending under `[db.other]` is not this waiter's slot, however
        // full its own key is.
        drop(admit(capped(&generation, "other", 1)).expect("`other` holds none"));

        assert_eq!(woken.get(), 0);
        assert!(waiting.slot().is_none());
        drop(held);
        assert!(waiting.slot().is_some());
    }
}
