//! `rule:http-server/a-session-store-answers-four-operations`'s session store:
//! the identifier a store issues, the record it keeps under it, and the two directives that decide
//! where that store is and how long a record survives.
//!
//! **What is on disk is § 2's store and all seven of § 1's members.** The four operations a backend
//! answers are here — [`mint`], [`load`], [`save`], [`destroy`] and the key they share — over the
//! shared tier's wire, and [`nvs_core_session_start`] is the member that reaches them: it takes the
//! identifier the client presented, loads the record the store issued it for, and issues a fresh
//! one where there is none. The other six operate on the record `start` left on the request
//! ([`nvs_runtime::Session`]), and each of them throws until `start` has run — the whole benefit
//! `rule:core-classes/session-is-started-explicitly` was buying, and worth nothing if
//! the first `get` can silently start one.
//!
//! **§ 4's write-back is on disk too.** [`write_back`] marks the record changed on the request,
//! [`send_at_end`] sends it when the program that opened it ends, and
//! [`nvs_core_session_regenerate`] and [`nvs_core_session_destroy`] reach the store as they land
//! because § 4 makes those two immediate. The section below is where the send is decided.
//!
//! **`db` is a store § 3 admits and this build cannot serve.** Every operation below is written
//! against the shared tier's wire, so `start` under `backend = "db"` throws naming the half that
//! is unwritten. Deliberately at run time rather than at boot: the word names a store the ADR
//! admits, and refusing it where it is written would be this build claiming the *decision* was
//! wrong rather than that its second half has not landed.
//!
//! # Decision: a dirty record is sent by the *program's* end, and it travels there itself
//!
//! § 4 says the record is written back "when the request ends" and leaves open who does it. Three
//! crates have a claim and none of them can hold it alone: this one owns the store and cannot see
//! a request end; `nvs-host` ends every isolate and may not name `nvs-stdlib`; `nvs-server` ends
//! the HTTP request and names neither. So the send is a `fn` pointer that rides on the record —
//! [`nvs_runtime::Session::write_back`], filled in here by `start` and `regenerate`, called
//! through `Ctx::end_session` by whoever ends the program. That method's own docs own the
//! mechanism; what is decided here is the *where*, and it is three refusals:
//!
//! **Not the door.** The obvious reading of § 4 puts the send in `nvs-server`, at the line where a
//! request's response is collected — and the door does not have the record. A request is a root
//! isolate (`rule:security/isolate-shares-nothing`), so
//! `Core\Session::start` opened the session on the *isolate's* context, which is built and dropped
//! inside `nvs-host` (`rule:security/isolate-teardown-is-a-drain-then-a-sweep`
//! ) and is nothing the connection's own context can reach. Giving `nvs-server` a dependency on
//! this crate would not have fixed that; it would have bought the wrong context with a new edge.
//!
//! **Not a method on `nvs_runtime::host::Host`.** That trait is the seam a `Core` member reaches
//! its *scheduler* through, and its one implementor is `nvs-host`'s — which has no dependency on
//! this crate, so the method would have had no body that could reach a store. The direction is
//! also backwards: every other method on it is a member asking the host for something, and this is
//! the host telling a member the request is over.
//!
//! **Not a second thread-local beside that one.** A host is per core and installed once per
//! thread; a session is per request. A thread-local write-back would have to be installed by every
//! binary and every test fixture that could ever run a request, to say something the record itself
//! already knows.
//!
//! What this leaves as the rule: **a session is written back when the program that started it
//! ends**, which is every isolate — so every HTTP request — and `nvs run`'s root task, after
//! `rule:observability/script-on-exit`'s exit hooks, since
//! a hook is user code that may still write. A cancelled task is the one end that sends nothing,
//! because the send parks and a task being torn down may not park.
//!
//! # Decision: the local tier is unreachable from here, structurally
//!
//! `rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent` refuses
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
//! # Decision: an identifier this store cannot have issued is *absent*, and costs no round trip
//!
//! § 2 refuses a separate `validateId`, because a second question is a second thing that can
//! disagree with the first. [`issuable`] is not that second question. It answers **absent** — the
//! same answer [`load`] gives for an expired identifier and for one an attacker minted, and
//! `start` responds to all three identically by issuing a fresh one. Having no third answer to
//! give, it has nothing to disagree with.
//!
//! What it buys is that a presented identifier of a megabyte never becomes a key this core sends
//! to the shared store, and it buys it *before* the round trip rather than after. [`mint`]'s
//! output is 22 base64url characters, so anything else is a string this store has not issued, by
//! construction rather than by lookup.
//!
//! # What this spends
//!
//! One round trip to the configured store per request that starts a session, and a second only for
//! a request that changed the record (§ 4). A request that presents no identifier spends a draw
//! and one write. Memory is one encoded record per in-flight request that started one, released
//! with the request heap — O(in-flight), never O(sessions served).
//!
//! A `setSecret` spends one XChaCha20-Poly1305 seal and a `getSecret` one open per key of the ring
//! tried, over a record entry holding the value plus a nonce and a tag. Nothing is held between
//! calls: the ring is the program's and the ciphertext is the record's.
//!
//! **Every member that touches the record decodes it, and every member that changes it encodes it
//! back** — O(record) per call, over the bytes [`nvs_runtime::Session`] already holds, with the
//! decoded array living only for the length of the call. That is the accepted trade rather than an
//! oversight: a session record is a handful of keys beside the network round trip `start` has
//! already spent, and the alternative — holding the decoded array on the context — would put an
//! object at teardown that an
//! `rule:config/an-edit-reaches-the-next-request-without-a-restart` unit swap could strand,
//! which is the whole reason that struct holds bytes.

use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use nvs_config::session::Backend;
use nvs_runtime::{Ctx, Fault, NvsArray, NvsStr, ThrownClass, Value};
use nvs_syntax::duration;

