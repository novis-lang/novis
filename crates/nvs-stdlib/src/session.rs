//! [ADR 0139](../../../../docs/adr/0139-a-session-is-a-record-its-store-issued.md)'s session store:
//! the identifier a store issues, the record it keeps under it, and the two directives that decide
//! where that store is and how long a record survives.
//!
//! **What is on disk is § 2's store, not § 1's class.** The four operations a backend answers are
//! here — [`mint`], [`load`], [`save`] and the key they share — over the shared tier's wire, and
//! `Core\Session`'s seven members are a later slice with no registry rows yet. The split is
//! deliberate rather than incidental: the rule this milestone's acceptance check asserts is *where
//! a record lives*, which is a property of the store, and a member surface built over an
//! undecided store would have to be rewritten when it was decided.
//!
//! # Decision: the local tier is unreachable from here, structurally
//!
//! [ADR 0059](../../../../docs/adr/0059-cross-request-state-is-explicit.md) § 4 refuses
//! `Core\Session` the per-core tier, and § 4's own word for the refusal is *enforced*. Two things
//! carry that here and neither is a comment. [`nvs_config::session::Backend`] has no variant naming
//! the local tier, so there is no value this module could match on to select it; and every
//! operation below is written against [`crate::cache::redis::Connection`] rather than against
//! [`crate::cache`]'s tier enum, so the per-core map is not merely unselected but absent from the
//! type this module can reach. A test asserting the ban would pass over a module that reached
//! `store_put` on a branch nobody exercised — which is why
//! `a_session_is_never_backed_by_the_local_cache_tier` asserts the record's *absence* from that map
//! after a real write, and not only the roster.
//!
//! # Decision: the identifier is drawn, never derived
//!
//! [`mint`] draws 128 bits from the context's generator and renders them base64url. Not a hash of
//! anything the client supplied and not a counter: for the length of its life an id is a bearer
//! credential, and its one required property is that guessing one is not a strategy. 128 bits is
//! `Core\Uuid`'s draw and the same argument — the birthday bound over any number of sessions a
//! fleet will ever hold is far below the collision probability of anything else in the request
//! path.
//!
//! Drawn through [`crate::random::draw`] like every other draw in `Core`, so that a
//! `#[Test(seed: …)]` fixes this sequence with the rest rather than leaving one member
//! irreproducible.
//!
//! # What this spends
//!
//! One round trip to the configured store per request that starts a session, and a second only for
//! a request that changed the record (§ 4). Memory is one decoded record per in-flight request that
//! started one, released with the request heap — O(in-flight), never O(sessions served).

// Nothing outside this module's own tests calls any of it yet: ADR 0139 § 1's seven members are the
// next slice of this feature, and until they land `dead_code` is naming a slice that has not
// happened rather than an item nothing will use. `crates/nvs-cli/src/cache.rs` carries the same
// escape for the same reason, and both go away when their call sites arrive.
#![allow(dead_code)]

use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use nvs_config::session::Backend;
use nvs_runtime::{Ctx, Fault};
use nvs_syntax::duration;

use crate::cache::configured;
use crate::cache::redis::Connection;

/// What every session key starts with, so an operator sharing one store between a cache, a limiter
/// and a session store can tell the three apart in it.
///
/// [`crate::ratelimit`]'s `PREFIX` is the same decision, and the two must not collide: a session id
/// is drawn and a limiter's key is application-supplied, so without the prefixes an application
/// could name a limiter key that reads a session record.
pub(crate) const PREFIX: &str = "nvs:session:";

/// `[session] backend` — ADR 0139 § 3's store, as `nvs_config::session::Backend` spells it.
const BACKEND: &str = "session.backend";

/// `[session] ttl` — how long an untouched record survives.
const TTL: &str = "session.ttl";

/// `[session] cookie` — the name the identifier rides under.
const COOKIE: &str = "session.cookie";

/// The record lifetime a deployment that configured none inherits.
///
/// Two hours rather than PHP's twenty-four minutes: `session.gc_maxlifetime`'s default is a number
/// chosen when a session was a file a sweeper had to walk, and the cost of a longer one there was
/// disk that accumulated. Here the store expires the entry itself (§ 5), so the only thing the
/// number trades is how long a stolen id stays useful against how often an idle user is signed out,
/// and two hours is the ordinary answer to that pair. It is deliberately not "until the browser
/// closes": a session with no server-side expiry is one an attacker's copy never loses either.
const DEFAULT_TTL: Duration = Duration::from_secs(2 * 60 * 60);

/// The cookie name a deployment that configured none inherits.
///
/// Not `PHPSESSID` and not anything naming a framework: a cookie name is the one part of a session
/// that a scanner reads to decide what it is talking to, and there is nothing to gain by answering.
const DEFAULT_COOKIE: &str = "nvsid";

