//! The hand-off between cores —
//! [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md) § 4's "a
//! publish from a connection on core 3 reaches subscribers on core 0", as the
//! one place the thread-per-core design is crossed on purpose.
//!
//! [`crate::topic`] owns the bus a program sees: the per-core subscriber table,
//! § 4's three members, and the copy each subscriber is handed. This module is
//! the transport underneath it and knows nothing about either. What crosses a
//! core boundary here is a topic's **name** and the **bytes**
//! [ADR 0023](/docs/adr/0023-clone-serialize-and-cross-boundary-copy.md) § 2's
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
//! § 4's priority-1 rule is that a publisher is never blocked and no queue
//! grows without bound. A mailbox therefore holds [`MAILBOX_CAP`] envelopes and
//! refuses the next one, and a refused envelope's subscribers are **not**
//! counted — so a publish answers what it actually queued. A core cannot reach
//! that cap while it is running, because it drains its whole mailbox at every
//! `receive()`; reaching it means that core has not run at all for a very long
//! time, and § 4's answer for a subscriber in that state is to close it, which
//! is the per-subscriber bound `crate::topic` owes and not this one.
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
    /// ADR 0023 § 2's encoding of the published value.
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
    fn open() -> Self {
        let mine = Arc::new(Mailbox::default());
        cores_mut().push(Arc::clone(&mine));
        Self(mine)
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
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
    let payload: Arc<[u8]> = Arc::from(payload);
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
                payload: Arc::clone(&payload),
            });
            queued += listening;
        }
        queued
    })
    .unwrap_or(0)
}

/// Everything other cores have handed this one since the last drain.
///
/// The whole queue rather than one envelope: the drain runs on a connection's
/// own stack at a point where it is about to wait anyway, and leaving an
/// envelope behind would make a delivery's latency depend on how many other
/// connections happen to call `receive()`.
pub(crate) fn take_all() -> Vec<Envelope> {
    with_mine(|mine| queue(mine).drain(..).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{MAILBOX_CAP, hand_off, note_subscribers, subscribers_elsewhere, take_all};

    /// A core does not hand an envelope to itself: the publisher's own
    /// subscribers are `crate::topic`'s local walk, and counting them twice is
    /// what an unfiltered registry would do.
    #[test]
    fn a_core_is_never_its_own_neighbour() {
        note_subscribers("bus:self", 3);
        assert_eq!(subscribers_elsewhere("bus:self"), 0);
        assert_eq!(hand_off("bus:self", vec![1, 2, 3]), 0);
        assert!(take_all().is_empty());
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
            let taken = take_all().len();
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
}
