//! `hyper`'s two IO traits over the parking stream.
//!
//! [ADR 0138](../../../docs/adr/0138-a-connection-future-is-driven-by-the-coroutine-that-owns-it.md)
//! is this module's specification. The whole of the adapter is one sentence:
//! **try the syscall, and on `WouldBlock` arm the reactor and answer
//! `Pending`** — never suspend, because a suspend inside a `poll` is that
//! ADR's rejected alternative and the deadlock it names. The coroutine would
//! park with the connection future's borrow held and the drive that owns the
//! waker never reached, so the readiness that ends the park would resume a
//! stack standing in the middle of `hyper` rather than in the loop that
//! re-polls it. [`nvs_host::NvsStream::poll_read`] is the half of the stream
//! that stops one step short for exactly this, and
//! [`nvs_host::block_on()`] is where the park it declines to do actually happens.
//!
//! # The waker is not how this connection is woken
//!
//! `Context` is taken and not used, which looks like a bug and is the design.
//! The reactor wakes a task **by id** — it queues the id on the core that
//! parked and pokes that core's poller — so socket readiness never travels
//! through a `Waker` at all. What the waker in `block_on`'s `Context` exists
//! for is the other kind of wake: a `hyper` upgrade holding a clone past the
//! call, or anything else that concludes progress is possible without the
//! socket saying so. Both routes end in the same place, which is why the drive
//! clears its flag before each poll rather than trusting either one.
//!
//! Two things follow, and both are load-bearing. The adapter may not store the
//! waker, because a stored waker would be a second, staler route to the same
//! task. And a `Pending` from here is only honest because
//! [`nvs_host::NvsStream::poll_read`] armed the reactor *before* it answered —
//! ADR 0115's rule 1, which is why the arming lives in `nvs-host` beside the
//! registration it touches and not in this module.
//!
//! # What the copy spends, and why it is taken
//!
//! `hyper` hands a read an uninitialised [`ReadBufCursor`], and the way to fill
//! one without initialising it first is an `unsafe` cast of
//! `&mut [MaybeUninit<u8>]` to `&mut [u8]` — handing a syscall a buffer the
//! compiler believes may be read before it is written. This crate inherits the
//! workspace's `unsafe_code = "forbid"` rather than taking a fourth exception
//! to it, so the read goes into a zeroed stack buffer and is copied into the
//! cursor with [`ReadBufCursor::put_slice`].
//!
//! **What that spends**, per [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md):
//! [`SCRATCH`] bytes of the accepting coroutine's own stack while a read is in
//! flight, and one `memcpy` of at most that much per readable poll. Per
//! connection and only while it is polling, so O(in-flight) and nothing held
//! between polls. AGENTS.md's ordering puts security and simplicity above
//! priority 3, and a bounded `memcpy` on an L1-resident buffer is the cheap
//! side of that trade; if a benchmark ever says otherwise, the thing to change
//! is the buffer's size, and only then the `forbid`.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::rt::{Read, ReadBufCursor, Write};
use nvs_host::NvsTcp;

/// The scratch buffer one read borrows from the coroutine's stack.
///
/// 8 KiB is a socket read's ordinary size and comfortably more than an h1
/// request head, so the common connection — a head, then a small body — is one
/// pass through here. It is a *ceiling* and not a target: a poll asks for
/// `min(what hyper has room for, this)`, so a cursor with less room than this
/// costs less than this.
pub const SCRATCH: usize = 8 * 1024;

/// One accepted connection, as the two traits `hyper` drives it through.
///
/// Concrete over [`NvsTcp`] rather than generic over the stream: ADR 0097 § 1
/// gives this listener no TLS and no second transport, so a type parameter here
/// would have exactly one instantiation and would put `mio`'s `Source` in this
/// crate's public signatures to get it.
#[derive(Debug)]
pub struct ConnectionIo {
    stream: NvsTcp,
}

impl ConnectionIo {
    /// Takes over an accepted connection.
    #[must_use]
    pub fn new(stream: NvsTcp) -> Self {
        Self { stream }
    }

    /// The stream underneath, for the caller that has to set a deadline on it.
    ///
    /// ADR 0074 § 5's idle timeouts are the stream's own bound and not
    /// something `hyper` knows about, so the connection loop reaches through
    /// here to move them between the request head, the body and the response.
    pub fn stream_mut(&mut self) -> &mut NvsTcp {
        &mut self.stream
    }

    /// Gives the connection back, which is what closes it.
    ///
    /// `hyper` finishing with a connection is not the same event as the socket
    /// closing: [`Write::poll_shutdown`] below says the writing is done, and
    /// the FIN goes out when this is dropped. Handing the stream back is how a
    /// caller keeps it past that — an upgrade to a WebSocket, which
    /// [ADR 0083](../../../docs/adr/0083-persistent-connections-are-isolates.md)
    /// makes an isolate over the same descriptor.
    #[must_use]
    pub fn into_stream(self) -> NvsTcp {
        self.stream
    }
}

impl Read for ConnectionIo {
    /// The module's one sentence, on the readable interest.
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        mut cursor: ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        let want = cursor.remaining().min(SCRATCH);
        if want == 0 {
            // `hyper` has nowhere to put anything, so a syscall here could only
            // read zero bytes and be mistaken for the peer's end of stream.
            return Poll::Ready(Ok(()));
        }
        let mut scratch = [0_u8; SCRATCH];
        match self.stream.poll_read(&mut scratch[..want]) {
            Poll::Ready(Ok(read)) => {
                cursor.put_slice(&scratch[..read]);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Write for ConnectionIo {
    /// The module's one sentence, on the writable interest.
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.stream.poll_write(buf)
    }

    /// Nothing is buffered on this side, so there is nothing to push.
    ///
    /// The stream writes straight through to the socket — `nvs_host::net`'s
    /// module doc says so of its `flush` for the same reason — so this is
    /// `Ready` rather than a syscall that would do nothing.
    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let _ = &mut self.stream;
        Poll::Ready(Ok(()))
    }

    /// Says the writing is finished; the FIN is the drop's.
    ///
    /// A half-close here would take the choice away from
    /// [`ConnectionIo::into_stream`], and an upgraded connection reads and
    /// writes on the same descriptor after `hyper` has let go of it.
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}
