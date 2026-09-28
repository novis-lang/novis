//! `nvs run --peer <file>` — a WebSocket peer read off a file, so a program
//! runs as a connection isolate with no server in front of it.
//!
//! `--request`'s twin for the other kind of program a server starts. A
//! `Core\Socket` member asks one question of its context — whether it was
//! handed a [`nvs_runtime::PeerSocket`] — and `nvs_server::socket` is the only
//! thing that answers it yes. That leaves an example, an attack or a bench of
//! `receive` and `send` with nothing to run against but the `LogicError` every
//! other program gets. This module is the second thing that answers yes: the
//! frames the peer sends are the file's lines, and every frame the program
//! sends is a line on standard output.
//!
//! # The format
//!
//! One line per event, read top to bottom. A blank line and a line starting
//! `#` are skipped.
//!
//! ```text
//! text: Hello          a text frame the peer sends; the payload is the rest of the line
//! bytes: 00 ff 10      a binary frame, as hex pairs, spaces between them optional
//! fail: the reason     the socket fails here, and the `receive` that reaches it throws
//! reads: 2             the peer reads the program's first two frames and then stops reading
//! ```
//!
//! The end of the file is the peer closing, which is `receive`'s `null`.
//! `reads:` is one number for the whole file, wherever it is written; without
//! it the peer reads everything. A send past it fails as a send whose wait
//! expired does on a served connection, which is the one way a real peer that
//! stopped reading reaches a program.
//!
//! # What goes to standard output
//!
//! `sent: <payload>` for a text frame and `sent bytes: <hex pairs>` for a
//! binary one, and `closed: <code> <reason>` once, when the runtime closes the
//! socket from its own side. They are written through [`std::io::stdout`], the
//! handle `nvs_runtime::Ctx::stdout`'s `echo` writes through, so the two
//! interleave in the order the program ran them and an example's `.out` reads
//! as the conversation.
//!
//! # Beside `--request`
//!
//! Given with `--request`, the program is the request rather than the
//! connection. A request file whose `Upgrade` header names `websocket` is
//! offered the slot `Core\Socket::upgrade` fills ([`upgradable`]), and the
//! connection it prepares runs over this file's peer once the request ends
//! ([`connect`]). That is `nvs_server::serve_connection`'s order, with the
//! `101` left out because there is no client to read it.
//!
//! **What it spends:** the file's frames, held for the run, and nothing per
//! frame sent. A file is written by hand, so its size is the author's and no
//! request's.

use std::collections::VecDeque;
use std::io::Write;

use nvs_runtime::{Closing, PeerError, PeerFrame, PeerSocket};

/// One line of the file that the program's `receive` reaches.
#[derive(Debug, PartialEq, Eq)]
enum Incoming {
    /// A frame the peer sends.
    Frame(PeerFrame),
    /// The socket failing, with what the framing layer would have reported.
    Fail(String),
}

/// A peer whose frames were read off a file and whose sends are printed.
///
/// The default is the empty file: a peer that reads everything and closes
/// before it sends anything.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Scripted {
    /// What successive `receive` calls answer; an empty queue is the peer
    /// having closed.
    incoming: VecDeque<Incoming>,
    /// How many of the program's frames the peer reads, or `None` for all.
    reads: Option<usize>,
    /// How many it has read so far.
    sent: usize,
    /// Whether the runtime closed the socket from its own side. A closed
    /// socket answers every later call the way a served one does: `receive`
    /// finds the peer gone, `send` fails, and a second close sends nothing.
    closed: bool,
}

/// Reads the file `nvs run --peer <file>` names.
///
/// # Errors
///
/// The file could not be read, or [`read`] refused its text.
pub(crate) fn from_file(path: &std::path::Path) -> Result<Scripted, String> {
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    read(&text)
}