use crate::cache::redis::Connection;
use crate::cache::{application, bound, configured, on_shared, open_configured, sealed_key};
use crate::registry::{Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// The class name, once, for the messages that all name it.
pub(crate) const NAME: &str = r"Core\Session";

/// § 1's roster, of which `start` is the member that talks to the store.
///
/// One class and no instance side: a session is the request's, not an object a program holds, so
/// there is nothing for a handle to be and nothing to hand back. Every member is static for that
/// reason, and this is § 1's list in its order, with the sealed pair
/// `rule:http-server/a-session-holds-a-secret-only-sealed` adds standing after it.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "start",
            names: &["presented"],
            // `?tainted string` as this registry spells it: `Nullable` for the `?`, and
            // `Text(Qual::Neutral)` for the rest — `CoreTy::TaintedStr` is return position only,
            // and a `Qual` is what says a `tainted` argument is admitted here (`rule:security/unclassified-parameter-refuses-tainted`). It
            // is admitted because § 2's strict-id rule makes the value a lookup key and never an
            // instruction: what the store did not issue is absent, whatever it was.
            params: &[CoreTy::Nullable(&CoreTy::Text(Qual::Neutral))],
            defaults: &[Const::Null],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_start",
            doc: Some(&START_DOC),
        },
        CoreMethod {
            name: "get",
            names: &["key"],
            // `Qual::Neutral` on the key, which is `Core\Cache\Store::get`'s judgement over the
            // same boundary: not a byte of it reaches the answer, and a record key derived from
            // the request is data rather than an instruction (`rule:security/unclassified-parameter-refuses-tainted`).
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            // `mixed` rather than `rule:core-api/shape-rules` R7's `?T`: what went in is any value the byte carrier
            // admits, so there is no `T` to make nullable, and `mixed` already spells absent.
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_session_get",
            doc: Some(&GET_DOC),
        },
        CoreMethod {
            name: "set",
            names: &["key", "value"],
            // The value is unclassified — `CoreTy::Mixed` — so a `tainted` one is refused, again
            // as `Core\Cache\Store::put` refuses it and for that member's reason: a qualifier is a
            // compile-time fact and the record is bytes, so `get` has nowhere to carry it back out
            // and admitting one here would launder it.
            params: &[CoreTy::Text(Qual::Neutral), CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_set",
            doc: Some(&SET_DOC),
        },
        CoreMethod {
            name: "remove",
            names: &["key"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_remove",
            doc: Some(&REMOVE_DOC),
        },
        CoreMethod {
            name: "clear",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_clear",
            doc: Some(&CLEAR_DOC),
        },
        CoreMethod {
            name: "regenerate",
            names: &[],
            // No argument, because PHP's `$delete_old_session` chose between a fixation window and
            // a lost session and only one of those is correct — `rule:core-api/session-roster`, and the migration
            // guide's *Sessions, requests and headers* is where the pair is named.
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_regenerate",
            doc: Some(&REGENERATE_DOC),
        },
        CoreMethod {
            name: "destroy",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_destroy",
            doc: Some(&DESTROY_DOC),
        },
        CoreMethod {
            name: "setSecret",
            names: &["key", "value", "keys"],
            // The value is a demand and not an admission, as `Core\Cache\Store::putSecret`'s is:
            // the qualifier widens on and narrows through nothing, so a plain `string` reaches
            // this parameter and a `secret` one does too, with nothing laundered either way. What
            // the row says is that this member is written for a confidential value — `set`'s
            // `mixed` refuses one, and this is where it goes instead.
            //
            // No `ttl` where the cache's door has one: a session value lives as long as its
            // session (`rule:http-server/session-expiry-belongs-to-the-store`).
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::SecretText(Qual::Neutral),
                CoreTy::Array(&crate::keyring::KEY),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_set_secret",
            doc: Some(&SET_SECRET_DOC),
        },
        CoreMethod {
            name: "getSecret",
            names: &["key", "keys"],
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Array(&crate::keyring::KEY),
            ],
            defaults: &[],
            // `rule:core-api/shape-rules` R7's `?T` where `get` had no `T` to make nullable: a
            // sealed value is the one type `setSecret` admitted. The `secret` is a promise rather
            // than a conditional — what comes back out is confidential whatever the `string` that
            // went in was typed as — which is why this spells `CoreTy::SecretStr` and not the
            // parameter form beside it.
            return_ty: CoreTy::Nullable(&CoreTy::SecretStr),
            symbol: "nvs_core_session_get_secret",
            doc: Some(&GET_SECRET_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Session::start`'s reference card — `rule:core-api/reference-card`.
const START_DOC: MethodDoc = MethodDoc {
    short: "Opens the session the store issued, taking the identifier from the session cookie \
            unless one is given — and issuing a fresh one where the store has no record under it.",
    params: &[ParamDoc {
        name: "presented",
        desc: "The identifier to open, for a client that carries it somewhere other than the \
               cookie. Omitted — the ordinary case — it is read from the `[session] cookie` \
               field of the request. An identifier this store did not issue, one that has \
               expired and one an attacker minted are the same answer: a fresh session, with a \
               new identifier in the response's cookie.",
        shape: &[],
    }],
    ret: "Nothing. Afterwards the other six members of this class operate on the record; before \
          it, each of them throws.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "This program is answering no request — a CLI program, a scheduled script, a \
                   job worker, a test, or a spawned isolate inside a request rather than a \
                   request of its own. A session belongs to the client the request came from, so \
                   there is none to open here and none to issue.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "No `[session] backend` is configured, so there is no store a record could \
                   live in; the configured store is `db`, whose half of § 2 is not on disk; or \
                   `[cache.shared] url` is unset, unreachable by capability, or this request has \
                   already started a session.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The configured store cannot be reached. It throws rather than answering as \
                   though the record were absent, since a store that is down must not read as a \
                   forged identifier — the two have opposite responses.",
        },
    ],
};

/// `Core\Session::get`'s reference card — `rule:core-api/reference-card`.
const GET_DOC: MethodDoc = MethodDoc {
    short: "Reads one key of the record this request's session holds, answering `null` where the \
            record does not hold it.",
    params: &[ParamDoc {
        name: "key",
        desc: "The key to read. One the record does not hold is `null` rather than a refusal, so a \
               session that stored a `null` and one that stored nothing read alike.",
        shape: &[],
    }],
    ret: "The value stored under `$key`, or `null`. Reading never marks the record changed, so a \
          request that starts a session and only reads it makes no second round trip.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, so there is no record to read.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "The stored record names a class this program cannot resolve — what a record \
                   written by a unit that declared the class and read by one that does not looks \
                   like.",
        },
    ],
};

/// `Core\Session::set`'s reference card — `rule:core-api/reference-card`.
const SET_DOC: MethodDoc = MethodDoc {
    short: "Writes one key of the record this request's session holds, replacing whatever was \
            under it.",
    params: &[
        ParamDoc {
            name: "key",
            desc: "The key to write.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "What to store under it — any value the cross-boundary copy admits, which is the \
                   same carrier a `Core\\Cache` entry crosses on. It is not `tainted`: a qualifier \
                   is a compile-time fact and a record is bytes, so nothing could carry one back \
                   out of `get()`.",
            shape: &[],
        },
    ],
    ret: "Nothing. The record is marked changed, which is what earns it a write back to the store \
          when the request ends.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, so there is no record to write.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$value` holds something the cross-boundary copy refuses — a closure, a \
                   resource, or an object holding one.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "As `get()`, because writing one key reads the whole record first.",
        },
    ],
};

/// `Core\Session::remove`'s reference card — `rule:core-api/reference-card`.
const REMOVE_DOC: MethodDoc = MethodDoc {
    short: "Takes one key out of the record this request's session holds.",
    params: &[ParamDoc {
        name: "key",
        desc: "The key to take out. One the record does not hold is not a refusal, and does not \
               mark the record changed either — there is nothing to write back.",
        shape: &[],
    }],
    ret: "Nothing. Removing a key the record held marks it changed; removing one it did not hold \
          leaves it exactly as it was.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, so there is no record to change.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "As `set()`: what is left of the record is encoded again, and the carrier \
                   refuses the same graphs on the way out as on the way in.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "As `get()`, because removing one key reads the whole record first.",
        },
    ],
};

/// `Core\Session::clear`'s reference card — `rule:core-api/reference-card`.
const CLEAR_DOC: MethodDoc = MethodDoc {
    short: "Empties the record this request's session holds, keeping the session and its \
            identifier.",
    params: &[],
    ret: "Nothing. The session stays open under the same identifier, so what this clears is the \
          record and not the client's claim to it — `destroy()` is the member that takes both.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "This request has not called `start()`, so there is no record to empty.",
    }],
};

/// `Core\Session::regenerate`'s reference card — `rule:core-api/reference-card`.
const REGENERATE_DOC: MethodDoc = MethodDoc {
    short: "Issues a new identifier, moves the record to it and forgets the old entry — what to \
            call the moment a request changes who the session speaks for.",
    params: &[],
    ret: "Nothing. The response carries the new identifier in its session cookie, and the record \
          survives the move unchanged. There is no argument for keeping the old entry: one of the \
          two answers is a fixation window and the other is a lost session.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, so there is no session to move; the \
                   shared store is unconfigured or refused by capability; or `[session] cookie` is \
                   not a cookie name.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The configured store cannot be reached. Unlike `start()`, this is not \
                   recoverable by issuing a fresh session: the old identifier is still live \
                   wherever the store is, which is the whole thing this member was called to end.",
        },
    ],
};

/// `Core\Session::destroy`'s reference card — `rule:core-api/reference-card`.
const DESTROY_DOC: MethodDoc = MethodDoc {
    short: "Forgets the record in the store and closes the session on this request, which is what \
            signing out is.",
    params: &[],
    ret: "Nothing. Afterwards this request has no session at all, so every member of this class \
          throws again until `start()` opens one — the same answer they give before the first \
          `start()`, because it is the same state.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, so there is no session to forget; or \
                   the shared store is unconfigured or refused by capability.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The configured store cannot be reached, so the record is still there. It \
                   throws rather than closing the session quietly, because a program told the \
                   sign-out succeeded would stop trying.",
        },
    ],
};

