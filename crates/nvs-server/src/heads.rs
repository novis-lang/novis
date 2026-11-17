//! The three rows of `rule:errors/http-message-defects` that `hyper` accepts on
//! a request: a `Content-Length` written twice with the same value, which it
//! merges; a `Content-Length` beside `Transfer-Encoding`, which it frames by
//! the encoding; and a bare LF ending a line of the head, which `httparse`
//! reads as a line ending. Every other row of that table `hyper` already
//! refuses.
//!
//! None is visible once `hyper` has framed the request, because the merged or
//! ignored header is gone from the map and the line endings were never kept. So
//! [`Scanner`] reads the bytes on their way into `hyper`, in
//! [`crate::io::ConnectionIo`]'s read, and files what it found in [`Heads`],
//! which `crate::serve`'s service reads before anything else about a request.
//! The service counts the requests it is given, and the scanner counts the
//! heads it has read; the two counts are how a verdict is matched to its
//! request on a connection that carries many.
//!
//! # Following the body is what makes the next head findable
//!
//! A pipelined request arrives in the same read as the body before it, so the
//! scanner skips each body by the framing its own head named: the length, or
//! the chunks and the trailers after them. That is a second reading of the
//! framing, and the place two readers could disagree. The scanner never
//! settles a disagreement in a request's favour: any framing it cannot follow
//! makes it stop, and every request from the head it could not read is refused
//! ([`Heads::refused`]). The framings it stops on are the ones `hyper` refuses
//! too, so a connection that reaches them is already closing.
//!
//! **What it spends**, per `rule:programs/memory-priority`: one [`Scanner`]
//! of about a hundred bytes per connection, held for the connection's life,
//! and one search per line of a head for the CR or LF that ends it. Only a
//! line that can be `Content-Length` or `Transfer-Encoding` is stepped through
//! a byte at a time, with each line's ending and the bytes that tell the other
//! lines apart from those two. A body is skipped by its length, except for each
//! chunk's size line, and a trailer line is searched like a head line.

use std::cell::Cell;
use std::rc::Rc;

/// The bytes of one header line the scanner keeps: enough for every
/// `Content-Length` and `Transfer-Encoding` line a client sends. A longer line
/// of either is one the scanner cannot read, and stops on, except that a comma
/// anywhere in a `Content-Length` line is seen whether it was kept or not.
const LINE: usize = 64;

/// How the two header lines that frame a body start, with their colon.
const FRAMING: [&[u8]; 2] = [b"content-length:", b"transfer-encoding:"];

/// The length of the longer of [`FRAMING`]. A line kept this far that still
/// matches one of them is a framing line.
const FRAMING_LONGEST: usize = b"transfer-encoding:".len();

/// Whether a head line that starts with `prefix` can be a framing line,
/// with header names compared without case.
fn may_frame(prefix: &[u8]) -> bool {
    FRAMING.iter().any(|name| {
        let shared = prefix.len().min(name.len());
        prefix[..shared].eq_ignore_ascii_case(&name[..shared])
    })
}

/// What the scanner has found on one connection, shared with the service that
/// answers its requests.
#[derive(Debug, Default)]
pub struct Heads {
    /// How many heads have been read to their end with nothing found.
    clean: Cell<u64>,
    /// The index of the head a defect was found in, once one was.
    defect: Cell<Option<u64>>,
    /// The index of the first head the scanner did not read, once it stopped.
    lost: Cell<Option<u64>>,
}

impl Heads {
    /// Whether the request at `index` on this connection, counted from `0`,
    /// must be answered with a `400` that closes the connection.
    ///
    /// It must when its head carried a defect, when the scanner stopped before
    /// its head, or when the scanner never saw that head end. The last one is
    /// a request `hyper` framed and this module did not, which is the
    /// disagreement the module doc refuses.
    #[must_use]
    pub fn refused(&self, index: u64) -> bool {
        self.defect.get() == Some(index)
            || self.lost.get().is_some_and(|at| at <= index)
            || index >= self.clean.get()
    }
}

