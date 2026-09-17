//! The shared tier's wire: RESP over `nvs_host`'s parking stream, the two
//! commands `rule:concurrency/a-cached-value-is-copied-across-the-boundary`'s operations become, and the server-side steps
//! `rule:core-classes/ratelimit-two-members`'s limiter and
//! `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`'s fleet lease
//! need.
//!
//! [`super`] owns the *policy* — which store is configured, where the grant
//! approved reaching it, and what an entry's bytes are. What is here is
//! the part that talks, and the split is the one
//! [`crate::http::transport`](../http/transport/index.html) already makes for the
//! same reason: this module takes a [`Target`] rather than a `Ctx`, so
//! nothing here can widen a decision the door already made and a test can drive
//! a whole exchange against a listener on loopback with no capability snapshot
//! in front of it.
//!
//! **Which transport that target names is the only thing this file reads off
//! it.** A store over TCP and a store over a Unix-domain socket speak the same
//! RESP down the same parking stream, so [`Transport`] is where the two part
//! company and every line after it is written once — the whole of what
//! `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host` costs the wire.
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
//! The fleet lease is the other decision only the store can take, and it is not
//! `Core\Cache`'s either. [`Connection::set_if_absent`] is `SET … NX PX`, whose
//! whole value is that two hosts asking at once get two different answers, and
//! [`Connection::renew_if_holder`] is [`RENEW`], a second `EVAL` that pushes the
//! expiry out only while the key still holds the token that took it.
//! `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease` is what needs
//! both and `nvs serve` is their only caller: a compare-and-set a *program*
//! could reach is new cross-request coordination
//! (`rule:concurrency/cross-request-state-is-explicit`), so the tier's own wire
//! stays `put` and `get` and these two are internal to this crate.
//!
//! Only the reply shapes those commands answer are read — a simple
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
use std::time::{Duration, Instant};

use nvs_host::net::NvsTcp;
#[cfg(unix)]
use nvs_host::net::NvsUnix;
use nvs_host::tls::NvsTls;

use super::{Dial, Target};

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
/// process's footprint. The scripts this client sends answer three integers and
/// one.
const ELEMENT_CEILING: i64 = 64;

/// The fleet lease's renewal, run where the lease lives.
///
/// A script rather than a `GET` here and a `PEXPIRE` from the caller, because
/// between those two round trips the lease can expire and another host take it —
/// and the extension would then land on *that* host's lease, which is the
/// failure carrying a token exists to catch. `PEXPIRE` answers `1` when it moved
/// an expiry and `0` when there was no key to move, and the table around it is
/// what makes both that and the refusal the one reply shape
/// [`Connection::eval`] already reads.
const RENEW: &str = "\
if redis.call('GET', KEYS[1]) == ARGV[1] then
  return {redis.call('PEXPIRE', KEYS[1], ARGV[2])}
end
return {0}
";

/// One core's connection to the shared store.
///
/// The dial is the one [`super`]'s door settled out of the configuration, and
/// its address is never resolved again here —
/// `rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`'s
/// rule that every attempt of one approval reuses the approved address, which is
/// what closes the window a second DNS answer would open. A socket target has no
/// address and so nothing to re-resolve, which is the same property arrived at
/// for free.
pub(crate) struct Connection {
    /// Which store was approved to be reached, and everything applied over the
    /// socket to reach it.
    dial: Dial,
    /// Every wait's bound: the handshake's, and each command's.
    timeout: Duration,
    /// Absent before the first dial and after a failed command.
    stream: Option<Transport>,
}

impl Connection {
    /// A connection on `dial`, not yet made.
    pub(crate) const fn new(dial: Dial, timeout: Duration) -> Self {
        Self {
            dial,
            timeout,
            stream: None,
        }
    }

    /// What this one is made with — what [`super`] compares a later approval
    /// against before reusing it.
    pub(crate) const fn dial(&self) -> &Dial {
        &self.dial
    }

    /// Dials, unless this connection already holds a stream, and applies the
    /// rest of the dial to a socket it just opened.
    ///
    /// **This is the only place the handshake goes out**, and it is here rather
    /// than anywhere a store is configured because [`Connection::command`] comes
    /// back through it after a dropped socket: a credential applied once would
    /// be a reconnect answering the wrong database under no authority, silently,
    /// on the one connection nobody asked for.
    ///
    /// # Errors
    ///
    /// The connect failure as text, which is what the door turns into the throw
    /// its card promises for a store that cannot be reached, or the store's own
    /// refusal of the credential or the index. A connection whose handshake
    /// failed is dropped rather than held: what is left of it is a socket the
    /// store refuses every command on.
    pub(crate) fn ensure(&mut self) -> Result<(), String> {
        if self.stream.is_some() {
            return Ok(());
        }
        let stream = Transport::dial(&self.dial.target, self.timeout)
            .map_err(|err| format!("connecting to {} failed: {err}", self.dial.target))?;
        self.stream = Some(stream);
        if let Err(refused) = self.handshake() {
            self.stream = None;
            return Err(refused);
        }
        Ok(())
    }