/// `Core\Session::setSecret`'s reference card — `rule:core-api/reference-card`.
const SET_SECRET_DOC: MethodDoc = MethodDoc {
    short: "Writes one key of this request's session record sealed under a key ring, which is the \
            only way a user's own secret is held in a session.",
    params: &[
        ParamDoc {
            name: "key",
            desc: "The key to write. It names a value no other member of this class can read: \
                   `get()` answers `null` there, and `getSecret()` under the same ring is the one \
                   door back.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The secret to seal — an access or refresh token a request holds on the user's \
                   behalf. What reaches the store is ciphertext, and the record crosses it as the \
                   byte carrier it already was.",
            shape: &[],
        },
        ParamDoc {
            name: "keys",
            desc: "The key ring, newest first. The newest key seals; every key of it is tried \
                   when the value is read back, so a rotation leaves what it wrote readable.",
            shape: &[],
        },
    ],
    ret: "Nothing. The record is marked changed, as `set()` marks it, which is what earns it a \
          write back to the store when the request ends.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, so there is no record to write.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$keys` is empty or holds something that is not a key — a ring that cannot \
                   seal anything is the program's own bug rather than a value to write.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "As `get()`, because writing one key reads the whole record first.",
        },
    ],
};

/// `Core\Session::getSecret`'s reference card — `rule:core-api/reference-card`.
const GET_SECRET_DOC: MethodDoc = MethodDoc {
    short: "Reads back a value `setSecret` sealed into this request's session record, answering \
            `null` where the ring does not open one.",
    params: &[
        ParamDoc {
            name: "key",
            desc: "The key `setSecret` wrote. The name is sealed in as well as looked up, so a \
                   value is not readable under a second one.",
            shape: &[],
        },
        ParamDoc {
            name: "keys",
            desc: "The key ring, newest first. Every key of it is tried, so a value sealed before \
                   a rotation stays readable until it is written again.",
            shape: &[],
        },
    ],
    ret: "The secret sealed under `$key`, or `null`. Every way of not opening one is that same \
          `null` — a ring that has rotated past it, a value moved to another name or another \
          application, a tampered payload — so a caller learns nothing about the ring from a \
          value it cannot read.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, so there is no record to read.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$keys` is empty or holds something that is not a key, which is the one thing \
                   here that is not a miss.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "As `get()`: the stored record names a class this program cannot resolve.",
        },
    ],
};

/// What every session key starts with, so an operator sharing one store between a cache, a limiter
/// and a session store can tell the three apart in it.
///
/// [`crate::ratelimit`]'s `PREFIX` is the same decision, and the two must not collide: a session id
/// is drawn and a limiter's key is application-supplied, so without the prefixes an application
/// could name a limiter key that reads a session record.
pub(crate) const PREFIX: &str = "nvs:session:";

/// `[session] backend` — `rule:http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot`'s store, as `nvs_config::session::Backend` spells it.
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
/// write. `rule:http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot`: an absent block is not a default backend, because the safe answer for a
/// store nobody chose is no store — the same direction
/// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` takes for everything it
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
                 find it (`rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent`)"
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

/// [`cookie`]'s answer for a reader holding the tree rather than a context.
///
/// The server door is that reader: `rule:security/csrf-is-on-by-default` has it
/// verify a token against the session the request rides under, and it decides
/// that before an isolate — and so before a `Ctx` — exists. It is here rather
/// than in `nvs-server` because the default is this module's, and a door that
/// spelled `nvsid` itself would be a second answer to one question.
#[must_use]
pub fn cookie_in(config: &nvs_config::Config) -> &str {
    config
        .session
        .as_ref()
        .and_then(|session| session.cookie.as_deref())
        .unwrap_or(DEFAULT_COOKIE)
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
/// `rule:php-migration/a-session-id-the-store-did-not-issue-is-rejected`
/// 's cases at once: an id no store issued, one that has expired, and one an attacker minted.
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

/// § 2's `destroy` — the record under `id`, forgotten.
///
/// One key, as [`save`] writes one: a session is one entry, so there is nothing here that a store
/// with no multi-key atomic step could not do.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn destroy(open: &mut Connection, id: &str) -> Result<(), String> {
    open.del(&key_of(id))
}

/// § 2's `save` — the record under `id`, replaced, with its expiry refreshed.
///
/// The whole record and not one key of it, per § 4: one session is one entry, which is what keeps
/// `clear`, `regenerate` and `destroy` single-key operations over a store
/// `rule:core-classes/ratelimit-two-members` gives no multi-key atomic step.
///
/// The expiry is the store's own, per § 5: a record written with no expiry at all is that
/// section's sweeper coming back, which is why this reaches the command that carries one rather
/// than the bare `SET` beside it.
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
    open.set_expiring(&key_of(id), record, ttl)
}

/// How long an identifier [`mint`] draws renders to, derived from the draw rather than written.
///
/// Base64 is four characters per three octets, unpadded — so widening [`ID_BYTES`] widens this,
/// and [`issuable`] keeps agreeing with what this store issues instead of pinning yesterday's
/// width.
const ID_CHARS: usize = ID_BYTES.div_ceil(3) * 4 - (3 - ID_BYTES % 3) % 3;

/// Whether `presented` is a string this store could have issued — the module doc's decision.
///
/// Not a second question about validity: an identifier that fails here is **absent**, exactly as
/// one [`load`] finds no record for is, and `start` takes the same branch for both. What it saves
/// is the round trip, and what it prevents is an arbitrary presented string becoming a key this
/// core sends to a store it shares with a cache and a limiter.
pub(crate) fn issuable(presented: &str) -> bool {
    presented.len() == ID_CHARS
        && presented
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::start(?tainted string $presented = null): void` — `rule:core-api/session-roster`'s member that
    /// talks to the store, replacing `session_start`.
    ///
    /// **Absent is one answer with one response.** § 2 gives `load` three ways to answer absent —
    /// an identifier the store never issued, one it issued and has since expired, and one an
    /// attacker minted — and this member does not distinguish them: each discards the presented
    /// identifier and issues a fresh one. There is no `validateId` for the two to disagree about,
    /// and the pre-store shape check ([`issuable`]) is not one, for the reason the module doc
    /// gives.
    ///
    /// **A store that cannot be reached throws.** [`on_shared`] classifies it as an `IOError`
    /// rather than answering as though the record were absent, because absence means *issue a new
    /// session* — so a store that is down would silently sign every user out and look like a
    /// forged identifier while doing it.
    ///
    /// **The cookie is written only for an identifier this request issued.** A request that
    /// presented one the store knew already has it, and a session cookie carries no `Max-Age` to
    /// refresh — the record's lifetime is `[session] ttl`, on the store, which is the only side
    /// that can expire it. Every other attribute is `[http.cookies]`'s, through the same
    /// [`crate::response::Cookie`] `Core\Response::addCookie` renders, so the policy has one home
    /// and the line has one spelling.
    ///
    /// **A program answering no request has no session to open**, and is told that before it is
    /// told anything about the store. `rule:security/request-state-throws-in-an-isolate`: a
    /// session belongs to the client a request arrived from, so a CLI program, a job worker, a
    /// test and a spawned isolate inside a request have nothing to open and nothing to issue a
    /// cookie on. The situation is asked about before the configuration because the answer does
    /// not depend on it: a deployment that *has* a store would otherwise mint a record and a
    /// cookie for a caller that can never present either, which is the ambient authority
    /// `rule:security/isolate-shares-nothing` keeps out of a child. The refusal is
    /// [`crate::request::served`]'s, so the class and the sentence have one home.
    ///
    /// # Errors
    ///
    /// As [`START_DOC`] lists them: a thrown `LogicError` where no request arrived; a thrown
    /// `RuntimeError` for a second `start` on one request, for a tree that configured no store,
    /// for the `db` store this build does not serve, and for a shared store that is unconfigured
    /// or refused by capability; a thrown `IOError` for one that cannot be reached.
    fn nvs_core_session_start(ctx, args: [1]) {
        crate::request::served(ctx, &format!("{NAME}::start"))?;

        if ctx.session().is_some() {
            return Err(Fault::thrown(format!(
                "{NAME}::start(): this request has already started a session — a second `start()` \
                 would discard whatever the first one's record has collected since, so the member \
                 that deliberately replaces a session is `regenerate()`"
            )));
        }

        match backend(ctx, "start")? {
            Backend::Shared => {}
            Backend::Db => {
                return Err(Fault::thrown(format!(
                    "{NAME}::start(): `[session] backend = \"db\"` names a store `rule:http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot` \
                     admits and this build does not serve yet — write `backend = \"shared\"`, or \
                     see `crates/nvs-stdlib/src/session.rs`'s module doc for which half is on disk"
                )));
            }
        }

        let member = format!("{NAME}::start()");
        open_configured(
            ctx,
            &member,
            ", which is where a record has to live for a request on another core to find it \
             (`rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent`)",
        )?;

        // A presented identifier this store cannot have issued is absent, and is absent here
        // rather than one round trip later.
        let named = cookie(ctx);
        let presented = match args[0].as_text() {
            Some(given) => Some(given.to_owned()),
            // The ordinary case: the identifier rides in the `[session] cookie` field, read
            // through the same `crate::request::cookie_of` `Core\Request::cookie` reads it with —
            // one reading, so a `__Host-` name that arrived twice is invisible to both rather
            // than to one of them.
            None => ctx.inbound().and_then(|inbound| {
                crate::request::cookie_of(inbound, named.as_bytes())
                    .and_then(|value| std::str::from_utf8(value).ok())
                    .map(str::to_owned)
            }),
        }
        .filter(|id| issuable(id));
        let opened = match presented {
            Some(id) => on_shared(NAME, "start", |open| load(open, &id))?
                .map(|record| nvs_runtime::Session {
                    id,
                    record,
                    dirty: false,
                    write_back: send_at_end,
                }),
            None => None,
        };

        let session = match opened {
            Some(session) => session,
            None => {
                let id = mint(ctx);
                let lifetime = ttl(ctx);
                // The empty record is zero bytes — `nvs_runtime::Session::record`'s own doc owns
                // why — so issuing one builds no value and encodes nothing.
                on_shared(NAME, "start", |open| save(open, &id, &[], lifetime))?;
                issue_cookie(ctx, "start", &id)?;
                nvs_runtime::Session {
                    id,
                    record: Vec::new(),
                    dirty: false,
                    write_back: send_at_end,
                }
            }
        };
        ctx.open_session(session);
        Ok(Value::null())
    }
}

