//! A bounded channel between tasks on one core: a `send` that suspends when it
//! is full rather than growing, and a `recv` that suspends when it is empty.
//!
//! This is the primitive `Core\Task`'s roster is assembled on — a producer and
//! a consumer that are two tasks rather than two threads, so nothing here is
//! `Send`, nothing here locks, and the only thing that ever waits is a
//! coroutine handing its core back. A [`Sender`] and a [`Receiver`] are two
//! handles onto one allocation, exactly as they are in `std::sync::mpsc`, and
//! the whole of the difference is what happens when the queue is at its bound.
//!
//! # The bound is the point
//!
//! An unbounded queue between a fast producer and a slow consumer is a leak
//! wearing a channel's clothes: its footprint is O(messages produced), which is
//! precisely what `rule:programs/memory-priority`
//! calls growth with total traffic rather than with concurrency. A bounded one
//! turns that into backpressure — the producer stops being scheduled until the
//! consumer has taken something — and what a channel can hold is then
//! `capacity` values and no more, decided once at construction and readable
//! from [`Sender::capacity`].
//!
//! What it costs is worth saying plainly: a full channel **suspends the sending
//! task**, so a producer nobody consumes from is parked indefinitely. That is
//! the same shape as every other park in this crate and it ends the same two
//! ways — the task is cancelled (§ *A cancelled waiter*), or the other end is
//! dropped, which wakes every waiter to see the disconnection.
//!
//! There is no rendezvous channel here. A `capacity` of 0 is raised to 1,
//! because a zero-capacity queue is not a smaller version of this primitive but
//! a different one — a handoff, in which the *sender* is what a receiver waits
//! on — and that is not a semantic anybody should acquire by passing a 0 they
//! computed.
//!
//! # The wake route: through the task tree, not through the scheduler
//!
//! A task that fills a channel has to wake the task waiting to read it, and it
//! cannot reach `&mut Scheduler` to call [`Scheduler::wake`] — the scheduler is
//! the frame that is resuming it. So a channel wakes the way a task spawns: it
//! holds a [`Wake`] for each parked peer, which queues the [`TaskId`](crate::scheduler::TaskId) on the
//! task tree — the half of a scheduler a running task *can* reach — and the
//! scheduler drains it and moves that task from parked to ready.
//! `scheduler`'s module doc § *The task tree* owns that split.
//!
//! The handle is taken while the waiting task is running and fired later, from
//! wherever the peer happens to be. That is what lets an end of a channel held
//! *outside* any task — an accept loop's, or a test's — wake the task waiting
//! on it, which a route reading a thread-local at firing time could not do.
//!
//! The alternative considered and rejected was the reactor's thread-local,
//! waking by token. A channel has no descriptor and no readiness: that route
//! would make every channel require a reactor to exist, and would put a wake
//! that never involves the kernel through the one mechanism whose entire
//! subject is the kernel.
//!
//! A wake goes to exactly one waiter, in the order they arrived, because one
//! `send` frees exactly one slot. Waking all of them would also be correct —
//! every waiter re-checks the state it woke for — but it spends a resume per
//! waiter to hand one value to one of them.
//!
//! # A cancelled waiter takes itself out of the queue
//!
//! A task parked on a channel is torn down like any other: it is already
//! standing on a safepoint, so [`Scheduler::run`]'s sweep over the parked set
//! force-unwinds it where it is. That unwind runs native `Drop` and no script
//! code, which is `rule:concurrency/cancellation-runs-no-user-code`
//! , and it is the whole mechanism by which a channel does not accumulate
//! the wakes of dead tasks: a registration is an RAII guard living on the
//! waiting task's own stack, so whatever ends the wait — a wake, a
//! disconnection, or a cancellation — removes it.
//!
//! That is what makes the single-waiter wake above correct rather than merely
//! tidy. If a cancelled task's registration stayed in the queue, a `send` would
//! spend its one wake on a task that can never be resumed while a live waiter
//! slept through it, and the channel would deadlock a request that did nothing
//! wrong.
//!
//! [`Scheduler::wake`]: crate::Scheduler::wake
//! [`Scheduler::run`]: crate::Scheduler::run

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::rc::Rc;

