//! `rule:http-server/an-upload-is-received-only-through-files`'s request body, at the one seam where it crosses between two
//! tasks: `hyper`'s [`Incoming`] on the connection, and
//! [`nvs_runtime::RequestBody`] in the request.
//!
//! # Why a shared cell and not the `Incoming` itself
//!
//! The obvious implementation hands the isolate the [`Incoming`] and lets it
//! pull. It cannot work, and the reason is a fact about `hyper` rather than a
//! preference: an [`Incoming`] is polled with the **connection's** `Context`,
//! and what fills it is that connection's own dispatcher loop
//! (`rule:concurrency/one-future-per-connection`). The isolate is a *peer* task
//! ([`crate::serve::serve_connection`]'s docs own that shape), so a pull made
//! on its stack has no connection context to poll with and no way to make the
//! read side run.
//!
//! So the body crosses as two halves over one cell. [`Supply`] stays on the
//! connection and is pumped from inside the service future's poll, where a
//! `Context` exists; [`Pull`] goes to the isolate inside [`nvs_runtime::Inbound`]
//! and is the [`nvs_runtime::RequestBody`] the program reads. Between them sits
//! a wake pair rather than a queue: the puller marks that it *wants* a chunk and
//! wakes the connection, the connection reads exactly one and wakes the puller
//! back. Nothing is read from the wire before a program asks for it, and nothing
//! accumulates — one chunk is in flight at a time, which is what makes
//! [`nvs_runtime::RequestBody`]'s "valid until the next pull" contract free to
//! keep.
//!
//! An [`Rc`] rather than an `Arc`: both halves live on one core by construction
//! — the isolate is a child task of the connection's, and a task never leaves
//! the scheduler it was spawned on — and nothing here is `Send` anyway.
//!
//! # What it spends
//!
//! Per request that carries a body: one [`Rc`] cell, one boxed [`Incoming`], and
//! at most one chunk of `hyper`'s own read buffer held at a time — a [`Bytes`]
//! handle, so the chunk is shared rather than copied. O(in-flight) by
//! construction, since [`Supply::pump`] reads one chunk per want and never runs
//! ahead of the program.
//!

use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use hyper::HeaderMap;
use hyper::body::{Body as _, Bytes, Incoming};
use hyper::header::CONTENT_LENGTH;
use nvs_host::{Waiting, Wake, suspend_current};
use nvs_runtime::RequestBody;

/// `rule:http-server/request-body-and-upload-total-are-two-caps`'s `upload_total`: the total bytes of one request body this
/// server will carry, consumed and drained alike.
///
/// The ADR's own default, as a constant, because `[limits] upload_total` is not
/// a `nvs_config` row yet — that directive and its `[limits.hard]` ceiling are
/// the configuration slice's, and this constant is the one line it replaces.
/// The number is load-bearing either way: § 5 makes it the *only* thing
/// bounding a streamed body, and `rule:http-server/admission-is-arithmetic-not-a-number` multiplies it by the in-flight
/// ceiling to get what the machine must hold.
///
pub const UPLOAD_TOTAL: u64 = 256 * 1024 * 1024;

/// What the door found where a body would be.
pub enum Arrived {
    /// No body at all — a `GET`, or a `POST` the peer closed with nothing in
    /// it. [`nvs_runtime::Inbound::set_body`] is left unset, which is the
    /// distinction RFC 9110 § 8.6 draws and not "the body is empty".
    Absent,
    /// A body whose declared length is already over [`UPLOAD_TOTAL`], refused
    /// before anything is dispatched — `rule:http-server/request-body-and-upload-total-are-two-caps`'s honest oversized client,
    /// which never reaches application code.
    ///
    TooLarge,
    /// A body to stream: the connection's half, and the program's.
    Streaming(Supply, Box<dyn RequestBody>),
}

impl std::fmt::Debug for Arrived {
    /// Hand-written for one reason: [`nvs_runtime::RequestBody`] is a trait a
    /// supplier implements and not a value to print, so the boxed half has no
    /// `Debug` to derive through.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Absent => f.write_str("Absent"),
            Self::TooLarge => f.write_str("TooLarge"),
            Self::Streaming(supply, _) => f.debug_tuple("Streaming").field(supply).finish(),
        }
    }
}