/// The `Set-Cookie` line carrying `id`, under `[session] cookie` and `[http.cookies]`' policy.
///
/// # Errors
///
/// A thrown `RuntimeError` for a `[session] cookie` that is not a cookie name. That is an
/// operator's typo rather than anything a program did, and it is refused here because the value
/// reaches a header line: a name carrying a `;` or a newline would end the line and begin one
/// nobody wrote. The identifier needs no such check — it is 22 characters this core drew.
fn issue_cookie(ctx: &mut Ctx, member: &str, id: &str) -> Result<(), Fault> {
    let name = cookie(ctx);
    if !crate::response::nameable(&name) {
        return Err(Fault::thrown(format!(
            "{NAME}::{member}(): `[session] cookie = \"{name}\"` is not a cookie name — a name is \
             a non-empty token, and this one reaches a `Set-Cookie` line"
        )));
    }

    let policy = crate::response::configured_cookies(ctx);
    let same_site = policy.same_site;
    let line = crate::response::Cookie {
        name: &name,
        value: id,
        path: &policy.path,
        // A session cookie: no `Domain`, which is the narrower of that attribute's two meanings,
        // and no `Max-Age`, because the record's lifetime is `[session] ttl` on the store and a
        // cookie that outlived it would present an identifier the store answers absent for.
        domain: None,
        max_age: None,
        secure: policy.secure,
        http_only: policy.http_only,
        same_site,
    }
    .line();
    ctx.append_header(crate::response::SET_COOKIE_HEADER, &line);
    Ok(())
}

/// The throw every member but `start` makes while this request has started no session.
///
/// **One spelling for all of them**, because `rule:core-api/session-roster`'s rule is one rule: a member called
/// before `start` throws naming it, and a program that meets it from `get` should read the same
/// sentence it would have read from `remove`.
/// `tests/conformance/core/session-every-member-refuses-a-record-nobody-opened.nvst` asserts that
/// agreement by counting the distinct answers rather than by reading any one of them.
fn unstarted(member: &str) -> Fault {
    Fault::thrown(format!(
        "{NAME}::{member}(): this request has not started a session — call `{NAME}::start()` \
         first, which is the line in the source that says this request uses sessions (`rule:statements/no-host-populated-variables` \
         § 4)"
    ))
}

/// The `$key` argument, as the checker has already guaranteed it.
fn key_at<'a>(key: &'a Value, member: &str) -> Result<&'a str, Fault> {
    // Unreachable from source: a `string` parameter, refused at the checker with `E0401` before
    // any of this runs. The guard is what makes the answer below total.
    key.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a string key, got tag {}",
            key.tag_byte()
        ))
    })
}

/// The `$value` argument of [`nvs_core_session_set_secret`], as the checker has already
/// guaranteed it.
///
/// A `secret string` is a `string` by the time anything runs: the qualifier is checked once and
/// erased before codegen, so what arrives here is text or compiled code's bug — which is
/// [`crate::cache`]'s reading of its own `putSecret` value, over the same erasure.
fn secret_at<'a>(value: &'a Value, member: &str) -> Result<&'a str, Fault> {
    // Unreachable from source: a `secret string` parameter, refused at the checker before any of
    // this runs. The guard is what makes the answer below total.
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a string value, got tag {}",
            value.tag_byte()
        ))
    })
}

/// The octet this class's sealed record entries open their additional data with.
///
/// [`crate::cache`]'s `SEAL_DOMAIN` is the other one taken, and the two differing is the whole of
/// why a value sealed for a cache does not open as a session secret: the construction, the ring
/// and the application can all be the same, and the first octet under the tag is not.
const SEAL_DOMAIN: u8 = 2;

/// Seals `value` under the ring's newest key and writes it into `held` under the sealed name.
///
/// **The additional data is the domain octet, the application and the key, and never the session
/// identifier** — `rule:http-server/a-session-holds-a-secret-only-sealed`: `regenerate` issues a
/// new id over the same record, and a value bound to the old one would stop opening at exactly the
/// moment a login hardens. The plaintext is the secret and nothing else, because a session value
/// has no lifetime of its own to seal in beside it: it lives as long as its session, which is the
/// store's to expire (`rule:http-server/session-expiry-belongs-to-the-store`).
///
/// # Errors
///
/// [`crate::keyring`]'s `LogicError` for a ring whose newest entry is not a key, and a
/// `RuntimeError` for a sealed value this process cannot spare the buffer for.
fn sealed_into(
    ctx: &mut Ctx,
    held: &mut NvsArray,
    key: &str,
    value: &[u8],
    ring: &NvsArray,
    who: &str,
) -> Result<(), Fault> {
    let (slot, newest) = crate::keyring::newest(ring);
    let cipher = crate::keyring::cipher_at(&newest, slot, who)?;
    let aad = bound(SEAL_DOMAIN, application(ctx), key.as_bytes());
    let sealed = crate::crypto::seal_under(ctx, &cipher, &aad, value, who)?;
    held.set(
        NvsStr::new(&sealed_key(key.as_bytes())),
        Value::bytes(NvsStr::new(&sealed)),
    );
    Ok(())
}

/// The secret `held` carries under `key`, or `None` for every way of not opening one.
///
/// **Every way of not opening is the same miss**, which is [`crate::cache`]'s answer at its own
/// sealed door and buys the same thing: a caller that could tell a rotated ring from a tampered
/// payload would learn something about the ring from a value it cannot read. Every key of the ring
/// is tried rather than the newest alone, so a value sealed before a rotation stays readable.
///
/// A record entry in the sealed space that is not bytes is that same miss. A program can reach the
/// space — a session key is arbitrary text, and `set` will write whatever it is handed under one —
/// and what it finds there is a value no ring opens.
///
/// # Errors
///
/// [`crate::keyring`]'s `LogicError` for an entry of the ring that is not a key, and a
/// `RuntimeError` for a plaintext this process cannot spare the buffer for.
fn opened_from(
    ctx: &Ctx,
    held: &NvsArray,
    key: &str,
    ring: &NvsArray,
    who: &str,
) -> Result<Option<Value>, Fault> {
    let Some(found) = held.get(&sealed_key(key.as_bytes())) else {
        return Ok(None);
    };
    let Some(sealed) = found.as_bytes() else {
        return Ok(None);
    };

    let aad = bound(SEAL_DOMAIN, application(ctx), key.as_bytes());
    for (slot, entry) in crate::keyring::entries(ring) {
        let cipher = crate::keyring::cipher_at(&entry, slot, who)?;
        if let Some(plain) = crate::crypto::open_under(&cipher, &aad, sealed, who)? {
            return Ok(Some(Value::str(NvsStr::new(&plain))));
        }
    }
    Ok(None)
}

