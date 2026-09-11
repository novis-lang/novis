//! `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
//! events, framed: the bytes one event is on the wire, as a function over its
//! payload.
//!
//! This is `nvs_server::socket`'s mirror for the other door, and the shape is
//! deliberately the opposite one. There the framing is a codec that owns a
//! descriptor, a clock and a connection's worth of state; here it is a
//! **function over bytes**. Nothing in this module holds a socket, reads a
//! bound or knows which half of the server is writing — an event is framed by
//! whoever has the payload and written by whoever has the body — which is what
//! makes the property below provable without a server running.
//!
//! It is also the only place in this workspace that writes a `data:` line, and
//! that is why it sits here rather than beside its mirror. Both halves have to
//! reach it: the payload belongs to a `Core\Sse` member in `nvs_stdlib`, which
//! cannot name `nvs_server` without closing a cycle, and the body belongs to
//! the connection. This crate is the one both already depend on, and it is
//! where [`crate::stream`] — the seam a body written over time crosses — lives
//! for the same reason.
//!
//! # A payload cannot escape its own event
//!
//! The client's parser terminates a line on `\r\n`, on `\r` **and** on `\n`,
//! and `\r` alone is the one a program reaches by accident — a Windows file
//! read in, a classic-Mac line ending, a string a spreadsheet produced. Left
//! as it arrived, the bytes after it start a new line the client reads as a
//! **field**, so a payload could name its own event or set the connection's
//! id. So every payload is normalized to `\n` and then split, one `data:` line
//! per line, and what a program supplies can only ever land in a data buffer.
//!
//! The goal's § *Standing decisions* admits `tainted` on the payload on
//! exactly that argument, which is the one `Core\Response::json` already
//! makes: framing belongs to the framer and never to concatenation
//! (`rule:security/sink-predicate`). The
//! event name and the id are the other side of the same predicate — a client
//! dispatches on the name, so those are sinks and are refused rather than
//! normalized.
//!
//! The split is exact and not merely safe. A client appends each data line and
//! a `\n` to its buffer and strips one trailing `\n` when it dispatches, so the
//! payload handed to [`Event::frame`] and the payload the client sees are the
//! same bytes — trailing newline and all — whichever of the three line endings
//! it arrived with.
//!
//! # The refusals are the framing's
//!
//! [`Refused`]'s three cases belong here rather than to the `Core` member,
//! because each of them is a way the framing could silently stop being true:
//! stripping a `\n` out of an event name would change the name an application
//! dispatches on, and a NUL makes a client discard the id outright. The member
//! turns one into a `LogicError` and adds nothing to it; the goal's
//! § *Standing decisions* is where all three are argued.
//!
//! # What it spends
//!
//! One `Vec` per event, the payload's length plus a line's prefix per line,
//! owned by the caller and released when the chunk it became has been written.
//! Nothing here is retained between events, so the cost is O(in-flight) rather
//! than O(events sent), as `rule:programs/memory-priority` asks a spender to
//! state.

use std::time::Duration;

/// The keepalive: a comment line and the blank line after it.
///
/// A comment carries no field, so a client parses it, resets its reconnection
/// timer and dispatches nothing — which is the whole job, since what an idle
/// event stream needs is a byte through every proxy between here and the
/// browser rather than an event no application asked for.
///
/// **The connection writes this and the isolate never does.** The connection
/// is the only half that knows the wire has gone quiet; the isolate above it
/// may be parked waiting for something to send. That makes this a property of
/// where it is written and not a preference.
pub const KEEPALIVE: &[u8] = b":\n\n";

/// The reconnection block: a `retry:` line and the blank line after it, with no
/// `data:` line and so nothing for a client to dispatch.
///
/// A block of its own rather than the field [`Event`] also carries, because what
/// a program says with it is "do not come back for an hour" ahead of a drain it
/// can see coming — and an event it had to attach that to would be an event it
/// did not otherwise want to send.
///
/// The wire spells the wait in milliseconds, so a wait shorter than one is `0`.
/// That is the client's floor rather than a refusal: a program asking for less
/// than the wire can say is asking for the least the wire can say.
#[must_use]
pub fn reconnect_after(wait: Duration) -> Vec<u8> {
    let mut out = Vec::with_capacity(24);
    field(&mut out, b"retry", wait.as_millis().to_string().as_bytes());
    out.push(b'\n');
    out
}

