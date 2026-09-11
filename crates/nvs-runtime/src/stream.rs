//! A response body written over time, at the one seam where it crosses between
//! two tasks: the isolate that writes it, and the connection that frames it
//! onto the wire.
//!
//! `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection` draws
//! the line between the two spellings — a stream that ends with its response,
//! and one that outlives it — and both of them write through this cell. What
//! differs is which task holds [`Emit`]: the request's own isolate for the
//! first, a connection isolate for the second. The connection holds [`Drain`]
//! either way, because it is the only task that can put a byte on the wire.
//!
//! # Why a cell and not the response buffer
//!
//! [`nvs_server::body`](/crates/nvs-server/src/body.rs) is this module's shape
//! in the other direction, and the argument it makes is the one repeated here:
//! `hyper`'s body types are polled with the **connection's** `Context`, and an
//! isolate is a peer task, so nothing on the isolate's stack can make the
//! connection's side of the socket run. A response an isolate assembles whole
//! crosses as a buffer and needs none of this; a response written over time has
//! the same seam as an arriving one, and takes the same answer.
//!
//! Between the two halves is a wake pair rather than a queue, and **one chunk
//! is in flight at a time**: [`Emit::send`] parks the writing task while the
//! cell is full, and the connection wakes it once it has taken the chunk. That
//! is the whole of the backpressure story — a program that writes faster than
//! the peer reads is slowed by the peer, nothing accumulates in between, and
//! there is no queue depth to configure because there is no queue.
//!
//! The failure this shape has instead of an unbounded buffer is a producer that
//! parks forever, so [`Emit`] carries a **send timeout**. The duration is
//! `nvs_server::bounds`' `Connection::send`, passed in at [`open`] rather than
//! named here: a bound on a connection belongs to
//! `rule:concurrency/connection-bounds-are-finite`'s table, and a cell that
//! chose its own would be a second, invisible bound beside it. A consumer that
//! has stopped reading closes its producer at that deadline; a consumer that is
//! *gone* closes it at once, without waiting for one.
//!
//! An [`Rc`] rather than an `Arc`, for
//! [`nvs_server::body`](/crates/nvs-server/src/body.rs)'s own reason: both
//! halves live on one core by construction — the isolate is a task of the
//! connection's scheduler, and a task never leaves the scheduler it was spawned
//! on — and nothing here is `Send`.
//!
//! # Two wakes, because the two sides are woken by different machinery
//!
//! The writing side is a Novis task and parks through [`crate::host::Host::park`],
//! so its wake is a [`crate::host::Waker`]: the one-shot closure that seam
//! hands out. The draining side is a body `hyper` polls, so its wake is the
//! [`std::task::Waker`] of the poll that found the cell empty. Neither is
//! convertible into the other, so the cell holds one of each rather than
//! inventing a third that both would have to be adapted to.
//!
//! Neither half is re-exported at the crate root, and [`Drain`] is why:
//! [`crate::Drain`] is already the server's drain bit, and two types of that
//! name reachable by one path would be a name a reader has to disambiguate on
//! every sight of it. Both halves are `stream::` at every call site instead.
//!
//! # What it spends
//!
//! Per response written over time: one [`Rc`] cell, and at most one chunk held
//! at a time — bytes the writer allocated and the connection takes ownership
//! of, never copied between them. O(in-flight) by construction rather than by
//! accounting, since the producer cannot run ahead of the consumer, so the
//! number of events a stream has sent is not a term in what it holds
//! (`rule:programs/memory-priority`).

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::task::Waker;
use std::time::{Duration, Instant};

use crate::host::{self, Woken};

/// Opens a response body's two halves over one cell.
///
/// `send_timeout` is how long one [`Emit::send`] may wait for the connection to
/// take the chunk before the stream is closed under it. It is the connection's
/// own bound, passed in for the module doc's reason.
#[must_use]
pub fn open(send_timeout: Duration) -> (Emit, Drain) {
    let wire = Rc::new(RefCell::new(Wire::default()));
    let emit = Emit {
        wire: Rc::clone(&wire),
        send_timeout,
        spent: false,
    };
    (emit, Drain { wire })
}