/// How many bytes an identifier is drawn from — 128 bits, per the module doc.
const ID_BYTES: usize = 16;

/// The store key one identifier's record lives under.
pub(crate) fn key_of(id: &str) -> Vec<u8> {
    let mut key = Vec::with_capacity(PREFIX.len() + id.len());
    key.extend_from_slice(PREFIX.as_bytes());
    key.extend_from_slice(id.as_bytes());
    key
}

/// Which store this deployment's records live in.
///
/// # Errors
///
/// A thrown `RuntimeError` for a tree that configured no `[session]` block, naming the block to
/// write. ADR 0139 § 3: an absent block is not a default backend, because the safe answer for a
/// store nobody chose is no store — the same direction
/// [ADR 0074](../../../../docs/adr/0074-http-defaults-safe-and-finite.md) takes for everything it
/// leaves unconfigured.
///
/// A word this module cannot spell is the same throw, and it is unreachable from a server that
/// booted: `nvs_config::session::validate` refused it at `E0626`. It is answered rather than
/// asserted because this crate is also reachable from a test driving a hand-built context, and a
/// panic there would report the fixture as an engine bug.
pub(crate) fn backend(ctx: &Ctx, member: &str) -> Result<Backend, Fault> {
    let written = configured(ctx, BACKEND);
    written.as_deref().and_then(Backend::of).ok_or_else(|| {
        Fault::thrown(format!(
            "Session::{member}(): no session store is configured — write `[session] backend = \
                 \"shared\"`, which is where a record has to live for a request on another core to \
                 find it (ADR 0059 § 4)"
        ))
    })
}

/// `[session] ttl`, or [`DEFAULT_TTL`].
///
/// A directive that will not parse, or that parses to zero, is the shipped lifetime rather than a
/// refusal — [`crate::cache`]'s `timeout_of` reasoning: `nvs.toml` is validated where it is loaded,
/// and a zero here would be a session expiring the instant it is written.
pub(crate) fn ttl(ctx: &Ctx) -> Duration {
    configured(ctx, TTL)
        .and_then(|text| duration::parse(&text).ok())
        .map(|nanos| Duration::from_nanos(nanos.unsigned_abs()))
        .filter(|lifetime| !lifetime.is_zero())
        .unwrap_or(DEFAULT_TTL)
}

/// `[session] cookie`, or [`DEFAULT_COOKIE`].
pub(crate) fn cookie(ctx: &Ctx) -> String {
    configured(ctx, COOKIE).unwrap_or_else(|| DEFAULT_COOKIE.to_owned())
}

/// An identifier no store has issued: 128 drawn bits, base64url with no padding.
pub(crate) fn mint(ctx: &mut Ctx) -> String {
    use rand::Rng as _;

    let mut drawn = [0_u8; ID_BYTES];
    crate::random::draw(ctx, |rng| rng.fill_bytes(&mut drawn));
    // The engine directly rather than through `crate::encoding`: that module's two members answer
    // `Core\Encoding`'s *arguments*, so reaching one from here would mean building a `Value` to
    // encode a `[u8; 16]` this function already holds.
    URL_SAFE_NO_PAD.encode(drawn)
}

/// § 2's `load` — the record under `id`, or **absent**.
///
/// Absent is the answer to all three of
/// [ADR 0124](../../../../docs/adr/0124-php-86-lands-as-four-refusals-and-one-session-rule.md)
/// § 6's cases at once: an id no store issued, one that has expired, and one an attacker minted.
/// `Core\Session::start()` responds to it by issuing a fresh id, which is why there is no separate
/// `validateId` for the two of them to disagree about.
///
/// # Errors
///
/// The store's own failure as text, for [`crate::cache::on_shared`] to classify. Never an answer
/// that looks like absence: a store that cannot be reached must not read as a forged id, because
/// the two have opposite responses.
pub(crate) fn load(open: &mut Connection, id: &str) -> Result<Option<Vec<u8>>, String> {
    open.get(&key_of(id))
}