/// One event, before anything has framed it.
///
/// The payload is required and the other three fields are the wire's optional
/// ones. A borrowed record rather than an owned one because framing is the
/// only thing that happens to it: the bytes exist in the caller already, and
/// the one allocation this module makes is the frame it returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event<'a> {
    /// What the client's data buffer receives, exactly, whatever line endings
    /// it carries. Empty is [`Refused::EmptyData`].
    pub data: &'a [u8],
    /// The `event:` field — the name a client dispatches on. A sink: a line
    /// break or a NUL in it is [`Refused::EventName`].
    pub event: Option<&'a [u8]>,
    /// The `id:` field, which the client echoes back in `Last-Event-ID` when
    /// it reconnects. A sink for the same reason: a line break or a NUL in it
    /// is [`Refused::Id`].
    pub id: Option<&'a [u8]>,
    /// The `retry:` field — how long a client waits before reconnecting. The
    /// wire spells it in milliseconds, and taking a [`Duration`] is what keeps
    /// the unit from having to be remembered at the call site.
    pub retry: Option<Duration>,
}

impl<'a> Event<'a> {
    /// An event that is its payload and nothing else.
    #[must_use]
    pub const fn carrying(data: &'a [u8]) -> Self {
        Self {
            data,
            event: None,
            id: None,
            retry: None,
        }
    }

    /// The bytes this event is: its optional fields, one `data:` line per line
    /// of the payload, and the blank line that dispatches it.
    ///
    /// # Errors
    ///
    /// [`Refused`], whose three cases are the module doc's.
    pub fn frame(&self) -> Result<Vec<u8>, Refused> {
        if self.data.is_empty() {
            return Err(Refused::EmptyData);
        }
        if self.event.is_some_and(reframes_the_stream) {
            return Err(Refused::EventName);
        }
        if self.id.is_some_and(reframes_the_stream) {
            return Err(Refused::Id);
        }

        let mut out = Vec::with_capacity(self.data.len() + 16);
        if let Some(name) = self.event {
            field(&mut out, b"event", name);
        }
        if let Some(id) = self.id {
            field(&mut out, b"id", id);
        }
        if let Some(wait) = self.retry {
            field(&mut out, b"retry", wait.as_millis().to_string().as_bytes());
        }

        // Normalize and split in one pass: a line ends at the first `\r` or
        // `\n`, and a `\r\n` is that one ending rather than two. A payload
        // ending in a line break therefore produces a trailing empty line,
        // which is exactly what makes the round trip exact — the client strips
        // one `\n` off its buffer at dispatch.
        let mut rest = self.data;
        loop {
            match rest
                .iter()
                .position(|byte| *byte == b'\n' || *byte == b'\r')
            {
                Some(at) => {
                    field(&mut out, b"data", &rest[..at]);
                    let ending = usize::from(rest[at] == b'\r' && rest.get(at + 1) == Some(&b'\n'));
                    rest = &rest[at + 1 + ending..];
                }
                None => {
                    field(&mut out, b"data", rest);
                    break;
                }
            }
        }

        out.push(b'\n');
        Ok(out)
    }
}

/// Why an event cannot be framed.
///
/// Three cases, each its own message, because each is a distinct thing the
/// program did rather than three spellings of "bad event" — and a caller that
/// had to read the text to tell them apart would be one parsing a sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// An event name carrying a line break or a NUL.
    EventName,
    /// An id carrying a line break or a NUL.
    Id,
    /// A payload with no bytes in it.
    EmptyData,
}

impl Refused {
    /// The sentence a program is told, and the whole of what the `Core` member
    /// throws.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::EventName => {
                "an event name cannot carry a line break or a NUL: a client dispatches on this \
                 name, and either byte would end the event early"
            }
            Self::Id => {
                "an event id cannot carry a line break or a NUL: either byte would end the event \
                 early, and a client discards an id containing a NUL outright"
            }
            Self::EmptyData => {
                "an event needs a payload: a client dispatches nothing for an empty data buffer, \
                 so this event could not arrive"
            }
        }
    }
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for Refused {}