/// The cell both halves read, and the two wakes that drive it.
#[derive(Default)]
struct Wire {
    /// The chunk the program has written and the connection has not taken yet.
    /// Never more than one: a write parks while it is full.
    chunk: Option<Vec<u8>>,
    /// The end of the body, once the writing half has said so — which a
    /// dropped [`Emit`] also says, so an isolate that ended never leaves a
    /// response open.
    ended: bool,
    /// The consumer is gone or has given up: nothing will take another chunk,
    /// and a write answers [`CLOSED`] rather than parking.
    closed: bool,
    /// The writing task, to wake when its chunk has been taken. One-shot, so it
    /// is taken rather than cloned.
    writer: Option<host::Waker>,
    /// The poll that found the cell empty, to wake when a chunk lands.
    reader: Option<Waker>,
}

impl fmt::Debug for Wire {
    /// Hand-written for one reason: a [`crate::host::Waker`] is a boxed closure
    /// with no `Debug` to derive through. What a reader of this wants is the
    /// state anyway — what is in flight, and which end is finished.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wire")
            .field("in_flight", &self.chunk.is_some())
            .field("ended", &self.ended)
            .field("closed", &self.closed)
            .finish()
    }
}

/// The writing half: the program's end of the body, held by the isolate that
/// produces it.
#[derive(Debug)]
pub struct Emit {
    wire: Rc<RefCell<Wire>>,
    /// The connection's send bound, applied to one [`Emit::send`] at a time.
    send_timeout: Duration,
    /// Whether this half is finished — ended, or closed under it. A spent half
    /// refuses every later write rather than answering a different reason each
    /// time.
    spent: bool,
}

impl Emit {
    /// Writes one chunk, parking the writing task while the connection still
    /// holds the last one.
    ///
    /// An empty chunk is nothing to frame and is not an error: it neither parks
    /// nor reaches the wire. A caller for which emptiness is a *program*
    /// mistake — an event with no data — refuses it in its own member, where
    /// the diagnostic can say which argument was empty.
    ///
    /// # Errors
    ///
    /// [`SEND_TIMED_OUT`] where the connection did not take the chunk within
    /// the send timeout, which closes the stream: a peer that has stopped
    /// reading is a bound met, not a wait to extend. [`CLOSED`] where the
    /// consumer is already gone. [`CANCELLED`] where the task was torn down
    /// while it waited, and [`NO_TASK`] where there is no task to park at all,
    /// since blocking the core instead would stop every other request on it.
    pub fn send(&mut self, chunk: Vec<u8>) -> Result<(), Box<str>> {
        if self.spent {
            return Err(CLOSED.into());
        }
        if chunk.is_empty() {
            return Ok(());
        }
        let deadline = Instant::now().checked_add(self.send_timeout);
        loop {
            {
                let mut wire = self.wire.borrow_mut();
                if wire.closed {
                    break;
                }
                if wire.chunk.is_none() {
                    wire.chunk = Some(chunk);
                    if let Some(reader) = wire.reader.take() {
                        reader.wake();
                    }
                    return Ok(());
                }
                // The cell is full, so what this write is about to wait for is
                // the connection taking the last chunk — and the deadline
                // bounds that wait wherever the loop re-enters it, including
                // the first turn. A send timeout already spent is the bound
                // met, not a park with nowhere to go.
                if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                    self.spent = true;
                    wire.closed = true;
                    return Err(SEND_TIMED_OUT.into());
                }
                // The wake is published before the borrow is released, so the
                // take this park waits for either fires it or has already
                // happened — and the loop re-reads the cell either way. The
                // reverse order is the lost wake.
                let Some(here) = host::with_current(|host| host.waker()).flatten() else {
                    self.spent = true;
                    return Err(NO_TASK.into());
                };
                wire.writer = Some(here);
            }
            match host::with_current(|host| host.park(deadline)) {
                None => {
                    self.spent = true;
                    return Err(NO_TASK.into());
                }
                Some(Woken::Cancelled) => {
                    self.spent = true;
                    return Err(CANCELLED.into());
                }
                // A wake is a hint and the deadline is read from this side's
                // own clock, so both answers land here and the next turn of the
                // loop is what decides which of them it was.
                Some(Woken::Elapsed) => {}
            }
        }
        self.spent = true;
        Err(CLOSED.into())
    }

    /// Ends the body: the connection frames whatever it still holds, then the
    /// end of the stream.
    pub fn finish(&mut self) {
        self.spent = true;
        let mut wire = self.wire.borrow_mut();
        wire.ended = true;
        if let Some(reader) = wire.reader.take() {
            reader.wake();
        }
    }

    /// Whether this half is finished — the consumer gone, a bound met, or the
    /// body ended. A program asks so that it can stop producing what nothing
    /// will read.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        let wire = self.wire.borrow();
        self.spent || wire.closed
    }
}