    /// Everything the dial carries beyond the socket, in the order a store
    /// applies it: `AUTH`, then `SELECT`.
    ///
    /// That order because a store behind a credential refuses `SELECT` like
    /// every other command until it has one. Each goes out only when the dial
    /// carries it, so a deployment that configured neither — which is one that
    /// set only `[cache.shared] url` — spends no round trip on either and dials
    /// exactly as it did before there was anything to apply.
    ///
    /// # Errors
    ///
    /// The exchange's failure, or a reply that is not the `+OK` each answers,
    /// which carries the store's own text: a credential the store refuses is
    /// read off its `-WRONGPASS`, and an index it does not have off its `-ERR`.
    /// Never the credential itself, which no message this client writes names.
    fn handshake(&mut self) -> Result<(), String> {
        if let Some(password) = &self.dial.password {
            let request = wire_command(&[b"AUTH", password.as_bytes()]);
            self.applied(&request, "AUTH")?;
        }
        if let Some(index) = self.dial.database {
            let request = wire_command(&[b"SELECT", index.to_string().as_bytes()]);
            self.applied(&request, "SELECT")?;
        }
        Ok(())
    }

    /// One handshake step sent and its `+OK` read, named by `step` in whatever
    /// it answers instead.
    ///
    /// # Errors
    ///
    /// As [`Connection::handshake`]. There is no replay here: a handshake is
    /// sent on a socket this call just opened, so a failure on it is that dial
    /// failing rather than a stale connection to be made again.
    fn applied(&mut self, request: &[u8], step: &str) -> Result<(), String> {
        match self.exchange(request).map_err(|failure| failure.why)? {
            Reply::Simple(word) if word == "OK" => Ok(()),
            other => Err(other.unexpected(step)),
        }
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

    /// `SET key payload PX milliseconds` — the entry replaced, and the store
    /// told when to forget it.
    ///
    /// A second method rather than an `Option<Duration>` on
    /// [`Connection::set`], because writing an entry with no expiry at all and
    /// writing one with a lifetime are two decisions, and a default argument
    /// lets the first be reached by leaving the second out: a `Core\Cache`
    /// entry written without a `ttl` is bounded by the tier it is in, while a
    /// [`crate::session`] record written without one is
    /// `rule:http-server/session-expiry-belongs-to-the-store`
    /// 's sweeper coming back. Two names cannot be omitted.
    ///
    /// Milliseconds rather than `EX`'s seconds, and what a lifetime shorter
    /// than one becomes, is [`expiry`]'s: the lease's two commands carry the
    /// same lifetime through the same conversion.
    ///
    /// # Errors
    ///
    /// As [`Connection::set`].
    pub(crate) fn set_expiring(
        &mut self,
        key: &[u8],
        payload: &[u8],
        ttl: Duration,
    ) -> Result<(), String> {
        let millis = expiry(ttl);
        match self.command(
            &[b"SET", key, payload, b"PX", millis.as_bytes()],
            Replay::Idempotent,
        )? {
            Reply::Simple(word) if word == "OK" => Ok(()),
            other => Err(other.unexpected("SET")),
        }
    }

    /// `SET key token NX PX milliseconds` — the key taken **only if nothing
    /// holds it**, and the answer to whether this caller is what holds it now.
    ///
    /// `true` is a key that was absent or expired and now carries `token`;
    /// `false` is another holder's token already there. `NX` is what makes that
    /// one decision the store's rather than this process's: a read followed by a
    /// write hands two hosts asking at once the same `true` under exactly the
    /// load that makes it matter, which is the failure
    /// `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease` exists to
    /// prevent.
    ///
    /// The lifetime is not optional and has no unbounded spelling: a lease is
    /// released by expiry when the host holding it dies, so one written without
    /// an expiry would block its key for as long as the store lived
    /// (`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`).
    ///
    /// # Errors
    ///
    /// As [`Connection::set`]. Not replayed past a request that reached the
    /// store, because here the **reply is the effect**: a second attempt reads
    /// back the key its own first attempt took and reports the lease lost to its
    /// own winning write.
    pub(crate) fn set_if_absent(
        &mut self,
        key: &[u8],
        token: &[u8],
        ttl: Duration,
    ) -> Result<bool, String> {
        let millis = expiry(ttl);
        match self.command(
            &[b"SET", key, token, b"NX", b"PX", millis.as_bytes()],
            Replay::OnlyIfUnsent,
        )? {
            Reply::Simple(word) if word == "OK" => Ok(true),
            Reply::Nil => Ok(false),
            other => Err(other.unexpected("SET")),
        }
    }

    /// `DEL key` — the entry forgotten, whether or not it was ever there.
    ///
    /// The count the store answers with is discarded on purpose:
    /// `rule:http-server/a-session-store-answers-four-operations`
    /// 's `destroy` is "forget the record under an id", and an id there was
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

    /// [`RENEW`] over `key` — the lease held for `ttl` longer, and only while
    /// `token` is still what holds it.
    ///
    /// `true` is the expiry moved. `false` is a key carrying another host's
    /// token, and a key carrying nothing at all: a lease that has already
    /// expired belongs to whoever takes it next, so renewal never writes one
    /// back into existence.
    ///
    /// # Errors
    ///
    /// As [`Connection::eval`], plus a reply that is not the one integer
    /// [`RENEW`] answers with.
    pub(crate) fn renew_if_holder(
        &mut self,
        key: &[u8],
        token: &[u8],
        ttl: Duration,
    ) -> Result<bool, String> {
        let millis = expiry(ttl);
        let answered = self.eval(RENEW, key, &[token, millis.as_bytes()])?;
        match answered.as_slice() {
            [1] => Ok(true),
            [0] => Ok(false),
            _ => Err(format!(
                "the store answered {answered:?} to a renewal, which is not the one integer \
                 the script returns"
            )),
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
        let target = &self.dial.target;
        let deadline = Instant::now() + self.timeout;
        let stream = self.stream.as_mut().ok_or_else(|| Failure {
            sent: false,
            why: format!("no connection to {target} to send on"),
        })?;
        stream.set_deadline(Some(deadline));
        stream
            .write_all(request)
            .and_then(|()| stream.flush())
            .map_err(|err| Failure {
                sent: false,
                why: format!("sending to {target} failed: {err}"),
            })?;
        Wire {
            stream,
            target,
            buf: Vec::new(),
            at: 0,
        }
        .reply()
        .map_err(|why| Failure { sent: true, why })
    }
}

/// The transports one store may be reached over, behind the three operations
/// everything above this line uses: a deadline, a write and a read.
///
/// An enum rather than a type parameter on [`Connection`], because which
/// transport a deployment configured is not something any caller knows at compile
/// time — [`Target`] is read out of `nvs.toml` — and a generic would push that
/// choice through every one of [`super`]'s signatures to buy nothing here: the
/// bytes are the same bytes either way. That holds for the TLS arm too: what a
/// session changes is which bytes leave the socket and nothing above it, so
/// every command in this module is written once.
///
/// The Unix arm is `#[cfg(unix)]` and [`Target::Socket`] is too, so this match is
/// exhaustive on both platforms with no arm that exists only to refuse.
enum Transport {
    /// A store that named a host, reached over TCP.
    Tcp(NvsTcp),
    /// A store that named a host under `rediss://`, reached over the same TCP
    /// with `rule:security/one-tls-client`'s session on top of it.
    ///
    /// Boxed, which is the direction that helps both arms: a session's state is
    /// an order of magnitude larger than a socket, and inline it would be the
    /// size of every [`Connection`] this process holds including each plaintext
    /// one. What the indirection costs is one pointer hop per record, on a path
    /// that is already decrypting one.
    Tls(Box<NvsTls<NvsTcp>>),
    /// A store that named a path, reached over a Unix-domain socket.
    #[cfg(unix)]
    Unix(NvsUnix),
}

impl Transport {
    /// Dials `target`, bounded by `timeout` as the TCP half always was.
    ///
    /// The TLS arm is that same connect and then a handshake, and it asks for no
    /// relaxation: verification against the compiled-in anchors is
    /// [`NvsTls::over`]'s only behaviour, and
    /// `rule:security/tls-trust-is-relaxed-only-under-a-host-grant` puts every
    /// weakening behind a grant proved at a call site with a host, which a store
    /// an operator configured is not.
    ///
    /// The handshake is bounded by the socket's own deadline rather than by a
    /// second clock, which is `nvs_host::tls`'s contract: one clock, on the
    /// thing that waits, set before the session and set again by the command
    /// that follows.
    ///
    /// # Errors
    ///
    /// The platform's connect failure — including `TimedOut`, which a local
    /// connect reaches only through a listener whose backlog is full — or, for a
    /// TLS store, the handshake's: `InvalidData` for a certificate no anchor
    /// vouches for or a name it does not answer to, and `TimedOut` for a peer
    /// that did not finish in time.
    fn dial(target: &Target, timeout: Duration) -> std::io::Result<Self> {
        match target {
            Target::Tcp(address) => NvsTcp::connect_timeout(*address, timeout).map(Self::Tcp),
            Target::Tls { address, name } => {
                let mut stream = NvsTcp::connect_timeout(*address, timeout)?;
                stream.set_deadline(Some(Instant::now() + timeout));
                NvsTls::over(stream, name).map(|session| Self::Tls(Box::new(session)))
            }
            #[cfg(unix)]
            Target::Socket(path) => NvsUnix::connect_timeout(path, timeout).map(Self::Unix),
        }
    }

    /// Ends every wait on this connection by `at`.
    fn set_deadline(&mut self, at: Option<Instant>) {
        match self {
            Self::Tcp(stream) => stream.set_deadline(at),
            Self::Tls(stream) => stream.set_deadline(at),
            #[cfg(unix)]
            Self::Unix(stream) => stream.set_deadline(at),
        }
    }
}

impl Read for Transport {
    /// Whatever has arrived, from whichever socket this is.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.read(buf),
            Self::Tls(stream) => stream.read(buf),
            #[cfg(unix)]
            Self::Unix(stream) => stream.read(buf),
        }
    }
}

