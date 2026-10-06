//! The hand-off between cores —
//! `rule:core-classes/topic`'s "a
//! publish from a connection on core 3 reaches subscribers on core 0", as the
//! one place the thread-per-core design is crossed on purpose.
//!
//! [`crate::topic`] owns the bus a program sees: the per-core subscriber table,
//! § 4's three members, and the copy each subscriber is handed. This module is
//! the transport underneath it and knows nothing about either. What crosses a
//! core boundary here is a topic's **name** and the **bytes**
//! `rule:classes/graph-copy`'s
//! [`nvs_runtime::encode`] made of the published value — never a
//! [`nvs_runtime::Value`], which is refcounted on the core that made it, so a
//! second core touching that count is exactly the data race the thread-per-core
//! design exists not to have. The receiving core reads the bytes back with
//! `decode` against its own class table, which is the same carrier
//! [`crate::cache`]'s shared tier crosses a process boundary with.
//!
//! # Decision: a registry of mailboxes, not one shared subscriber table
//!
//! § 4 says the crossing is "a bounded message hand-off rather than shared
//! state", and this is that sentence as a structure. The **subscriber table
//! stays per core** and is never read from another one; what is shared is a
//! `Vec` of mailboxes, one per core that has touched the bus, each holding a
//! queue of envelopes and the count of live subscribers its own core last saw
//! per topic. A publish reads the counts, queues one envelope per interested
//! core, and returns; the interested core decodes and fans out on its own
//! stack, where the values it makes belong.
//!
//! The alternative — one process-wide table of every subscriber — puts a lock
//! in front of the queue every `receive()` drains, and makes a delivery an
//! allocation one core made and another releases. That is the shared state § 4
//! declines, and it is why the count a publisher answers with is read here
//! rather than the queues themselves.
//!
//! # Decision: a core registers itself, and its entry goes when the thread does
//!
//! There is no `register_this_core` for an embedder to call. A core appears in
//! the registry the first time it subscribes, publishes or drains, and its
//! entry is taken out by the [`Registration`] that a thread-local holds — so
//! the registry is O(live cores) rather than O(threads ever started), which
//! [AGENTS.md](/AGENTS.md)'s memory rule calls the difference between a bound
//! and a leak. It also means `nvs-server` needs no edge to this crate to make
//! § 4 whole across its cores: the connection isolates already running on each
//! one are what put it there.
//!
//! # Decision: the queue is bounded, and an overflow is refused rather than waited on
//!
//! § 4's security rule is that a publisher is never blocked and no queue
//! grows without bound. A mailbox therefore holds [`MAILBOX_CAP`] envelopes and
//! refuses the next one, and a refused envelope's subscribers are **not**
//! counted — so a publish answers what it actually queued. A core cannot reach
//! that cap while it is running, because it drains its whole mailbox at every
//! `receive()`; reaching it means that core has not run at all for a very long
//! time, and § 4's answer for a subscriber in that state is to close it — which
//! is the *per-subscriber* bound, one layer up: [`nvs_runtime::Inbox`] holds it
//! and `crate::topic`'s fan-out raises it. The two refusals are the same rule at
//! two granularities, and this one is deliberately the blunter: a mailbox is a
//! whole core's, so what it declines is an envelope rather than a subscriber,
//! and nobody is closed for it.
//!
//! # Decision: the bus's bytes are the process's, and each end brackets its own
//!
//! An envelope is allocated on the publishing core and freed on the receiving
//! one, so neither request may be measured by it:
//! `rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance`. Every
//! path that allocates or frees what the registry holds — a core's
//! [`Registration`], the topic counts [`note_subscribers`] writes, the envelope
//! [`hand_off`] queues and the [`Drained`] one a delivery gives back — runs
//! inside [`nvs_runtime::budget::Detached`], so those bytes move the process's
//! balance and leave every request's reading, and every ceiling armed against
//! one, where they found it.
//!
//! The two ends are two brackets rather than one because they are on two
//! threads, and the detached balance is per thread and **signed**: the
//! publisher's rises with the envelope and the receiver's falls with it, which
//! is the ordinary shape of a store one core fills and another empties rather
//! than an imbalance. What the pair owes is that a block is allocated and freed
//! on the same side of the boundary, which is why [`hand_off`] copies the bytes
//! it is handed instead of keeping the caller's `Vec` — the request frees its
//! own buffer outside the bracket, and the store frees the carrier inside one.
//!
//! # What it spends
//!
//! One [`Mailbox`] per live core — two locks and two empty collections — and,
//! per topic that core has a subscriber for, one map entry. Per publish that
//! anybody elsewhere is listening to: one [`Arc<[u8]>`](std::sync::Arc) holding
//! the encoded value once however many cores take it, and one `VecDeque` slot
//! per core. All of it is O(live cores × topics each holds) and O(in-flight
//! envelopes); none of it is O(publishes) or O(connections served). On the
//! common path — one core, or a topic nobody joined elsewhere — a publish costs
//! one read lock and no encode at all.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use nvs_runtime::budget;