/// Parses the module doc's format.
///
/// # Errors
///
/// A line that is none of the four kinds, a `bytes:` line that is not hex
/// pairs, and a second `reads:` line, each named by its line number.
pub(crate) fn read(text: &str) -> Result<Scripted, String> {
    let mut incoming = VecDeque::new();
    let mut reads = None;
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((kind, rest)) = line.split_once(':') else {
            return Err(format!(
                "line {number}: expected `text:`, `bytes:`, `fail:` or `reads:`"
            ));
        };
        // The one space after the colon belongs to the syntax, and anything
        // past it is the payload, so a text frame keeps its own spacing.
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        match kind {
            "text" => incoming.push_back(Incoming::Frame(PeerFrame::Text(rest.to_owned()))),
            "bytes" => {
                let octets = hex(rest).map_err(|why| format!("line {number}: {why}"))?;
                incoming.push_back(Incoming::Frame(PeerFrame::Binary(octets)));
            }
            "fail" => incoming.push_back(Incoming::Fail(rest.to_owned())),
            "reads" => {
                if reads.is_some() {
                    return Err(format!("line {number}: `reads:` is written once"));
                }
                reads = Some(rest.trim().parse::<usize>().map_err(|_| {
                    format!("line {number}: `reads:` takes a whole number, got `{rest}`")
                })?);
            }
            other => {
                return Err(format!(
                    "line {number}: `{other}:` is not one of `text:`, `bytes:`, `fail:` or `reads:`"
                ));
            }
        }
    }
    Ok(Scripted {
        incoming,
        reads,
        sent: 0,
        closed: false,
    })
}

/// Whether `nvs run --request` offers this request an upgrade slot: its
/// `Upgrade` header names `websocket`.
///
/// A served request is offered one when `hyper` framed an upgrade for it and
/// it carries a `Sec-WebSocket-Key` of version 13. A request file describes a
/// handshake that already succeeded, so the header that asks is the whole
/// question here, and a file that leaves out the key still opens a connection.
pub(crate) fn upgradable(inbound: &nvs_runtime::Inbound) -> bool {
    inbound.headers().any(|(name, value)| {
        name.eq_ignore_ascii_case("upgrade")
            && value
                .split(|&byte| byte == b',')
                .any(|token| token.trim_ascii().eq_ignore_ascii_case(b"websocket"))
    })
}

/// Starts the connection a request's `Core\Socket::upgrade` prepared, over
/// `socket`, and waits for it to end.
///
/// What the connection `echo`es is `nvs_host::Output::Inherit`'s: it reaches
/// standard output when the connection ends, so it follows every `sent:` line
/// rather than interleaving with them. A server discards that output and has
/// no one to tell of a connection that failed; a run has a terminal, so a
/// failure is printed on standard error. The run's exit status stays the
/// request's, because the request is what `nvs run` was asked to answer.
pub(crate) fn connect(ctx: &mut nvs_runtime::Ctx, upgrade: nvs_runtime::Upgrade, socket: Scripted) {
    let (program, args) = upgrade.into_parts();
    match nvs_host::Isolate::new(program, args, nvs_host::Output::Inherit)
        .over_socket(Box::new(socket))
        .run(ctx)
    {
        Ok(mut done) => {
            done.discard_value();
            if let Some(failure) = done.error {
                eprintln!(
                    "error: the connection ended with {}: {}",
                    failure.class, failure.message
                );
            }
        }
        Err(refused) => {
            eprintln!("error: the connection could not start: {refused}");
        }
    }
}

/// Hex pairs, with any spacing between them.
fn hex(text: &str) -> Result<Vec<u8>, String> {
    let digits: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    if !digits.len().is_multiple_of(2) {
        return Err("`bytes:` takes whole hex pairs".to_owned());
    }
    digits
        .chunks(2)
        .map(|pair| {
            std::str::from_utf8(pair)
                .ok()
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                .ok_or_else(|| {
                    format!(
                        "`{}` is not a hex pair",
                        String::from_utf8_lossy(pair).into_owned()
                    )
                })
        })
        .collect()
}