impl Write for Transport {
    /// As much of `buf` as the socket took.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.write(buf),
            Self::Tls(stream) => stream.write(buf),
            #[cfg(unix)]
            Self::Unix(stream) => stream.write(buf),
        }
    }

    /// Pushes what is held, which is nothing on a plain socket and a record
    /// `rustls` has buffered on a session.
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Tcp(stream) => stream.flush(),
            Self::Tls(stream) => stream.flush(),
            #[cfg(unix)]
            Self::Unix(stream) => stream.flush(),
        }
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
    /// left may be retried. `EVAL` of the limiter's script, and the lease's two
    /// commands, whose reply *is* their effect: a replayed `SET … NX` answers
    /// `false` about the key its own first attempt took.
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
    stream: &'a mut Transport,
    /// Named by every failure, since a reply that will not frame is a fact
    /// about the peer.
    target: &'a Target,
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
                .map_err(|_| format!("{} sent an integer that is not one: {rest}", self.target)),
            "*" => {
                let count: i64 = rest.parse().map_err(|_| {
                    format!(
                        "{} sent an array length that is not a number: {rest}",
                        self.target
                    )
                })?;
                if count < 0 {
                    return Ok(Reply::Nil);
                }
                if count > ELEMENT_CEILING {
                    return Err(format!(
                        "{} answered {count} elements, past this client's {ELEMENT_CEILING}",
                        self.target
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
                        self.target
                    )
                })?;
                if len < 0 {
                    return Ok(Reply::Nil);
                }
                let len = usize::try_from(len).unwrap_or(usize::MAX);
                if len > REPLY_CEILING {
                    return Err(format!(
                        "{} answered {len} bytes, past this client's {REPLY_CEILING}-byte ceiling",
                        self.target
                    ));
                }
                Ok(Reply::Bulk(self.exact(len)?))
            }
            _ => Err(format!(
                "{} sent a reply this client does not read: {head}",
                self.target
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
            .map_err(|err| format!("reading from {} failed: {err}", self.target))?;
        if read == 0 {
            return Err(format!(
                "{} closed the connection part-way through a reply",
                self.target
            ));
        }
        if self.buf.len() + read > REPLY_CEILING {
            return Err(format!(
                "{} sent more than this client's {REPLY_CEILING}-byte reply ceiling",
                self.target
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
                    .map_err(|_| format!("{} sent a reply line that is not text", self.target));
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
                self.target
            ));
        }
        Ok(self.buf[start..start + len].to_vec())
    }
}