/// Where in the byte stream the scanner is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    /// Before a request line, where empty lines are skipped as `httparse`
    /// skips them.
    Start,
    /// Inside a head, on its request line or a header line.
    Head,
    /// Inside a body of a known length, with this many bytes left.
    Sized(u64),
    /// On a chunk's size line, with the size read so far and whether a digit
    /// has been read and the extension after the digits has started.
    ChunkSize {
        size: u64,
        digits: bool,
        extension: bool,
    },
    /// Inside a chunk's data, with this many bytes left.
    ChunkData(u64),
    /// On the line ending after a chunk's data.
    ChunkEnd,
    /// Among the trailer lines after the last chunk, with whether the line
    /// being read has any bytes yet.
    Trailers { started: bool },
    /// A defect was found or the framing could not be followed. Nothing after
    /// it is read, because the connection is closed after the next answer.
    Stopped,
}

/// The framing facts one head has given so far.
#[derive(Debug, Default)]
struct Framing {
    /// How many lengths the `Content-Length` lines carried. A line with a
    /// comma in it counts as two.
    lengths: u32,
    /// The length, when every `Content-Length` line was one run of digits.
    length: Option<u64>,
    /// A `Content-Length` line the scanner could not read as a number.
    length_unreadable: bool,
    /// Whether a `Transfer-Encoding` line was seen, and whether the last coding
    /// of the last one is `chunked`.
    chunked: Option<bool>,
}

/// The reader of one connection's request bytes. The module doc says what
/// it is for.
#[derive(Debug)]
pub struct Scanner {
    heads: Rc<Heads>,
    state: State,
    /// Whether the byte before this one was a CR.
    cr: bool,
    /// Whether the line being read is the head's request line.
    request_line: bool,
    line: [u8; LINE],
    len: usize,
    /// Whether the line being read is longer than [`LINE`].
    long: bool,
    /// Whether the line being read has a comma in it, kept or not.
    comma: bool,
    /// Whether the head line being read is known not to be a framing line.
    /// Its bytes up to its CR or LF are then skipped and not kept.
    plain: bool,
    framing: Framing,
    /// How many bytes [`Scanner::step`] has read one at a time.
    #[cfg(test)]
    stepped: usize,
}

impl Default for Scanner {
    fn default() -> Self {
        Self {
            heads: Rc::default(),
            state: State::Start,
            cr: false,
            request_line: true,
            line: [0; LINE],
            len: 0,
            long: false,
            comma: false,
            plain: false,
            framing: Framing::default(),
            #[cfg(test)]
            stepped: 0,
        }
    }
}

impl Scanner {
    /// The verdicts, for the service that answers this connection's requests.
    #[must_use]
    pub fn heads(&self) -> Rc<Heads> {
        Rc::clone(&self.heads)
    }

    /// Reads the next bytes the connection received, in the order it received
    /// them.
    ///
    /// A head line that cannot be a framing line, and a trailer line, are
    /// skipped up to their first CR or LF with one search. That byte and all
    /// the others outside a body's data go through [`Scanner::step`], so a
    /// bare LF or a stray CR is found as it is found a byte at a time.
    pub fn feed(&mut self, mut bytes: &[u8]) {
        while let Some((&byte, rest)) = bytes.split_first() {
            match self.state {
                State::Stopped => return,
                State::Sized(left) | State::ChunkData(left) => {
                    let skipped = usize::try_from(left).unwrap_or(usize::MAX).min(bytes.len());
                    let left = left - skipped as u64;
                    bytes = &bytes[skipped..];
                    self.state = match self.state {
                        State::Sized(_) if left == 0 => self.next_head(),
                        State::Sized(_) => State::Sized(left),
                        _ if left == 0 => State::ChunkEnd,
                        _ => State::ChunkData(left),
                    };
                    continue;
                }
                State::Head | State::Trailers { started: true } if self.skips_line() => {
                    let end = bytes
                        .iter()
                        .position(|&byte| byte == b'\r' || byte == b'\n')
                        .unwrap_or(bytes.len());
                    if end > 0 {
                        bytes = &bytes[end..];
                        continue;
                    }
                    self.step_counted(byte);
                }
                _ => self.step_counted(byte),
            }
            bytes = rest;
        }
    }

