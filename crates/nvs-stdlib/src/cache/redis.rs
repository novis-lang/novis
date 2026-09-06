//! The shared tier's wire: RESP over `nvs_host`'s parking stream, the two
//! commands `rule:concurrency/a-cached-value-is-copied-across-the-boundary`'s operations become, and the one `EVAL` `rule:core-classes/ratelimit-two-members`'s
//! limiter needs.
//!
//! [`super`] owns the *policy* — which store is configured, which address the
//! capability approved it at, and what an entry's bytes are. What is here is
//! the part that talks, and the split is the one
//! [`crate::http::transport`](../http/transport/index.html) already makes for the
//! same reason: this module takes a [`SocketAddr`] rather than a `Ctx`, so
//! nothing here can widen a decision the door already made and a test can drive
//! a whole exchange against a listener on loopback with no capability snapshot
//! in front of it.
//!
//! # Two commands, hand-written, and no client library
//!
//! `SET` and `GET` over binary-safe bulk strings is the whole protocol this tier
//! uses, and RESP's framing for those two is four lines of parser. A crate would
//! bring a connection pool, an async runtime of its own and a command surface
//! forty times the size of what `rule:concurrency/a-cached-value-is-copied-across-the-boundary` defines — the second of which is
//! the disqualifying one, since a client with its own reactor would be the
//! neighbour-starving blocking read this project's parking stream exists to
//! remove. What that costs is this file; what it buys is that a shared `put`
//! parks on the same reactor every other outbound byte does.
//!
//! The third command is [`Connection::eval`], and it is not `Core\Cache`'s: the
//! GCRA step `Core\RateLimit::consume` needs is one read, one decision and one
//! write, and only the store can run those as one. The script is that member's
//! ([`crate::ratelimit`] owns it and the reasoning); what is here is `EVAL` and
//! the array of integers it answers with.
//!
//! Only the reply shapes those three commands answer are read — a simple
//! string, a bulk string, a null bulk, an error, an integer and an array of
//! integers. An array of anything else arrives here as "a reply this client does
//! not read" rather than as a variant nothing constructs.
//!
//! # One connection per core, and one reconnection behind a command that can take one
//!
//! [`super`] holds one [`Connection`] per core and hands it to every request
//! that runs there, so a `put` costs a round trip and not a handshake. The
//! server closes an idle connection eventually, and it closes it silently: the
//! failure arrives as an I/O error on the *next* command rather than as an
//! event. So [`Connection::command`] dials again and replays once. A failure on
//! the fresh connection is the one reported, because it is the one that
//! describes the store rather than the socket that had already gone.
//!
//! **Which failures may be replayed is the command's to say, and [`Replay`] is
//! where it says it.** `SET` and `GET` are idempotent — a `SET` written twice
//! leaves the same entry, and a `GET` has nothing to leave — so either may be
//! replayed however far it got. An `EVAL` of a limiter is not: a script that ran
//! and whose reply was lost has already charged the arrival, and replaying it
//! would charge a second one and refuse a request the policy admits. So that one
//! is replayed only when the request provably never left, which [`Failure::sent`]
//! is the whole of. The remaining case — the store ran the script and the socket
//! died before the reply — is a throw, and `rule:core-classes/ratelimit-unreachable-store-throws` is why that is the right
//! answer rather than a guess in either direction.
//!
//! What a command spends, per `rule:programs/memory-priority`:
//! the request text and one buffer holding the whole reply, both released with
//! the call and capped at [`REPLY_CEILING`]; plus one socket per core, which is
//! O(cores) and deliberately not O(requests served).

use std::io::{Read, Write};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use nvs_host::net::NvsTcp;

/// The port a `redis://host` with no `:port` on it names.
pub(super) const DEFAULT_PORT: u16 = 6379;