/// A lifetime as the whole milliseconds `PX` and `PEXPIRE` both count in.
///
/// Milliseconds rather than `EX`'s seconds, because a lifetime is a
/// `Core\Time\Duration` and a cache entry may be written for less than a
/// second — which under `EX` would round to the zero that is `SET`'s spelling
/// for an error. Anything shorter than a millisecond rounds **up** to one for
/// the same reason. A lifetime longer than a `u64` of them saturates rather
/// than wrapping: no deployment reaches that ceiling, and every one of them
/// would notice a wrap.
fn expiry(ttl: Duration) -> String {
    u64::try_from(ttl.as_millis().max(1))
        .unwrap_or(u64::MAX)
        .to_string()
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
    use std::net::{Ipv4Addr, SocketAddr, TcpListener};
    use std::time::Duration;

    use super::{Connection, Dial, Target};

    /// A listener on loopback and the address it took — the shape
    /// `crate::http::transport`'s own cases use, and the reason this module
    /// takes a target rather than a `Ctx`.
    fn listening() -> (TcpListener, SocketAddr) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let address = listener.local_addr().expect("the port it took");
        (listener, address)
    }

    /// Reads until `wanted` bytes have arrived, which is how a fake server knows
    /// a whole command is in hand without parsing one.
    ///
    /// Over whichever socket the fake store took: a command is the same bytes
    /// on both, which is the half of `Transport` these cases are here to show.
    fn read_exactly<S: Read>(stream: &mut S, wanted: usize) -> Vec<u8> {
        let mut got = vec![0_u8; wanted];
        stream.read_exact(&mut got).expect("the client's command");
        got
    }

    /// A bound Unix-domain listener and the path it took — [`listening`]'s
    /// sibling, and the only line a socket case writes that its TCP twin does
    /// not.
    ///
    /// The name is short on purpose: `sun_path` is 108 bytes, and a bind past
    /// it fails with `InvalidInput`, which reads like a bug in the stream
    /// rather than in the name it was handed. A run that was killed leaves the
    /// node behind and `bind` refuses an existing one with `AddrInUse`, so the
    /// path is cleared first.
    #[cfg(unix)]
    fn socket_listening(name: &str) -> (std::os::unix::net::UnixListener, std::path::PathBuf) {
        let mut path = std::env::temp_dir();
        path.push(format!("nvs-redis-{}-{name}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener =
            std::os::unix::net::UnixListener::bind(&path).expect("the OS refused the path");
        (listener, path)
    }

    /// A lifetime reaches this tier as a **length of time** and never as an
    /// instant, because the store keeps its own clock — as whole milliseconds,
    /// rounded up, since zero is `SET`'s spelling for an error rather than for
    /// an entry that is already gone.
    ///
    /// Two writes on one connection, because either alone passes a command
    /// that had the scale right and the rounding wrong.
    #[test]
    fn a_lifetime_crosses_as_whole_milliseconds_rounded_up() {
        const LONG: &[u8] =
            b"*5\r\n$3\r\nSET\r\n$1\r\nk\r\n$2\r\nhi\r\n$2\r\nPX\r\n$5\r\n90000\r\n";
        const SHORT: &[u8] = b"*5\r\n$3\r\nSET\r\n$1\r\nk\r\n$2\r\nhi\r\n$2\r\nPX\r\n$1\r\n1\r\n";

        let (listener, address) = listening();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the client dials once");
            let first = read_exactly(&mut stream, LONG.len());
            stream.write_all(b"+OK\r\n").expect("the reply");
            let second = read_exactly(&mut stream, SHORT.len());
            stream.write_all(b"+OK\r\n").expect("the reply");
            (first, second)
        });

        let mut connection = Connection::new(
            Dial::configured(Target::Tcp(address), None, None),
            Duration::from_secs(5),
        );
        connection.ensure().expect("the fake store is listening");
        connection
            .set_expiring(b"k", b"hi", Duration::from_secs(90))
            .expect("a `SET … PX` answering `+OK`");
        connection
            .set_expiring(b"k", b"hi", Duration::from_micros(500))
            .expect("a lifetime under the store's granularity is still a write");

        let (first, second) = server.join().expect("the fake store runs to completion");
        assert_eq!(
            first, LONG,
            "a lifetime crosses as milliseconds, not as the seconds `EX` counts"
        );
        assert_eq!(
            second, SHORT,
            "and one under a millisecond rounds up rather than to the zero `SET` refuses"
        );
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

        let mut connection = Connection::new(
            Dial::configured(Target::Tcp(address), None, None),
            Duration::from_secs(5),
        );
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

        let mut connection = Connection::new(
            Dial::configured(Target::Tcp(address), None, None),
            Duration::from_secs(5),
        );
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

    /// A dial carrying both halves a handshake applies, which the two cases
    /// below read off the socket as [`AUTH`] and [`SELECT`].
    fn credentialled(address: SocketAddr) -> Dial {
        Dial {
            target: Target::Tcp(address),
            password: Some("s3cret".to_owned()),
            database: Some(3),
        }
    }

    /// The credential [`credentialled`] dials with, on the wire.
    const AUTH: &[u8] = b"*2\r\n$4\r\nAUTH\r\n$6\r\ns3cret\r\n";
    /// The index it dials at, on the wire.
    const SELECT: &[u8] = b"*2\r\n$6\r\nSELECT\r\n$1\r\n3\r\n";
    /// The command that opens the connection both steps precede.
    const FIRST: &[u8] = b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n";

    /// Both halves of the dial go out **ahead of the command that opened the
    /// connection**, and `AUTH` goes out ahead of `SELECT`.
    ///
    /// The order is the claim, not an arrangement: a store behind a credential
    /// refuses `SELECT` like every other command until it has one, so a client
    /// that chose the database first would be refused by exactly the stores
    /// the credential is there for. Nothing in a reply distinguishes that from
    /// an index the store does not have.
    #[test]
    fn auth_and_select_precede_the_first_command_in_that_order() {
        let (listener, address) = listening();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the client dials once");
            let mut sent = Vec::new();
            sent.push(read_exactly(&mut stream, AUTH.len()));
            stream.write_all(b"+OK\r\n").expect("the `AUTH` reply");
            sent.push(read_exactly(&mut stream, SELECT.len()));
            stream.write_all(b"+OK\r\n").expect("the `SELECT` reply");
            sent.push(read_exactly(&mut stream, FIRST.len()));
            stream.write_all(b"$2\r\nhi\r\n").expect("the `GET` reply");
            sent
        });

        let mut connection = Connection::new(credentialled(address), Duration::from_secs(5));
        let answer = connection
            .get(b"k")
            .expect("a `GET` behind a handshake the store answered");

        let sent = server.join().expect("the fake store runs to completion");
        assert_eq!(
            sent,
            vec![AUTH.to_vec(), SELECT.to_vec(), FIRST.to_vec()],
            "the credential is proved and the index chosen before anything the program asked for"
        );
        assert_eq!(
            answer,
            Some(b"hi".to_vec()),
            "and the answer came from the store the dial names"
        );
    }

    /// The dial is applied on **every** connection this client makes, never
    /// once for the process: a socket the store dropped is dialled again behind
    /// the next command, and both steps go out on that one too.
    ///
    /// This is the half [`Connection::ensure`] is the handshake's one home for,
    /// and the half a handshake applied at boot fails. The reconnection is made
    /// without the program asking for it, so a store that closed a socket under
    /// an idle core would otherwise answer the next request's `GET` out of
    /// database zero, unauthenticated, and answer it *successfully* — a wrong
    /// answer rather than a failure, which is the one outcome nothing upstream
    /// can catch.
    #[test]
    fn a_reconnect_after_a_dropped_socket_sends_both_again() {
        let (listener, address) = listening();
        let server = std::thread::spawn(move || {
            let mut sent = Vec::new();
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().expect("the client dials");
                sent.push(read_exactly(&mut stream, AUTH.len()));
                stream.write_all(b"+OK\r\n").expect("the `AUTH` reply");
                sent.push(read_exactly(&mut stream, SELECT.len()));
                stream.write_all(b"+OK\r\n").expect("the `SELECT` reply");
                sent.push(read_exactly(&mut stream, FIRST.len()));
                stream.write_all(b"$2\r\nhi\r\n").expect("the `GET` reply");
                // The socket is dropped with the entry answered, which is a
                // store closing a connection it has finished with: the next
                // command meets a dead socket and dials again unasked.
            }
            sent
        });

        let mut connection = Connection::new(credentialled(address), Duration::from_secs(5));
        let dialled = connection
            .get(b"k")
            .expect("a `GET` on the socket just opened");
        let redialled = connection
            .get(b"k")
            .expect("a `GET` that dialled again behind the dropped socket");

        let sent = server.join().expect("the fake store runs to completion");
        assert_eq!(
            sent,
            vec![
                AUTH.to_vec(),
                SELECT.to_vec(),
                FIRST.to_vec(),
                AUTH.to_vec(),
                SELECT.to_vec(),
                FIRST.to_vec(),
            ],
            "the dial is applied to the reconnection too, or a dropped socket comes back \
             unauthenticated and pointed at a database nobody configured"
        );
        assert_eq!(
            (dialled, redialled),
            (Some(b"hi".to_vec()), Some(b"hi".to_vec())),
            "both answers came from the store the dial names"
        );
    }

    /// A store configured with neither a credential nor an index dials exactly
    /// as it did: the first bytes on a fresh connection are the command itself.
    ///
    /// That is every deployment on this chain today, so a handshake sent
    /// unconditionally would be a round trip added to every cold connection in
    /// the fleet to apply nothing.
    #[test]
    fn a_store_with_no_credential_and_no_index_sends_no_handshake() {
        const GET: &[u8] = b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n";

        let (listener, address) = listening();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the client dials once");
            let first = read_exactly(&mut stream, GET.len());
            stream.write_all(b"$-1\r\n").expect("the null reply");
            first
        });

        let mut connection = Connection::new(
            Dial::configured(Target::Tcp(address), None, None),
            Duration::from_secs(5),
        );
        assert_eq!(connection.get(b"k").expect("absence is an answer"), None);

        let first = server.join().expect("the fake store runs to completion");
        assert_eq!(
            first, GET,
            "a store with nothing to apply spends no round trip applying it"
        );
    }

    /// `rule:security/one-tls-client` on the wire: a store an operator spelled
    /// `rediss://` hands its socket to `nvs_host::tls` and speaks nothing in the
    /// clear, which is the failure the scheme was refused rather than
    /// half-served for.
    ///
    /// What the fake store asserts is the **first bytes off the socket**: a TLS
    /// record of type `0x16` carrying a handshake of type `0x01`, and not a
    /// `SET` or an `AUTH`. That is the whole claim, and it is the one an
    /// exchange against a real store could not make — a client that dropped the
    /// session and sent RESP would pass every assertion a Redis server makes.
    ///
    /// The second half is that the connection **fails**: the fake store speaks
    /// no TLS, so a client that treated a refused handshake as something to
    /// carry on past would come back holding a stream, and that is the shape of
    /// the bug. The refusal names the store as its URL spelled it.
    ///
    /// `crate::tests::outbound_client` is reached first because there is one
    /// client per process and this case would otherwise build it — that
    /// function's own doc is why, and it is the same client
    /// `crate::http::transport`'s cases run against.
    #[test]
    fn a_rediss_url_hands_the_socket_to_the_one_outbound_tls_client() {
        crate::tests::outbound_client();
        let (listener, address) = listening();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the client dials once");
            let mut opening = [0_u8; 6];
            stream
                .read_exact(&mut opening)
                .expect("the client's first bytes");
            opening
        });

        let target = super::super::endpoint(
            &format!("rediss://127.0.0.1:{}", address.port()),
            "Core\\Cache::shared()",
        )
        .expect("`rediss://` is a scheme this client reads");
        let mut connection =
            Connection::new(Dial::configured(target, None, None), Duration::from_secs(5));

        let refused = connection
            .ensure()
            .expect_err("a listener that speaks no TLS completes no handshake");
        assert!(
            refused.contains(&format!("127.0.0.1:{}", address.port())),
            "the refusal names the store the URL spelled: {refused}"
        );

        let opening = server.join().expect("the fake store runs to completion");
        assert_eq!(
            opening[0], 0x16,
            "the first byte off the socket is a TLS handshake record and not RESP"
        );
        assert_eq!(opening[1], 0x03, "and its record version is a TLS one");
        assert_eq!(
            opening[5], 0x01,
            "and the handshake it carries is a `ClientHello`"
        );
    }

    /// `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host` on the
    /// wire: a store an operator spelled `unix:` is reached over a socket and
    /// answers the same two commands, byte for byte, as the store one line
    /// above spelled `redis://`.
    ///
    /// What this pins is that [`Transport`] is the *whole* of what the second
    /// spelling costs. The bytes asserted here are the bytes
    /// [`a_put_and_a_get_are_one_set_and_one_get_on_one_connection`] asserts,
    /// deliberately: a socket that framed a command differently would mean a
    /// second RESP writer had grown above the transport, and that is the only
    /// failure the split exists to prevent.
    ///
    /// The case is unconditional and its **body** is gated, not the other way
    /// round, so a build with no `AF_UNIX` transport still has a test of this
    /// name rather than one that reads as never written. On that side there is
    /// no `Target::Socket` to dial, so what stands in its place is the fact
    /// that makes the exchange unreachable: the door refuses the spelling, and
    /// the wire is never handed a target it could not have dialled.
    #[test]
    fn a_record_written_through_a_socket_is_read_back_through_it() {
        #[cfg(unix)]
        {
            let (listener, path) = socket_listening("record");
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

            let mut connection = Connection::new(
                Dial::configured(Target::Socket(path.clone()), None, None),
                Duration::from_secs(5),
            );
            connection.ensure().expect("the fake store is listening");
            connection
                .set(b"k", b"hi")
                .expect("a `SET` answering `+OK`");
            let got = connection.get(b"k").expect("a `GET` answering its bulk");

            let (set, get) = server.join().expect("the fake store runs to completion");
            assert_eq!(
                set, b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$2\r\nhi\r\n",
                "the socket framed a `SET` differently from the TCP twin"
            );
            assert_eq!(
                get, b"*2\r\n$3\r\nGET\r\n$1\r\nk\r\n",
                "the socket framed a `GET` differently from the TCP twin"
            );
            assert_eq!(
                got,
                Some(b"hi".to_vec()),
                "the record went through the socket and did not come back"
            );
            let _ = std::fs::remove_file(&path);
        }

        #[cfg(not(unix))]
        {
            assert!(
                super::super::endpoint("unix:/run/redis.sock", "Core\\Cache::shared()").is_err(),
                "a spelling this build cannot dial must be refused before the wire sees it"
            );
        }
    }

    /// The port `tests/db/compose.yaml` publishes its `redis` on — non-standard
    /// on purpose, so a developer's own store is never the one a case writes to.
    const REDIS: u16 = 16379;

    /// Two clients onto that store and a key no other run holds, or [`None`]
    /// when nothing is listening there.
    ///
    /// The two lease cases below are the only ones in this module that are not
    /// written against a fake, and they cannot be: a set-if-absent and an expiry
    /// are the **server's** semantics, and a fake store agrees with whatever the
    /// client that scripted it sent. What they assert is that a real Redis
    /// answers `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`'s
    /// two questions the way `Leases` needs them answered.
    ///
    /// A machine with no store running skips rather than fails, which is the
    /// rule the `.nvst` cases over this tier already carry in their `--SKIPIF--`
    /// section: what cannot be reached cannot be asserted, and the skip says so
    /// on stderr rather than quietly.
    ///
    /// The key carries this process's id so two runs at once are two leases, and
    /// it is deleted on the way in because a run that was killed mid-case leaves
    /// one behind for as long as its TTL.
    // The workspace denies a print in a library because user-facing output
    // belongs in `nvs-cli`. A case that asserted nothing is not that output: it
    // is addressed to whoever is reading the test run, and a skip nobody is told
    // about reads exactly like a case that passed.
    #[allow(clippy::print_stderr)]
    fn lease_case(label: &str) -> Option<(Connection, Connection, Vec<u8>)> {
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, REDIS));
        let mut first = Connection::new(
            Dial::configured(Target::Tcp(address), None, None),
            Duration::from_secs(5),
        );
        if let Err(why) = first.ensure() {
            eprintln!(
                "the {label} lease case asserted nothing: no store at {address} ({why}). \
                 `docker compose -f tests/db/compose.yaml up -d --wait redis` starts the one \
                 these cases are written against."
            );
            return None;
        }
        let mut second = Connection::new(
            Dial::configured(Target::Tcp(address), None, None),
            Duration::from_secs(5),
        );
        second
            .ensure()
            .expect("a second connection to a store that has already answered one");
        let key = format!("nvs:test:lease:{}:{label}", std::process::id()).into_bytes();
        first.del(&key).expect("a key an earlier run may have left");
        Some((first, second, key))
    }

    /// A lease is one host's for as long as it holds it, and the store is what
    /// decides which host that is.
    ///
    /// The second of two clients asking for a key the first holds is told
    /// `false` rather than made a second holder, and the same client asking
    /// again once the TTL has passed is told `true`. Those are § 3's two halves:
    /// at most one run per interval, and a host that dies holding the lease
    /// releases it by expiry instead of blocking the next interval forever.
    #[test]
    fn a_shared_tier_lease_is_taken_by_one_of_two_clients_and_expires_after_its_ttl() {
        let Some((mut first, mut second, key)) = lease_case("two-clients") else {
            return;
        };
        let held = Duration::from_secs(1);

        assert!(
            first
                .set_if_absent(&key, b"first", held)
                .expect("the store answers the set-if-absent"),
            "nothing held the key, so the first client takes it"
        );
        assert!(
            !second
                .set_if_absent(&key, b"second", held)
                .expect("the store answers the set-if-absent"),
            "the key is held, so the second client is refused rather than made a second holder"
        );
        assert_eq!(
            first.get(&key).expect("the key the first client took"),
            Some(b"first".to_vec()),
            "the refused attempt overwrote the holder's token"
        );

        std::thread::sleep(held * 2);

        assert!(
            second
                .set_if_absent(&key, b"second", held)
                .expect("the store answers the set-if-absent"),
            "the lease expired with its TTL, so the next interval is takeable"
        );
        second.del(&key).expect("the key this case wrote");
    }

    /// The token is what a renewal is checked against, so a host extends only
    /// the lease it is actually holding.
    ///
    /// A lease taken for one second and renewed for thirty is still held two
    /// seconds later, which is the extension landing; the other client's renewal
    /// is refused while the first holds it; and once the key is gone no renewal
    /// writes it back, because an expired lease belongs to whoever takes it next.
    #[test]
    fn a_shared_tier_lease_is_renewed_only_while_the_token_is_still_the_holders() {
        let Some((mut holder, mut other, key)) = lease_case("renewal") else {
            return;
        };
        let held = Duration::from_secs(1);
        let extended = Duration::from_secs(30);

        assert!(
            holder
                .set_if_absent(&key, b"holder", held)
                .expect("the store answers the set-if-absent"),
            "nothing held the key, so this client takes it"
        );
        assert!(
            !other
                .renew_if_holder(&key, b"other", extended)
                .expect("the store runs the renewal"),
            "a host that does not hold the lease cannot extend it"
        );
        assert!(
            holder
                .renew_if_holder(&key, b"holder", extended)
                .expect("the store runs the renewal"),
            "the holder's own token extends the lease it took"
        );

        std::thread::sleep(held * 2);

        assert!(
            !other
                .set_if_absent(&key, b"other", held)
                .expect("the store answers the set-if-absent"),
            "the renewal pushed the expiry past the TTL the lease was taken for"
        );
        assert_eq!(
            other.get(&key).expect("the key the holder took"),
            Some(b"holder".to_vec()),
            "the refused renewal replaced the holder's token"
        );

        holder.del(&key).expect("the key this case wrote");
        assert!(
            !holder
                .renew_if_holder(&key, b"holder", extended)
                .expect("the store runs the renewal"),
            "a lease nobody holds is not renewed back into existence"
        );
    }
}