    /// Reads every byte outside a body's data through [`Scanner::step`], with
    /// no search for line ends. It is the reference the tests compare
    /// [`Scanner::feed`] with.
    #[cfg(test)]
    fn feed_bytewise(&mut self, mut bytes: &[u8]) {
        while let Some((&byte, rest)) = bytes.split_first() {
            match self.state {
                State::Stopped => return,
                State::Sized(left) | State::ChunkData(left) => {
                    let skipped = usize::try_from(left).unwrap_or(usize::MAX).min(bytes.len());
                    let left = left - skipped as u64;
                    bytes = &bytes[skipped..];
                    self.state = match self.state {
                        State::Sized(_) if left == 0 => self.next_head(),
                        State::Sized(_) => State::Sized(left),
                        _ if left == 0 => State::ChunkEnd,
                        _ => State::ChunkData(left),
                    };
                    continue;
                }
                _ => self.step(byte),
            }
            bytes = rest;
        }
    }

    /// Whether the bytes up to the next CR or LF can be skipped: the line is a
    /// head line known not to frame a body, or a trailer line, and no CR is
    /// waiting for its LF.
    fn skips_line(&self) -> bool {
        !self.cr
            && match self.state {
                State::Head => self.plain,
                State::Trailers { started } => started,
                _ => false,
            }
    }

    /// One byte through [`Scanner::step`], and then whether the head line it
    /// is in can still be a framing line.
    fn step_counted(&mut self, byte: u8) {
        self.step(byte);
        #[cfg(test)]
        {
            self.stepped += 1;
        }
        if self.state == State::Head
            && !self.plain
            && self.len > 0
            && self.len <= FRAMING_LONGEST
            && !self.long
        {
            self.plain = self.request_line || !may_frame(&self.line[..self.len]);
        }
    }

    /// One byte outside a body's data.
    fn step(&mut self, byte: u8) {
        let ends_line = self.cr && byte == b'\n';
        if self.cr && !ends_line {
            // A CR that does not end a line. `httparse` refuses it in a head
            // and `hyper` in a chunk's framing.
            return self.lose();
        }
        if byte == b'\n' && !ends_line {
            return match self.state {
                State::Start | State::Head => self.defect(),
                _ => self.lose(),
            };
        }
        self.cr = byte == b'\r';
        if self.cr {
            return;
        }
        match self.state {
            State::Start if ends_line => {}
            State::Start => {
                self.state = State::Head;
                self.keep(byte);
            }
            State::Head if ends_line => self.end_line(),
            State::Head => self.keep(byte),
            State::ChunkSize {
                size,
                digits,
                extension,
            } => {
                if ends_line {
                    self.state = match (digits, size) {
                        (false, _) => return self.lose(),
                        (true, 0) => State::Trailers { started: false },
                        (true, size) => State::ChunkData(size),
                    };
                    return;
                }
                let digit = char::from(byte).to_digit(16);
                self.state = match digit {
                    Some(digit) if !extension => {
                        let Some(size) = size
                            .checked_mul(16)
                            .and_then(|size| size.checked_add(u64::from(digit)))
                        else {
                            return self.lose();
                        };
                        State::ChunkSize {
                            size,
                            digits: true,
                            extension,
                        }
                    }
                    _ if !digits => return self.lose(),
                    _ => State::ChunkSize {
                        size,
                        digits,
                        extension: true,
                    },
                };
            }
            State::ChunkEnd if ends_line => {
                self.state = State::ChunkSize {
                    size: 0,
                    digits: false,
                    extension: false,
                };
            }
            State::ChunkEnd => self.lose(),
            State::Trailers { started } if ends_line => {
                self.state = if started {
                    State::Trailers { started: false }
                } else {
                    self.next_head()
                };
            }
            State::Trailers { .. } => self.state = State::Trailers { started: true },
            State::Sized(_) | State::ChunkData(_) | State::Stopped => {}
        }
    }