/// The most one reply will hold.
///
/// A cap and not a configuration, for [`crate::http::transport`]'s reason: a
/// reply is bytes another host chose, so "until memory runs out" is that host
/// deciding this process's footprint. An entry this tier wrote is bounded by
/// what the request could build; an entry something else wrote is not.
const REPLY_CEILING: usize = 8 * 1024 * 1024;

/// How much is read from the socket at a time.
const CHUNK: usize = 4096;

/// The most elements one array reply will hold.
///
/// [`REPLY_CEILING`]'s reason one level up: the count is a number the store
/// chose, so allocating against it unbounded is that store deciding this
/// process's footprint. The one script this client sends answers three.
const ELEMENT_CEILING: i64 = 64;

/// One core's connection to the shared store.
///
/// The address is the one [`super`]'s door pinned and is never re-resolved
/// here — [ADR 0058](/docs/adr/0058-outbound-request-policy.md) § 4's
/// rule that every attempt of one approval reuses the approved address, which is
/// what closes the window a second DNS answer would open.
pub(crate) struct Connection {
    /// Where the store was approved to be reached.
    address: SocketAddr,
    /// Every wait's bound: the handshake's, and each command's.
    timeout: Duration,
    /// Absent before the first dial and after a failed command.
    stream: Option<NvsTcp>,
}

impl Connection {
    /// A connection to `address`, not yet dialled.
    pub(crate) const fn new(address: SocketAddr, timeout: Duration) -> Self {
        Self {
            address,
            timeout,
            stream: None,
        }
    }

    /// Where this one was pinned — what [`super`] compares a later approval
    /// against before reusing it.
    pub(crate) const fn address(&self) -> SocketAddr {
        self.address
    }

    /// Dials, unless this connection already holds a stream.
    ///
    /// # Errors
    ///
    /// The connect failure as text, which is what the door turns into the throw
    /// its card promises for a store that cannot be reached.
    pub(crate) fn ensure(&mut self) -> Result<(), String> {
        if self.stream.is_some() {
            return Ok(());
        }
        let stream = NvsTcp::connect_timeout(self.address, self.timeout)
            .map_err(|err| format!("connecting to {} failed: {err}", self.address))?;
        self.stream = Some(stream);
        Ok(())
    }

    /// `SET key payload` — the entry replaced, whatever was there.
    ///
    /// # Errors
    ///
    /// The exchange's failure, or a reply that is not the `+OK` this command
    /// answers — including the store's own error text, which is the case an
    /// operator can act on.
    pub(crate) fn set(&mut self, key: &[u8], payload: &[u8]) -> Result<(), String> {
        match self.command(&[b"SET", key, payload], Replay::Idempotent)? {
            Reply::Simple(word) if word == "OK" => Ok(()),
            other => Err(other.unexpected("SET")),
        }
    }

    /// `SET key payload EX seconds` — the entry replaced, and the store told
    /// when to forget it.
    ///
    /// A second method rather than an `Option<u64>` on [`Connection::set`],
    /// because the two callers are two decisions and neither may drift into the
    /// other's: `Core\Cache`'s entries have no expiry at all
    /// (`rule:core-api/two-cache-tiers` gives the tier a cap and not a clock), while every
    /// [`crate::session`] record has one and a record written without one is
    /// [ADR 0139](/docs/adr/0139-a-session-is-a-record-its-store-issued.md)
    /// § 5's sweeper coming back. A default argument would let a caller reach
    /// the wrong one by omission; two names cannot be omitted.
    ///
    /// # Errors
    ///
    /// As [`Connection::set`].
    pub(crate) fn set_expiring(
        &mut self,
        key: &[u8],
        payload: &[u8],
        seconds: u64,
    ) -> Result<(), String> {
        let ttl = seconds.to_string();
        match self.command(
            &[b"SET", key, payload, b"EX", ttl.as_bytes()],
            Replay::Idempotent,
        )? {
            Reply::Simple(word) if word == "OK" => Ok(()),
            other => Err(other.unexpected("SET")),
        }
    }