use crate::scheduler::{Waiting, Wake, suspend_current};

/// Creates a bounded channel and returns its two ends.
///
/// `capacity` is how many values the channel holds before a [`send`] suspends;
/// `0` is raised to `1`, and the module doc says why there is no rendezvous
/// channel here. Both ends are `!Send` and belong to one core.
///
/// [`send`]: Sender::send
#[must_use]
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let inner = Rc::new(Inner {
        capacity: capacity.max(1),
        state: RefCell::new(State {
            queue: VecDeque::new(),
            send_waiters: Waiters::default(),
            recv_waiters: Waiters::default(),
            senders: 1,
            receivers: 1,
        }),
    });
    (
        Sender {
            inner: Rc::clone(&inner),
        },
        Receiver { inner },
    )
}

/// The shared body of a channel. One allocation, however many handles.
struct Inner<T> {
    capacity: usize,
    state: RefCell<State<T>>,
}

/// Everything a channel operation touches, behind one borrow.
///
/// One `RefCell` over the whole state rather than one per field, so that "what
/// is true of this channel" is read and written at a single point. **No borrow
/// of it is ever held across a suspend** — the rule `net`'s parking stream
/// states as its rule 1, and for the same reason: the next task to run may be
/// the peer.
struct State<T> {
    queue: VecDeque<T>,
    send_waiters: Waiters,
    recv_waiters: Waiters,
    senders: usize,
    receivers: usize,
}

/// One side's waiters, in the order they arrived.
///
/// Each waiter is stored under a ticket, and tickets count up, so the first
/// entry is the one that has waited longest. A map rather than a list, so that
/// a [`Waiter`] taking itself out costs O(log waiters). That matters because
/// the ordinary case is a waiter that was already taken out when it was woken,
/// and a list would be read to its end to find that out.
#[derive(Default)]
struct Waiters {
    next_ticket: u64,
    queue: BTreeMap<u64, Wake>,
}

impl Waiters {
    /// Puts `wake` at the back and returns its ticket.
    fn push_back(&mut self, wake: Wake) -> u64 {
        let ticket = self.next_ticket;
        self.next_ticket += 1;
        self.queue.insert(ticket, wake);
        ticket
    }

    /// Takes the waiter that has waited longest.
    fn pop_front(&mut self) -> Option<Wake> {
        self.queue.pop_first().map(|(_, wake)| wake)
    }

    /// Takes the waiter under `ticket` out, if it is still there.
    fn remove(&mut self, ticket: u64) {
        self.queue.remove(&ticket);
    }

    /// Takes every waiter, in the order they arrived. The ticket counter is
    /// kept, so a [`Waiter`] registered before this can never remove one
    /// registered after it.
    fn take_all(&mut self) -> Vec<Wake> {
        std::mem::take(&mut self.queue).into_values().collect()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.queue.len()
    }

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    #[cfg(test)]
    fn front(&self) -> Option<&Wake> {
        self.queue.first_key_value().map(|(_, wake)| wake)
    }
}

/// Which of the two waiter queues a [`Waiter`] put itself on.
#[derive(Clone, Copy)]
enum Side {
    Send,
    Recv,
}

/// A registration on one of the waiter queues, removed when dropped.
///
/// Lives on the waiting task's stack for exactly the length of one park, so a
/// forced unwind under cancellation deregisters it as surely as an ordinary
/// wake does — the module doc's last section owns why that is not merely tidy.
struct Waiter<T> {
    inner: Rc<Inner<T>>,
    ticket: u64,
    side: Side,
}