/// How many envelopes one core's mailbox holds before the next is refused.
///
/// The module doc owns why an overflow is refused rather than waited on, and
/// why a core that is running cannot reach this.
const MAILBOX_CAP: usize = 1024;

/// One published value on its way to the cores that are not the publisher's.
///
/// The payload is shared rather than copied per core: it is immutable bytes,
/// and the receiving core reads it back into a value of its own.
#[derive(Clone, Debug)]
pub(crate) struct Envelope {
    /// The topic's name, as the publishing core wrote it.
    topic: Box<str>,
    /// `rule:classes/graph-copy`'s encoding of the published value.
    payload: Arc<[u8]>,
}

impl Envelope {
    /// The topic this was published to.
    pub(crate) fn topic(&self) -> &str {
        &self.topic
    }

    /// The encoded value, for the `decode` that reads it back.
    pub(crate) fn payload(&self) -> &[u8] {
        &self.payload
    }
}

/// One core's end of the bus: what other cores have handed it, and what it has
/// told them it is listening to.
#[derive(Debug, Default)]
struct Mailbox {
    /// Arrival order, drained whole. Written by every other core and read only
    /// by this one.
    queue: Mutex<VecDeque<Envelope>>,
    /// This core's live subscriber count per topic — written only by this core
    /// and read by every publisher, which is why it is an `RwLock` rather than
    /// the queue's `Mutex`.
    topics: RwLock<HashMap<Box<str>, usize>>,
}

/// Every core that has touched the bus.
///
/// A `Vec` rather than a map because it is walked far more often than it is
/// written: a publish reads all of it, and an entry is added or removed once
/// per core for the life of the process.
static CORES: RwLock<Vec<Arc<Mailbox>>> = RwLock::new(Vec::new());

/// This core's entry in [`CORES`], taken out again when the thread ends.
///
/// The module doc owns why registration is lazy and why the removal is a
/// [`Drop`] rather than something an embedder calls.
#[derive(Debug)]
struct Registration(Arc<Mailbox>);

impl Registration {
    /// Puts a fresh mailbox in the registry and keeps the handle.
    ///
    /// Bracketed because the mailbox and the registry slot naming it outlive
    /// the request whose first touch of the bus made them, and the request that
    /// happens to be running when the thread ends is not the one to be credited
    /// for them either.
    fn open() -> Self {
        let _bracket = budget::Detached::begin();
        let mine = Arc::new(Mailbox::default());
        cores_mut().push(Arc::clone(&mine));
        Self(mine)
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        let _bracket = budget::Detached::begin();
        cores_mut().retain(|core| !Arc::ptr_eq(core, &self.0));
    }
}

thread_local! {
    /// This core's mailbox, made and registered on first use.
    static MINE: Registration = Registration::open();
}

/// The registry, for reading.
///
/// A poisoned lock is taken anyway: what it guards is a list of handles, and a
/// panic on another core cannot have left it half-written — every mutation is
/// one `push` or one `retain`. Refusing to publish because an unrelated core
/// panicked would turn one connection's failure into the bus's.
fn cores() -> RwLockReadGuard<'static, Vec<Arc<Mailbox>>> {
    CORES.read().unwrap_or_else(PoisonError::into_inner)
}

/// The registry, for adding or removing this core.
fn cores_mut() -> RwLockWriteGuard<'static, Vec<Arc<Mailbox>>> {
    CORES.write().unwrap_or_else(PoisonError::into_inner)
}

/// One mailbox's queue, on the same terms [`cores`] is taken on.
fn queue(mailbox: &Mailbox) -> MutexGuard<'_, VecDeque<Envelope>> {
    mailbox.queue.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Runs `use_it` with this core's mailbox, registering it if this is the first
/// touch.
///
/// `None` once the thread-local has been destroyed, which is a core on its way
/// out: there is nothing to publish to it and nothing left of it to publish
/// from, so every caller reads that as "no mailbox" rather than as a failure.
fn with_mine<T>(use_it: impl FnOnce(&Arc<Mailbox>) -> T) -> Option<T> {
    MINE.try_with(|mine| use_it(&mine.0)).ok()
}