    /// `DEL key` — the entry forgotten, whether or not it was ever there.
    ///
    /// The count the store answers with is discarded on purpose:
    /// [ADR 0139](/docs/adr/0139-a-session-is-a-record-its-store-issued.md)
    /// § 2's `destroy` is "forget the record under an id", and an id there was
    /// no record under is already forgotten — the same answer `load` gives it.
    /// Reading the count would be a second question about existence, which that
    /// section refuses for the reason it refuses `validateId`.
    ///
    /// # Errors
    ///
    /// As [`Connection::set`]. Replayable for that command's reason: sending
    /// `DEL` twice reaches the state sending it once reaches.
    pub(crate) fn del(&mut self, key: &[u8]) -> Result<(), String> {
        match self.command(&[b"DEL", key], Replay::Idempotent)? {
            Reply::Number(_) => Ok(()),
            other => Err(other.unexpected("DEL")),
        }
    }

    /// `GET key` — the entry's bytes, or `None` for one that is not there.
    ///
    /// # Errors
    ///
    /// As [`Connection::set`]. Absence is [`Reply::Nil`] and an ordinary answer
    /// rather than a failure, which is § 1's contract read off the wire.
    pub(crate) fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, String> {
        match self.command(&[b"GET", key], Replay::Idempotent)? {
            Reply::Bulk(bytes) => Ok(Some(bytes)),
            Reply::Nil => Ok(None),
            other => Err(other.unexpected("GET")),
        }
    }

    /// `EVAL script 1 key argv…` — the one server-side step, and the integers it
    /// answers with.
    ///
    /// One key always, because the only script this client sends is
    /// [`crate::ratelimit`]'s and it reads one: a script over several keys is a
    /// multi-key atomic check, which `rule:core-classes/ratelimit-two-members` names as deliberately absent.
    ///
    /// # Errors
    ///
    /// As [`Connection::set`], plus a reply that is not an array of integers —
    /// which means the store ran a script that is not the one this sent, and
    /// reading it as a decision would be inventing one. Never replayed past a
    /// request that reached the store, per the module doc's second section.
    pub(crate) fn eval(
        &mut self,
        script: &str,
        key: &[u8],
        argv: &[&[u8]],
    ) -> Result<Vec<i64>, String> {
        let mut parts: Vec<&[u8]> = vec![b"EVAL", script.as_bytes(), b"1", key];
        parts.extend_from_slice(argv);
        match self.command(&parts, Replay::OnlyIfUnsent)? {
            Reply::Array(numbers) => Ok(numbers),
            other => Err(other.unexpected("EVAL")),
        }
    }

    /// One command, with one reconnection behind it — the module doc's second
    /// section is why that replay is sound, and `replay` is where each command
    /// says how far it may be taken.
    ///
    /// # Errors
    ///
    /// The failure of the attempt on a *fresh* connection, never the one that
    /// only proved the old socket had gone — except for a command that may not
    /// be replayed once its request has left, whose first failure is the one
    /// reported precisely because a second attempt would be a second effect.
    fn command(&mut self, parts: &[&[u8]], replay: Replay) -> Result<Reply, String> {
        let request = wire_command(parts);
        if self.stream.is_some() {
            match self.exchange(&request) {
                Ok(reply) => return Ok(reply),
                Err(failure) => {
                    self.stream = None;
                    if failure.sent && replay == Replay::OnlyIfUnsent {
                        return Err(failure.why);
                    }
                }
            }
        }
        self.ensure()?;
        self.exchange(&request).map_err(|failure| {
            self.stream = None;
            failure.why
        })
    }

