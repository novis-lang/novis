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
//! The other number [`Emit`] carries is that table's **message bound**, and it
//! is here for the same reason read the other way: one chunk in flight bounds
//! what a stream holds only once a chunk itself has a size. The number is
//! `nvs_server::bounds`' `Connection::message` rather than its `frame` — a
//! frame is the WebSocket codec's unit and this door has none, so what crosses
//! here whole is a message, which on an event stream is one event. A chunk over
//! it is refused before it reaches the cell ([`CHUNK_TOO_LARGE`]), and the
//! stream stays open: nothing arrived from a peer and nothing was half-written,
//! so this is a program handing over more than the connection may hold and the
//! member that asked throws, exactly as the framing's own refusals do.
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

use crate::DeclaredHeader;
use crate::host::{self, Woken};

/// Opens a response body's two halves over one cell.
///
/// `send_timeout` is how long one [`Emit::send`] may wait for the connection to
/// take the chunk before the stream is closed under it, and `largest_chunk` is
/// the most one write may hand over. Both are the connection's own bounds,
/// passed in for the module doc's reason.
#[must_use]
pub fn open(send_timeout: Duration, largest_chunk: usize) -> (Emit, Drain) {
    let wire = Rc::new(RefCell::new(Wire::default()));
    let emit = Emit {
        wire: Rc::clone(&wire),
        send_timeout,
        largest_chunk,
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
    /// The connection's message bound, applied to the chunk one
    /// [`Emit::send`] hands over. The cell holds one chunk, so this is the
    /// whole of what a stream may have in flight.
    largest_chunk: usize,
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
    /// A chunk over the connection's message bound is refused and this half
    /// stays open, for the module doc's reason: the bytes never reached the
    /// cell, so there is nothing half-written to resynchronize after and
    /// nothing this stream's other events did wrong.
    ///
    /// # Errors
    ///
    /// [`CHUNK_TOO_LARGE`] where the chunk is over the message bound, which is
    /// the one refusal here that leaves the stream writable. [`SEND_TIMED_OUT`]
    /// where the connection did not take the chunk within the send timeout,
    /// which closes the stream: a peer that has stopped reading is a bound met,
    /// not a wait to extend. [`CLOSED`] where the consumer is already gone.
    /// [`CANCELLED`] where the task was torn down while it waited, and
    /// [`NO_TASK`] where there is no task to park at all, since blocking the
    /// core instead would stop every other request on it.
    pub fn send(&mut self, chunk: Vec<u8>) -> Result<(), Box<str>> {
        if self.spent {
            return Err(CLOSED.into());
        }
        if chunk.is_empty() {
            return Ok(());
        }
        if chunk.len() > self.largest_chunk {
            return Err(over_the_message_bound(chunk.len(), self.largest_chunk));
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

/// What a request that opened a streaming body left for the connection still
/// sending the response: the head to write, and the half the bytes arrive on.
///
/// **The head is everything the response declares**, because it goes out when
/// the opening member is called and nothing said afterwards can reach it: the
/// media type that member named, beside the status and the headers the request
/// had declared by then. A declaration made once the stream is open is one made
/// about a head already on the wire, which is the line `Core\Response::stream`
/// draws and the cost of answering early.
///
/// The head and the drain travel together because neither is a response on its
/// own — a head with no body would be a length the connection cannot supply,
/// and a drain with no media type would be a body the peer cannot read.
///
/// Fields rather than accessors: every one of them is the connection's to move
/// out, and this is the same hand-over between two tasks that
/// [`crate::host::Completion`] is already shaped as.
#[derive(Debug)]
pub struct Opened {
    /// The media type the opening member declared.
    pub content_type: Box<str>,
    /// What the request had declared this response *means* — spec § 15's
    /// status — and `None` where it had declared nothing by the time it opened
    /// the stream.
    pub status: Option<u16>,
    /// What else the request had declared the response carries, in the order it
    /// declared them, and applied the way [`DeclaredHeader::append`] says.
    pub headers: Vec<DeclaredHeader>,
    /// The consumer's half, taken by whoever is framing the response.
    pub drain: Drain,
}

/// The place a **request-scoped** streaming body is left, shared between the
/// request that opens it and the connection that frames it —
/// `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
/// first spelling.
///
/// A cell rather than a value on the carrier, for `nvs_runtime::UpgradeSlot`'s
/// reason: the two sides are a task apart, and a connection that reached into
/// the request's context for the drain would be reading a context the request
/// owns and may already have dropped. What differs from the two upgrade cells
/// is *when* the reader looks — a connection takes an upgrade after the
/// request's future has ended, and takes this one while that future is still
/// running, because answering the head is what lets the body arrive at all.
///
/// **The cell carries the connection's bounds**, so [`Self::open`] is the one
/// place a request-scoped stream reaches [`open`]. They are put here by whoever
/// offered the cell, for the reason the module doc gives: a cell that chose its
/// own would be a second bound beside
/// `rule:concurrency/connection-bounds-are-finite`'s table.
///
/// **What it spends:** one allocation per request a server offers one to,
/// holding two words and an [`Option`] until a program asks for a stream.
#[derive(Clone, Debug)]
pub struct BodySlot {
    send_timeout: Duration,
    largest_chunk: usize,
    cell: Rc<RefCell<Opening>>,
}

/// [`BodySlot`]'s contents: whether the body was opened, what is left for the
/// connection until it takes it, and the poll that is waiting for it.
///
/// Two fields rather than one [`Option`], because the connection **takes** the
/// head and a second `stream()` call after that take is still a second body.
/// The flag is what the refusal reads, so the two questions — has a body been
/// opened, and is one still waiting to be framed — do not collapse into each
/// other.
#[derive(Debug, Default)]
struct Opening {
    opened: bool,
    head: Option<Opened>,
    /// The connection's poll that found no head yet, to wake when one lands —
    /// [`Wire::reader`]'s shape one level up, and needed for the same reason:
    /// a program that opens a stream and then parks has published a head that
    /// nothing else would come back to look for, and the head is what the
    /// whole body waits behind.
    watcher: Option<Waker>,
}

impl BodySlot {
    /// An empty cell held inside the connection's send and message bounds,
    /// whose other half is a [`Clone`] of it.
    #[must_use]
    pub fn new(send_timeout: Duration, largest_chunk: usize) -> Self {
        Self {
            send_timeout,
            largest_chunk,
            cell: Rc::new(RefCell::new(Opening::default())),
        }
    }

    /// Opens the body at `content_type`, answering the writing half and leaving
    /// the head for the connection.
    ///
    /// `status` and `headers` are what the request has declared so far, which
    /// is what the head carries: [`Opened`] owns why a later declaration
    /// reaches nothing. They are taken rather than read, so the completion this
    /// request eventually files carries none of them and no answer applies them
    /// twice.
    ///
    /// `None` where this request has already opened a body — a response has
    /// one, and the second call is the one that is wrong. An [`Option`] rather
    /// than a `Result` carrying a refusal, because there is exactly one reason
    /// and nothing crossed to hand back: the caller still holds its own
    /// argument, and the member that asked is where the sentence belongs.
    #[must_use]
    pub fn open(
        &self,
        content_type: &str,
        status: Option<u16>,
        headers: Vec<DeclaredHeader>,
    ) -> Option<Emit> {
        let mut cell = self.cell.borrow_mut();
        if cell.opened {
            return None;
        }
        let (emit, drain) = open(self.send_timeout, self.largest_chunk);
        cell.opened = true;
        cell.head = Some(Opened {
            content_type: content_type.into(),
            status,
            headers,
            drain,
        });
        if let Some(watcher) = cell.watcher.take() {
            watcher.wake();
        }
        Some(emit)
    }

    /// Takes the head and the drain, leaving the cell open but empty — the
    /// connection's half, and `None` until a program has asked for a stream.
    ///
    /// `waker` is the asking poll's own, registered only where the answer is
    /// `None`: what a caller that got a head does next is frame it, and what a
    /// caller that got nothing needs is to be told when there is something.
    /// [`Drain::next_chunk`] is the same contract one layer down, and the two
    /// together are the whole of how a streamed response reaches the wire
    /// without either end polling on a loop.
    #[must_use]
    pub fn take(&self, waker: &Waker) -> Option<Opened> {
        let mut cell = self.cell.borrow_mut();
        if let Some(head) = cell.head.take() {
            return Some(head);
        }
        cell.watcher = Some(waker.clone());
        None
    }

    /// Whether this request has opened a streaming body, whether or not the
    /// connection has taken it yet.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.cell.borrow().opened
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

/// One write handed over more than
/// `rule:concurrency/connection-bounds-are-finite`'s message bound allows —
/// on an event stream, one event larger than the connection may hold.
///
/// Public and a prefix rather than the whole sentence, because the two numbers
/// are what a program needs to see and a caller matching on the bound it met
/// should not have to parse them back out.
pub const CHUNK_TOO_LARGE: &str =
    "this response body chunk is larger than the connection's message bound";

/// [`CHUNK_TOO_LARGE`], carrying what was written and what may be.
fn over_the_message_bound(wrote: usize, bound: usize) -> Box<str> {
    format!("{CHUNK_TOO_LARGE}: {wrote} bytes against a bound of {bound}").into()
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::task::Waker;
    use std::time::{Duration, Instant};

    use super::{Drain, Drained, SEND_TIMED_OUT, open};
    use crate::ctx::Ctx;
    use crate::host::{
        Bounds, Entry, Host, Job, Outcome, Output, Placement, Running, StartError, Woken, install,
    };
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
            _entry: Entry,
            _args: Value,
            _output: Output,
            _placement: Placement,
        ) -> Result<Box<dyn Running>, StartError> {
            unreachable!("a response body cell starts no isolate")
        }
    }

    /// A message bound far above anything these cases write, so what they
    /// assert is the wake pair and the send timeout rather than the size of a
    /// chunk. `nvs_server::bounds`' own cases are where the bound is the claim.
    const ROOMY: usize = 1 << 20;

    #[test]
    fn a_chunk_written_is_the_chunk_the_consumer_takes() {
        let (mut emit, mut drain) = open(Duration::from_secs(5), ROOMY);
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
        let (mut emit, drain) = open(Duration::from_secs(5), ROOMY);
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
        let (mut emit, drain) = open(Duration::from_secs(30), ROOMY);
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
        let (mut emit, _drain) = open(Duration::from_millis(5), ROOMY);
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