/// The record this request has open, decoded — or an empty array where it holds none.
///
/// Every member below reaches the record through this and none of them reads
/// [`nvs_runtime::Session`]'s bytes directly, because the decode is where a record naming a class
/// this unit cannot resolve is refused and a second reader would be a second place that could
/// forget. An empty record is zero bytes rather than the encoding of an empty array — that
/// struct's own doc owns why — so the empty case builds an array here and decodes nothing.
///
/// # Errors
///
/// [`unstarted`] while this request has started no session. A thrown `ParseError` for a record
/// [`nvs_runtime::decode`] refuses, which is [`crate::cache`]'s `get` refusal over the same
/// resolver — the program's own class table — and for the same reason.
fn record(ctx: &Ctx, member: &str) -> Result<NvsArray, Fault> {
    let Some(session) = ctx.session() else {
        return Err(unstarted(member));
    };
    if session.record.is_empty() {
        return Ok(NvsArray::new());
    }

    let resolve = |name: &str| ctx.class_desc(name);
    let decoded = nvs_runtime::decode(&session.record, &resolve).map_err(|why| {
        Fault::thrown_as(ThrownClass::Parse, format!("{NAME}::{member}(): {why}"))
    })?;
    match decoded.array_ptr() {
        // The reference `decode` handed back becomes this handle's, and the handle releases it.
        Some(array) =>
        {
            #[expect(
                unsafe_code,
                reason = "`decode` answers with one reference it no longer holds, \
                          and this handle takes over exactly that reference"
            )]
            Ok(unsafe { NvsArray::from_raw(array) })
        }
        // Unreachable from source: nothing but `write_back` writes a session record, and it
        // encodes an array. Released rather than leaked, because the answer is still a reference.
        None => {
            #[expect(
                unsafe_code,
                reason = "`decode` answers with one reference nothing else holds"
            )]
            unsafe {
                decoded.release();
            }
            Err(Fault::fatal(format!(
                "{NAME}::{member}(): the stored record is not an array"
            )))
        }
    }
}