    /// Writes the request and reads one reply back, under one deadline covering
    /// both halves.
    ///
    /// # Errors
    ///
    /// An I/O failure at either end, or a reply this client cannot frame — the
    /// second is as fatal to the connection as the first, since a desynchronized
    /// stream has no way back. Each carries whether the request had already left
    /// this process, which is what [`Connection::command`] replays on.
    fn exchange(&mut self, request: &[u8]) -> Result<Reply, Failure> {
        let address = self.address;
        let deadline = Instant::now() + self.timeout;
        let stream = self.stream.as_mut().ok_or_else(|| Failure {
            sent: false,
            why: format!("no connection to {address} to send on"),
        })?;
        stream.set_deadline(Some(deadline));
        stream
            .write_all(request)
            .and_then(|()| stream.flush())
            .map_err(|err| Failure {
                sent: false,
                why: format!("sending to {address} failed: {err}"),
            })?;
        Wire {
            stream,
            address,
            buf: Vec::new(),
            at: 0,
        }
        .reply()
        .map_err(|why| Failure { sent: true, why })
    }
}

/// How far one command may be taken again after a failure — the module doc's
/// second section is the reasoning, and this is the whole of the mechanism.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Replay {
    /// Running it twice leaves what running it once leaves, so a failure at any
    /// point may be retried on a fresh connection. `SET` and `GET`.
    Idempotent,
    /// Running it twice is two effects, so only a request that provably never
    /// left may be retried. `EVAL` of the limiter's script.
    OnlyIfUnsent,
}

/// One failed exchange: what went wrong, and whether the request had already
/// reached the store when it did.
struct Failure {
    /// `true` once the request is written and flushed — from which point the
    /// store may have acted on it, whatever this end saw next.
    sent: bool,
    /// The failure as text, which is what [`super`] turns into the throw.
    why: String,
}

/// The six reply shapes [`Connection`]'s commands answer with.
enum Reply {
    /// `+OK`, and nothing else this client asks for.
    Simple(String),
    /// `-ERR …` — the store refusing, which is not this client failing.
    Error(String),
    /// `$<n>` and the bytes under it.
    Bulk(Vec<u8>),
    /// `$-1` — the entry is not there.
    Nil,
    /// `:<n>` — an integer, which is what a Lua script's numbers come back as.
    Number(i64),
    /// `*<n>` and `n` integers under it: the shape [`Connection::eval`]'s script
    /// answers, and the only array this client frames. An element that is not an
    /// integer is refused where it is read, because a script answering something
    /// else is not the script that was sent.
    Array(Vec<i64>),
}

impl Reply {
    /// What a reply that is not the one this command answers reads as.
    fn unexpected(self, command: &str) -> String {
        match self {
            Self::Error(why) => format!("the store refused `{command}`: {why}"),
            Self::Simple(word) => {
                format!("`{command}` answered `{word}`, which it does not answer")
            }
            Self::Bulk(_) => format!("`{command}` answered a value where it answers none"),
            Self::Nil => format!("`{command}` answered nothing where it answers a value"),
            Self::Number(n) => format!("`{command}` answered the number {n}, which it does not"),
            Self::Array(_) => format!("`{command}` answered a list where it answers one value"),
        }
    }
}

/// One reply being read off one stream: what has arrived, and how far it has
/// been consumed.
struct Wire<'a> {
    /// The stream the rest of the reply is still coming down.
    stream: &'a mut NvsTcp,
    /// Named by every failure, since a reply that will not frame is a fact
    /// about the peer.
    address: SocketAddr,
    /// Everything read so far, which for one command is at most one reply.
    buf: Vec<u8>,
    /// How much of [`Wire::buf`] the parse has taken.
    at: usize,
}