impl Drop for Emit {
    /// An isolate that ended ends its response. Without this a connection would
    /// wait for a chunk from a task that no longer exists, which is the one
    /// failure a stream with no length of its own cannot report to the peer.
    fn drop(&mut self) {
        self.finish();
    }
}

/// The connection's half: what the framing layer polls for the next chunk.
#[derive(Debug)]
pub struct Drain {
    wire: Rc<RefCell<Wire>>,
}

/// What one poll of [`Drain::next_chunk`] found.
#[derive(Debug)]
pub enum Drained {
    /// A chunk to frame, owned by the caller from here on.
    Chunk(Vec<u8>),
    /// Nothing yet, and the waker that was passed in will fire when there is.
    Pending,
    /// The body is over: the writer finished it, or the isolate holding the
    /// writer ended.
    Ended,
}

impl Drain {
    /// Takes the chunk in flight, waking the writer that was parked on it.
    ///
    /// `waker` is the poll's own, registered only where the answer is
    /// [`Drained::Pending`] — a caller that got a chunk is going to be polled
    /// again for the next one, and does not need waking to do it.
    pub fn next_chunk(&mut self, waker: &Waker) -> Drained {
        let mut wire = self.wire.borrow_mut();
        if let Some(chunk) = wire.chunk.take() {
            if let Some(writer) = wire.writer.take() {
                writer();
            }
            return Drained::Chunk(chunk);
        }
        if wire.ended {
            return Drained::Ended;
        }
        wire.reader = Some(waker.clone());
        Drained::Pending
    }
}

impl Drop for Drain {
    /// The consumer going away closes the producer rather than leaving it
    /// parked: nothing will take another chunk, so a write learns it at once
    /// instead of at the send timeout.
    fn drop(&mut self) {
        let mut wire = self.wire.borrow_mut();
        wire.closed = true;
        if let Some(writer) = wire.writer.take() {
            writer();
        }
    }
}

/// The peer stopped reading and the stream met the connection's send bound —
/// `rule:concurrency/connection-bounds-are-finite`'s defined close rather than
/// a wait with no end. Public because the bound is reported as itself, and a
/// report derived from a private string would be a second copy of it.
pub const SEND_TIMED_OUT: &str =
    "the response body stream met its send timeout with the client no longer reading it";

/// The consumer is gone: the connection dropped its half, or a bound already
/// closed this stream.
const CLOSED: &str = "the client is no longer reading this response body";

/// A write made where there is no task to park, so nothing could give the core
/// back while the connection catches up. A refusal rather than a block, since
/// blocking here would stop the core that owes the bytes.
const NO_TASK: &str = "a response body cannot be streamed from outside the task that owns it";