/// How many live subscribers `mailbox`'s core last reported for `topic`.
fn count_in(mailbox: &Mailbox, topic: &str) -> u64 {
    let topics = mailbox
        .topics
        .read()
        .unwrap_or_else(PoisonError::into_inner);
    topics
        .get(topic)
        .map_or(0, |&live| u64::try_from(live).unwrap_or(u64::MAX))
}

/// Records that this core has `live` subscribers for `topic`, which is what
/// every other core's publish counts and hands an envelope to.
///
/// Called from every place `crate::topic`'s table changes or is walked, so the
/// number is as fresh as this core's own last look at that row — the staleness
/// that leaves, and why it is the one a publish's count carries, is that
/// module's own decision.
pub(crate) fn note_subscribers(topic: &str, live: usize) {
    with_mine(|mine| {
        // The row's name is put there by one subscriber and dropped by
        // whichever one leaves last, so both ends of it are the process's.
        let _bracket = budget::Detached::begin();
        let mut topics = mine.topics.write().unwrap_or_else(PoisonError::into_inner);
        if live == 0 {
            topics.remove(topic);
        } else if let Some(held) = topics.get_mut(topic) {
            *held = live;
        } else {
            topics.insert(Box::from(topic), live);
        }
    });
}

/// How many subscribers `topic` has on cores other than this one.
///
/// Asked before the value is encoded, so a publish to a topic nobody joined
/// elsewhere — every publish on a single-core server — pays one read lock and
/// no carrier at all.
pub(crate) fn subscribers_elsewhere(topic: &str) -> u64 {
    with_mine(|mine| {
        cores()
            .iter()
            .filter(|core| !Arc::ptr_eq(core, mine))
            .map(|core| count_in(core, topic))
            .sum()
    })
    .unwrap_or(0)
}

/// Hands `payload` to every other core listening to `topic`, answering how many
/// subscribers it was queued for.
///
/// The count is that core's own, read as the envelope is queued. A core whose
/// mailbox is full takes none of it and contributes none of it, which is the
/// module doc's refusal.
pub(crate) fn hand_off(topic: &str, payload: Vec<u8>) -> u64 {
    let queued = {
        // Everything the queue comes to hold is allocated here, and the carrier
        // is a *copy* of what the caller handed over rather than that `Vec`
        // taken by the ownership it came with: the request allocated the `Vec`
        // on its own balance and has to free it on the same one, which it does
        // below, outside this block.
        let _bracket = budget::Detached::begin();
        let carrier: Arc<[u8]> = Arc::from(&payload[..]);
        with_mine(|mine| {
            let mut queued = 0;
            for core in cores().iter().filter(|core| !Arc::ptr_eq(core, mine)) {
                let listening = count_in(core, topic);
                if listening == 0 {
                    continue;
                }
                let mut waiting = queue(core);
                if waiting.len() >= MAILBOX_CAP {
                    continue;
                }
                waiting.push_back(Envelope {
                    topic: Box::from(topic),
                    payload: Arc::clone(&carrier),
                });
                queued += listening;
            }
            queued
        })
    };
    queued.unwrap_or(0)
}

/// What one drain took, and the balance it hands the envelopes back to.
///
/// They were allocated on the publishing core inside [`hand_off`]'s bracket, so
/// they are freed inside one here — a [`Drop`] rather than a call at the end of
/// the fan-out, so that a panic partway through it gives them back to the same
/// balance a return does. The fan-out itself reads them from *outside* the
/// bracket, because the values it decodes are the receiving request's own.
#[derive(Debug)]
pub(crate) struct Drained(Vec<Envelope>);

impl Drained {
    /// What the drain took, in arrival order.
    pub(crate) fn envelopes(&self) -> &[Envelope] {
        &self.0
    }
}

impl Drop for Drained {
    fn drop(&mut self) {
        let _bracket = budget::Detached::begin();
        drop(std::mem::take(&mut self.0));
    }
}