/// Splits an arrived body into the two halves that will read it.
///
/// **Called on the task that will poll [`Supply::pump`]**, which for this
/// server is the connection's: the wake this takes is what a pull uses to bring
/// the connection back, and it is read here rather than passed in because the
/// door and the service future are the same task by construction. Called off a
/// task there is nothing to wake, and every pull answers the error that says so.
#[must_use]
pub fn of(headers: &HeaderMap, incoming: Incoming) -> Arrived {
    if incoming.is_end_stream() {
        return Arrived::Absent;
    }
    let declared = headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok());
    if declared.is_some_and(|length| length > UPLOAD_TOTAL) {
        return Arrived::TooLarge;
    }
    let wire = Rc::new(RefCell::new(Wire {
        connection: Wake::current(),
        ..Wire::default()
    }));
    let supply = Supply {
        incoming: Box::pin(incoming),
        wire: Rc::clone(&wire),
        carried: 0,
        spent: false,
    };
    let pull = Pull {
        wire,
        held: Bytes::new(),
    };
    Arrived::Streaming(supply, Box::new(pull))
}

/// The cell both halves read, and the two wakes that drive it.
#[derive(Debug, Default)]
struct Wire {
    /// The chunk the connection has read and the program has not taken yet.
    /// Never more than one: the connection reads on demand.
    chunk: Option<Bytes>,
    /// Whether the program is waiting for a chunk. Set before the connection is
    /// woken and cleared by whatever answers it, so a wake can be late but
    /// never lost.
    wanted: bool,
    /// The end of the body, once the connection has seen it.
    ended: bool,
    /// What ended it instead. An `Err` ends the body for good
    /// ([`nvs_runtime::RequestBody::next_chunk`]'s contract), so this is taken
    /// once and the stream is over either way.
    failed: Option<Box<str>>,
    /// The request's task, to wake when a chunk lands.
    puller: Option<Wake>,
    /// The connection's task, to wake when one is wanted.
    connection: Option<Wake>,
}

/// The connection's half: the [`Incoming`] and the cell it feeds.
///
/// Held by the service future that is waiting for the request, and pumped from
/// inside its poll — that is the only place a `Context` able to drive `hyper`'s
/// read side exists.
#[derive(Debug)]
pub struct Supply {
    /// Boxed rather than held inline, because [`Incoming`] is polled through a
    /// `Pin` and this struct moves into the service future after it is built.
    incoming: Pin<Box<Incoming>>,
    wire: Rc<RefCell<Wire>>,
    /// Body bytes delivered so far, against [`UPLOAD_TOTAL`].
    carried: u64,
    /// Whether the [`Incoming`] is finished — ended, failed, or over the cap.
    spent: bool,
}

impl Supply {
    /// Reads one chunk if the program is waiting for one, and nothing otherwise.
    ///
    /// Called on every poll of the service future, which is every turn of
    /// `hyper`'s dispatcher loop. The `want` handshake is what keeps this from
    /// running ahead: with nothing asked for, this does not poll the
    /// [`Incoming`] at all, so no read interest is registered and no byte of the
    /// body is buffered by anyone. With a chunk still uncollected it does not
    /// poll either, which is the whole of the O(in-flight) bound.
    ///
    /// `rule:http-server/request-body-and-upload-total-are-two-caps`'s `upload_total` is counted here, because this is the only
    /// thing that sees every byte: a body that crosses it mid-stream ends as an
    /// error at the pull that would have received the offending chunk, which is
    /// where the ADR puts it for a body that declared no length.
    ///
    pub fn pump(&mut self, cx: &mut Context<'_>) {
        loop {
            if self.spent || !self.wire.borrow().wanted || self.wire.borrow().chunk.is_some() {
                return;
            }
            match self.incoming.as_mut().poll_frame(cx) {
                // `hyper` holds the waker now, and the read side of this
                // connection's own loop is what will fire it.
                Poll::Pending => return,
                Poll::Ready(None) => {
                    self.spent = true;
                    self.wire.borrow_mut().ended = true;
                    self.deliver();
                    return;
                }
                Poll::Ready(Some(Err(failed))) => {
                    self.spent = true;
                    self.wire.borrow_mut().failed = Some(failed.to_string().into_boxed_str());
                    self.deliver();
                    return;
                }
                Poll::Ready(Some(Ok(frame))) => {
                    // A frame that is not data is trailers, which carry no body
                    // bytes and which nothing reads yet: the want is still
                    // outstanding, so ask again rather than answering with
                    // something the program cannot use. A zero-length data
                    // frame is the same case.
                    let Ok(data) = frame.into_data() else {
                        continue;
                    };
                    if data.is_empty() {
                        continue;
                    }
                    self.carried = self
                        .carried
                        .saturating_add(u64::try_from(data.len()).unwrap_or(u64::MAX));
                    if self.carried > UPLOAD_TOTAL {
                        self.spent = true;
                        self.wire.borrow_mut().failed = Some(OVER_CAP.into());
                    } else {
                        self.wire.borrow_mut().chunk = Some(data);
                    }
                    self.deliver();
                    return;
                }
            }
        }
    }