impl Wire<'_> {
    /// One whole reply.
    ///
    /// # Errors
    ///
    /// A read failure, a reply past [`REPLY_CEILING`], or a first byte naming a
    /// RESP shape none of this client's three commands answers with.
    fn reply(mut self) -> Result<Reply, String> {
        self.one()
    }

    /// One reply, from wherever the parse has reached — [`Wire::reply`] with the
    /// receiver borrowed, so that an array can read its own elements.
    ///
    /// # Errors
    ///
    /// As [`Wire::reply`], plus an array element that is not an integer and an
    /// array of more than [`ELEMENT_CEILING`] elements.
    fn one(&mut self) -> Result<Reply, String> {
        let head = self.line()?;
        let (marker, rest) = head.split_at(head.chars().next().map_or(0, char::len_utf8));
        match marker {
            "+" => Ok(Reply::Simple(rest.to_owned())),
            "-" => Ok(Reply::Error(rest.to_owned())),
            ":" => rest
                .parse()
                .map(Reply::Number)
                .map_err(|_| format!("{} sent an integer that is not one: {rest}", self.address)),
            "*" => {
                let count: i64 = rest.parse().map_err(|_| {
                    format!(
                        "{} sent an array length that is not a number: {rest}",
                        self.address
                    )
                })?;
                if count < 0 {
                    return Ok(Reply::Nil);
                }
                if count > ELEMENT_CEILING {
                    return Err(format!(
                        "{} answered {count} elements, past this client's {ELEMENT_CEILING}",
                        self.address
                    ));
                }
                let mut numbers = Vec::new();
                for _ in 0..count {
                    match self.one()? {
                        Reply::Number(n) => numbers.push(n),
                        other => return Err(other.unexpected("an array element")),
                    }
                }
                Ok(Reply::Array(numbers))
            }
            "$" => {
                let len: i64 = rest.parse().map_err(|_| {
                    format!(
                        "{} sent a bulk length that is not a number: {rest}",
                        self.address
                    )
                })?;
                if len < 0 {
                    return Ok(Reply::Nil);
                }
                let len = usize::try_from(len).unwrap_or(usize::MAX);
                if len > REPLY_CEILING {
                    return Err(format!(
                        "{} answered {len} bytes, past this client's {REPLY_CEILING}-byte ceiling",
                        self.address
                    ));
                }
                Ok(Reply::Bulk(self.exact(len)?))
            }
            _ => Err(format!(
                "{} sent a reply this client does not read: {head}",
                self.address
            )),
        }
    }

    /// More bytes, or the failure that says there will not be any.
    ///
    /// # Errors
    ///
    /// A read failure, end of file part-way through a reply, or a reply that has
    /// already passed [`REPLY_CEILING`].
    fn fill(&mut self) -> Result<(), String> {
        let mut chunk = [0_u8; CHUNK];
        let read = self
            .stream
            .read(&mut chunk)
            .map_err(|err| format!("reading from {} failed: {err}", self.address))?;
        if read == 0 {
            return Err(format!(
                "{} closed the connection part-way through a reply",
                self.address
            ));
        }
        if self.buf.len() + read > REPLY_CEILING {
            return Err(format!(
                "{} sent more than this client's {REPLY_CEILING}-byte reply ceiling",
                self.address
            ));
        }
        self.buf.extend_from_slice(&chunk[..read]);
        Ok(())
    }

    /// The next CRLF-terminated line, without its terminator.
    ///
    /// # Errors
    ///
    /// [`Wire::fill`]'s, or a line that is not text — every line this client
    /// reads is a marker and a number or a word.
    fn line(&mut self) -> Result<String, String> {
        loop {
            if let Some(offset) = self.buf[self.at..]
                .windows(2)
                .position(|pair| pair == b"\r\n")
            {
                let start = self.at;
                self.at = start + offset + 2;
                return String::from_utf8(self.buf[start..start + offset].to_vec())
                    .map_err(|_| format!("{} sent a reply line that is not text", self.address));
            }
            self.fill()?;
        }
    }

    /// Exactly `len` bytes and the CRLF after them — the bulk body, which is
    /// binary and is never read as text.
    ///
    /// # Errors
    ///
    /// [`Wire::fill`]'s, or a body whose terminator is not where its length says
    /// it is.
    fn exact(&mut self, len: usize) -> Result<Vec<u8>, String> {
        while self.buf.len() < self.at + len + 2 {
            self.fill()?;
        }
        let start = self.at;
        self.at = start + len + 2;
        if &self.buf[start + len..self.at] != b"\r\n" {
            return Err(format!(
                "{} sent a bulk reply that does not end where its length says",
                self.address
            ));
        }
        Ok(self.buf[start..start + len].to_vec())
    }
}