    /// One more byte of the line being read.
    fn keep(&mut self, byte: u8) {
        self.comma |= byte == b',';
        if self.len < LINE {
            self.line[self.len] = byte;
            self.len += 1;
        } else {
            self.long = true;
        }
    }

    /// The end of a line inside a head.
    fn end_line(&mut self) {
        let empty = self.len == 0 && !self.long;
        if empty {
            self.end_head();
        } else if self.request_line {
            self.request_line = false;
        } else if !self.plain {
            self.header_line();
        }
        self.len = 0;
        self.long = false;
        self.comma = false;
        self.plain = false;
    }

    /// A header line, read for the two framing headers and nothing else.
    fn header_line(&mut self) {
        let line = &self.line[..self.len];
        let Some(colon) = line.iter().position(|&byte| byte == b':') else {
            return;
        };
        let (name, value) = (&line[..colon], line[colon + 1..].trim_ascii());
        if name.eq_ignore_ascii_case(b"content-length") {
            self.framing.lengths += 1;
            if self.comma {
                // One line carrying two lengths is the same defect as two
                // lines carrying one each. The comma is looked for on the
                // whole line, so a second length past the kept bytes is seen
                // too.
                self.framing.lengths += 1;
            }
            let digits = !self.long && !value.is_empty() && value.iter().all(u8::is_ascii_digit);
            self.framing.length = digits
                .then(|| std::str::from_utf8(value).ok()?.parse().ok())
                .flatten();
            if self.framing.length.is_none() {
                self.framing.length_unreadable = true;
            }
        } else if name.eq_ignore_ascii_case(b"transfer-encoding") {
            let last = value
                .rsplit(|&byte| byte == b',')
                .next()
                .unwrap_or_default();
            self.framing.chunked =
                Some(!self.long && last.trim_ascii().eq_ignore_ascii_case(b"chunked"));
        }
    }

    /// The empty line that ends a head: the verdict on it, and the framing of
    /// the body after it.
    fn end_head(&mut self) {
        let framing = std::mem::take(&mut self.framing);
        // Two lengths, or a length beside `Transfer-Encoding`: `hyper` frames
        // the second by the encoding, and a proxy in front may have framed it
        // by the length.
        if framing.lengths > 1 || (framing.lengths > 0 && framing.chunked.is_some()) {
            return self.defect();
        }
        self.heads.clean.set(self.heads.clean.get() + 1);
        self.state = match (framing.chunked, framing.length) {
            (Some(true), _) => State::ChunkSize {
                size: 0,
                digits: false,
                extension: false,
            },
            (Some(false), _) => return self.lose(),
            (None, _) if framing.length_unreadable => return self.lose(),
            (None, Some(length)) if length > 0 => State::Sized(length),
            (None, _) => self.next_head(),
        };
    }

    /// The state that reads the next request's head.
    fn next_head(&mut self) -> State {
        self.request_line = true;
        State::Start
    }

    /// A defect in the head being read, which is the next one to be counted.
    fn defect(&mut self) {
        self.heads.defect.set(Some(self.heads.clean.get()));
        self.state = State::Stopped;
    }

    /// Framing the scanner cannot follow. Every head from the next one not yet
    /// read to its end is refused.
    fn lose(&mut self) {
        self.heads.lost.set(Some(self.heads.clean.get()));
        self.state = State::Stopped;
    }
}