/// One field line. The space after the colon is what a client strips, and
/// every line gets it — including an empty one, because one prefix is a rule a
/// reader can hold and two is a case to check.
fn field(out: &mut Vec<u8>, name: &[u8], value: &[u8]) {
    out.extend_from_slice(name);
    out.extend_from_slice(b": ");
    out.extend_from_slice(value);
    out.push(b'\n');
}

/// Whether a value would end the line it is written on, or the field it is
/// written in. The NUL is here with the two line breaks because a client that
/// reads one in an id discards the id, so the effect is the same: what the
/// application wrote is not what the peer acts on.
fn reframes_the_stream(value: &[u8]) -> bool {
    value
        .iter()
        .any(|byte| matches!(*byte, b'\n' | b'\r' | b'\x00'))
}

#[cfg(test)]
mod tests {
    use super::{Event, KEEPALIVE, Refused};
    use std::time::Duration;

    /// The three bytes a client strips from the head of a stream, which is why
    /// no frame may ever start with them.
    const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

    fn framed(event: Event<'_>) -> Vec<u8> {
        event.frame().expect("this event is well formed")
    }

    #[test]
    fn a_payload_carrying_a_lone_carriage_return_is_one_event_and_not_two() {
        let out = framed(Event::carrying(b"one\rtwo"));

        assert_eq!(
            out,
            b"data: one\ndata: two\n\n".to_vec(),
            "the `\\r` was not normalized"
        );
        assert!(!out.contains(&b'\r'), "a carriage return reached the wire");
        assert_eq!(
            out.windows(2).filter(|pair| *pair == b"\n\n").count(),
            1,
            "the payload dispatched a second event"
        );
        assert!(out.ends_with(b"\n\n"), "the terminator is not at the end");
    }

    #[test]
    fn every_line_of_a_multiline_payload_becomes_its_own_data_line() {
        let out = framed(Event::carrying(b"first\n\nthird\n"));

        // Four lines, the empty one in the middle and the empty one the
        // trailing break leaves, so the client's buffer strips back to exactly
        // the bytes that went in.
        assert_eq!(
            out,
            b"data: first\ndata: \ndata: third\ndata: \n\n".to_vec(),
            "the payload was not split line by line"
        );
        assert_eq!(
            out.windows(6).filter(|window| *window == b"data: ").count(),
            4,
            "a line did not get its own data field"
        );
    }

    #[test]
    fn a_crlf_payload_frames_the_same_as_the_lf_one() {
        let lf = framed(Event::carrying(b"one\ntwo\nthree"));
        let crlf = framed(Event::carrying(b"one\r\ntwo\r\nthree"));
        let cr = framed(Event::carrying(b"one\rtwo\rthree"));

        assert_eq!(crlf, lf, "a `\\r\\n` framed as two line endings");
        assert_eq!(cr, lf, "a lone `\\r` framed differently from a `\\n`");
        assert_eq!(lf, b"data: one\ndata: two\ndata: three\n\n".to_vec());
    }

    #[test]
    fn an_event_name_carrying_a_line_break_is_refused() {
        for bad in [&b"up\ndate"[..], &b"up\rdate"[..], &b"up\x00date"[..]] {
            let event = Event {
                event: Some(bad),
                ..Event::carrying(b"hi")
            };
            assert_eq!(
                event.frame(),
                Err(Refused::EventName),
                "{bad:?} framed an event name"
            );
        }

        let fine = Event {
            event: Some(b"update"),
            ..Event::carrying(b"hi")
        };
        assert!(fine.frame().is_ok(), "an ordinary name was refused");
    }

    #[test]
    fn an_id_carrying_a_line_break_or_a_nul_is_refused() {
        for bad in [&b"7\n9"[..], &b"7\r9"[..], &b"7\x009"[..]] {
            let event = Event {
                id: Some(bad),
                ..Event::carrying(b"hi")
            };
            assert_eq!(event.frame(), Err(Refused::Id), "{bad:?} framed an id");
        }

        let fine = Event {
            id: Some(b"79"),
            ..Event::carrying(b"hi")
        };
        assert!(fine.frame().is_ok(), "an ordinary id was refused");
        assert_ne!(
            Refused::Id.message(),
            Refused::EventName.message(),
            "the two sinks share one message"
        );
    }