/// A binary payload as the file writes it back: hex pairs, one space apart.
fn pairs(octets: &[u8]) -> String {
    octets
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// One line on standard output. A failed write is not the program's to see,
/// for the reason `echo`'s is not: the run's own output is closed.
fn say(line: &str) {
    let mut out = std::io::stdout();
    let _ = out.write_all(line.as_bytes());
    let _ = out.write_all(b"\n");
}

impl PeerSocket for Scripted {
    fn receive(&mut self) -> Result<Option<PeerFrame>, PeerError> {
        if self.closed {
            return Ok(None);
        }
        match self.incoming.pop_front() {
            None => Ok(None),
            Some(Incoming::Frame(frame)) => Ok(Some(frame)),
            Some(Incoming::Fail(why)) => Err(PeerError::new(why)),
        }
    }

    fn send(&mut self, frame: PeerFrame) -> Result<(), PeerError> {
        if self.closed {
            return Err(PeerError::new("the connection is closed"));
        }
        if self.reads.is_some_and(|reads| self.sent >= reads) {
            return Err(PeerError::new(
                "the client stopped reading, and the send wait expired",
            ));
        }
        self.sent += 1;
        match frame {
            PeerFrame::Text(text) => say(&format!("sent: {text}")),
            PeerFrame::Binary(octets) => say(&format!("sent bytes: {}", pairs(&octets))),
        }
        Ok(())
    }

    fn close(&mut self, why: Closing) {
        if !std::mem::replace(&mut self.closed, true) {
            say(&format!("closed: {} {}", why.code(), why.reason()));
        }
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{PeerFrame, PeerSocket};

    use super::read;

    /// Every kind of line, in the order the program meets them, and the end of
    /// the file as the peer closing.
    #[test]
    fn a_peer_file_answers_its_frames_in_order_and_then_closes() {
        let mut peer = read(
            "# a comment\n\ntext: Hello, world\nbytes: 00ff 10\nfail: the peer went away\ntext:\n",
        )
        .expect("the file is well formed");
        assert_eq!(
            peer.receive(),
            Ok(Some(PeerFrame::Text("Hello, world".to_owned())))
        );
        assert_eq!(
            peer.receive(),
            Ok(Some(PeerFrame::Binary(vec![0x00, 0xff, 0x10])))
        );
        let failed = peer.receive().expect_err("the socket fails at `fail:`");
        assert_eq!(failed.message(), "the peer went away");
        assert_eq!(peer.receive(), Ok(Some(PeerFrame::Text(String::new()))));
        assert_eq!(peer.receive(), Ok(None));
    }

    /// `reads:` bounds the sends, and the one past it fails the way a send
    /// whose wait expired does.
    #[test]
    fn a_peer_that_stops_reading_fails_the_send_past_its_count() {
        let mut peer = read("reads: 1\n").expect("the file is well formed");
        peer.send(PeerFrame::Text("first".to_owned()))
            .expect("the peer reads the first frame");
        let refused = peer
            .send(PeerFrame::Binary(vec![1]))
            .expect_err("the peer reads no second frame");
        assert!(refused.message().contains("stopped reading"));
    }

    /// After the runtime closes the socket, the peer is gone: frames still in
    /// the file are never answered and a send fails.
    #[test]
    fn a_closed_peer_answers_nothing_and_takes_no_frame() {
        let mut peer = read("text: waiting\n").expect("the file is well formed");
        peer.close(nvs_runtime::Closing::SlowSubscriber);
        peer.close(nvs_runtime::Closing::SlowSubscriber);
        assert_eq!(peer.receive(), Ok(None));
        peer.send(PeerFrame::Text("late".to_owned()))
            .expect_err("a closed socket takes no frame");
    }

    /// A malformed line is refused with its number, and so is a second
    /// `reads:`.
    #[test]
    fn a_malformed_peer_file_is_refused_by_line() {
        for (text, needle) in [
            ("text: ok\nhello\n", "line 2"),
            ("bytes: 0g\n", "not a hex pair"),
            ("bytes: 0\n", "whole hex pairs"),
            ("reads: 1\nreads: 2\n", "written once"),
            ("reads: many\n", "whole number"),
            ("close: now\n", "`close:`"),
        ] {
            let refused = read(text).expect_err(text);
            assert!(
                refused.contains(needle),
                "{text:?} was refused with {refused:?}"
            );
        }
    }
}