#[cfg(test)]
mod tests {
    use super::{Scanner, State};

    /// The scanner's verdict on each request `bytes` carry, in order, for as
    /// many requests as `count` names.
    fn verdicts(bytes: &[u8], count: u64) -> Vec<bool> {
        let mut scanner = Scanner::default();
        scanner.feed(bytes);
        let heads = scanner.heads();
        (0..count).map(|index| heads.refused(index)).collect()
    }

    /// The same bytes fed one at a time give the same verdicts, because a
    /// head can arrive split across any number of reads.
    fn verdicts_bytewise(bytes: &[u8], count: u64) -> Vec<bool> {
        let mut scanner = Scanner::default();
        for byte in bytes {
            scanner.feed(std::slice::from_ref(byte));
        }
        let heads = scanner.heads();
        (0..count).map(|index| heads.refused(index)).collect()
    }

    #[test]
    fn a_repeated_length_and_a_bare_line_feed_are_refused_in_their_own_request() {
        let rows: [(&str, &[u8], Vec<bool>); 12] = [
            (
                "a plain request",
                b"GET / HTTP/1.1\r\nHost: a\r\n\r\n",
                vec![false],
            ),
            (
                "two equal lengths",
                b"POST / HTTP/1.1\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\nok",
                vec![true],
            ),
            (
                "two equal lengths on one line",
                b"POST / HTTP/1.1\r\nContent-Length: 2, 2\r\n\r\nok",
                vec![true],
            ),
            (
                "two equal lengths on one line, the second past the bytes the scanner keeps",
                b"POST / HTTP/1.1\r\nContent-Length: 2                                                  , 2\r\n\r\nok",
                vec![true],
            ),
            (
                "a bare LF ending the request line",
                b"GET / HTTP/1.1\nHost: a\r\n\r\n",
                vec![true],
            ),
            (
                "a bare LF ending a header line",
                b"GET / HTTP/1.1\r\nHost: a\n\r\n",
                vec![true],
            ),
            (
                "a bare LF in the second of two requests",
                b"GET /a HTTP/1.1\r\n\r\nGET /b HTTP/1.1\r\nHost: a\n\r\n",
                vec![false, true],
            ),
            (
                "a sized body before a second request",
                b"POST /a HTTP/1.1\r\nContent-Length: 5\r\n\r\nhelloGET /b HTTP/1.1\r\n\r\n",
                vec![false, false],
            ),
            (
                "a chunked body with an extension and a trailer before a second request",
                b"POST /a HTTP/1.1\r\nTransfer-Encoding: gzip, chunked\r\n\r\n5;x=y\r\nhello\r\nA\r\n0123456789\r\n0\r\nT: 1\r\n\r\nGET /b HTTP/1.1\r\n\r\n",
                vec![false, false],
            ),
            (
                "a length beside a chunked encoding",
                b"POST / HTTP/1.1\r\nContent-Length: 5\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
                vec![true],
            ),
            (
                "a chunked encoding beside a length",
                b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\nContent-Length: 5\r\n\r\n0\r\n\r\n",
                vec![true],
            ),
            (
                "a body whose bare LF is the body's own",
                b"POST /a HTTP/1.1\r\nContent-Length: 3\r\n\r\na\nbGET /b HTTP/1.1\r\n\r\n",
                vec![false, false],
            ),
        ];
        for (row, bytes, expected) in rows {
            let count = expected.len() as u64;
            assert_eq!(verdicts(bytes, count), expected, "{row}");
            assert_eq!(
                verdicts_bytewise(bytes, count),
                expected,
                "{row}, a byte at a time"
            );
        }
    }