/// `parts` as a RESP array of bulk strings, which is how every command is sent.
///
/// Bulk strings and never inline text, because a key is whatever the program
/// wrote and a payload is arbitrary bytes: a length-prefixed frame has nothing
/// to quote and so nothing to get wrong.
fn wire_command(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = format!("*{}\r\n", parts.len()).into_bytes();
    for part in parts {
        out.extend_from_slice(format!("${}\r\n", part.len()).as_bytes());
        out.extend_from_slice(part);
        out.extend_from_slice(b"\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
    use std::time::Duration;

    use super::Connection;

    /// A listener on loopback and the address it took — the shape
    /// `crate::http::transport`'s own cases use, and the reason this module
    /// takes an address rather than a `Ctx`.
    fn listening() -> (TcpListener, SocketAddr) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let address = listener.local_addr().expect("the port it took");
        (listener, address)
    }

    /// Reads until `wanted` bytes have arrived, which is how a fake server knows
    /// a whole command is in hand without parsing one.
    fn read_exactly(stream: &mut TcpStream, wanted: usize) -> Vec<u8> {
        let mut got = vec![0_u8; wanted];
        stream.read_exact(&mut got).expect("the client's command");
        got
    }

    /// `rule:concurrency/a-cached-value-is-copied-across-the-boundary`: the shared tier's two operations are one `SET` and one
    /// `GET`, sent as RESP arrays of bulk strings, on **one** connection — the
    /// second command is not a second handshake.
    #[test]
    fn a_put_and_a_get_are_one_set_and_one_get_on_one_connection() {
        let (listener, address) = listening();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the client dials once");
            let set = read_exactly(
                &mut stream,
                b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$2\r\nhi\r\n".len(),
            );
            stream.write_all(b"+OK\r\n").expect("the reply");
            let get = read_exactly(&mut stream, b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n".len());
            stream.write_all(b"$2\r\nhi\r\n").expect("the reply");
            (set, get)
        });

        let mut connection = Connection::new(address, Duration::from_secs(5));
        connection.ensure().expect("the fake store is listening");
        connection
            .set(b"k", b"hi")
            .expect("a `SET` answering `+OK`");
        let got = connection.get(b"k").expect("a `GET` answering its bulk");

        let (set, get) = server.join().expect("the fake store runs to completion");
        assert_eq!(set, b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$2\r\nhi\r\n");
        assert_eq!(get, b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n");
        assert_eq!(got, Some(b"hi".to_vec()));
    }

    /// `rule:core-api/two-cache-tiers`: an entry that is not there is an answer and not a failure,
    /// and on the wire that answer is the null bulk. A store's own `-ERR` is the
    /// other half — that one *is* a failure, and it carries the store's text so
    /// an operator can act on it.
    #[test]
    fn a_missing_entry_is_the_null_bulk_and_a_refusal_is_not() {
        let (listener, address) = listening();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the client dials once");
            read_exactly(&mut stream, b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n".len());
            stream.write_all(b"$-1\r\n").expect("the null reply");
            read_exactly(&mut stream, b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n".len());
            stream
                .write_all(b"-NOAUTH Authentication required.\r\n")
                .expect("the refusal");
        });

        let mut connection = Connection::new(address, Duration::from_secs(5));
        connection.ensure().expect("the fake store is listening");
        assert_eq!(connection.get(b"k").expect("absence is an answer"), None);

        let refused = connection
            .get(b"k")
            .expect_err("a refusal is not an answer");
        assert!(
            refused.contains("NOAUTH"),
            "the store's own text is what an operator acts on: {refused}"
        );
        server.join().expect("the fake store runs to completion");
    }
}