impl<T> Waiter<T> {
    /// Puts `wake` at the back of `side`'s queue. The borrow it takes is
    /// released before this returns, because the caller's next act is to
    /// suspend.
    fn register(inner: &Rc<Inner<T>>, wake: Wake, side: Side) -> Self {
        let ticket = {
            let mut state = inner.state.borrow_mut();
            match side {
                Side::Send => state.send_waiters.push_back(wake),
                Side::Recv => state.recv_waiters.push_back(wake),
            }
        };
        Self {
            inner: Rc::clone(inner),
            ticket,
            side,
        }
    }
}

impl<T> Drop for Waiter<T> {
    fn drop(&mut self) {
        let mut state = self.inner.state.borrow_mut();
        let queue = match self.side {
            Side::Send => &mut state.send_waiters,
            Side::Recv => &mut state.recv_waiters,
        };
        // Already absent is the ordinary case: a waiter that was woken was
        // taken off the queue by whoever woke it.
        queue.remove(self.ticket);
    }
}

/// The sending end of a [`channel`].
///
/// Clone it for more than one producer; the channel disconnects when the last
/// one is dropped.
pub struct Sender<T> {
    inner: Rc<Inner<T>>,
}

/// The receiving end of a [`channel`].
///
/// Clone it for more than one consumer; the channel disconnects when the last
/// one is dropped.
pub struct Receiver<T> {
    inner: Rc<Inner<T>>,
}

/// Why a [`Sender::send`] did not send.
///
/// The value comes back in every case, because a send that did not happen must
/// not also lose what it was given.
pub enum SendError<T> {
    /// Every [`Receiver`] has been dropped, so nothing will ever read this.
    Disconnected(T),
    /// The channel is full and there is no task to suspend, because no
    /// scheduler is turning beneath this call.
    ///
    /// Waiting here would block the thread, which is the one thing this crate
    /// exists not to do (`rule:http-server/a-core-is-never-blocked-on-a-syscall`
    /// ), and unlike a socket there is nothing to poll instead: a channel is
    /// only ever drained by another task on this core, so off a core there is
    /// nobody who could make room.
    WouldBlock(T),
}

/// Why a [`Sender::try_send`] did not send.
pub enum TrySendError<T> {
    /// Every [`Receiver`] has been dropped.
    Disconnected(T),
    /// The channel is at its capacity right now.
    Full(T),
}

/// Why a [`Receiver::recv`] did not receive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecvError {
    /// The channel is empty and every [`Sender`] has been dropped.
    Disconnected,
    /// The channel is empty and there is no task to suspend — the same
    /// situation as [`SendError::WouldBlock`], from the other side.
    WouldBlock,
}

/// Why a [`Receiver::try_recv`] did not receive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TryRecvError {
    /// The channel is empty and every [`Sender`] has been dropped.
    Disconnected,
    /// The channel is empty right now.
    Empty,
}