    /// A framing the scanner cannot follow refuses every later request on the
    /// connection, and never the one whose head it did read.
    #[test]
    fn a_body_the_scanner_cannot_follow_refuses_what_comes_after_it() {
        let rows: [(&str, &[u8]); 4] = [
            (
                "a chunk size that is not hexadecimal",
                b"POST /a HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\nGET /b HTTP/1.1\r\n\r\n",
            ),
            (
                "a chunked terminator with a bare LF",
                b"POST /a HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\n\nGET /b HTTP/1.1\r\n\r\n",
            ),
            (
                "a final coding that is not chunked",
                b"POST /a HTTP/1.1\r\nTransfer-Encoding: chunked, gzip\r\n\r\nGET /b HTTP/1.1\r\n\r\n",
            ),
            (
                "a length that is not a number",
                b"POST /a HTTP/1.1\r\nContent-Length: +5\r\n\r\nhelloGET /b HTTP/1.1\r\n\r\n",
            ),
        ];
        for (row, bytes) in rows {
            assert_eq!(verdicts(bytes, 2), vec![false, true], "{row}");
        }
    }

    /// A request whose head has not ended yet has no verdict, so the service
    /// that is somehow asked about it refuses it.
    #[test]
    fn a_head_not_yet_read_to_its_end_is_refused() {
        assert_eq!(verdicts(b"GET / HTTP/1.1\r\nHost: a\r\n", 1), vec![true]);
    }

    /// Every request the other tests in this file read, and a few more that
    /// change the case of a framing header, share its first letters, or put a
    /// CR or a bare LF inside a trailer.
    const REQUESTS: &[&[u8]] = &[
        b"GET / HTTP/1.1\r\nHost: a\r\n\r\n",
        b"POST / HTTP/1.1\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\nok",
        b"POST / HTTP/1.1\r\nContent-Length: 2, 2\r\n\r\nok",
        b"POST / HTTP/1.1\r\nContent-Length: 2                                                  , 2\r\n\r\nok",
        b"GET / HTTP/1.1\nHost: a\r\n\r\n",
        b"GET / HTTP/1.1\r\nHost: a\n\r\n",
        b"GET /a HTTP/1.1\r\n\r\nGET /b HTTP/1.1\r\nHost: a\n\r\n",
        b"POST /a HTTP/1.1\r\nContent-Length: 5\r\n\r\nhelloGET /b HTTP/1.1\r\n\r\n",
        b"POST /a HTTP/1.1\r\nTransfer-Encoding: gzip, chunked\r\n\r\n5;x=y\r\nhello\r\nA\r\n0123456789\r\n0\r\nT: 1\r\n\r\nGET /b HTTP/1.1\r\n\r\n",
        b"POST / HTTP/1.1\r\nContent-Length: 5\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
        b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\nContent-Length: 5\r\n\r\n0\r\n\r\n",
        b"POST /a HTTP/1.1\r\nContent-Length: 3\r\n\r\na\nbGET /b HTTP/1.1\r\n\r\n",
        b"POST /a HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\nGET /b HTTP/1.1\r\n\r\n",
        b"POST /a HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\n\nGET /b HTTP/1.1\r\n\r\n",
        b"POST /a HTTP/1.1\r\nTransfer-Encoding: chunked, gzip\r\n\r\nGET /b HTTP/1.1\r\n\r\n",
        b"POST /a HTTP/1.1\r\nContent-Length: +5\r\n\r\nhelloGET /b HTTP/1.1\r\n\r\n",
        b"GET / HTTP/1.1\r\nHost: a\r\n",
        b"POST /a HTTP/1.1\r\ncontent-length: 2\r\nCONTENT-LENGTH: 2\r\n\r\nokGET /b HTTP/1.1\r\n\r\n",
        b"POST /a HTTP/1.1\r\ntransfer-ENCODING: chunked\r\n\r\n0\r\n\r\nGET /b HTTP/1.1\r\n\r\n",
        b"POST /a HTTP/1.1\r\nContent-Type: text/plain\r\nTransfer-Encodings: x\r\nContent-Length: 2\r\n\r\nokGET /b HTTP/1.1\r\n\r\n",
        b"POST /a HTTP/1.1\r\nContent-Length : 2\r\n\r\nGET /b HTTP/1.1\r\n\r\n",
        b"GET / HTTP/1.1\r\nHost: a\rb\r\n\r\n",
        b"GET /a\r HTTP/1.1\r\n\r\n",
        b"POST /a HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nT: 1\nGET /b HTTP/1.1\r\n\r\n",
        b"POST /a HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nT: 1\rx\r\n\r\nGET /b HTTP/1.1\r\n\r\n",
        b"\r\n\r\nGET /a HTTP/1.1\r\nHost: a\r\n\r\nGET /b HTTP/1.1\r\nHost: b\r\n\r\n",
    ];