    #[test]
    fn an_empty_data_payload_is_refused_rather_than_sent_undispatchable() {
        assert_eq!(
            Event::carrying(b"").frame(),
            Err(Refused::EmptyData),
            "an event no client dispatches was framed anyway"
        );

        // A payload that is one line ending is not an empty payload: the
        // client's buffer keeps a `\n` after the strip, so it dispatches.
        assert_eq!(
            framed(Event::carrying(b"\n")),
            b"data: \ndata: \n\n".to_vec()
        );
    }

    #[test]
    fn an_event_name_and_an_id_are_their_own_lines_before_the_data() {
        let out = framed(Event {
            event: Some(b"order"),
            id: Some(b"41"),
            ..Event::carrying(b"placed")
        });

        assert_eq!(out, b"event: order\nid: 41\ndata: placed\n\n".to_vec());
    }

    #[test]
    fn a_retry_line_is_milliseconds_in_ascii_digits() {
        let out = framed(Event {
            retry: Some(Duration::from_secs(3)),
            ..Event::carrying(b"hi")
        });

        assert_eq!(
            out,
            b"retry: 3000\ndata: hi\n\n".to_vec(),
            "the unit reached the wire wrong"
        );

        let sub_second = framed(Event {
            retry: Some(Duration::from_millis(250)),
            ..Event::carrying(b"hi")
        });
        assert!(
            sub_second.starts_with(b"retry: 250\n"),
            "a fraction of a second was lost"
        );

        let digits = &out[b"retry: ".len()..out.iter().position(|byte| *byte == b'\n').unwrap()];
        assert!(
            digits.iter().all(u8::is_ascii_digit),
            "the milliseconds are not ASCII digits"
        );
    }

    #[test]
    fn a_reconnect_block_carries_the_wait_alone_and_dispatches_nothing() {
        // The block is the `retry:` line and the blank line after it, so a
        // client takes the new wait and dispatches nothing — which is the whole
        // of what a program saying "do not come back for an hour" needs, and an
        // event with a payload attached to it would not be.
        assert_eq!(
            super::reconnect_after(Duration::from_secs(3600)),
            b"retry: 3600000\n\n".to_vec(),
            "the reconnection block is not one `retry:` line and the blank line after it"
        );
        assert!(
            !super::reconnect_after(Duration::from_secs(1))
                .windows(5)
                .any(|run| run == b"data:"),
            "the reconnection block carries a payload a client would dispatch"
        );
        assert_eq!(
            super::reconnect_after(Duration::from_micros(10)),
            b"retry: 0\n\n".to_vec(),
            "a wait shorter than the wire can spell is not the floor the wire can spell"
        );
    }

    #[test]
    fn a_keepalive_is_a_comment_and_dispatches_nothing() {
        assert_eq!(KEEPALIVE, b":\n\n", "the keepalive is not a bare comment");
        assert!(
            KEEPALIVE.starts_with(b":"),
            "a comment line does not start with a colon"
        );
        assert!(
            !KEEPALIVE.windows(5).any(|window| window == b"data:"),
            "the keepalive carries a field"
        );
    }

    #[test]
    fn no_byte_order_mark_is_ever_written() {
        let every_shape = [
            framed(Event::carrying(b"hi")),
            framed(Event {
                event: Some(b"order"),
                ..Event::carrying(b"hi")
            }),
            framed(Event {
                id: Some(b"41"),
                ..Event::carrying(b"hi")
            }),
            framed(Event {
                retry: Some(Duration::from_secs(1)),
                ..Event::carrying(b"hi")
            }),
            KEEPALIVE.to_vec(),
        ];
        for out in &every_shape {
            assert!(
                !out.starts_with(BOM),
                "a frame began with a byte order mark"
            );
            assert!(
                !out.windows(BOM.len()).any(|window| window == BOM),
                "a mark was written"
            );
        }

        // A payload that carries one keeps it: it is inside a data line, which
        // is never the head of the stream, so nothing strips it.
        let carried = framed(Event::carrying(b"\xEF\xBB\xBFhi"));
        assert!(
            carried.starts_with(b"data: "),
            "a payload's mark reached the head of a frame"
        );
        assert!(
            carried.ends_with(b"\xEF\xBB\xBFhi\n\n"),
            "the payload's own bytes changed"
        );
    }
}