/// § 2's `save` — the record under `id`, replaced, with its expiry refreshed.
///
/// The whole record and not one key of it, per § 4: one session is one entry, which is what keeps
/// `clear`, `regenerate` and `destroy` single-key operations over a store
/// [ADR 0075](../../../../docs/adr/0075-core-ratelimit.md) § 4 gives no multi-key atomic step.
///
/// A `ttl` under a second rounds **up** to one rather than down to zero: zero is `SET`'s spelling
/// for an error, and a record written with no expiry at all is § 5's sweeper coming back.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn save(
    open: &mut Connection,
    id: &str,
    record: &[u8],
    ttl: Duration,
) -> Result<(), String> {
    let seconds = ttl.as_secs().max(1);
    open.set_expiring(&key_of(id), record, seconds)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    use nvs_config::session::{BACKENDS, Backend};
    use nvs_runtime::Ctx;

    use super::{DEFAULT_TTL, PREFIX, backend, cookie, key_of, load, mint, save, ttl};
    use crate::cache::redis::Connection;
    use crate::cache::{store_get, store_put};

    /// The record two cores exchange, and the id it lives under.
    const ID: &str = "PmA5tKz1QvR3sYbN";
    const RECORD: &[u8] = b"user=17;csrf=aa";

    /// A listener on loopback and the address it took — `crate::cache::redis`'s own cases' shape.
    fn listening() -> (TcpListener, SocketAddr) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let address = listener.local_addr().expect("the port it took");
        (listener, address)
    }

    /// One RESP command read off `stream`, as its parts, or `None` at end of input.
    fn command(stream: &mut BufReader<TcpStream>) -> Option<Vec<Vec<u8>>> {
        let count = header(stream, b'*')?;
        let mut parts = Vec::with_capacity(count);
        for _ in 0..count {
            let len = header(stream, b'$')?;
            let mut part = vec![0_u8; len + 2];
            stream.read_exact(&mut part).expect("a bulk string");
            part.truncate(len);
            parts.push(part);
        }
        Some(parts)
    }

    /// The number on a `*N` or `$N` line.
    fn header(stream: &mut BufReader<TcpStream>, marker: u8) -> Option<usize> {
        let mut line = String::new();
        if stream.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let digits = line.trim_end().strip_prefix(marker as char)?;
        digits.parse().ok()
    }

    /// A store that keeps what it is told, serving every connection that arrives until it is
    /// dropped — the second half of what makes this a two-core test rather than a wire test: both
    /// cores talk to *one* map, which is what a shared tier is.
    ///
    /// `SET … EX` and `GET` only, because those are the two commands [`save`] and [`load`] send;
    /// anything else is a panic rather than a silent `+OK`, so a third command added upstream
    /// fails here instead of passing untested.
    fn serving(listener: TcpListener, held: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>) {
        while let Ok((stream, _)) = listener.accept() {
            let held = Arc::clone(&held);
            thread::spawn(move || {
                let mut writing = stream.try_clone().expect("the write half");
                let mut reading = BufReader::new(stream);
                while let Some(parts) = command(&mut reading) {
                    let reply = match parts[0].as_slice() {
                        b"SET" => {
                            assert_eq!(
                                parts[3], b"EX",
                                "a record is always written with an expiry"
                            );
                            held.lock()
                                .expect("the store")
                                .insert(parts[1].clone(), parts[2].clone());
                            b"+OK\r\n".to_vec()
                        }
                        b"GET" => match held.lock().expect("the store").get(&parts[1]) {
                            Some(record) => {
                                let mut bulk = format!("${}\r\n", record.len()).into_bytes();
                                bulk.extend_from_slice(record);
                                bulk.extend_from_slice(b"\r\n");
                                bulk
                            }
                            None => b"$-1\r\n".to_vec(),
                        },
                        other => panic!(
                            "the session store sends no {:?}",
                            String::from_utf8_lossy(other)
                        ),
                    };
                    writing.write_all(&reply).expect("the reply");
                }
            });
        }
    }

    /// ADR 0059 § 4, as the roster and as the write.
    ///
    /// Three claims, because each alone passes something broken. The roster admits no local tier —
    /// so no later reader can select one — and it is read from `nvs_config` rather than spelled
    /// here, so adding a variant there fails this. The word is refused by `Backend::of` rather than
    /// resolving to something. And a record actually written through [`save`] is **absent from the
    /// per-core map** afterwards: a module that reached `crate::cache::store_put` on some branch
    /// would satisfy the first two and fail this one, which is the failure the section is about.
    #[test]
    fn a_session_is_never_backed_by_the_local_cache_tier() {
        assert!(
            BACKENDS.iter().all(|(word, _)| *word != "local"),
            "ADR 0059 § 4 is enforced by the roster having no local entry: {BACKENDS:?}"
        );
        assert_eq!(Backend::of("local"), None);

        let (listener, address) = listening();
        let held = Arc::new(Mutex::new(HashMap::new()));
        let store = Arc::clone(&held);
        thread::spawn(move || serving(listener, store));

        let mut open = Connection::new(address, Duration::from_secs(5));
        save(&mut open, ID, RECORD, DEFAULT_TTL).expect("the record reaches the configured store");

        assert_eq!(
            held.lock().expect("the store").get(&key_of(ID)).cloned(),
            Some(RECORD.to_vec()),
            "the record is in the shared store"
        );
        assert_eq!(
            store_get(&key_of(ID)),
            None,
            "and nowhere in the per-core tier ADR 0059 § 4 refuses it"
        );
    }

    /// ADR 0139 § 3's whole reason, asserted as the contrast rather than as a round trip.
    ///
    /// Two threads are two cores: `crate::cache`'s local tier and its shared connection are both
    /// `thread_local`, so a second thread is exactly what a request landing on another core sees.
    /// The session record crosses — saved on one, loaded on the other through a connection the
    /// first never touched — while the same bytes put through the local tier on the first do not.
    /// A round trip on one thread would pass for a store that is merely reachable; the local half
    /// is what makes this a test of *where the record is*.
    #[test]
    fn a_session_survives_a_request_landing_on_another_core() {
        let (listener, address) = listening();
        let held: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
        thread::spawn(move || serving(listener, held));

        let wrote = thread::spawn(move || {
            let mut open = Connection::new(address, Duration::from_secs(5));
            save(&mut open, ID, RECORD, DEFAULT_TTL).expect("the record is written");
            store_put(&key_of(ID), RECORD.to_vec(), None);
            assert_eq!(store_get(&key_of(ID)), Some(RECORD.to_vec()));
        });
        wrote.join().expect("the first core");

        let read = thread::spawn(move || {
            let mut open = Connection::new(address, Duration::from_secs(5));
            (
                load(&mut open, ID).expect("the store answers"),
                store_get(&key_of(ID)),
            )
        });
        let (shared, local) = read.join().expect("the second core");

        assert_eq!(
            shared,
            Some(RECORD.to_vec()),
            "the session the first core wrote is the session the second core reads"
        );
        assert_eq!(
            local, None,
            "while the local tier's copy of the same bytes never left the core that wrote it — \
             which is what ADR 0059 § 4 refuses a session for"
        );
    }

    /// The key is prefixed and carries the id, so one store holding a cache, a limiter and a
    /// session store keeps the three apart — and an application-named limiter key cannot be
    /// spelled to read a session record.
    #[test]
    fn a_record_is_keyed_under_its_own_prefix() {
        let key = key_of(ID);
        assert!(key.starts_with(PREFIX.as_bytes()));
        assert_eq!(&key[PREFIX.len()..], ID.as_bytes());
        assert_ne!(PREFIX, crate::ratelimit::PREFIX);
    }

    /// An identifier is drawn, so two of them differ and neither is derived from anything the
    /// client supplied.
    ///
    /// The length is asserted because it is the only visible consequence of the draw's width: 16
    /// bytes is 22 base64url characters with no padding, so an id that shortened would say the
    /// entropy had, and a `Core\Uuid`-shaped 128 bits is what the module doc claims. The
    /// *distinctness* is one draw against another through the same context, which is
    /// `crate::random::draw`'s own contract — the state is written back, so two draws are two
    /// numbers.
    #[test]
    fn an_identifier_is_drawn_rather_than_derived() {
        let mut ctx = Ctx::buffered();
        let first = mint(&mut ctx);
        let second = mint(&mut ctx);
        assert_eq!(first.len(), 22, "128 bits, base64url, unpadded: {first}");
        assert_ne!(first, second);
        assert!(
            first
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
            "an id reaches the client in a cookie, so it carries nothing needing an escape: {first}"
        );
    }

    /// The three directives, each read off a tree an operator could have written.
    ///
    /// § 5's lifetime at both ends — the shipped one with nothing configured, and a written one
    /// preferred to it — and § 3's "absent is not a default backend": a tree with no `[session]`
    /// block throws naming the block, rather than quietly selecting a store nobody chose.
    #[test]
    fn the_block_decides_the_store_and_absence_decides_nothing() {
        let mut ctx = Ctx::buffered();
        assert_eq!(ttl(&ctx), DEFAULT_TTL);
        assert_eq!(cookie(&ctx), "nvsid");
        let refused = backend(&ctx, "start").expect_err("no block, no store");
        let nvs_runtime::Fault::Thrown(_, said) = refused else {
            panic!("an unconfigured store is a throw the program can catch");
        };
        assert!(
            said.contains("[session] backend"),
            "the throw names the block to write: {said}"
        );

        ctx.set_config(crate::tests::granting(
            "[session]\nbackend = \"shared\"\nttl = \"30m\"\ncookie = \"sid\"\n",
        ));
        assert_eq!(
            backend(&ctx, "start").expect("a store § 3 admits"),
            Backend::Shared
        );
        assert_eq!(ttl(&ctx), Duration::from_secs(30 * 60));
        assert_eq!(cookie(&ctx), "sid");
    }
}