/// § 4's "mutate the copy in the request's own heap": `record` encoded back onto the request, and
/// marked as owing the store a write.
///
/// **The record is written whole**, per § 4 — one session is one entry, so there is no key-wise
/// update to send and no lock to hold while sending it. **Empty is zero bytes**, never the encoding
/// of an empty array, which is the invariant [`nvs_runtime::Session`] states and `start` already
/// relies on for a session it minted.
///
/// # Errors
///
/// A thrown `LogicError` for a record holding something the byte carrier refuses, which is
/// [`crate::cache`]'s `put` refusal over the same walk: the two carriers refuse the same graphs
/// because they share the walk that decides. [`unstarted`] cannot fire here — every caller reached
/// [`record`] first — and is answered rather than asserted for that member's reason.
fn write_back(ctx: &mut Ctx, record: NvsArray, member: &str) -> Result<Value, Fault> {
    let payload = if record.is_empty() {
        drop(record);
        Vec::new()
    } else {
        nvs_runtime::encode(Value::array(record)).map_err(|why| {
            Fault::thrown_as(ThrownClass::Logic, format!("{NAME}::{member}(): {why}"))
        })?
    };

    let session = ctx.session_mut().ok_or_else(|| unstarted(member))?;
    session.record = payload;
    session.dirty = true;
    Ok(Value::null())
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::get(string $key): mixed` — `rule:core-api/session-roster`'s read of the record `start` loaded,
    /// replacing `$_SESSION[$key]`.
    ///
    /// **A key the record does not hold is `null`**, exactly as `Core\Cache\Store::get` answers a
    /// miss: `mixed` already spells absent, so there is no second member to ask whether a key is
    /// there and no `?T` to make nullable. A session that stored a `null` and one that stored
    /// nothing are the same record, which is PHP's answer as well.
    ///
    /// **This member does not mark the record changed**, which is § 4's "writing only when the
    /// record changed" written as the thing it buys: a request that starts a session and only
    /// reads it makes one round trip and not two.
    ///
    /// # Errors
    ///
    /// As [`record`]: [`unstarted`] before `start`, and a thrown `ParseError` for a record naming
    /// a class this program cannot resolve.
    fn nvs_core_session_get(ctx, args: [1]) {
        let key = key_at(&args[0], "get")?;
        let held = record(ctx, "get")?;
        // `NvsArray::get` borrows rather than retains, and the handle releases the whole record
        // when it is dropped below — so the answer needs a reference of its own first.
        let answer = match held.get(key.as_bytes()) {
            Some(found) => {
                #[expect(
                    unsafe_code,
                    reason = "the record holds a live reference to this value until \
                              the handle is dropped, which is after the retain"
                )]
                unsafe {
                    found.retain();
                }
                found
            }
            None => Value::null(),
        };
        drop(held);
        Ok(answer)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::set(string $key, mixed $value): void` — `rule:core-api/session-roster` and `rule:http-server/a-session-is-loaded-once-and-written-whole`'s write into the
    /// request's own copy of the record, replacing `$_SESSION[$key] = …`.
    ///
    /// **Nothing reaches the store here.** § 4 loads the record once and writes it whole when the
    /// request ends, so this member encodes the changed record onto the request and sets the flag
    /// that write-back reads. Two requests writing one session concurrently is last-write-wins over
    /// the whole record, which that section states rather than repairs.
    ///
    /// # Errors
    ///
    /// As [`record`] and [`write_back`]: [`unstarted`] before `start`, a thrown `ParseError` for a
    /// record this unit cannot decode, and a thrown `LogicError` for a `$value` the cross-boundary
    /// copy refuses.
    fn nvs_core_session_set(ctx, args: [2]) {
        let key = key_at(&args[0], "set")?;
        let mut held = record(ctx, "set")?;
        // The record takes over one reference and the argument slot keeps its own, which is the
        // same split `Core\Cache\Store::put` makes before handing a value to the carrier.
        #[expect(
            unsafe_code,
            reason = "the argument slot holds a live reference for the length of \
                      the call, which is `Value::retain`'s whole obligation"
        )]
        unsafe {
            args[1].retain();
        }
        held.set(NvsStr::new(key.as_bytes()), args[1]);
        write_back(ctx, held, "set")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::setSecret(string $key, secret string $value, array<secret bytes> $keys):
    /// void` — `rule:http-server/a-session-holds-a-secret-only-sealed`'s door for a user's own
    /// secret: the access or refresh token a web application holds on their behalf.
    ///
    /// **It is the session and not a cache tier** because a cache may evict at any time, and a
    /// user logged out by a footprint decision is what storing one there buys. It is sealed
    /// because the record crosses the store as bytes — so `rule:security/secret-crosses-no-boundary`
    /// is untouched here, exactly as it is at `Core\Cache\Store::putSecret`: no `secret` value
    /// crosses the copy at all, and what the store holds is ciphertext.
    ///
    /// **What the seal buys, and what it does not.** Code that knows a key's name but not the ring
    /// cannot read the value, and a record moved, tampered with or lifted into another application
    /// opens under none of it. It does not protect a secret from a compromised process: the ring
    /// is in the same memory as the plaintext.
    ///
    /// The refusal before `start` comes first, so this member answers the same way its siblings do
    /// for a request that opened no session rather than reporting the ring it never got to use.
    ///
    /// # Errors
    ///
    /// As [`record`] and [`write_back`], plus [`crate::keyring`]'s `LogicError` for a ring that is
    /// empty or holds something that is not a key.
    fn nvs_core_session_set_secret(ctx, args: [3]) {
        let mut held = record(ctx, "setSecret")?;
        let key = key_at(&args[0], "setSecret")?;
        let value = secret_at(&args[1], "setSecret")?;

        let who = format!("{NAME}::setSecret");
        let ring = crate::keyring::borrow(args, 2, &who)?;
        sealed_into(ctx, &mut held, key, value.as_bytes(), &ring, &who)?;
        write_back(ctx, held, "setSecret")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::getSecret(string $key, array<secret bytes> $keys): ?secret string` —
    /// [`nvs_core_session_set_secret`]'s door in the other direction.
    ///
    /// **A value that does not open is absent rather than an error**, and that is the whole of
    /// what a program sees of a rotated ring, a tampered record or a name nothing sealed under.
    /// The one thing that is not a miss is a ring that cannot key anything, which is the program's
    /// own bug and throws.
    ///
    /// **A secret survives `regenerate`**, because what the value is bound to is the application
    /// and the key rather than the identifier — the rule's own paragraph on why, and the reason a
    /// login that hardens its session does not log the user out of the token it just stored.
    ///
    /// Reading marks nothing changed, as [`nvs_core_session_get`] does not, so a request that only
    /// reads its secrets still makes one round trip.
    ///
    /// # Errors
    ///
    /// As [`record`], plus [`crate::keyring`]'s `LogicError` for a ring that is empty or holds
    /// something that is not a key.
    fn nvs_core_session_get_secret(ctx, args: [2]) {
        let held = record(ctx, "getSecret")?;
        let key = key_at(&args[0], "getSecret")?;

        let who = format!("{NAME}::getSecret");
        let ring = crate::keyring::borrow(args, 1, &who)?;
        let answer = opened_from(ctx, &held, key, &ring, &who)?;
        drop(held);
        Ok(answer.unwrap_or_else(Value::null))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::remove(string $key): void` — `rule:core-api/session-roster` and `rule:http-server/a-session-is-loaded-once-and-written-whole`, replacing `unset($_SESSION
    /// [$key])`.
    ///
    /// **A key the record does not hold is not a refusal, and does not mark it changed either.**
    /// Nothing was written, so there is nothing for § 4's write-back to send, and a `remove` that
    /// dirtied the record regardless would put a request that changed nothing back on the write
    /// path — which is the one thing "writing only when the record changed" is there to prevent.
    ///
    /// # Errors
    ///
    /// As [`nvs_core_session_set`].
    fn nvs_core_session_remove(ctx, args: [1]) {
        let key = key_at(&args[0], "remove")?;
        let mut held = record(ctx, "remove")?;
        if !held.has_key(key.as_bytes()) {
            drop(held);
            return Ok(Value::null());
        }
        held.unset(key.as_bytes());
        write_back(ctx, held, "remove")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::clear(): void` — `rule:core-api/session-roster` and `rule:http-server/a-session-is-loaded-once-and-written-whole`, replacing `$_SESSION = []`.
    ///
    /// **The session survives; only the record goes.** The identifier stays live in the store and
    /// in the client's cookie, which is what separates this from `destroy()`: a program clearing a
    /// basket is not signing anybody out, and one signing a user out must not leave the identifier
    /// that request arrived with usable.
    ///
    /// **The one member that does not decode the record**, because it does not read it: a record
    /// naming a class this unit can no longer resolve is still clearable, which is the answer a
    /// program recovering from exactly that needs.
    ///
    /// # Errors
    ///
    /// [`unstarted`] before `start`, and nothing else — there is no encode to refuse.
    fn nvs_core_session_clear(ctx, _args: [0]) {
        let session = ctx.session_mut().ok_or_else(|| unstarted("clear"))?;
        // An empty record cleared is not a change, so it does not earn the write § 4's flag is
        // there to withhold — [`nvs_core_session_remove`] makes the same judgement for a key the
        // record does not hold.
        if session.record.is_empty() {
            return Ok(Value::null());
        }
        session.record = Vec::new();
        session.dirty = true;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::regenerate(): void` — `rule:core-api/session-roster`'s move to a new identifier, replacing
    /// `session_regenerate_id`.
    ///
    /// **In § 1's order: issue, move the record, forget the old entry.** A failure between the
    /// second step and the third leaves the record readable under two identifiers until
    /// `[session] ttl` expires the old one; the same failure in the other order leaves it readable
    /// under none, which is a signed-out user. The first is the recoverable one, so it is the one
    /// this member risks.
    ///
    /// **Immediately rather than at the end of the request** — § 4 names this member and `destroy`
    /// as the two exceptions to its own write-back, because both of them are about an identifier
    /// rather than about a record, and an identifier that is only retired when the request ends is
    /// live for the length of the response.
    ///
    /// # Errors
    ///
    /// [`unstarted`] before `start`; a thrown `RuntimeError` for a shared store that is
    /// unconfigured or refused by capability, and for a `[session] cookie` that is not a cookie
    /// name; a thrown `IOError` for a store that cannot be reached.
    fn nvs_core_session_regenerate(ctx, _args: [0]) {
        let Some(open) = ctx.session() else {
            return Err(unstarted("regenerate"));
        };
        // Copied rather than taken, so a store that fails halfway leaves the request holding the
        // session it already had. O(record), which this member spends once and no other does.
        let (retired, record) = (open.id.clone(), open.record.clone());

        let member = format!("{NAME}::regenerate()");
        open_configured(ctx, &member, ", which is where this session's record already lives")?;
        let fresh = mint(ctx);
        let lifetime = ttl(ctx);
        on_shared(NAME, "regenerate", |open| save(open, &fresh, &record, lifetime))?;
        on_shared(NAME, "regenerate", |open| destroy(open, &retired))?;
        issue_cookie(ctx, "regenerate", &fresh)?;

        // Not dirty: the record was written whole a line ago, so § 4's write-back has nothing left
        // to send. `Ctx::open_session` replaces rather than refuses for exactly this call.
        ctx.open_session(nvs_runtime::Session {
            id: fresh,
            record,
            dirty: false,
            write_back: send_at_end,
        });
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::destroy(): void` — `rule:core-api/session-roster` and `rule:http-server/a-session-is-loaded-once-and-written-whole`, replacing `session_destroy`.
    ///
    /// **The request is left with no session**, so every member of this class throws again
    /// afterwards — the same answer as before the first `start()`, because it is the same state.
    /// PHP leaves `$_SESSION` populated after `session_destroy()` and that is the divergence: a
    /// record that outlives the entry it came from is a copy of something that no longer exists,
    /// and § 4's write-back would put it straight back.
    ///
    /// **The store is told before the request forgets**, so a store that cannot be reached throws
    /// with the session still open rather than reporting a sign-out that did not happen.
    ///
    /// # Errors
    ///
    /// As [`nvs_core_session_regenerate`], less the cookie: no `Set-Cookie` is written, because the
    /// identifier the client holds now names nothing and a store that answers absent is already the
    /// whole of § 2's strict-id rule.
    fn nvs_core_session_destroy(ctx, _args: [0]) {
        let Some(open) = ctx.session() else {
            return Err(unstarted("destroy"));
        };
        let held = open.id.clone();

        let member = format!("{NAME}::destroy()");
        open_configured(ctx, &member, ", which is where this session's record lives")?;
        on_shared(NAME, "destroy", |open| destroy(open, &held))?;
        ctx.close_session();
        Ok(Value::null())
    }
}

/// How the write-back names itself in a failure: not as a member, because no member called it.
///
/// The request did, by ending. A message reading `Core\Session::save()` would name a member of a
/// class that has none, and the operator reading it is looking for the request rather than for a
/// line of the program.
const SENDER: &str = "Core\\Session's write-back at the end of the request";

/// `rule:http-server/a-session-is-loaded-once-and-written-whole`'s write-back, as the function every session this module opens carries.
///
/// Installed on [`nvs_runtime::Session::write_back`] by `start` and `regenerate`, and reached
/// through [`Ctx::end_session`] at the end of the program that opened the record — the isolate an
/// HTTP request is, or `nvs run`'s root task. Only a record something changed gets here: the flag
/// is § 4's whole "writing only when the record changed", and that context method is where it is
/// read.
///
/// **It reports rather than throws.** There is nothing left to throw *to* — the program has ended,
/// so no `catch` can be reached and the ladder's tiers 1 to 3 are all behind us — and a `Fault`
/// returned from here would have nowhere to go but the floor by a longer road. So it calls the
/// floor directly, at [`nvs_render::Level::Error`], and the record says the writes are lost rather
/// than that a write failed: that is what the request will look like from the next one.
///
/// **Losing them is the answer, rather than holding the response until the store comes back.** A
/// request that cannot reach its store at the end has already produced its output; waiting there
/// is the wedge `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`
/// is named after, and § 4's last-write-wins already declines to repair a lost write.
fn send_at_end(ctx: &mut Ctx) {
    // Nothing is copied out of the record: `open_configured` and `on_shared` both borrow, and the
    // one call that needs the context by value — the report below — happens after this borrow has
    // ended. A write-back that cloned the record would spend O(record) on the request path to say
    // exactly what the bytes already on the context say.
    let sent = match ctx.session() {
        None => return,
        Some(open) => {
            let lifetime = ttl(ctx);
            open_configured(ctx, SENDER, ", which is where this session's record lives").and_then(
                |()| {
                    on_shared(NAME, "the write-back", |store| {
                        save(store, &open.id, &open.record, lifetime)
                    })
                },
            )
        }
    };
    if let Err(why) = sent {
        let record = nvs_runtime::floor::note(
            nvs_render::Level::Error,
            &format!(
                "this request's changes to its session were not sent, so they are lost — {}",
                reason(&why)
            ),
        );
        nvs_runtime::floor::report(ctx, &record);
    }
}

/// The text a [`Fault`] carries, for the one caller in this module that has nowhere to throw it.
///
/// [`Fault::Pending`] cannot arrive here — it is what a helper that already ran compiled Novis
/// code returns, and neither call in [`send_at_end`] does — but it carries no message by
/// construction, so it is answered rather than asserted. It is answered by the *wildcard* because
/// the enum is `#[non_exhaustive]`: a variant added later is a failure with no text this module
/// knows how to read, which is the same case.
fn reason(fault: &Fault) -> &str {
    match fault {
        Fault::Thrown(_, message)
        | Fault::ThrownWithSlots(_, message, _)
        | Fault::Fatal(message) => message,
        _ => "the store refused it",
    }
}

/// The address of one of *this* module's symbols, or `None` for a symbol that belongs to another
/// domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_session_start" => (nvs_core_session_start as *const ()).cast(),
        "nvs_core_session_get" => (nvs_core_session_get as *const ()).cast(),
        "nvs_core_session_set" => (nvs_core_session_set as *const ()).cast(),
        "nvs_core_session_remove" => (nvs_core_session_remove as *const ()).cast(),
        "nvs_core_session_clear" => (nvs_core_session_clear as *const ()).cast(),
        "nvs_core_session_regenerate" => (nvs_core_session_regenerate as *const ()).cast(),
        "nvs_core_session_destroy" => (nvs_core_session_destroy as *const ()).cast(),
        "nvs_core_session_set_secret" => (nvs_core_session_set_secret as *const ()).cast(),
        "nvs_core_session_get_secret" => (nvs_core_session_get_secret as *const ()).cast(),
        _ => return None,
    })
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
    use nvs_runtime::{Ctx, NvsArray, NvsStr, Value};

    use super::{
        DEFAULT_TTL, PREFIX, SEAL_DOMAIN, backend, cookie, destroy, key_of, load, mint,
        opened_from, record, save, sealed_into, sealed_key, ttl, write_back,
    };
    use crate::cache::redis::Connection;
    use crate::cache::{Dial, Target, store_get, store_put};

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
    /// `SET … PX`, `GET` and `DEL` only, because those are the three commands [`save`], [`load`]
    /// and [`destroy`] send; anything else is a panic rather than a silent `+OK`, so a fourth
    /// command added upstream fails here instead of passing untested.
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
                                parts[3], b"PX",
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
                        b"DEL" => {
                            let gone = held.lock().expect("the store").remove(&parts[1]);
                            format!(":{}\r\n", usize::from(gone.is_some())).into_bytes()
                        }
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

    /// `rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent`, as the roster and as the write.
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
            "`rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent` is enforced by the roster having no local entry: {BACKENDS:?}"
        );
        assert_eq!(Backend::of("local"), None);

        let (listener, address) = listening();
        let held = Arc::new(Mutex::new(HashMap::new()));
        let store = Arc::clone(&held);
        thread::spawn(move || serving(listener, store));

        let mut open = Connection::new(
            Dial::configured(Target::Tcp(address), None, None),
            Duration::from_secs(5),
        );
        save(&mut open, ID, RECORD, DEFAULT_TTL).expect("the record reaches the configured store");

        assert_eq!(
            held.lock().expect("the store").get(&key_of(ID)).cloned(),
            Some(RECORD.to_vec()),
            "the record is in the shared store"
        );
        assert_eq!(
            store_get(&key_of(ID)),
            None,
            "and nowhere in the per-core tier `rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent` refuses it"
        );
    }

    /// `rule:http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot`'s whole reason, asserted as the contrast rather than as a round trip.
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
            let mut open = Connection::new(
                Dial::configured(Target::Tcp(address), None, None),
                Duration::from_secs(5),
            );
            save(&mut open, ID, RECORD, DEFAULT_TTL).expect("the record is written");
            store_put(
                &key_of(ID),
                RECORD.to_vec(),
                crate::cache::Lifetime::Forever,
                None,
            );
            assert_eq!(store_get(&key_of(ID)), Some(RECORD.to_vec()));
        });
        wrote.join().expect("the first core");

        let read = thread::spawn(move || {
            let mut open = Connection::new(
                Dial::configured(Target::Tcp(address), None, None),
                Duration::from_secs(5),
            );
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
             which is what `rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent` refuses a session for"
        );
    }

    /// §§ 1 and 4's read and write over one record: what `set` put there, `get` reads back, and
    /// `remove` takes out again — with the record's own bytes asserted beside the answers.
    ///
    /// **Here rather than in a `.nvst` case**, because no conformance case can reach a session
    /// store and none of this needs one: § 4 loads the record once and every change until the end
    /// of the request happens in the request's own heap, so the members below never touch a socket.
    /// The corpus asks the *language* rule instead — that a member called before `start` throws.
    ///
    /// **The bytes are asserted, not only the answers.** A member that kept a decoded array of its
    /// own on the side would satisfy every assertion about what `get` said and leave
    /// `nvs_runtime::Session::record` empty, so the write-back at the end of the request would send
    /// nothing. And the empty record is asserted as *zero bytes* rather than as the encoding of an
    /// empty array, which is the invariant `start` already relies on for a session it minted.
    #[test]
    fn the_record_carries_what_set_wrote_and_loses_what_remove_took() {
        let mut ctx = Ctx::buffered();
        ctx.open_session(nvs_runtime::Session {
            id: ID.to_owned(),
            record: Vec::new(),
            dirty: false,
            write_back: super::send_at_end,
        });

        let empty = record(&ctx, "get").expect("a started session has a record to read");
        assert!(
            empty.is_empty(),
            "a record of zero bytes decodes to no keys, rather than refusing as a payload that \
             carries no marker"
        );
        drop(empty);

        let mut writing = record(&ctx, "set").expect("a started session has a record to write");
        writing.set(NvsStr::new(b"cart"), Value::int(17));
        write_back(&mut ctx, writing, "set").expect("the changed record encodes");

        let after = ctx.session().expect("the session is still open");
        assert!(
            after.dirty,
            "§ 4's write-back is what the flag earns, and a `set` that left it clear would be a \
             change the store never hears about"
        );
        assert!(!after.record.is_empty());

        let reading = record(&ctx, "get").expect("the record decodes again");
        assert_eq!(
            reading.get(b"cart").and_then(Value::as_int),
            Some(17),
            "what `set` wrote is what `get` reads, across the encode and the decode between them"
        );
        drop(reading);

        let mut removing = record(&ctx, "remove").expect("the record decodes to be changed");
        removing.unset(b"cart");
        write_back(&mut ctx, removing, "remove").expect("what is left of the record encodes");

        let emptied = ctx.session().expect("the session is still open");
        assert!(
            emptied.record.is_empty(),
            "a record with no keys left is zero bytes again, not the encoding of an empty array — \
             `nvs_runtime::Session::record`'s own doc owns why, and `start` relies on it"
        );
    }

    /// § 4's write-back, asserted **both ways**: the send a changed record earns, and the round
    /// trip a read-only request does not make.
    ///
    /// One direction alone passes something broken. A build that sent unconditionally would
    /// satisfy "a `set` reaches the store" while spending a write on every request that read its
    /// session — the cost § 4 names when it says only a changed record is written — and a build
    /// that never sent would satisfy "a read-only request makes one round trip" perfectly.
    ///
    /// **The flag is earned through the real member path**, by the same [`write_back`] `set` calls,
    /// rather than written by hand: a `set` that stopped marking the record would pass a test that
    /// set `dirty` itself and lose every write in production.
    ///
    /// **The send is a test's own function pointer**, which is what the seam is for: whether the
    /// bytes reach a store is [`save`]'s question and three cases above already ask it, while what
    /// is unproven here is that `Ctx::end_session` calls what the record carries exactly when the
    /// flag says to. The third assertion is the idempotence that method promises — sending clears
    /// the flag — which is what lets both ends of a program call it without agreeing on which of
    /// them is the real one.
    #[test]
    fn a_changed_record_is_sent_when_the_program_ends_and_an_unchanged_one_is_not() {
        thread_local! {
            static SENT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        }
        fn counting(_ctx: &mut Ctx) {
            SENT.with(|sent| sent.set(sent.get() + 1));
        }

        let mut ctx = Ctx::buffered();
        ctx.open_session(nvs_runtime::Session {
            id: ID.to_owned(),
            record: Vec::new(),
            dirty: false,
            write_back: counting,
        });

        // The read-only request, made of the one member that reads.
        drop(record(&ctx, "get").expect("a started session has a record to read"));
        ctx.end_session();
        assert_eq!(
            SENT.with(std::cell::Cell::get),
            0,
            "a request that only read its session makes one round trip, not two — § 4's whole \
             reason for the flag"
        );

        let mut writing = record(&ctx, "set").expect("a started session has a record to write");
        writing.set(NvsStr::new(b"cart"), Value::int(17));
        write_back(&mut ctx, writing, "set").expect("the changed record encodes");

        ctx.end_session();
        assert_eq!(
            SENT.with(std::cell::Cell::get),
            1,
            "and a request that changed it sends it, through the pointer the record carries"
        );

        ctx.end_session();
        assert_eq!(
            SENT.with(std::cell::Cell::get),
            1,
            "sending clears the flag, so a second end sends nothing"
        );
    }

    /// § 2's fourth operation: after `destroy` the store answers absent, which is the same answer
    /// it gives an identifier it never issued.
    ///
    /// That equality is the point rather than a coincidence — it is the whole of the strict-id
    /// rule, and it is what makes a signed-out identifier worth nothing to whoever still holds it.
    /// The second `destroy` is asserted too: `Core\Session::destroy` reaches this with whatever id
    /// the request had, and an id with no record under it is already forgotten, so a store that
    /// treated the repeat as a failure would turn a double sign-out into a thrown request.
    #[test]
    fn a_destroyed_record_is_gone_and_forgetting_it_twice_is_not_a_failure() {
        let (listener, address) = listening();
        let held: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
        thread::spawn(move || serving(listener, held));

        let answered = thread::spawn(move || {
            let mut open = Connection::new(
                Dial::configured(Target::Tcp(address), None, None),
                Duration::from_secs(5),
            );
            save(&mut open, ID, RECORD, DEFAULT_TTL).expect("the record is written");
            let before = load(&mut open, ID).expect("the store answers");
            destroy(&mut open, ID).expect("the record is forgotten");
            let after = load(&mut open, ID).expect("the store answers");
            destroy(&mut open, ID).expect("an id with no record under it is already forgotten");
            (before, after)
        });
        let (before, after) = answered.join().expect("the core");

        assert_eq!(before, Some(RECORD.to_vec()));
        assert_eq!(after, None);
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

    /// Releases one reference a case built, which is the whole of what it owns.
    fn dropped(value: Value) {
        #[expect(
            unsafe_code,
            reason = "a test frame owns exactly the reference it built"
        )]
        unsafe {
            value.release();
        }
    }

    /// A user's secret reaches the record only sealed: the entry `setSecret` leaves holds
    /// ciphertext, the ordinary read is blind to it, and the sealed door reads it back.
    ///
    /// Asserted over the **encoded** record as well as the entry, because a member that sealed the
    /// value and then also left it somewhere in the clear would pass on the entry alone — and what
    /// crosses to the store is those bytes, not the array this process decoded.
    #[test]
    fn session_secret_is_stored_as_ciphertext_in_the_record() {
        const TOKEN: &[u8] = b"sk-live-7";

        let mut ctx = Ctx::buffered();
        ctx.set_random_state(23);
        ctx.open_session(nvs_runtime::Session {
            id: ID.to_owned(),
            record: Vec::new(),
            dirty: false,
            write_back: super::send_at_end,
        });

        let ring = crate::keyring::tests::ring_of(&[&[7_u8; 32]]);
        let keys = crate::keyring::tests::borrowed(&ring);

        let mut writing = record(&ctx, "setSecret").expect("a started session has a record");
        sealed_into(&mut ctx, &mut writing, "token", TOKEN, &keys, "test")
            .expect("a ring of one key seals a short value");
        write_back(&mut ctx, writing, "setSecret").expect("the changed record encodes");

        let stored = ctx
            .session()
            .expect("the session is still open")
            .record
            .clone();
        assert!(
            !stored.windows(TOKEN.len()).any(|window| window == TOKEN),
            "the bytes the store is handed carry the secret nowhere in the clear"
        );

        let reading = record(&ctx, "getSecret").expect("the record decodes again");
        let entry = reading
            .get(&sealed_key(b"token"))
            .expect("the sealed entry is under the sealed name");
        assert_ne!(
            entry.as_bytes(),
            Some(TOKEN),
            "and neither does the entry itself"
        );
        assert!(
            reading.get(b"token").is_none(),
            "`get` is blind to it, so the sealed door is the only door here too"
        );

        let opened = opened_from(&ctx, &reading, "token", &keys, "test")
            .expect("nothing here is unaffordable")
            .expect("its own ring opens what it sealed");
        assert_eq!(
            opened.as_text(),
            Some("sk-live-7"),
            "what went in is what comes back out, across the encode and the decode between them"
        );

        dropped(opened);
        drop(reading);
        dropped(ring);
    }

    /// A value sealed for a cache does not open as a session secret, and a session secret does not
    /// open as a cached value.
    ///
    /// The ring, the application and the name are the same on both sides here, so the one thing
    /// that can refuse either direction is the domain octet under the tag — which makes this an
    /// assertion about the two constructions rather than about a key that happened to differ. The
    /// bound is asserted on the other side too: the session's own construction still opens it.
    #[test]
    fn a_value_sealed_for_the_cache_does_not_open_as_a_session_secret() {
        const TOKEN: &[u8] = b"sk-live-7";
        const KEY: &[u8] = b"token";

        let mut ctx = Ctx::buffered();
        ctx.set_random_state(29);
        let ring = crate::keyring::tests::ring_of(&[&[9_u8; 32]]);
        let keys = crate::keyring::tests::borrowed(&ring);
        let (slot, newest) = crate::keyring::newest(&keys);
        let cipher =
            crate::keyring::cipher_at(&newest, slot, "test").expect("the fixture is a key");

        let app = crate::cache::application(&ctx).to_vec();
        let cached = crate::cache::bound(crate::cache::SEAL_DOMAIN, &app, KEY);
        let sessioned = crate::cache::bound(SEAL_DOMAIN, &app, KEY);

        let mut held = NvsArray::new();
        let for_the_cache = crate::crypto::seal_under(&mut ctx, &cipher, &cached, TOKEN, "test")
            .expect("a short value seals");
        held.set(
            NvsStr::new(&sealed_key(KEY)),
            Value::bytes(NvsStr::new(&for_the_cache)),
        );
        assert!(
            opened_from(&ctx, &held, "token", &keys, "test")
                .expect("nothing here is unaffordable")
                .is_none(),
            "a ciphertext an operator moved out of a cache and into a record is a miss"
        );

        sealed_into(&mut ctx, &mut held, "token", TOKEN, &keys, "test")
            .expect("a ring of one key seals a short value");
        let entry = held
            .get(&sealed_key(KEY))
            .expect("the sealed entry is there");
        let sealed = entry.as_bytes().expect("a sealed entry is bytes");
        assert!(
            crate::crypto::open_under(&cipher, &cached, sealed, "test")
                .expect("nothing here is unaffordable")
                .is_none(),
            "and the same ring reading it as a cached value gets nothing back"
        );
        assert!(
            crate::crypto::open_under(&cipher, &sessioned, sealed, "test")
                .expect("nothing here is unaffordable")
                .is_some(),
            "while the construction that wrote it opens it, which is what makes the octet the \
             thing under test"
        );

        drop(held);
        dropped(ring);
    }
}