impl<T> Sender<T> {
    /// Sends `value`, suspending this task while the channel is full.
    ///
    /// Returns once the value is in the queue. The task may be resumed more
    /// than once before that happens — a wake is a hint, so this re-checks the
    /// state it woke for rather than assuming the wake was its own.
    ///
    /// # Errors
    ///
    /// [`SendError::Disconnected`] once the last [`Receiver`] is gone, and
    /// [`SendError::WouldBlock`] for a full channel with no scheduler beneath
    /// the call. The value is handed back in both.
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        let mut value = value;
        loop {
            match self.try_send(value) {
                Ok(()) => return Ok(()),
                Err(TrySendError::Disconnected(back)) => {
                    return Err(SendError::Disconnected(back));
                }
                Err(TrySendError::Full(back)) => {
                    value = back;
                    let Some(wake) = Wake::current() else {
                        return Err(SendError::WouldBlock(value));
                    };
                    // Register, release the borrow, *then* yield — never the
                    // other way round. The guard takes the registration back
                    // off the queue however this ends, cancellation included.
                    let waiter = Waiter::register(&self.inner, wake, Side::Send);
                    let parked = suspend_current(Waiting::Parked);
                    drop(waiter);
                    // A cancellation ends the wait the same way a missing
                    // scheduler does, and for the same reason: there is nothing
                    // to wait for any more, so the value comes back to its
                    // owner rather than the caller looping to be told again.
                    if !parked.suspended() || parked.cancelled() {
                        return Err(SendError::WouldBlock(value));
                    }
                }
            }
        }
    }

    /// Sends `value` if there is room for it right now, and never suspends.
    ///
    /// # Errors
    ///
    /// [`TrySendError::Full`] or [`TrySendError::Disconnected`], carrying the
    /// value back.
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        let woken = {
            let mut state = self.inner.state.borrow_mut();
            if state.receivers == 0 {
                return Err(TrySendError::Disconnected(value));
            }
            if state.queue.len() >= self.inner.capacity {
                return Err(TrySendError::Full(value));
            }
            state.queue.push_back(value);
            // One value arrived, so exactly one reader can now make progress.
            state.recv_waiters.pop_front()
        };
        if let Some(wake) = woken {
            wake.wake();
        }
        Ok(())
    }

    /// How many values this channel holds before a [`Sender::send`] suspends.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.inner.capacity
    }

    /// How many values are queued right now.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.state.borrow().queue.len()
    }

    /// Whether nothing is queued right now.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether the last [`Receiver`] has been dropped.
    #[must_use]
    pub fn is_disconnected(&self) -> bool {
        self.inner.state.borrow().receivers == 0
    }
}

impl<T> Receiver<T> {
    /// Takes the next value, suspending this task while the channel is empty.
    ///
    /// # Errors
    ///
    /// [`RecvError::Disconnected`] when the channel is empty and the last
    /// [`Sender`] is gone — values queued before that are still delivered
    /// first — and [`RecvError::WouldBlock`] with no scheduler beneath the
    /// call.
    pub fn recv(&self) -> Result<T, RecvError> {
        loop {
            match self.try_recv() {
                Ok(value) => return Ok(value),
                Err(TryRecvError::Disconnected) => return Err(RecvError::Disconnected),
                Err(TryRecvError::Empty) => {
                    let Some(wake) = Wake::current() else {
                        return Err(RecvError::WouldBlock);
                    };
                    let waiter = Waiter::register(&self.inner, wake, Side::Recv);
                    let parked = suspend_current(Waiting::Parked);
                    drop(waiter);
                    // Beside the send's, for its reason: a cancelled receiver
                    // has stopped waiting, and looping would only park it again.
                    if !parked.suspended() || parked.cancelled() {
                        return Err(RecvError::WouldBlock);
                    }
                }
            }
        }
    }

    /// Takes the next value if there is one right now, and never suspends.
    ///
    /// # Errors
    ///
    /// [`TryRecvError::Empty`] or [`TryRecvError::Disconnected`].
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let (value, woken) = {
            let mut state = self.inner.state.borrow_mut();
            match state.queue.pop_front() {
                // A slot opened, so exactly one writer can now make progress.
                Some(value) => (value, state.send_waiters.pop_front()),
                // Queued values outlive the senders that queued them: the
                // disconnection is only reported once the queue is drained.
                None if state.senders == 0 => return Err(TryRecvError::Disconnected),
                None => return Err(TryRecvError::Empty),
            }
        };
        if let Some(wake) = woken {
            wake.wake();
        }
        Ok(value)
    }

    /// How many values this channel holds before a [`Sender::send`] suspends.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.inner.capacity
    }

    /// How many values are queued right now.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.state.borrow().queue.len()
    }

    /// Whether nothing is queued right now.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether the last [`Sender`] has been dropped.
    #[must_use]
    pub fn is_disconnected(&self) -> bool {
        self.inner.state.borrow().senders == 0
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.inner.state.borrow_mut().senders += 1;
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        self.inner.state.borrow_mut().receivers += 1;
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        // Disconnection is the one event that wakes every waiter rather than
        // one: it is not a slot opening, it is the answer for all of them, and
        // it happens once.
        let woken = {
            let mut state = self.inner.state.borrow_mut();
            state.senders -= 1;
            if state.senders == 0 {
                state.recv_waiters.take_all()
            } else {
                Vec::new()
            }
        };
        for wake in woken {
            wake.wake();
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let woken = {
            let mut state = self.inner.state.borrow_mut();
            state.receivers -= 1;
            if state.receivers == 0 {
                state.send_waiters.take_all()
            } else {
                Vec::new()
            }
        };
        for wake in woken {
            wake.wake();
        }
    }
}