/// The writing task was torn down while it waited —
/// `rule:concurrency/nothing-is-still-running-when-a-call-returns`'s teardown
/// reached it, or the peer went away.
const CANCELLED: &str = "the response ended before its body finished being written";

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::task::Waker;
    use std::time::{Duration, Instant};

    use super::{Drain, Drained, SEND_TIMED_OUT, open};
    use crate::ctx::Ctx;
    use crate::graph::GraphError;
    use crate::host::{Bounds, Entry, Host, Job, Outcome, Output, Running, Woken, install};
    use crate::script::Program;
    use crate::value::Value;

    /// A host whose park *is* the connection running: it takes whatever chunk
    /// is in flight, which is what the consumer does while a writer waits.
    ///
    /// `drain` is held here rather than by the test so that the take happens
    /// inside the park, which is the ordering a real connection has and the one
    /// the parking half has to be correct under.
    #[derive(Debug)]
    struct Consumer {
        drain: RefCell<Option<Drain>>,
        taken: RefCell<Vec<Vec<u8>>>,
        parks: Cell<usize>,
    }

    impl Consumer {
        fn holding(drain: Option<Drain>) -> &'static Self {
            // The seam takes a `&'static dyn Host` and a leak is the only way to
            // hand one over; a test's own host is a few words held for the
            // length of the binary.
            Box::leak(Box::new(Self {
                drain: RefCell::new(drain),
                taken: RefCell::new(Vec::new()),
                parks: Cell::new(0),
            }))
        }
    }

    impl Host for Consumer {
        fn run_group(&self, _ctx: &mut Ctx, _jobs: Vec<Job>, _bounds: Bounds) -> Outcome {
            Outcome::Completed(Vec::new())
        }

        fn sleep(&self, _duration: Duration) -> Woken {
            Woken::Elapsed
        }

        fn waker(&self) -> Option<crate::host::Waker> {
            // A wake that does nothing, because this host's park drains rather
            // than waiting: what the cell is being proved on is the parking, not
            // the scheduler's own delivery.
            Some(Box::new(|| {}))
        }

        fn park(&self, deadline: Option<Instant>) -> Woken {
            self.parks.set(self.parks.get() + 1);
            if let Some(drain) = self.drain.borrow_mut().as_mut()
                && let Drained::Chunk(chunk) = drain.next_chunk(Waker::noop())
            {
                self.taken.borrow_mut().push(chunk);
                return Woken::Elapsed;
            }
            // Nothing took a chunk, so this park ends at the deadline the way a
            // scheduler's does. A test host that returned at once instead would
            // spin the wait it is standing in for.
            if let Some(deadline) = deadline {
                std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
            }
            Woken::Elapsed
        }

        fn start_isolate(
            &self,
            _ctx: &mut Ctx,
            _program: Program,
            _args: Value,
            _output: Output,
            _entry: Entry,
        ) -> Result<Box<dyn Running>, GraphError> {
            unreachable!("a response body cell starts no isolate")
        }
    }

    #[test]
    fn a_chunk_written_is_the_chunk_the_consumer_takes() {
        let (mut emit, mut drain) = open(Duration::from_secs(5));
        assert!(
            matches!(drain.next_chunk(Waker::noop()), Drained::Pending),
            "nothing has been written yet"
        );
        emit.send(b"one".to_vec()).expect("the cell is empty");
        let Drained::Chunk(chunk) = drain.next_chunk(Waker::noop()) else {
            panic!("the chunk that was written is the one in flight")
        };
        assert_eq!(chunk, b"one");
        assert!(
            matches!(drain.next_chunk(Waker::noop()), Drained::Pending),
            "one chunk in flight, and it has been taken"
        );
        emit.finish();
        assert!(matches!(drain.next_chunk(Waker::noop()), Drained::Ended));
    }

    #[test]
    fn a_second_write_parks_until_the_first_chunk_has_been_taken() {
        let (mut emit, drain) = open(Duration::from_secs(5));
        let host = Consumer::holding(Some(drain));
        let _installed = install(host);

        emit.send(b"first".to_vec()).expect("the cell is empty");
        assert_eq!(
            host.parks.get(),
            0,
            "a write into an empty cell parks nobody"
        );

        emit.send(b"second".to_vec())
            .expect("the consumer takes the first chunk while this one waits");
        assert_eq!(host.parks.get(), 1, "the second write waited for the take");
        assert_eq!(
            host.taken.borrow().as_slice(),
            [b"first".to_vec()],
            "and what it waited for is the first chunk leaving"
        );
    }

    #[test]
    fn a_dropped_consumer_ends_the_producer_rather_than_parking_it_forever() {
        let (mut emit, drain) = open(Duration::from_secs(30));
        let host = Consumer::holding(None);
        let _installed = install(host);
        drop(drain);

        let refused = emit
            .send(b"one".to_vec())
            .expect_err("nothing is reading this response");
        assert!(refused.contains("no longer reading"), "{refused}");
        assert_eq!(host.parks.get(), 0, "and it did not wait to find out");
        assert!(emit.is_closed());
    }

    #[test]
    fn a_reader_that_takes_nothing_closes_the_stream_at_the_send_timeout() {
        // The consumer holds no drain, so the chunk in flight is never taken
        // and the wait for it ends only at the deadline.
        let (mut emit, _drain) = open(Duration::from_millis(5));
        let host = Consumer::holding(None);
        let _installed = install(host);

        emit.send(b"first".to_vec()).expect("the cell is empty");
        let refused = emit
            .send(b"second".to_vec())
            .expect_err("the first chunk is still in flight");
        assert_eq!(&*refused, SEND_TIMED_OUT);
        assert_eq!(host.parks.get(), 1, "it waited before it gave up");
        assert!(emit.is_closed());
    }
}