    /// Fails the body for a client that went away before it ended, and wakes a
    /// program waiting for a chunk.
    ///
    /// Called where the connection lets go of a request that runs on without
    /// it (`rule:http-server/a-request-outlives-a-client-that-goes-away`):
    /// nothing pumps this supply after that, so a pull that parked would wait
    /// for a chunk no task will read. The connection's wake is cleared with
    /// it, so every later pull fails at once as well.
    pub fn hang_up(mut self) {
        if !self.spent {
            self.spent = true;
            self.wire.borrow_mut().failed = Some(HUNG_UP.into());
        }
        self.wire.borrow_mut().connection = None;
        self.deliver();
    }

    /// Answers the outstanding want and wakes the task that made it.
    fn deliver(&self) {
        let mut wire = self.wire.borrow_mut();
        wire.wanted = false;
        if let Some(puller) = wire.puller.as_ref() {
            puller.wake();
        }
    }
}

/// The program's half: [`nvs_runtime::RequestBody`] over the cell, parking the
/// isolate's own task while the connection reads.
#[derive(Debug)]
pub struct Pull {
    wire: Rc<RefCell<Wire>>,
    /// The chunk this pull answered, held for exactly as long as the slice it
    /// lent out is valid — until the next pull replaces it.
    held: Bytes,
}

/// What one turn of the wait resolved to, with no borrow of `self` in it: the
/// loop below has to be able to take a fresh `&mut self` on every iteration,
/// which it could not if the chunk were returned as a slice from inside it.
enum Landed {
    Chunk(Bytes),
    End,
    Failed(Box<str>),
}

impl RequestBody for Pull {
    fn next_chunk(&mut self) -> Result<Option<&[u8]>, Box<str>> {
        let landed = loop {
            {
                let mut wire = self.wire.borrow_mut();
                if let Some(failed) = wire.failed.take() {
                    break Landed::Failed(failed);
                }
                if let Some(chunk) = wire.chunk.take() {
                    break Landed::Chunk(chunk);
                }
                if wire.ended {
                    break Landed::End;
                }
                // The want is published before the wake, so the connection
                // either sees it on the poll this wake causes or on one it was
                // already going to make. The reverse order is the lost wake.
                let Some(here) = Wake::current() else {
                    break Landed::Failed(NO_TASK.into());
                };
                if wire.connection.is_none() {
                    break Landed::Failed(NO_TASK.into());
                }
                wire.puller = Some(here);
                wire.wanted = true;
                if let Some(connection) = wire.connection.as_ref() {
                    connection.wake();
                }
            }
            let resumed = suspend_current(Waiting::Parked);
            if !resumed.suspended() {
                break Landed::Failed(NO_TASK.into());
            }
            if resumed.cancelled() {
                break Landed::Failed(CANCELLED.into());
            }
        };
        match landed {
            Landed::Chunk(chunk) => {
                self.held = chunk;
                Ok(Some(&self.held))
            }
            Landed::End => Ok(None),
            Landed::Failed(message) => Err(message),
        }
    }
}

/// A body crossing [`UPLOAD_TOTAL`] with no length declared, which is the case
/// `rule:http-server/request-body-and-upload-total-are-two-caps` leaves to the wire.
///
const OVER_CAP: &str = "the request body is larger than the upload_total limit allows";

/// A pull made where there is no task to park, so the connection could never be
/// polled again to answer it. A refusal rather than a block: blocking the
/// thread here would stop the core that owes the bytes.
const NO_TASK: &str = "the request body cannot be read from outside the request's own task";

/// The client closed the connection before it sent the whole body.
const HUNG_UP: &str = "the client closed the connection before the request body ended";

/// The request was cancelled while it waited — the peer went away, or `rule:concurrency/nothing-is-still-running-when-a-call-returns`
/// 's teardown reached it. Delivered once, so this stops waiting.
const CANCELLED: &str = "the request ended before its body finished arriving";