// The `Debug` impls below are hand-written rather than derived: deriving would
// demand `T: Debug` from every caller for a field none of them prints.
impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.inner.state.borrow();
        f.debug_struct("Sender")
            .field("capacity", &self.inner.capacity)
            .field("queued", &state.queue.len())
            .field("receivers", &state.receivers)
            .finish()
    }
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.inner.state.borrow();
        f.debug_struct("Receiver")
            .field("capacity", &self.inner.capacity)
            .field("queued", &state.queue.len())
            .field("senders", &state.senders)
            .finish()
    }
}

impl<T> fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Disconnected(_) => "SendError::Disconnected",
            Self::WouldBlock(_) => "SendError::WouldBlock",
        })
    }
}

impl<T> fmt::Debug for TrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Disconnected(_) => "TrySendError::Disconnected",
            Self::Full(_) => "TrySendError::Full",
        })
    }
}

impl<T> fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Disconnected(_) => "sending on a channel with no receiver left",
            Self::WouldBlock(_) => "sending on a full channel with no task to suspend",
        })
    }
}

impl fmt::Display for RecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Disconnected => "receiving on a channel with no sender left",
            Self::WouldBlock => "receiving on an empty channel with no task to suspend",
        })
    }
}

impl<T> std::error::Error for SendError<T> {}
impl std::error::Error for RecvError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{Scheduler, cancel_task};
    use nvs_runtime::{Ctx, OutputSink, TaskRoot};

    fn ctx() -> Ctx {
        Ctx::new(OutputSink::Sink)
    }

    #[test]
    fn a_bounded_channel_send_suspends_rather_than_growing() {
        let (tx, rx) = channel::<usize>(2);
        let mut sched = Scheduler::new();

        // The producer wants to put four values through a channel that holds
        // two, and records how far it had got each time it was resumed.
        let progress = Rc::new(RefCell::new(Vec::new()));
        let producer_progress = Rc::clone(&progress);
        let producer_tx = tx.clone();
        drop(tx);
        sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            for value in 0..4 {
                producer_tx.send(value).expect("the receiver is alive");
                producer_progress.borrow_mut().push(value);
            }
        });

        // Nothing has consumed anything, so the producer fills the channel and
        // parks inside `send`: the queue is at its bound, not past it.
        let first = sched.run();
        assert_eq!(first.parked, 1, "the producer must be parked inside send");
        assert_eq!(rx.len(), 2, "a full channel holds capacity and no more");
        assert_eq!(
            *progress.borrow(),
            vec![0, 1],
            "the third send must not have returned"
        );

        // Taking one value frees exactly one slot and wakes exactly one
        // sender, which is the whole of the backpressure contract.
        assert_eq!(rx.try_recv(), Ok(0));
        let second = sched.run();
        assert_eq!(rx.len(), 2, "the woken send refilled the one free slot");
        assert_eq!(*progress.borrow(), vec![0, 1, 2]);
        assert_eq!(second.parked, 1, "and parked again on the fourth");

        assert_eq!(rx.try_recv(), Ok(1));
        let third = sched.run();
        assert_eq!(third.finished, 1, "the last send had room and returned");
        assert_eq!(*progress.borrow(), vec![0, 1, 2, 3]);

        // Every value arrived once and in order, and the producer's own end
        // went down with its stack, so the reader is told rather than left.
        assert_eq!(rx.try_recv(), Ok(2));
        assert_eq!(rx.try_recv(), Ok(3));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
    }

    #[test]
    fn a_receiver_suspends_on_an_empty_channel_and_the_sender_wakes_it() {
        let (tx, rx) = channel::<&'static str>(4);
        let mut sched = Scheduler::new();

        let seen = Rc::new(RefCell::new(Vec::new()));
        let consumer_seen = Rc::clone(&seen);
        sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            while let Ok(value) = rx.recv() {
                consumer_seen.borrow_mut().push(value);
            }
        });

        let first = sched.run();
        assert_eq!(first.parked, 1, "an empty channel parks its reader");
        assert!(seen.borrow().is_empty());

        // The wake is issued from outside any task, which is the case a route
        // reading a thread-local when it fires could not serve.
        tx.try_send("a").expect("room for four");
        tx.try_send("b").expect("room for four");
        let second = sched.run();
        assert_eq!(second.parked, 1, "back to waiting, having taken both");
        assert_eq!(*seen.borrow(), vec!["a", "b"]);

        // Dropping the last sender is the answer to every waiting reader, so
        // the loop above ends rather than sleeping through the shutdown.
        drop(tx);
        let third = sched.run();
        assert_eq!(third.finished, 1);
        assert_eq!(third.parked, 0);
    }

    #[test]
    fn a_cancelled_task_blocked_on_a_channel_leaves_no_waiter_behind() {
        let (tx, rx) = channel::<usize>(1);
        let mut sched = Scheduler::new();
        let inner = Rc::clone(&rx.inner);

        // Two readers park on an empty channel; the first one in the queue is
        // then cancelled while it stands there.
        let doomed_rx = rx.clone();
        let doomed = sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            let _ = doomed_rx.recv();
            unreachable!("a cancelled task is torn down where it parked");
        });
        let survivor_rx = rx.clone();
        let got = Rc::new(RefCell::new(None));
        let survivor_got = Rc::clone(&got);
        sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            *survivor_got.borrow_mut() = survivor_rx.recv().ok();
        });

        let first = sched.run();
        assert_eq!(first.parked, 2);
        assert_eq!(inner.state.borrow().recv_waiters.len(), 2);

        assert_eq!(sched.cancel(doomed), 1);
        let second = sched.run();
        assert_eq!(second.cancelled, 1);
        assert_eq!(
            inner.state.borrow().recv_waiters.len(),
            1,
            "the unwind must take the dead task off the waiter queue"
        );

        // The one wake a send hands out therefore reaches the live reader,
        // rather than being spent on a task that can never be resumed.
        tx.try_send(7).expect("room for one");
        let third = sched.run();
        assert_eq!(third.finished, 1);
        assert_eq!(*got.borrow(), Some(7));
    }

    #[test]
    fn a_task_cancelled_while_it_waits_to_send_deregisters_too() {
        let (tx, rx) = channel::<usize>(1);
        let mut sched = Scheduler::new();
        let inner = Rc::clone(&tx.inner);

        let doomed_tx = tx.clone();
        sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            doomed_tx.try_send(1).expect("room for one");
            let _ = doomed_tx.send(2);
            unreachable!("the second send parks and the task dies there");
        });

        let first = sched.run();
        assert_eq!(first.parked, 1);
        assert_eq!(inner.state.borrow().send_waiters.len(), 1);

        let waiting = inner
            .state
            .borrow()
            .send_waiters
            .front()
            .expect("one waiter")
            .id();
        assert_eq!(sched.cancel(waiting), 1);
        let second = sched.run();
        assert_eq!(second.cancelled, 1);
        assert!(inner.state.borrow().send_waiters.is_empty());
        assert_eq!(rx.len(), 1, "what it did send is still queued");
    }

    #[test]
    fn a_send_with_no_scheduler_reports_rather_than_blocking_the_thread() {
        let (tx, rx) = channel::<usize>(1);
        tx.send(1).expect("room for one");
        match tx.send(2) {
            Err(SendError::WouldBlock(back)) => assert_eq!(back, 2),
            other => panic!("a full channel off a core must not wait: {other:?}"),
        }
        assert_eq!(rx.try_recv(), Ok(1));
        assert_eq!(rx.recv(), Err(RecvError::WouldBlock));
    }

    #[test]
    fn a_zero_capacity_channel_holds_one_rather_than_none() {
        let (tx, rx) = channel::<usize>(0);
        assert_eq!(tx.capacity(), 1);
        assert!(tx.try_send(1).is_ok());
        assert!(matches!(tx.try_send(2), Err(TrySendError::Full(2))));
        assert_eq!(rx.try_recv(), Ok(1));
    }

    #[test]
    fn queued_values_outlive_the_sender_that_queued_them() {
        let (tx, rx) = channel::<usize>(4);
        tx.try_send(1).expect("room");
        tx.try_send(2).expect("room");
        drop(tx);
        assert!(rx.is_disconnected());
        assert_eq!(rx.try_recv(), Ok(1));
        assert_eq!(rx.try_recv(), Ok(2));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
    }

    #[test]
    fn a_send_to_a_dropped_receiver_hands_the_value_back() {
        let (tx, rx) = channel::<String>(2);
        drop(rx);
        assert!(tx.is_disconnected());
        match tx.send("kept".to_string()) {
            Err(SendError::Disconnected(back)) => assert_eq!(back, "kept"),
            other => panic!("expected the value back: {other:?}"),
        }
    }

    #[test]
    fn a_channel_carries_work_between_two_tasks_on_one_core() {
        let (tx, rx) = channel::<usize>(2);
        let mut sched = Scheduler::new();
        let sum = Rc::new(RefCell::new(0));

        let producer_tx = tx.clone();
        drop(tx);
        sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            for value in 1..=10 {
                producer_tx.send(value).expect("the consumer is alive");
            }
        });
        let consumer_sum = Rc::clone(&sum);
        sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            while let Ok(value) = rx.recv() {
                *consumer_sum.borrow_mut() += value;
            }
        });

        let report = sched.run();
        assert_eq!(report.finished, 2, "neither end is left parked");
        assert_eq!(report.parked, 0);
        assert_eq!(*sum.borrow(), 55);
        // Ten values through a channel of two is several slices each, which is
        // the bound doing its job rather than a consumer that happened to keep
        // up: two tasks that never waited would be two resumes.
        assert!(
            report.resumes >= 6,
            "backpressure must interleave the two, got {} resumes",
            report.resumes
        );
    }

    #[test]
    fn a_task_can_cancel_the_peer_it_shares_a_channel_with() {
        let (tx, rx) = channel::<usize>(1);
        let mut sched = Scheduler::new();

        let reader = sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            let _ = rx.recv();
            unreachable!("cancelled where it parked");
        });
        let cancelled = Rc::new(RefCell::new(0));
        let reported = Rc::clone(&cancelled);
        sched.spawn(ctx(), TaskRoot::Worker, move |_| {
            // A running task reaches the tree and not the scheduler — the same
            // route the wake takes.
            *reported.borrow_mut() = cancel_task(reader);
            // And the disconnection races the cancellation: a task that is
            // already marked must be torn down rather than resumed by it.
            drop(tx);
        });

        let report = sched.run();
        assert_eq!(*cancelled.borrow(), 1);
        assert_eq!(report.cancelled, 1);
        assert_eq!(report.finished, 1);
        assert_eq!(report.parked, 0);
    }
}