/// Everything other cores have handed this one since the last drain.
///
/// The whole queue rather than one envelope: the drain runs on a connection's
/// own stack at a point where it is about to wait anyway, and leaving an
/// envelope behind would make a delivery's latency depend on how many other
/// connections happen to call `receive()`.
///
/// Answered inside a [`Drained`] rather than as a bare `Vec` so that the store
/// frees what the store allocated, which is the module doc's second bracket.
pub(crate) fn take_all() -> Drained {
    let _bracket = budget::Detached::begin();
    Drained(with_mine(|mine| queue(mine).drain(..).collect()).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::{MAILBOX_CAP, budget, hand_off, note_subscribers, subscribers_elsewhere, take_all};

    /// A core does not hand an envelope to itself: the publisher's own
    /// subscribers are `crate::topic`'s local walk, and counting them twice is
    /// what an unfiltered registry would do.
    #[test]
    fn a_core_is_never_its_own_neighbour() {
        note_subscribers("bus:self", 3);
        assert_eq!(subscribers_elsewhere("bus:self"), 0);
        assert_eq!(hand_off("bus:self", vec![1, 2, 3]), 0);
        assert!(take_all().envelopes().is_empty());
        note_subscribers("bus:self", 0);
    }

    /// A mailbox that nobody drains stops taking envelopes rather than growing,
    /// and the subscribers behind a refused one are not counted — § 4's
    /// "no queue grows without bound", asserted on both sides of the cap.
    #[test]
    fn a_full_mailbox_refuses_the_next_envelope_and_counts_nobody_for_it() {
        let (ready, listening) = std::sync::mpsc::channel();
        let (done, stop) = std::sync::mpsc::channel::<()>();
        let neighbour = std::thread::spawn(move || {
            note_subscribers("bus:flooded", 1);
            ready.send(()).expect("the publisher is waiting");
            stop.recv().expect("the publisher says when");
            let taken = take_all().envelopes().len();
            note_subscribers("bus:flooded", 0);
            taken
        });

        listening.recv().expect("the neighbour has joined");
        let accepted: u64 = (0..=MAILBOX_CAP)
            .map(|_| hand_off("bus:flooded", vec![0]))
            .sum();
        done.send(()).expect("the neighbour is waiting");

        assert_eq!(
            accepted,
            u64::try_from(MAILBOX_CAP).expect("the cap is a small number"),
            "one publish per slot is counted and the one past the cap is not"
        );
        assert_eq!(
            neighbour.join().expect("the neighbour finished"),
            MAILBOX_CAP,
            "and the queue held exactly the cap"
        );
    }

    /// An envelope is the process's bytes at both ends, and neither the request
    /// that published it nor the one that drains it is measured by them.
    ///
    /// `rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance` asked
    /// of a store whose two ends are on two cores. The publishing core's
    /// detached balance rises with the envelope while the balance its ceiling is
    /// armed against only *falls* — by the caller's own copy, which the store
    /// does not keep — and the receiving core's detached balance falls by that
    /// same envelope while its request's reading does not move at all. A bracket
    /// on one end alone leaves one of those two halves failing.
    #[test]
    fn a_publish_is_charged_to_the_process_at_both_ends_and_to_neither_request() {
        /// The payload one envelope has to show through the noise.
        const ENTRY: usize = 256 * 1024;

        let (ready, listening) = std::sync::mpsc::channel();
        let (done, stop) = std::sync::mpsc::channel::<()>();
        let neighbour = std::thread::spawn(move || {
            note_subscribers("bus:charged", 1);
            ready.send(()).expect("the publisher is waiting");
            stop.recv().expect("the publisher says when");

            let held = budget::detached_bytes();
            let live = budget::live_bytes();
            let taken = take_all().envelopes().len();
            let given_back = held - budget::detached_bytes();
            let moved = budget::live_bytes() - live;

            note_subscribers("bus:charged", 0);
            (taken, given_back, moved)
        });

        listening.recv().expect("the neighbour has joined");
        let payload = vec![b'e'; ENTRY];
        let held = budget::detached_bytes();
        let live = budget::live_bytes();
        assert_eq!(hand_off("bus:charged", payload), 1);

        assert!(
            budget::detached_bytes() - held >= ENTRY.cast_signed(),
            "the envelope reached no balance at all, so nothing holds its bytes to the process"
        );
        assert!(
            budget::live_bytes() <= live - ENTRY.cast_signed(),
            "the publishing request was charged for the store's carrier, or credited nothing \
             for the buffer it handed over"
        );

        done.send(()).expect("the neighbour is waiting");
        let (taken, given_back, moved) = neighbour.join().expect("the neighbour finished");
        assert_eq!(taken, 1, "the envelope reached the other core");
        assert!(
            given_back >= ENTRY.cast_signed(),
            "the receiving core gave the envelope back to its own balance instead of the \
             process's, so a request that drains one is credited for bytes it never held"
        );
        assert_eq!(
            moved, 0,
            "the drain moved the balance the receiving request's ceiling is armed against"
        );
    }
}