    /// What a scanner has found and where it is: the three counts in its
    /// `Heads`, its state, and whether a CR or the request line is pending.
    type Outcome = (u64, Option<u64>, Option<u64>, State, bool, bool);

    /// The [`Outcome`] of `scanner` now.
    fn outcome(scanner: &Scanner) -> Outcome {
        let heads = scanner.heads();
        (
            heads.clean.get(),
            heads.defect.get(),
            heads.lost.get(),
            scanner.state,
            scanner.cr,
            scanner.request_line,
        )
    }

    /// `feed` gives the same counts and state as the byte-at-a-time reader,
    /// after the first part of a request and after all of it, wherever the
    /// request is split in two. It does the same when fed one byte per call.
    #[test]
    fn the_line_scanner_agrees_with_the_byte_scanner_on_every_split() {
        for request in REQUESTS {
            let shown = String::from_utf8_lossy(request);
            for split in 0..=request.len() {
                let (first, second) = request.split_at(split);
                let mut reference = Scanner::default();
                let mut scanner = Scanner::default();
                reference.feed_bytewise(first);
                scanner.feed(first);
                assert_eq!(
                    outcome(&scanner),
                    outcome(&reference),
                    "{shown:?}, the first {split} bytes"
                );
                reference.feed_bytewise(second);
                scanner.feed(second);
                assert_eq!(
                    outcome(&scanner),
                    outcome(&reference),
                    "{shown:?}, split after {split} bytes"
                );
            }
            let mut reference = Scanner::default();
            reference.feed_bytewise(request);
            let mut scanner = Scanner::default();
            for byte in *request {
                scanner.feed(std::slice::from_ref(byte));
            }
            assert_eq!(
                outcome(&scanner),
                outcome(&reference),
                "{shown:?}, a byte at a time"
            );
        }
    }

    /// One read of a head with four ordinary headers and a `Content-Length`
    /// line. The bytes read one at a time are exactly these: the whole
    /// `Content-Length` line, the first byte of the request line and of each
    /// ordinary header (which already rules out a framing header), the CR
    /// and LF that end every line, and the empty line's CR and LF. The body
    /// is skipped by its length.
    #[test]
    fn the_line_scanner_looks_at_one_byte_at_a_time_only_in_a_framing_line() {
        let framing_line: &[u8] = b"Content-Length: 5";
        let request: &[u8] = b"POST /upload/a/longer/path HTTP/1.1\r\n\
            Host: example.com\r\n\
            User-Agent: example-client/1.0 (a longer product string)\r\n\
            Accept: text/html, application/json;q=0.9, */*;q=0.8\r\n\
            Accept-Language: en-US, en;q=0.9, de;q=0.5\r\n\
            Content-Length: 5\r\n\
            \r\n\
            hello";
        let mut scanner = Scanner::default();
        scanner.feed(request);
        assert!(!scanner.heads().refused(0));
        assert!(scanner.heads().refused(1));
        let other_lines = 5;
        let expected = framing_line.len() + 2 + other_lines * (1 + 2) + 2;
        assert_eq!(scanner.stepped, expected);
        assert!(
            scanner.stepped * 4 < request.len(),
            "{} of {}",
            scanner.stepped,
            request.len()
        );
    }
}
