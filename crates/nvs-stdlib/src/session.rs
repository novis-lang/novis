//! `rule:http-server/a-session-store-answers-five-operations`'s session store:
//! the identifier a store issues, the keys it keeps under it, and the two directives that decide
//! where that store is and how long a session survives.
//!
//! **What is on disk is the store `rule:http-server/a-session-store-answers-five-operations` names
//! and all twelve members of `rule:core-api/session-roster`.** The five operations a backend
//! answers are here — [`issue`], [`load`], [`save`], [`relocate`] (the rule's `move`) and
//! [`destroy`], with [`mint`] and the key they share — over the shared tier's wire, one hash per
//! session, and [`sqlite`]'s over the data folder's SQLite file. `issue` is the only write that
//! creates a session, and `save` and `move` change only a session that still exists, so a
//! destroyed session stays destroyed. [`opened`] picks one as a [`Store`], and [`first_use`] is
//! what reaches it, at the first call of any member in a request
//! (`rule:core-classes/a-session-opens-on-its-first-use`): it takes the identifier the client
//! presented, loads every key the store keeps under it, and opens a fresh, empty session where
//! there is none, which reaches the store only once something is written to it.
//! [`nvs_core_session_open_with_identifier`] does the same under an identifier the program read
//! itself. Every member then operates on the session left on the request
//! ([`nvs_runtime::Session`]).
//!
//! **The write-back of `rule:http-server/a-session-is-loaded-once-and-written-back-per-key` is on disk
//! too.** [`write`] changes one key on the request and marks it touched, [`send_at_end`] sends the
//! touched keys when the program that opened the session ends, and
//! [`nvs_core_session_regenerate`], the clears' moves and [`nvs_core_session_clear_all`] reach the
//! store as they land because that rule makes them immediate. The section below is where the send is decided.
//!
//! **`db` is the default store.** A tree that writes no `[session] backend` gets the data folder's
//! `session.sqlite` (`rule:http-server/no-session-block-means-the-data-folder-store`), and a
//! process with no usable data folder throws at the first session call naming `[session]`. [`sqlite`]'s module
//! doc owns the file's schema, its sweep, and why a request never waits on another.
//!
//! # Decision: a dirty record is sent by the *program's* end, and it travels there itself
//!
//! `rule:http-server/a-session-is-loaded-once-and-written-back-per-key` writes the record back when the
//! request ends and leaves open who does it. Three
//! crates have a claim and none of them can hold it alone: this one owns the store and cannot see
//! a request end; `nvs-host` ends every isolate and may not name `nvs-stdlib`; `nvs-server` ends
//! the HTTP request and names neither. So the send is a `fn` pointer that rides on the record —
//! [`nvs_runtime::Session::write_back`], filled in here by the open and by `regenerate`, called
//! through `Ctx::end_session` by whoever ends the program. That method's own docs own the
//! mechanism; what is decided here is the *where*, and it is three refusals:
//!
//! **Not the door.** The obvious reading of that rule puts the send in `nvs-server`, at the line where a
//! request's response is collected — and the door does not have the record. A request is a root
//! isolate (`rule:security/isolate-shares-nothing`), so
//! the first `Core\Session` call opened the session on the *isolate's* context, which is built and dropped
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
//! # Decision: the memory tier is unreachable from here, structurally
//!
//! `rule:concurrency/the-memory-tier-cannot-hold-what-must-be-coherent` refuses
//! `Core\Session` the memory tier, and
//! `rule:http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot`'s word for the
//! refusal is *enforced*. Two things
//! carry that here and neither is a comment. [`nvs_config::session::Backend`] has no variant naming
//! the memory tier, so there is no value this module could match on to select it; and every
//! operation below is written against [`crate::cache::redis::Connection`] rather than against
//! [`crate::cache`]'s tier enum, so the process's map is not merely unselected but absent from the
//! type this module can reach. A test asserting the ban would pass over a module that reached
//! that map on a branch nobody exercised — which is why
//! `a_session_is_never_backed_by_the_memory_cache_tier` asserts the record's *absence* from that
//! map after a real write, and not only the roster.
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
//! `rule:http-server/a-session-store-answers-five-operations` refuses a separate `validateId`,
//! because a second question is a second thing that can
//! disagree with the first. [`issuable`] is not that second question. It answers **absent** — the
//! same answer [`load`] gives for an expired identifier and for one an attacker minted, and
//! the open responds to all three identically by issuing a fresh one. Having no third answer to
//! give, it has nothing to disagree with.
//!
//! What it buys is that a presented identifier of a megabyte never becomes a key this core sends
//! to the shared store, and it buys it *before* the round trip rather than after. [`mint`]'s
//! output is 22 base64url characters, so anything else is a string this store has not issued, by
//! construction rather than by lookup.
//!
//! # What this spends
//!
//! One round trip to the configured store per request that opens a session, and a second only for
//! a request that changed the record (`rule:http-server/a-session-is-loaded-once-and-written-back-per-key`).
//! A request that presents no identifier spends a draw
//! and one write. Memory is one encoded record per in-flight request that opened one, released
//! with the request heap — O(in-flight), never O(sessions served).
//!
//! A `setShortLivedSecret` or `setLongLivedSecret` spends one XChaCha20-Poly1305 seal and a `getSecret` one open per key of the ring
//! tried, over a record entry holding the value plus a nonce and a tag. Nothing is held between
//! calls: the ring is the program's and the ciphertext is the record's.
//!
//! **Every member that touches the record decodes it, and every member that changes it encodes it
//! back** — O(record) per call, over the bytes [`nvs_runtime::Session`] already holds, with the
//! decoded array living only for the length of the call. That is the accepted trade rather than an
//! oversight: a session record is a handful of keys beside the network round trip the open has
//! already spent, and the alternative — holding the decoded array on the context — would put an
//! object at teardown that an
//! `rule:config/an-edit-reaches-the-next-request-without-a-restart` unit swap could strand,
//! which is the whole reason that struct holds bytes.

use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use nvs_config::session::Backend;
use nvs_runtime::{Ctx, Fault, NvsArray, NvsStr, ThrownClass, TraceKind, Value};
use nvs_syntax::duration;

use crate::cache::redis::{Connection, Renamed, Replay};
use crate::cache::{application, bound, configured, on_shared, open_configured, sealed_key};
use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

mod sqlite;

/// The class name, once, for the messages that all name it.
pub(crate) const NAME: &str = r"Core\Session";

/// `Core\Session`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Keeps values for one visitor from one request to the next. A short-lived value is kept \
            until the browser closes or the visitor is inactive for `[session] short_lived`. A \
            long-lived value is kept for `[session] long_lived`. The first call of any method \
            opens the session. By default, the values are saved in a SQLite file in the data \
            folder. `[session] backend = \"shared\"` saves them in the shared cache instead.",
};

/// The text type of a key, and of an identifier read from the request.
///
/// `Qual::Neutral`, which is `Core\Cache\Store::get`'s judgement over the same boundary: a record
/// key derived from the request is data rather than an instruction
/// (`rule:security/unclassified-parameter-refuses-tainted`).
const KEY: CoreTy = CoreTy::Text(Qual::Neutral);

/// `rule:core-api/session-roster`'s roster, in its order.
///
/// One class and no instance side: a session is the request's, not an object a program holds, so
/// there is nothing for a handle to be and nothing to hand back. Every member is static for that
/// reason. [`crate::session`]'s own module doc owns which of
/// `rule:http-server/a-session-store-answers-five-operations`'s five operations are on disk, why
/// the local tier is unreachable from there rather than merely unselected, and why a presented
/// identifier this store cannot have issued is *absent* rather than a second question about
/// validity.
// rank: 780
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "setShortLived",
            names: &["key", "value"],
            // The value is unclassified — `CoreTy::Mixed` — so a `tainted` one is refused, again
            // as `Core\Cache\Store::put` refuses it and for that member's reason: a qualifier is a
            // compile-time fact and the record is bytes, so `get` has nowhere to carry it back out
            // and admitting one here would launder it.
            params: &[KEY, CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_set_short_lived",
            doc: Some(&SET_SHORT_LIVED_DOC),
        },
        CoreMethod {
            name: "setLongLived",
            names: &["key", "value"],
            params: &[KEY, CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_set_long_lived",
            doc: Some(&SET_LONG_LIVED_DOC),
        },
        CoreMethod {
            name: "get",
            names: &["key"],
            params: &[KEY],
            defaults: &[],
            // `mixed` rather than `rule:core-api/shape-rules` R7's `?T`: what went in is any value the byte carrier
            // admits, so there is no `T` to make nullable, and `mixed` already spells absent.
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_session_get",
            doc: Some(&GET_DOC),
        },
        CoreMethod {
            name: "remove",
            names: &["key"],
            params: &[KEY],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_remove",
            doc: Some(&REMOVE_DOC),
        },
        CoreMethod {
            name: "clearShortLived",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_clear_short_lived",
            doc: Some(&CLEAR_SHORT_LIVED_DOC),
        },
        CoreMethod {
            name: "clearLongLived",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_clear_long_lived",
            doc: Some(&CLEAR_LONG_LIVED_DOC),
        },
        CoreMethod {
            name: "clearAll",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_clear_all",
            doc: Some(&CLEAR_ALL_DOC),
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
            name: "setShortLivedSecret",
            names: &["key", "value", "keys"],
            // The value is a demand and not an admission, as `Core\Cache\Store::putSecret`'s is:
            // the qualifier widens on and narrows through nothing, so a plain `string` reaches
            // this parameter and a `secret` one does too, with nothing laundered either way. What
            // the row says is that this member is written for a confidential value — the plain
            // setters' `mixed` refuses one, and this is where it goes instead.
            //
            // No `ttl` where the cache's door has one: a session value lives for its lifetime
            // (`rule:http-server/session-expiry-belongs-to-the-store`).
            params: &[
                KEY,
                CoreTy::SecretText(Qual::Neutral),
                CoreTy::Array(&crate::keyring::KEY),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_set_short_lived_secret",
            doc: Some(&SET_SHORT_LIVED_SECRET_DOC),
        },
        CoreMethod {
            name: "setLongLivedSecret",
            names: &["key", "value", "keys"],
            params: &[
                KEY,
                CoreTy::SecretText(Qual::Neutral),
                CoreTy::Array(&crate::keyring::KEY),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_set_long_lived_secret",
            doc: Some(&SET_LONG_LIVED_SECRET_DOC),
        },
        CoreMethod {
            name: "getSecret",
            names: &["key", "keys"],
            params: &[KEY, CoreTy::Array(&crate::keyring::KEY)],
            defaults: &[],
            // `rule:core-api/shape-rules` R7's `?T` where `get` had no `T` to make nullable: a
            // sealed value is the one type the sealed setters admitted. The `secret` is a promise
            // rather than a conditional — what comes back out is confidential whatever the
            // `string` that went in was typed as — which is why this spells `CoreTy::SecretStr`
            // and not the parameter form beside it.
            return_ty: CoreTy::Nullable(&CoreTy::SecretStr),
            symbol: "nvs_core_session_get_secret",
            doc: Some(&GET_SECRET_DOC),
        },
        CoreMethod {
            name: "openWithIdentifier",
            names: &["id"],
            // `tainted` is admitted, because the strict-id rule
            // (`rule:http-server/a-session-store-answers-five-operations`) makes the value a
            // lookup key and never an instruction: what the store did not issue is absent,
            // whatever it was.
            params: &[KEY],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_session_open_with_identifier",
            doc: Some(&OPEN_WITH_IDENTIFIER_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The error every method of the class lists for a program that is not answering a request, and
/// every `Core` member that opens the session lists too (`Core\Csrf::token`, `Core\Passkey`).
pub(crate) const NO_REQUEST: ErrorDoc = ErrorDoc {
    error: "LogicError",
    desc: "The program is not handling a request. A command-line program, a job and a test have \
           no session.",
};

/// The error every method lists for a store it cannot open or reach at the first call.
pub(crate) const NO_STORE: ErrorDoc = ErrorDoc {
    error: "RuntimeError",
    desc: "The session store cannot be opened. This happens only at the first call in the request \
           that uses the session. The server has \
           no data folder for the default store. With `[session] backend = \"shared\"`, \
           `[cache.shared] url` is not set, or the program does not have the `cache.shared` \
           capability.",
};

/// The error every method lists for a store that does not answer.
pub(crate) const STORE_DOWN: ErrorDoc = ErrorDoc {
    error: "IOError",
    desc: "The store cannot be reached. The user may still have a session there, so do not treat \
           them as signed out.",
};

/// The error every method that changes the session lists for a call where headers are sent.
const HEAD_CLOSED: ErrorDoc = ErrorDoc {
    error: "LogicError",
    desc: "The call is inside `Core\\Html::later` or `Core\\Task::afterResponse`. The response \
           headers are already sent there, so the session cannot change. Most of these calls do \
           not compile.",
};

/// `Core\Session::get`'s reference card — `rule:core-api/reference-card`.
const GET_DOC: MethodDoc = MethodDoc {
    short: "Returns the value saved under one key in the session of this request. The value can \
            be short-lived or long-lived.",
    params: &[ParamDoc {
        name: "key",
        desc: "The key to read. If the session has no value under it, the result is `null`. A key \
               that was set to `null` gives the same result.",
        shape: &[],
    }],
    ret: "The value saved under `$key`, or `null`. A short-lived value is `null` after the \
          browser was closed, or after the visitor was inactive for `[session] short_lived`.",
    errors: &[
        NO_REQUEST,
        NO_STORE,
        STORE_DOWN,
        ErrorDoc {
            error: "ParseError",
            desc: "The value under `$key` contains an object of a class this program does not \
                   declare. This happens when another program saved it.",
        },
    ],
};

/// The `$key` parameter of the two plain setters.
const SET_KEY: ParamDoc = ParamDoc {
    name: "key",
    desc: "The key to save the value under. If the key already has a value of either lifetime, \
           it is replaced, and the key gets the lifetime of this call.",
    shape: &[],
};

/// The `$value` parameter of the two plain setters.
const SET_VALUE: ParamDoc = ParamDoc {
    name: "value",
    desc: "The value to save: a string, a number, a bool, `null`, an array or an object. A \
           `tainted` value (text that came from the request) is not allowed here, so the call \
           does not compile.",
    shape: &[],
};

/// The errors of the two plain setters.
const SET_ERRORS: &[ErrorDoc] = &[
    NO_REQUEST,
    HEAD_CLOSED,
    NO_STORE,
    STORE_DOWN,
    ErrorDoc {
        error: "LogicError",
        desc: "`$value` cannot be saved: it is a callable or a resource, or it contains one.",
    },
    ErrorDoc {
        error: "RuntimeError",
        desc: "`[session] cookie` is not a valid cookie name. It is thrown when the call must \
               send the cookie.",
    },
];

/// `Core\Session::setShortLived`'s reference card — `rule:core-api/reference-card`.
const SET_SHORT_LIVED_DOC: MethodDoc = MethodDoc {
    short: "Saves a short-lived value under one key in the session of this request. Use it for \
            a login. The value is deleted when the browser closes, or when the visitor is \
            inactive for `[session] short_lived`, which is 2 hours by default.",
    params: &[SET_KEY, SET_VALUE],
    ret: "Nothing. When the request ends, the keys it changed are saved to the store. The first \
          short-lived value in a session also sends a second cookie. Its name is `[session] \
          cookie` with `-short` added. Two requests that change different keys at the same time \
          keep both changes. If they change the same key, the request that ends last wins.",
    errors: SET_ERRORS,
};

/// `Core\Session::setLongLived`'s reference card — `rule:core-api/reference-card`.
const SET_LONG_LIVED_DOC: MethodDoc = MethodDoc {
    short: "Saves a long-lived value under one key in the session of this request. Use it for a \
            choice the visitor made, such as their language or their cookie consent. The value \
            is kept for `[session] long_lived`, which is 180 days by default. It is kept after \
            the browser closes.",
    params: &[SET_KEY, SET_VALUE],
    ret: "Nothing. When the request ends, the keys it changed are saved to the store. While the \
          session has a long-lived value, the session cookie has a `Max-Age`, so the browser \
          keeps it after it closes. Do not save a login here: it would stay signed in after the \
          browser closes.",
    errors: SET_ERRORS,
};

/// `Core\Session::remove`'s reference card — `rule:core-api/reference-card`.
const REMOVE_DOC: MethodDoc = MethodDoc {
    short: "Deletes one key and its value from the session of this request. The value can be \
            short-lived or long-lived.",
    params: &[ParamDoc {
        name: "key",
        desc: "The key to delete. If the session has no value under it, `remove()` does not throw \
               an error and the session does not change.",
        shape: &[],
    }],
    ret: "Nothing. After it, `get()` returns `null` for `$key`. The other keys keep their values.",
    errors: &[NO_REQUEST, HEAD_CLOSED, NO_STORE, STORE_DOWN],
};

/// The errors of the two clears and `regenerate`.
const MOVE_ERRORS: &[ErrorDoc] = &[
    NO_REQUEST,
    HEAD_CLOSED,
    ErrorDoc {
        error: "RuntimeError",
        desc: "The store cannot be opened, or `[session] cookie` is not a valid cookie name.",
    },
    ErrorDoc {
        error: "IOError",
        desc: "The store cannot be reached. The old identifier may still work, so do not treat \
               the user as signed out.",
    },
];

/// `Core\Session::clearShortLived`'s reference card — `rule:core-api/reference-card`.
const CLEAR_SHORT_LIVED_DOC: MethodDoc = MethodDoc {
    short: "Deletes every short-lived value from the session of this request. Call it when a \
            user signs out. The long-lived values are kept.",
    params: &[],
    ret: "Nothing. The session gets a new identifier, and the response sends it in the session \
          cookie. The old identifier no longer finds the session. Short-lived values that another \
          request saved in the meantime are deleted too.",
    errors: MOVE_ERRORS,
};

/// `Core\Session::clearLongLived`'s reference card — `rule:core-api/reference-card`.
const CLEAR_LONG_LIVED_DOC: MethodDoc = MethodDoc {
    short: "Deletes every long-lived value from the session of this request. The short-lived \
            values are kept.",
    params: &[],
    ret: "Nothing. The session gets a new identifier, and the response sends it in the session \
          cookie. The cookie has no `Max-Age` any more, so the browser deletes it when it closes. \
          Long-lived values that another request saved in the meantime are deleted too.",
    errors: MOVE_ERRORS,
};

/// `Core\Session::clearAll`'s reference card — `rule:core-api/reference-card`.
const CLEAR_ALL_DOC: MethodDoc = MethodDoc {
    short: "Deletes the whole session of this request from the store, and tells the browser to \
            delete both session cookies.",
    params: &[],
    ret: "Nothing. After it, the request has an empty session with no identifier. If the \
          request saves a value after `clearAll()`, a new session starts with a new identifier.",
    errors: &[
        NO_REQUEST,
        HEAD_CLOSED,
        ErrorDoc {
            error: "RuntimeError",
            desc: "The store cannot be opened, or `[session] cookie` is not a valid cookie name.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The store cannot be reached. The data is still in the store, and the session \
                   stays open, so you can try again.",
        },
    ],
};

/// `Core\Session::regenerate`'s reference card — `rule:core-api/reference-card`.
const REGENERATE_DOC: MethodDoc = MethodDoc {
    short: "Gives the session a new identifier and deletes the old one from the store. Call it \
            when a user signs in, so an identifier that somebody else knew stops working.",
    params: &[],
    ret: "Nothing. Every value in the session is kept, also a value that another request saved \
          in the meantime. The response sends the new identifier in the session cookie. The old \
          identifier no longer finds the session.",
    errors: MOVE_ERRORS,
};

/// The `$keys` parameter of the two sealed setters.
const SECRET_KEYS: ParamDoc = ParamDoc {
    name: "keys",
    desc: "A list of encryption keys, newest first. `Core\\Crypto::generateKey()` creates one. \
           The first key encrypts the secret.",
    shape: &[],
};

/// The `$key` parameter of the two sealed setters.
const SECRET_KEY: ParamDoc = ParamDoc {
    name: "key",
    desc: "The key to save the secret under. `get()` returns `null` for this key. Only \
           `getSecret()` with the same `$keys` can read the secret.",
    shape: &[],
};

/// The errors of the two sealed setters.
const SECRET_ERRORS: &[ErrorDoc] = &[
    NO_REQUEST,
    HEAD_CLOSED,
    NO_STORE,
    STORE_DOWN,
    ErrorDoc {
        error: "LogicError",
        desc: "`$keys` is empty, or its first key is not 32 bytes long.",
    },
];

/// `Core\Session::setShortLivedSecret`'s reference card — `rule:core-api/reference-card`.
const SET_SHORT_LIVED_SECRET_DOC: MethodDoc = MethodDoc {
    short: "Saves a short-lived secret, such as an access token, in the session of this request. \
            The secret is encrypted with a key from `$keys`, so the store only sees encrypted \
            bytes. Use it for a secret that belongs to a login.",
    params: &[
        SECRET_KEY,
        ParamDoc {
            name: "value",
            desc: "The secret to save, for example an access token or a refresh token of the \
                   user.",
            shape: &[],
        },
        SECRET_KEYS,
    ],
    ret: "Nothing. The secret is saved to the store when the request ends. It is deleted with \
          the other short-lived values.",
    errors: SECRET_ERRORS,
};

/// `Core\Session::setLongLivedSecret`'s reference card — `rule:core-api/reference-card`.
const SET_LONG_LIVED_SECRET_DOC: MethodDoc = MethodDoc {
    short: "Saves a long-lived secret in the session of this request. The secret is encrypted \
            with a key from `$keys`, so the store only sees encrypted bytes.",
    params: &[
        SECRET_KEY,
        ParamDoc {
            name: "value",
            desc: "The secret to save.",
            shape: &[],
        },
        SECRET_KEYS,
    ],
    ret: "Nothing. The secret is saved to the store when the request ends, and it is kept for \
          `[session] long_lived`. Anybody with the session cookie can use it, also after the \
          browser closes. Save a secret that belongs to a login with `setShortLivedSecret()`.",
    errors: SECRET_ERRORS,
};

/// `Core\Session::getSecret`'s reference card — `rule:core-api/reference-card`.
const GET_SECRET_DOC: MethodDoc = MethodDoc {
    short: "Reads a secret that `setShortLivedSecret()` or `setLongLivedSecret()` saved in the \
            session of this request. If the secret cannot be decrypted with `$keys`, the result \
            is `null`.",
    params: &[
        ParamDoc {
            name: "key",
            desc: "The key the secret was saved under. A secret saved under one key cannot be \
                   read under another key.",
            shape: &[],
        },
        ParamDoc {
            name: "keys",
            desc: "A list of encryption keys, newest first. Every key in the list is tried. When \
                   you add a new key at the front, secrets saved with the older keys can still be \
                   read.",
            shape: &[],
        },
    ],
    ret: "The secret saved under `$key`, or `null`. The result is `null` when nothing is saved \
          under `$key`, when no key in `$keys` decrypts it, and when the saved bytes were \
          changed.",
    errors: &[
        NO_REQUEST,
        NO_STORE,
        STORE_DOWN,
        ErrorDoc {
            error: "LogicError",
            desc: "`$keys` is empty, or a key in it is not 32 bytes long.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "The value under the secret's key contains an object of a class this program \
                   does not declare. This happens only when a program saved a plain value there.",
        },
    ],
};

/// `Core\Session::openWithIdentifier`'s reference card — `rule:core-api/reference-card`.
const OPEN_WITH_IDENTIFIER_DOC: MethodDoc = MethodDoc {
    short: "Opens the session of this request under an identifier that your program read, for \
            example from a header. Most programs do not need it: the first call of any other \
            method opens the session from the session cookie.",
    params: &[ParamDoc {
        name: "id",
        desc: "The session identifier. If the store has no session under it, a new, empty \
               session is opened. This happens when the identifier expired, and when somebody \
               made it up.",
        shape: &[],
    }],
    ret: "Nothing. No cookie is read for this session. Short-lived values need only the \
          identifier, because there is no browser that could close.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "Another method of this class was already called in this request, so the \
                   session is already open. The message names that method. It is also thrown \
                   when the program is not handling a request.",
        },
        NO_STORE,
        STORE_DOWN,
    ],
};

/// What every session key starts with, so an operator sharing one store between a cache, a limiter
/// and a session store can tell the three apart in it.
///
/// [`crate::ratelimit`]'s `PREFIX` is the same decision, and the two must not collide: a session id
/// is drawn and a limiter's key is application-supplied, so without the prefixes an application
/// could name a limiter key that reads a session.
///
/// `-lives` names the layout: one hash per session, with each key's lifetime in its field's prefix.
/// A hash an earlier build wrote under `nvs:session-keys:`, or a string under `nvs:session:`, is
/// never read. Those expire by their own lifetime, and their sessions are not converted.
pub(crate) const PREFIX: &str = "nvs:session-lives:";

/// A session's keys and their encoded values, as a store loads them and the request keeps them.
pub(crate) type Entries = std::collections::BTreeMap<Vec<u8>, Vec<u8>>;

/// The names of a session's long-lived keys.
pub(crate) type LongKeys = std::collections::BTreeSet<Vec<u8>>;

/// What a short-lived key's field in the shared store's hash starts with.
///
/// The other fields are the store's own, and [`ISSUE`] and [`SAVE`] write them: `m` is the
/// session's counted size, `t` the short token, `x` the short clock and `y` the long clock, each
/// clock in milliseconds since the epoch. `m` and `x` also keep a session with no keys in the
/// store: Redis deletes a hash when its last field goes, and the next write-back would then find
/// the session destroyed. A program cannot reach them, because every field a program writes starts
/// with this or [`LONG_FIELD`].
const SHORT_FIELD: &[u8] = b"s:";

/// What a long-lived key's field starts with. It is as long as [`SHORT_FIELD`], so a key costs the
/// same counted size with either lifetime.
const LONG_FIELD: &[u8] = b"l:";

/// The bytes each session key costs beyond its name and value in the counted size: `HGETALL`'s RESP
/// framing for one field and one value.
const FRAMING: usize = 32;

/// The most counted bytes the shared store keeps under one session.
///
/// [`crate::cache::redis::REPLY_CEILING`] bounds the one reply `HGETALL` reads at the open, so a
/// session past it could never be loaded again until it expired. 64 KiB is left for `m`, the
/// reply's own framing and the counting's rounding.
const SIZE_CEILING: usize = crate::cache::redis::REPLY_CEILING - 64 * 1024;

/// The most session keys the shared store keeps under one session: `HGETALL`'s element ceiling, as
/// fields, less the one `m` takes.
#[expect(
    clippy::cast_possible_truncation,
    reason = "32 767 fits every pointer width Novis builds for"
)]
const KEY_CEILING: usize = (crate::cache::redis::STRING_ELEMENT_CEILING / 2) as usize - 1;

/// `rule:http-server/a-session-store-answers-five-operations`'s `issue` on the shared store, run where
/// the session lives.
///
/// `KEYS[1]` is the session's hash. `ARGV[1]` is now, `ARGV[2]` the short clock's end and `ARGV[3]`
/// the long clock's end or `0`, all in milliseconds since the epoch. `ARGV[4]` is the counted size,
/// `ARGV[5]` the short token or empty, and every pair after it a field and its value.
///
/// It answers `{0}` for a key that exists, which keeps what it has, and `{1}` once the hash is
/// written. The hash expires at the long clock's end where it has a long-lived field, and at the
/// short clock's end otherwise. Each field is written by its own `HSET`, so no call takes more
/// arguments than Lua's stack holds.
const ISSUE: &str = "\
if redis.call('EXISTS', KEYS[1]) == 1 then return {0} end
local now, x, y = tonumber(ARGV[1]), tonumber(ARGV[2]), tonumber(ARGV[3])
redis.call('HSET', KEYS[1], 'm', ARGV[4])
redis.call('HSET', KEYS[1], 'x', ARGV[2])
if y > 0 then redis.call('HSET', KEYS[1], 'y', ARGV[3]) end
if ARGV[5] ~= '' then redis.call('HSET', KEYS[1], 't', ARGV[5]) end
local expires = x
for i = 6, #ARGV, 2 do
  redis.call('HSET', KEYS[1], ARGV[i], ARGV[i + 1])
  if string.sub(ARGV[i], 1, 2) == 'l:' and y > expires then expires = y end
end
redis.call('PEXPIRE', KEYS[1], math.max(expires - now, 1))
return {1}";

/// `rule:http-server/a-session-store-answers-five-operations`'s `save` on the shared store, run where
/// the session lives.
///
/// `KEYS[1]` is the session's hash. `ARGV[1]` to `ARGV[3]` are now and the two clocks, as
/// [`ISSUE`] takes them, and `ARGV[4]` the short token or empty. `ARGV[5]` names the lifetimes the
/// request cleared: `s`, `l`, both or neither. `ARGV[6]` is the size ceiling, `ARGV[7]` the key
/// ceiling, and `ARGV[8]` how many
/// removed keys follow, each bare. Every pair after those is a changed field, with its lifetime's
/// prefix, and its value.
///
/// It answers `{0}` for a session that is gone and writes nothing. Then it counts the new size: `m`,
/// less each old field it replaces or removes under either prefix, plus each new one, each field as
/// its name, its value and [`FRAMING`]. A cleared lifetime's fields count as removed. A size or a
/// key count past its ceiling answers `{-1, size}` and writes nothing. Otherwise it deletes every
/// field of a cleared lifetime, deletes both fields of each removed or changed key, sets the
/// changed ones and `m`, and answers `{1, size}`.
///
/// Each clock moves forward only, so a request that loaded an older record never shortens one
/// another request refreshed. The token is written when one is given. The expiry is the long
/// clock's end where a long-lived field remains, and the short clock's end otherwise. Each step
/// reads the hash as it is then, so a second run leaves the same hash.
const SAVE: &str = "\
local key = KEYS[1]
if redis.call('EXISTS', key) == 0 then return {0} end
local now, x, y = tonumber(ARGV[1]), tonumber(ARGV[2]), tonumber(ARGV[3])
local clear = ARGV[5]
local first = 9 + tonumber(ARGV[8])
local size = tonumber(redis.call('HGET', key, 'm')) or 0
local count = redis.call('HLEN', key)
for _, own in ipairs({'m', 'x', 'y', 't'}) do count = count - redis.call('HEXISTS', key, own) end
local gone, dropped = {}, {}
if clear ~= '' then
  for _, field in ipairs(redis.call('HKEYS', key)) do
    local prefix = string.sub(field, 1, 2)
    if (prefix == 's:' and string.find(clear, 's', 1, true))
      or (prefix == 'l:' and string.find(clear, 'l', 1, true)) then
      size = size - #field - redis.call('HSTRLEN', key, field) - 32
      count = count - 1
      gone[#gone + 1] = field
      dropped[field] = true
    end
  end
end
local function replaced(field)
  if not dropped[field] and redis.call('HEXISTS', key, field) == 1 then
    size = size - #field - redis.call('HSTRLEN', key, field) - 32
    count = count - 1
  end
end
for i = 9, first - 1 do replaced('s:' .. ARGV[i]); replaced('l:' .. ARGV[i]) end
for i = first, #ARGV, 2 do
  local bare = string.sub(ARGV[i], 3)
  replaced('s:' .. bare); replaced('l:' .. bare)
  size = size + #ARGV[i] + #ARGV[i + 1] + 32
  count = count + 1
end
if size > tonumber(ARGV[6]) or count > tonumber(ARGV[7]) then return {-1, size} end
for _, field in ipairs(gone) do redis.call('HDEL', key, field) end
for i = 9, first - 1 do redis.call('HDEL', key, 's:' .. ARGV[i], 'l:' .. ARGV[i]) end
for i = first, #ARGV, 2 do
  local bare = string.sub(ARGV[i], 3)
  redis.call('HDEL', key, 's:' .. bare, 'l:' .. bare)
  redis.call('HSET', key, ARGV[i], ARGV[i + 1])
end
redis.call('HSET', key, 'm', size)
local held = tonumber(redis.call('HGET', key, 'x')) or 0
if x > held then redis.call('HSET', key, 'x', ARGV[2]) else x = held end
held = tonumber(redis.call('HGET', key, 'y')) or 0
if y > held then redis.call('HSET', key, 'y', ARGV[3]) else y = held end
if ARGV[4] ~= '' then redis.call('HSET', key, 't', ARGV[4]) end
local expires = x
if y > x then
  for _, field in ipairs(redis.call('HKEYS', key)) do
    if string.sub(field, 1, 2) == 'l:' then expires = y break end
  end
end
redis.call('PEXPIRE', key, math.max(expires - now, 1))
return {1, size}";

/// The two clocks a write sends, in milliseconds since the epoch, and the time it was taken.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Clocks {
    /// The time the write was taken: what an expiry is counted from.
    pub(crate) now: i64,
    /// When the short clock ends. A write-back always moves it to now plus `[session]
    /// short_lived`.
    pub(crate) short_until: i64,
    /// When the long clock ends, or `0` while the record has never held a long-lived value.
    pub(crate) long_until: i64,
}

impl Clocks {
    /// When a record expires: the later clock's end where it has a long-lived key, and the short
    /// clock's end otherwise.
    pub(crate) fn expiry(self, has_long: bool) -> i64 {
        if has_long {
            self.short_until.max(self.long_until)
        } else {
            self.short_until
        }
    }
}

/// A whole session, as `issue` writes it for the first time.
#[derive(Debug)]
pub(crate) struct Whole<'a> {
    /// Every key and its encoded value.
    pub(crate) entries: &'a Entries,
    /// The keys whose lifetime is long.
    pub(crate) long: &'a LongKeys,
    /// The short token, where the session has one.
    pub(crate) token: Option<&'a str>,
    /// The two clocks.
    pub(crate) clocks: Clocks,
}

impl Whole<'_> {
    /// Whether any key is long-lived, which decides the record's expiry.
    pub(crate) fn has_long(&self) -> bool {
        self.entries.keys().any(|key| self.long.contains(key))
    }
}

/// A session as a store loads it: every key with its lifetime, the short token and both clocks.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Record {
    /// Every key and its encoded value, of either lifetime.
    pub(crate) entries: Entries,
    /// The keys whose lifetime is long.
    pub(crate) long: LongKeys,
    /// The short token the record holds.
    pub(crate) token: Option<String>,
    /// When the short clock ends, in milliseconds since the epoch.
    pub(crate) short_until: i64,
    /// When the long clock ends, or `0`.
    pub(crate) long_until: i64,
}

/// One request's changes to its session, as the write-back sends them to `save`.
#[derive(Debug, Default)]
pub(crate) struct Changes<'a> {
    /// Whether the request called `clearShortLived`, so every short-lived key the store holds is
    /// deleted first.
    pub(crate) cleared_short: bool,
    /// The same for `clearLongLived` and the long-lived keys.
    pub(crate) cleared_long: bool,
    /// The keys the request removed.
    pub(crate) removed: Vec<&'a [u8]>,
    /// The keys the request set, each with its encoded value and whether it is long-lived.
    pub(crate) changed: Vec<(&'a [u8], &'a [u8], bool)>,
    /// The short token to keep, where the request has one. `None` leaves the stored one.
    pub(crate) token: Option<&'a str>,
    /// The two clocks. Each moves forward only.
    pub(crate) clocks: Clocks,
}

impl<'a> Changes<'a> {
    /// The changes `open` owes the store: each touched key with a value is a change, and each one
    /// without is a removal.
    fn of(open: &'a nvs_runtime::Session, clocks: Clocks) -> Self {
        let mut changes = Self {
            cleared_short: open.cleared_short,
            cleared_long: open.cleared_long,
            token: open.short_token.as_deref().filter(|_| !open.by_hand),
            clocks,
            ..Self::default()
        };
        for key in &open.touched {
            match open.entries.get(key) {
                Some(value) => changes.changed.push((key, value, open.long.contains(key))),
                None => changes.removed.push(key),
            }
        }
        changes
    }
}

/// What `save` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Saved {
    /// The changes are written, and the expiry is refreshed.
    Done,
    /// The session is gone, and nothing was written.
    Gone,
    /// The session would have been this many counted bytes, past the shared store's ceiling, and
    /// nothing was written.
    TooLarge(i64),
}

/// `[session] backend` — `rule:http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot`'s store, as `nvs_config::session::Backend` spells it.
const BACKEND: &str = "session.backend";

/// `[session] short_lived` — how long a short-lived value survives its last write or refresh.
const SHORT_LIVED: &str = "session.short_lived";

/// `[session] long_lived` — how long a long-lived value, and the record that has one, survives its
/// last refresh.
const LONG_LIVED: &str = "session.long_lived";

/// `[session] cookie` — the name the identifier rides under.
const COOKIE: &str = "session.cookie";

/// The record lifetime a deployment that configured none inherits.
///
/// Two hours rather than PHP's twenty-four minutes: `session.gc_maxlifetime`'s default is a number
/// chosen when a session was a file a sweeper had to walk, and the cost of a longer one there was
/// disk that accumulated. Here the store expires the entry itself
/// (`rule:http-server/session-expiry-belongs-to-the-store`), so the only thing the
/// number trades is how long a stolen id stays useful against how often an idle user is signed out,
/// and two hours is the ordinary answer to that pair. It is deliberately not "until the browser
/// closes": a session with no server-side expiry is one an attacker's copy never loses either.
const DEFAULT_SHORT_LIVED: Duration = nvs_config::defaults::SESSION_SHORT_LIVED;

/// The long clock a deployment that configured none inherits: 180 days, long enough for a consent
/// choice or a language to outlive a season of visits.
const DEFAULT_LONG_LIVED: Duration = nvs_config::defaults::SESSION_LONG_LIVED;

/// What the name of the cookie that carries the short token adds to `[session] cookie`.
const SHORT_COOKIE_SUFFIX: &str = "-short";

/// The cookie name a deployment that configured none inherits.
///
/// Not `PHPSESSID` and not anything naming a framework: a cookie name is the one part of a session
/// that a scanner reads to decide what it is talking to, and there is nothing to gain by answering.
const DEFAULT_COOKIE: &str = nvs_config::defaults::SESSION_COOKIE;

/// How many bytes an identifier is drawn from — 128 bits, per the module doc.
const ID_BYTES: usize = 16;

/// The shared store's key for `app`'s record under `id`: the prefix, the application, one `NUL`
/// and the id.
///
/// The application is [`crate::cache::application`]'s, so an id one `[[app]]` block issued is
/// absent in another. A path holds no `NUL` and an id is base64url, so no two pairs share a key.
/// What it spends: the application's path bytes once per store call.
pub(crate) fn key_of(app: &[u8], id: &str) -> Vec<u8> {
    let mut key = Vec::with_capacity(PREFIX.len() + app.len() + 1 + id.len());
    key.extend_from_slice(PREFIX.as_bytes());
    key.extend_from_slice(app);
    key.push(0);
    key.extend_from_slice(id.as_bytes());
    key
}

/// Which store this deployment's records live in: the written `[session] backend`, or `db` where
/// none is written (`rule:http-server/no-session-block-means-the-data-folder-store`).
///
/// # Errors
///
/// A thrown `RuntimeError` for a word this module cannot spell. It is unreachable from a server
/// that booted: `nvs_config::session::validate` refused it at `E0626`. It is answered rather than
/// asserted because this crate is also reachable from a test driving a hand-built context, and a
/// panic there would report the fixture as an engine bug.
pub(crate) fn backend(ctx: &Ctx, member: &str) -> Result<Backend, Fault> {
    let Some(written) = configured(ctx, BACKEND) else {
        return Ok(Backend::Db);
    };
    Backend::of(&written).ok_or_else(|| {
        Fault::thrown(format!(
            "{member}: `[session] backend = \"{written}\"` is not a session store. Write \
             `backend = \"db\"` or `backend = \"shared\"`."
        ))
    })
}

/// Where this request's record lives, once the store has been checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Store {
    /// The shared tier, over this core's connection, which [`open_configured`] has opened.
    Shared,
    /// The SQLite file at this path, [`sqlite`]'s store.
    File(std::path::PathBuf),
}

/// The store this deployment's sessions live in, ready for one of the five operations.
///
/// `shared` opens this core's connection, so a store that is unconfigured or not granted throws
/// here. `db`, which is also the store a tree that writes no `backend` gets, is the data folder's
/// `session.sqlite` (`rule:http-server/no-session-block-means-the-data-folder-store`).
///
/// # Errors
///
/// [`backend`]'s, [`open_configured`]'s, and a thrown `RuntimeError` naming `[session]` where this
/// process has no usable data folder.
pub(crate) fn opened(ctx: &Ctx, member: &str, remedy: &str) -> Result<Store, Fault> {
    match backend(ctx, member)? {
        Backend::Shared => {
            open_configured(ctx, member, remedy)?;
            Ok(Store::Shared)
        }
        Backend::Db => nvs_config::data::current()
            .map(|folder| Store::File(folder.sessions()))
            .ok_or_else(|| {
                Fault::thrown(format!(
                    "{member}: sessions are saved in the data folder by default, and this \
                     server has no data folder it can use. Write `[session] backend = \
                     \"shared\"` to save them in the shared cache."
                ))
            }),
    }
}

/// `load` of `app`'s row on `store`, with a failure thrown as an `IOError` naming `member`.
///
/// # Errors
///
/// A thrown `IOError` for a store that cannot be reached. Never an answer that looks like absence:
/// [`load`]'s doc says why.
fn fetched(store: &Store, app: &[u8], member: &str, id: &str) -> Result<Option<Record>, Fault> {
    match store {
        Store::Shared => on_shared(NAME, member, |open| load(open, app, id)),
        Store::File(path) => on_file(member, sqlite::load(path, app, id)),
    }
}

/// `issue` on `store`, as [`fetched`]: `true` once a new session holds `whole`, and `false` for an
/// `id` that is already live, which keeps what it has.
///
/// # Errors
///
/// As [`fetched`], and a thrown `RuntimeError` for a session past the shared store's ceiling.
fn issued(
    store: &Store,
    app: &[u8],
    member: &str,
    id: &str,
    whole: &Whole<'_>,
) -> Result<bool, Fault> {
    match store {
        Store::Shared => {
            let size = counted(whole.entries);
            if size > SIZE_CEILING || whole.entries.len() > KEY_CEILING {
                return Err(too_large(member, size));
            }
            on_shared(NAME, member, |open| issue(open, app, id, whole, size))
        }
        Store::File(path) => on_file(member, sqlite::issue(path, app, id, whole)),
    }
}

/// `save` on `store`, as [`fetched`]: what it did with `changes` to the session under `id`.
///
/// # Errors
///
/// As [`fetched`].
fn saved(
    store: &Store,
    app: &[u8],
    member: &str,
    id: &str,
    changes: &Changes<'_>,
) -> Result<Saved, Fault> {
    match store {
        Store::Shared => on_shared(NAME, member, |open| save(open, app, id, changes)),
        Store::File(path) => on_file(member, sqlite::save(path, app, id, changes))
            .map(|done| if done { Saved::Done } else { Saved::Gone }),
    }
}

/// `move` on `store`, as [`fetched`]: the session under `old` moved to `new`, the new one taken, or
/// the old one gone.
///
/// # Errors
///
/// As [`fetched`].
fn moved(store: &Store, app: &[u8], member: &str, old: &str, new: &str) -> Result<Renamed, Fault> {
    match store {
        Store::Shared => on_shared(NAME, member, |open| relocate(open, app, old, new)),
        Store::File(path) => on_file(member, sqlite::relocate(path, app, old, new)),
    }
}

/// The size the shared store counts for `entries`, as the write-back script counts it: each key's
/// field name, its value, and [`FRAMING`].
fn counted(entries: &Entries) -> usize {
    entries
        .iter()
        .map(|(key, value)| SHORT_FIELD.len() + key.len() + value.len() + FRAMING)
        .sum()
}

/// The throw for a session the shared store would not keep, naming its size.
fn too_large(member: &str, size: impl std::fmt::Display) -> Fault {
    Fault::thrown(format!(
        "{NAME}::{member}: the session would be {size} bytes. The shared store keeps at most \
         {SIZE_CEILING} bytes and {KEY_CEILING} keys for one session."
    ))
}

/// `destroy` on `store`, as [`fetched`]: whether a row was deleted.
///
/// # Errors
///
/// As [`fetched`].
fn forgot(store: &Store, app: &[u8], member: &str, id: &str) -> Result<bool, Fault> {
    match store {
        Store::Shared => on_shared(NAME, member, |open| destroy(open, app, id)),
        Store::File(path) => on_file(member, sqlite::destroy(path, app, id)),
    }
}

/// The SQLite store's answer, with its failure thrown as [`on_shared`] throws the shared tier's.
fn on_file<T>(member: &str, answer: std::io::Result<T>) -> Result<T, Fault> {
    answer.map_err(|why| Fault::thrown_as(ThrownClass::Io, format!("{NAME}::{member}: {why}")))
}

/// `[session] short_lived`, or [`DEFAULT_SHORT_LIVED`], in milliseconds.
///
/// A directive that will not parse, or that parses to zero, is the shipped lifetime rather than a
/// refusal — [`crate::cache`]'s `timeout_of` reasoning: `nvs.toml` is validated where it is loaded,
/// and a zero here would be a session expiring the instant it is written.
pub(crate) fn short_lived(ctx: &Ctx) -> i64 {
    lifetime(ctx, SHORT_LIVED, DEFAULT_SHORT_LIVED)
}

/// `[session] long_lived`, or [`DEFAULT_LONG_LIVED`], in milliseconds, read as [`short_lived`].
pub(crate) fn long_lived(ctx: &Ctx) -> i64 {
    lifetime(ctx, LONG_LIVED, DEFAULT_LONG_LIVED)
}

/// The lifetime `key` configures, or `default`, in whole milliseconds and never zero.
fn lifetime(ctx: &Ctx, key: &str, default: Duration) -> i64 {
    let lifetime = configured(ctx, key)
        .and_then(|text| duration::parse(&text).ok())
        .map(|nanos| Duration::from_nanos(nanos.unsigned_abs()))
        .filter(|lifetime| !lifetime.is_zero())
        .unwrap_or(default);
    i64::try_from(lifetime.as_millis())
        .unwrap_or(i64::MAX)
        .max(1)
}

/// Whether a clock that ends at `until` has less than half of its `lifetime` left at `now`:
/// `rule:http-server/session-expiry-belongs-to-the-store`'s half rule.
fn past_half(until: i64, lifetime: i64, now: i64) -> bool {
    until.saturating_sub(now) < lifetime / 2
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

/// `rule:http-server/a-session-store-answers-five-operations`'s `load` — the record under `id`, or
/// **absent**.
///
/// Absent is the answer to all three of
/// `rule:security/a-session-id-the-store-did-not-issue-is-rejected`
/// 's cases at once: an id no store issued, one that has expired, and one an attacker minted.
/// The first `Core\Session` call responds to it by issuing a fresh id, which is why there is no separate
/// `validateId` for the two of them to disagree about.
///
/// # Errors
///
/// The store's own failure as text, for [`crate::cache::on_shared`] to classify. Never an answer
/// that looks like absence: a store that cannot be reached must not read as a forged id, because
/// the two have opposite responses.
pub(crate) fn load(open: &mut Connection, app: &[u8], id: &str) -> Result<Option<Record>, String> {
    let Some(fields) = open.hash_all(&key_of(app, id))? else {
        return Ok(None);
    };
    let mut record = Record::default();
    for (field, value) in fields {
        if let Some(key) = field.strip_prefix(SHORT_FIELD) {
            record.entries.insert(key.to_vec(), value);
        } else if let Some(key) = field.strip_prefix(LONG_FIELD) {
            record.long.insert(key.to_vec());
            record.entries.insert(key.to_vec(), value);
        } else {
            // `m` and any other field are the store's own, and never a key.
            match field.as_slice() {
                b"t" => record.token = String::from_utf8(value).ok(),
                b"x" => record.short_until = integer_in(&value),
                b"y" => record.long_until = integer_in(&value),
                _ => {}
            }
        }
    }
    Ok(Some(record))
}

/// A clock field's decimal text as an integer, and `0` for text that is not one.
fn integer_in(text: &[u8]) -> i64 {
    std::str::from_utf8(text)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(0)
}

/// `rule:http-server/a-session-store-answers-five-operations`'s `destroy` — the session under `id`,
/// deleted, and whether there was one. `DEL` of its hash.
///
/// `false` tells `regenerate` that another request destroyed the session first.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn destroy(open: &mut Connection, app: &[u8], id: &str) -> Result<bool, String> {
    open.del(&key_of(app, id))
}

/// The hash field a session key is kept under: its lifetime's prefix and the key.
fn field_of(key: &[u8], long: bool) -> Vec<u8> {
    let prefix = if long { LONG_FIELD } else { SHORT_FIELD };
    let mut field = Vec::with_capacity(prefix.len() + key.len());
    field.extend_from_slice(prefix);
    field.extend_from_slice(key);
    field
}

/// An integer as the decimal text a script reads with `tonumber`.
fn decimal(number: impl std::fmt::Display) -> Vec<u8> {
    number.to_string().into_bytes()
}

/// `rule:http-server/a-session-store-answers-five-operations`'s `issue` — a new session under `id`
/// holding `whole`, and whether it was written. One `EVAL` of [`ISSUE`]: `false` is a key that
/// already exists, which keeps what it has.
///
/// `size` is [`counted`]'s, which the caller has already held to the ceiling. The expiry is the
/// store's own, per `rule:http-server/session-expiry-belongs-to-the-store`. A lost reply is not
/// replayed past a request that reached the store: a replay would find the key its own first run
/// wrote, and answer `false` about it.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn issue(
    open: &mut Connection,
    app: &[u8],
    id: &str,
    whole: &Whole<'_>,
    size: usize,
) -> Result<bool, String> {
    let fields: Vec<Vec<u8>> = whole
        .entries
        .keys()
        .map(|key| field_of(key, whole.long.contains(key)))
        .collect();
    let clocks = whole.clocks;
    let (now, short, long) = (
        decimal(clocks.now),
        decimal(clocks.short_until),
        decimal(clocks.long_until),
    );
    let size = decimal(size);
    let mut argv: Vec<&[u8]> = Vec::with_capacity(5 + 2 * whole.entries.len());
    argv.push(&now);
    argv.push(&short);
    argv.push(&long);
    argv.push(&size);
    argv.push(whole.token.unwrap_or_default().as_bytes());
    for (field, value) in fields.iter().zip(whole.entries.values()) {
        argv.push(field);
        argv.push(value);
    }
    let answer = open.eval(ISSUE, &key_of(app, id), &argv, Replay::OnlyIfUnsent)?;
    match answer.as_slice() {
        [1] => Ok(true),
        [0] => Ok(false),
        other => Err(format!("the issue script answered {other:?}")),
    }
}

/// `rule:http-server/a-session-store-answers-five-operations`'s `save` — one request's `changes`
/// applied to the session under `id`, with its expiry refreshed, **only while the session still
/// exists**. One `EVAL` of [`SAVE`].
///
/// That is what keeps a destroyed session destroyed: a request that loaded the session before
/// another request destroyed it writes nothing at its end. A lost reply is replayed, because a second
/// run leaves the same hash.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn save(
    open: &mut Connection,
    app: &[u8],
    id: &str,
    changes: &Changes<'_>,
) -> Result<Saved, String> {
    let changed: Vec<Vec<u8>> = changes
        .changed
        .iter()
        .map(|(key, _, long)| field_of(key, *long))
        .collect();
    let clocks = changes.clocks;
    let (now, short, long) = (
        decimal(clocks.now),
        decimal(clocks.short_until),
        decimal(clocks.long_until),
    );
    let ceiling = decimal(SIZE_CEILING);
    let keys = decimal(KEY_CEILING);
    let count = decimal(changes.removed.len());
    let mut argv: Vec<&[u8]> = Vec::with_capacity(8 + changes.removed.len() + 2 * changed.len());
    argv.push(&now);
    argv.push(&short);
    argv.push(&long);
    argv.push(changes.token.unwrap_or_default().as_bytes());
    argv.push(match (changes.cleared_short, changes.cleared_long) {
        (true, true) => b"sl".as_slice(),
        (true, false) => b"s",
        (false, true) => b"l",
        (false, false) => b"",
    });
    argv.push(&ceiling);
    argv.push(&keys);
    argv.push(&count);
    argv.extend(changes.removed.iter().copied());
    for (field, (_, value, _)) in changed.iter().zip(&changes.changed) {
        argv.push(field);
        argv.push(value);
    }
    let answer = open.eval(SAVE, &key_of(app, id), &argv, Replay::Idempotent)?;
    match answer.as_slice() {
        [1, _] => Ok(Saved::Done),
        [0] => Ok(Saved::Gone),
        [-1, size] => Ok(Saved::TooLarge(*size)),
        other => Err(format!("the save script answered {other:?}")),
    }
}

/// `rule:http-server/a-session-store-answers-five-operations`'s `move` — the session under `old`
/// renamed to `new` with its expiry, as one `RENAMENX`.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn relocate(
    open: &mut Connection,
    app: &[u8],
    old: &str,
    new: &str,
) -> Result<Renamed, String> {
    open.rename_if_absent(&key_of(app, old), &key_of(app, new))
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
/// one [`load`] finds no record for is, and the open takes the same branch for both. What it saves
/// is the round trip, and what it prevents is an arbitrary presented string becoming a key this
/// core sends to a store it shares with a cache and a limiter.
pub(crate) fn issuable(presented: &str) -> bool {
    presented.len() == ID_CHARS
        && presented
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

/// `rule:core-classes/a-session-opens-on-its-first-use`: the session of this request, opened by
/// the first call of any `Core\Session` member, and by `Core\Csrf::token` and `Core\Passkey`,
/// which keep their state in it. `who` is the caller's whole name (`Core\Session::get`), which a
/// refusal names and `openWithIdentifier` names later. A session that is already open is used as
/// it is, so every call after the first costs one branch.
///
/// # Errors
///
/// As [`opened_under`].
pub(crate) fn first_use(ctx: &mut Ctx, who: &'static str) -> Result<(), Fault> {
    if ctx.session().is_some() {
        return Ok(());
    }
    opened_under(ctx, who, None)
}

/// The open itself: the session under `given`, an identifier the program read itself, or under the
/// identifier the `[session] cookie` presented where `given` is `None`.
///
/// **Absent is one answer with one response.**
/// `rule:http-server/a-session-store-answers-five-operations` gives `load` three ways to answer
/// absent — an identifier the store never issued, one it issued and has since expired, and one an
/// attacker minted — and the open does not distinguish them: each discards the presented
/// identifier and opens a fresh, empty session. The pre-store shape check ([`issuable`]) is not a
/// second validity question, for the reason the module doc gives.
///
/// **A store that cannot be reached throws.** [`on_shared`] classifies it as an `IOError` rather
/// than answering as though the record were absent, because absence means *issue a new session* —
/// so a store that is down would silently sign every user out and look like a forged identifier
/// while doing it.
///
/// **A fresh session has no identifier until something is kept in it.** The open writes no row
/// and sends no cookie for one; [`write`] draws the identifier and sends the cookie on the first
/// write that leaves the record non-empty, and the end-of-request write-back creates the row. So
/// the store holds sessions somebody wrote, and a flood of cookieless requests that only read
/// leaves nothing behind. `regenerate` on such a session does nothing, and `clearAll` reaches no
/// store.
///
/// **A program answering no request has no session to open**, and is told that before it is told
/// anything about the store. `rule:security/request-state-throws-in-an-isolate`: a session belongs
/// to the client a request arrived from, so a CLI program, a job worker, a test and a spawned
/// isolate inside a request have nothing to open and nothing to issue a cookie on. A deployment
/// that *has* a store would otherwise mint a record and a cookie for a caller that can never
/// present either, which is the ambient authority `rule:security/isolate-shares-nothing` keeps out
/// of a child. The refusal is [`crate::request::served`]'s, so the class and the sentence have one
/// home.
///
/// # Errors
///
/// A thrown `LogicError` where no request arrived; a thrown `RuntimeError` for the `db` store in a
/// process with no usable data folder, and for a shared store that is unconfigured or refused by
/// capability; a thrown `IOError` for a store that cannot be reached.
fn opened_under(ctx: &mut Ctx, who: &'static str, given: Option<String>) -> Result<(), Fault> {
    crate::request::served(ctx, who)?;
    let member = format!("{who}()");
    let store = opened(
        ctx,
        &member,
        ". A session has to be in a store that every server process can read",
    )?;

    // A presented identifier this store cannot have issued is absent, and is absent here rather
    // than one round trip later.
    let named = cookie(ctx);
    let by_hand = given.is_some();
    // The short cookie is read only where the identifier rode in a cookie: a session opened
    // under an identifier the program read itself binds its short-lived values to that alone.
    let short = if by_hand {
        None
    } else {
        let short_name = format!("{named}{SHORT_COOKIE_SUFFIX}");
        ctx.inbound().and_then(|inbound| {
            crate::request::cookie_of(inbound, short_name.as_bytes())
                .and_then(|value| std::str::from_utf8(value).ok())
                .map(str::to_owned)
        })
    };
    let presented = match given {
        Some(given) => Some(given),
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
    let short_name = who.rsplit_once("::").map_or(who, |(_, member)| member);
    let opened = match presented {
        Some(id) => fetched(&store, crate::cache::application(ctx), short_name, &id)?
            .map(|record| opened_record(ctx, id, record, short.as_deref(), by_hand)),
        None => None,
    };

    // A visitor with no record gets an empty session with no identifier yet. Nothing reaches
    // the store and no cookie is sent until [`write`] keeps something in it, so a request that
    // only reads costs the store nothing.
    let mut session = opened.unwrap_or_else(|| nvs_runtime::Session {
        by_hand,
        ..fresh_session()
    });
    session.opened_by = who;
    ctx.open_session(session);
    refreshed_long_clock(ctx, who)
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::openWithIdentifier(tainted string $id): void` — the one explicit open
    /// (`rule:core-classes/a-session-opens-on-its-first-use`): the session under an identifier the
    /// program read itself, such as a header. No cookie is read for it, and none is sent while it
    /// changes, because no browser is involved; its short-lived values are bound to the identifier
    /// alone.
    ///
    /// The parameter admits `tainted` text, because the strict-id rule
    /// (`rule:http-server/a-session-store-answers-five-operations`) makes it a lookup key and
    /// never an instruction: what the store did not issue is absent, whatever it was.
    ///
    /// # Errors
    ///
    /// A thrown `LogicError` once the session of this request is open, naming the member whose
    /// call opened it, because the program would otherwise read two sessions in one request. And
    /// [`opened_under`]'s.
    fn nvs_core_session_open_with_identifier(ctx, args: [1]) {
        const WHO: &str = r"Core\Session::openWithIdentifier";
        if let Some(open) = ctx.session() {
            let by = if open.opened_by.is_empty() { "the server" } else { open.opened_by };
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{WHO}(): the session of this request is already open. `{by}()` opened \
                     it. Call `openWithIdentifier()` before any other `{NAME}` method."
                ),
            ));
        }
        let id = crate::string_arg(&args[0], format_args!("{WHO}'s $id"))?.to_owned();
        opened_under(ctx, WHO, Some(id))?;
        Ok(Value::null())
    }
}

/// An empty session with no identifier and no row in the store: what the first use opens for a
/// visitor with no record, and what `regenerate` leaves when another request destroyed the session
/// first.
fn fresh_session() -> nvs_runtime::Session {
    nvs_runtime::Session::empty(send_at_end)
}

/// The session a loaded `record` opens as, with
/// `rule:http-server/a-session-value-is-short-lived-or-long-lived`'s visibility applied.
///
/// **A short-lived value is visible only while the short clock runs, and only to the browser that
/// holds the record's short token.** The presented short cookie is compared with the record's
/// token in constant time, so a response time says nothing about how much of a guess was right. A
/// session the program opened under an identifier it read itself (`by_hand`) has no browser, so
/// no token is checked for it. A value that is not visible is removed from the request's copy and
/// marked touched, so the write-back deletes it from the store: one write per browser restart.
///
/// **The short clock's half rule** (`rule:http-server/session-expiry-belongs-to-the-store`): a
/// record whose short clock has less than half of its time left counts as changed, so the
/// write-back refreshes it. It applies where short-lived values are visible, and to a record with
/// no long-lived value, whose expiry is the short clock. Neither needs a cookie, because the short
/// cookie and a session with no long-lived value carry no `Max-Age`.
fn opened_record(
    ctx: &Ctx,
    id: String,
    record: Record,
    presented: Option<&str>,
    by_hand: bool,
) -> nvs_runtime::Session {
    use subtle::ConstantTimeEq as _;

    let now = crate::queue::now_millis();
    let matched = match (record.token.as_deref(), presented) {
        (Some(held), Some(given)) => bool::from(held.as_bytes().ct_eq(given.as_bytes())),
        _ => false,
    };
    let bound = by_hand || matched;
    let visible = bound && now < record.short_until;
    let mut session = nvs_runtime::Session {
        id,
        entries: record.entries,
        long: record.long,
        short_token: record.token.filter(|_| matched && !by_hand),
        long_until: record.long_until,
        by_hand,
        stored: true,
        ..fresh_session()
    };
    if !visible {
        let hidden: Vec<Vec<u8>> = session
            .entries
            .keys()
            .filter(|key| !session.long.contains(*key))
            .cloned()
            .collect();
        for key in hidden {
            session.entries.remove(&key);
            session.touched.insert(key);
            session.dirty = true;
        }
    }
    if (visible || session.long.is_empty()) && past_half(record.short_until, short_lived(ctx), now)
    {
        session.dirty = true;
    }
    session
}

/// The long clock's half rule, at the first use: a session with a long-lived value whose long
/// clock has less than half of its time left gets a fresh clock and an identifier cookie with a
/// fresh `Max-Age`, and counts as changed so the write-back stores the clock.
///
/// The clock and the cookie move together, so the store never keeps a record the browser has
/// already forgotten. Where the head is closed the refresh is skipped, and a later request does
/// it. A session the program opened by hand sends no cookie, so its clock moves only with a
/// long-lived write.
///
/// # Errors
///
/// [`issue_cookie`]'s.
fn refreshed_long_clock(ctx: &mut Ctx, who: &str) -> Result<(), Fault> {
    let now = crate::queue::now_millis();
    let lifetime = long_lived(ctx);
    let Some(open) = ctx.session() else {
        return Ok(());
    };
    if open.by_hand
        || open.id.is_empty()
        || open.long.is_empty()
        || !past_half(open.long_until, lifetime, now)
        || !head_is_open(ctx)
    {
        return Ok(());
    }
    let id = open.id.clone();
    issue_cookie(ctx, who, &id, Some(lifetime))?;
    if let Some(open) = ctx.session_mut() {
        open.long_until = now.saturating_add(lifetime);
        open.dirty = true;
    }
    Ok(())
}

/// Whether this request may still send a header: not inside `Core\Html::later` and not after the
/// response. [`crate::response::head_open`] is the same question asked as a throw.
fn head_is_open(ctx: &Ctx) -> bool {
    !ctx.in_deferred_work() && !ctx.in_later_slot()
}

/// The `Max-Age` the identifier cookie carries for this request's session, in milliseconds: what
/// is left of the long clock while the session has a long-lived value, and none otherwise.
fn identifier_age(ctx: &Ctx) -> Option<i64> {
    let now = crate::queue::now_millis();
    ctx.session()
        .filter(|open| !open.long.is_empty())
        .map(|open| open.long_until.saturating_sub(now).max(1))
}

/// The `Set-Cookie` line carrying `id`, under `[session] cookie` and `[http.cookies]`' policy, with
/// a `Max-Age` of `max_age` milliseconds where it has one.
///
/// `who` is the member a refusal names, written whole (`Core\Session::set`), because
/// `Core\Csrf::token` draws an identifier too.
///
/// # Errors
///
/// As [`set_cookie`].
fn issue_cookie(ctx: &mut Ctx, who: &str, id: &str, max_age: Option<i64>) -> Result<(), Fault> {
    let name = cookie(ctx);
    set_cookie(ctx, who, &name, id, max_age)
}

/// The `Set-Cookie` line carrying the short `token`, named `[session] cookie` plus `-short`. It
/// never has a `Max-Age`, so the browser deletes it when it closes, and the short-lived values are
/// no longer visible after that.
///
/// # Errors
///
/// As [`set_cookie`].
fn issue_short_cookie(ctx: &mut Ctx, who: &str, token: &str) -> Result<(), Fault> {
    let name = format!("{}{SHORT_COOKIE_SUFFIX}", cookie(ctx));
    set_cookie(ctx, who, &name, token, None)
}

/// One of the session's two cookies, under `[http.cookies]`' policy.
///
/// # Errors
///
/// A thrown `RuntimeError` for a `[session] cookie` that is not a cookie name. That is an
/// operator's typo rather than anything a program did, and it is refused here because the value
/// reaches a header line: a name carrying a `;` or a newline would end the line and begin one
/// nobody wrote. The value needs no such check — it is 22 characters this core drew.
fn set_cookie(
    ctx: &mut Ctx,
    who: &str,
    name: &str,
    value: &str,
    max_age: Option<i64>,
) -> Result<(), Fault> {
    if !crate::response::nameable(name) {
        return Err(Fault::thrown(format!(
            "{who}(): `[session] cookie = \"{name}\"` is not a cookie name — a name is \
             a non-empty token, and this one reaches a `Set-Cookie` line"
        )));
    }

    let policy = crate::response::configured_cookies(ctx);
    let same_site = policy.same_site;
    let line = crate::response::Cookie {
        name,
        value,
        path: &policy.path,
        // No `Domain`, which is the narrower of that attribute's two meanings. A `Max-Age` only
        // where the long clock gives one: the store expires the record, and a cookie that outlived
        // it would present an identifier the store answers absent for.
        domain: None,
        max_age: max_age.map(|millis| millis.saturating_add(999) / 1000),
        secure: policy.secure,
        http_only: policy.http_only,
        same_site,
    }
    .line();
    ctx.append_header(crate::response::SET_COOKIE_HEADER, &line);
    Ok(())
}

/// The failure of a helper that finds no open session after [`first_use`] has run.
///
/// No program reaches it: every member opens the session first, and only `clearAll` replaces it,
/// with a fresh one. It is a fatal fault rather than a throw for that reason, as a broken invariant
/// of this module, and it names the member so the report points at the helper that broke it.
fn unstarted(member: &str) -> Fault {
    Fault::fatal(format!(
        "{NAME}::{member}(): no session is open after the first use opened it"
    ))
}

/// The `$value` argument of the sealed members, as the checker has already guaranteed it.
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

/// Seals `value` under the ring's newest key and writes it into the session under the sealed name.
///
/// **The additional data is the domain octet, the application and the key, and never the session
/// identifier** — `rule:http-server/a-session-holds-a-secret-only-sealed`: `regenerate` issues a
/// new id over the same record, and a value bound to the old one would stop opening at exactly the
/// moment a login hardens. The plaintext is the secret and nothing else: its lifetime is the
/// session entry's, `lifetime`, which the store keeps beside it and expires
/// (`rule:http-server/session-expiry-belongs-to-the-store`).
///
/// # Errors
///
/// [`crate::keyring`]'s `LogicError` for a ring whose newest entry is not a key, and a
/// `RuntimeError` for a sealed value this process cannot spare the buffer for, and [`write`]'s.
fn sealed_into(
    ctx: &mut Ctx,
    key: &str,
    value: &[u8],
    ring: &NvsArray,
    who: &str,
    lifetime: Lifetime,
) -> Result<(), Fault> {
    let member = who.rsplit_once("::").map_or(who, |(_, member)| member);
    let (slot, newest) = crate::keyring::newest(ring);
    let cipher = crate::keyring::cipher_at(&newest, slot, who)?;
    let aad = bound(SEAL_DOMAIN, application(ctx), key.as_bytes());
    let sealed = crate::crypto::seal_under(ctx, &cipher, &aad, value, who)?;
    let encoded = encoded(Value::bytes(NvsStr::new(&sealed)), member)?;
    write(
        ctx,
        member,
        &sealed_key(key.as_bytes()),
        Some(encoded),
        lifetime,
    )
}

/// The secret the session carries under `key`, or `None` for every way of not opening one.
///
/// **Every way of not opening is the same miss**, which is [`crate::cache`]'s answer at its own
/// sealed door and buys the same thing: a caller that could tell a rotated ring from a tampered
/// payload would learn something about the ring from a value it cannot read. Every key of the ring
/// is tried rather than the newest alone, so a value sealed before a rotation stays readable.
///
/// A session value in the sealed space that is not bytes is that same miss. A program can reach the
/// space — a session key is arbitrary text, and `setShortLived` will write whatever it is handed
/// under one —
/// and what it finds there is a value no ring opens. A plaintext that opens but is not UTF-8 is the
/// same miss too, because a `string` must be UTF-8.
///
/// # Errors
///
/// [`read_bytes`]'s, [`crate::keyring`]'s `LogicError` for an entry of the ring that is not a key,
/// and a `RuntimeError` for a plaintext this process cannot spare the buffer for.
fn opened_from(ctx: &Ctx, key: &str, ring: &NvsArray, who: &str) -> Result<Option<Value>, Fault> {
    let Some(sealed) = read_bytes(ctx, "getSecret", &sealed_key(key.as_bytes()))? else {
        return Ok(None);
    };

    let aad = bound(SEAL_DOMAIN, application(ctx), key.as_bytes());
    for (slot, entry) in crate::keyring::entries(ring) {
        let cipher = crate::keyring::cipher_at(&entry, slot, who)?;
        if let Some(plain) = crate::crypto::open_under(&cipher, &aad, &sealed, who)?
            && let Ok(text) = std::str::from_utf8(&plain)
        {
            return Ok(Some(Value::text(text)));
        }
    }
    Ok(None)
}

/// The value this request's session holds under `key`, decoded, with one reference that is the
/// caller's — or `None` where it holds none.
///
/// Every member below reads a value through this and none of them decodes
/// [`nvs_runtime::Session`]'s bytes directly, because the decode is where a value naming a class
/// this unit cannot resolve is refused and a second reader would be a second place that could
/// forget. Only `key`'s own value is decoded, so a `get` costs one value's walk, whatever else the
/// session holds.
///
/// # Errors
///
/// [`unstarted`] while this request has started no session. A thrown `ParseError` for a value
/// [`nvs_runtime::decode`] refuses, which is [`crate::cache`]'s `get` refusal over the same
/// resolver — the program's own class table — and for the same reason.
fn read(ctx: &Ctx, member: &str, key: &[u8]) -> Result<Option<Value>, Fault> {
    let Some(session) = ctx.session() else {
        return Err(unstarted(member));
    };
    let Some(bytes) = session.entries.get(key) else {
        return Ok(None);
    };
    nvs_runtime::decode(bytes, &ctx.receiving())
        .map(Some)
        .map_err(|why| Fault::thrown_as(ThrownClass::Parse, format!("{NAME}::{member}(): {why}")))
}

/// The `bytes` value this request's session holds under `key`, copied out — or `None` where it
/// holds none, or holds a value that is not `bytes`.
///
/// # Errors
///
/// As [`read`].
fn read_bytes(ctx: &Ctx, member: &str, key: &[u8]) -> Result<Option<Vec<u8>>, Fault> {
    let Some(found) = read(ctx, member, key)? else {
        return Ok(None);
    };
    let copied = found.as_bytes().map(<[u8]>::to_vec);
    #[expect(
        unsafe_code,
        reason = "`read` answers with one reference nothing else holds, and nothing reads \
                  `found` after this"
    )]
    unsafe {
        found.release();
    }
    Ok(copied)
}

/// `value` encoded by the byte carrier, as one session value. **Consumes one reference to
/// `value`.**
///
/// # Errors
///
/// A thrown `LogicError` for a value holding something the byte carrier refuses, which is
/// [`crate::cache`]'s `put` refusal over the same walk: the two carriers refuse the same graphs
/// because they share the walk that decides.
fn encoded(value: Value, member: &str) -> Result<Vec<u8>, Fault> {
    nvs_runtime::encode(value)
        .map_err(|why| Fault::thrown_as(ThrownClass::Logic, format!("{NAME}::{member}(): {why}")))
}

/// How long a value lives (`rule:http-server/a-session-value-is-short-lived-or-long-lived`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lifetime {
    /// Visible while the short clock runs and the browser keeps its short cookie.
    Short,
    /// Kept for the long clock, and visible with the identifier cookie alone.
    Long,
}

/// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`'s "change the copy in the
/// request's own heap": `key` set to the encoded `value` with `lifetime`, or removed for `None`,
/// and marked as owing the store a write.
///
/// **Only `key` is marked.** The write-back sends the keys this request touched and no other, so
/// a request that sets `cart` never writes back the `userId` it only read. A removal on a session
/// nobody has written yet is nothing: there is no key to remove and no row to send it to.
///
/// **The cookies follow the write.** The first write draws the identifier and sends its cookie,
/// with a `Max-Age` when the value is long-lived. The first short-lived write in a session with no
/// short token draws one and sends the short cookie. The identifier cookie gets a `Max-Age` when
/// the session gains its first long-lived value, and loses it when the last one goes. A long-lived
/// write past the half of the long clock refreshes the clock and the cookie together. Every cookie
/// a write needs is sent before the session changes, so a cookie that cannot be sent leaves the
/// session as it was.
///
/// # Errors
///
/// [`first_use`]'s, and where a cookie must be drawn,
/// [`crate::response::head_open`]'s and [`issue_cookie`]'s.
fn write(
    ctx: &mut Ctx,
    member: &str,
    key: &[u8],
    value: Option<Vec<u8>>,
    lifetime: Lifetime,
) -> Result<(), Fault> {
    let who = format!("{NAME}::{member}");
    let now = crate::queue::now_millis();
    let long_lived = long_lived(ctx);
    let long = lifetime == Lifetime::Long && value.is_some();
    let session = ctx.session().ok_or_else(|| unstarted(member))?;
    let drawn = session.id.is_empty();
    if drawn {
        if value.is_none() {
            return Ok(());
        }
        // The first write to a session opened with no identifier.
        crate::response::head_open(ctx, &who)?;
        let id = mint(ctx);
        issue_cookie(ctx, &who, &id, long.then_some(long_lived))?;
        let session = ctx.session_mut().ok_or_else(|| unstarted(member))?;
        session.id = id;
        if long {
            session.long_until = now.saturating_add(long_lived);
        }
    }

    let session = ctx.session().ok_or_else(|| unstarted(member))?;
    if value.is_some() && !long && !session.by_hand && session.short_token.is_none() {
        crate::response::head_open(ctx, &who)?;
        let token = mint(ctx);
        issue_short_cookie(ctx, &who, &token)?;
        ctx.session_mut()
            .ok_or_else(|| unstarted(member))?
            .short_token = Some(token);
    }

    let session = ctx.session().ok_or_else(|| unstarted(member))?;
    let had_long = !session.long.is_empty();
    let has_long = long || session.long.iter().any(|held| held.as_slice() != key);
    if !drawn && !session.by_hand {
        let refresh =
            has_long && (!had_long || (long && past_half(session.long_until, long_lived, now)));
        let ended = had_long && !has_long;
        if (refresh || ended) && head_is_open(ctx) {
            let id = session.id.clone();
            issue_cookie(ctx, &who, &id, refresh.then_some(long_lived))?;
            if refresh {
                ctx.session_mut()
                    .ok_or_else(|| unstarted(member))?
                    .long_until = now.saturating_add(long_lived);
            }
        } else if refresh && !had_long {
            // The store still needs a long clock where the cookie cannot carry one.
            ctx.session_mut()
                .ok_or_else(|| unstarted(member))?
                .long_until = now.saturating_add(long_lived);
        }
    } else if !drawn && long && !had_long {
        ctx.session_mut()
            .ok_or_else(|| unstarted(member))?
            .long_until = now.saturating_add(long_lived);
    }

    let session = ctx.session_mut().ok_or_else(|| unstarted(member))?;
    match value {
        Some(value) => session.entries.insert(key.to_vec(), value),
        None => session.entries.remove(key),
    };
    if long {
        session.long.insert(key.to_vec());
    } else {
        session.long.remove(key);
    }
    session.touched.insert(key.to_vec());
    session.dirty = true;
    Ok(())
}

/// The identifier the response sends for this request's session. `Core\Csrf::token` binds its
/// token to it.
///
/// **The session opens here on its first use, and a session with no identifier draws one**, and
/// sends its cookie, as its first [`write`] would. So the token and the cookie name the same
/// session, and the door accepts the token on the next request
/// (`rule:security/csrf-is-on-by-default`).
///
/// **Only a drawn identifier marks the record changed**, even while it is empty, so the write-back
/// creates a row under it and the next request's first use finds it. What it spends is one row per
/// new visitor who is shown a token, which `[session] short_lived` expires. A session that already
/// has an identifier is left as it was: the token is sealed over the id on each call and never
/// stored, so a page that renders a token for a loaded session sends no store write.
///
/// # Errors
///
/// [`first_use`]'s, and, for a session with no identifier yet, [`crate::response::head_open`]'s
/// and [`issue_cookie`]'s.
pub(crate) fn sent_id(ctx: &mut Ctx, who: &'static str) -> Result<String, Fault> {
    first_use(ctx, who)?;
    let fresh = ctx.session().ok_or_else(|| unstarted(who))?.id.is_empty();
    if fresh {
        // Drawn and sent before the session changes, so a cookie that cannot be sent leaves the
        // session as it was.
        crate::response::head_open(ctx, who)?;
        let id = mint(ctx);
        issue_cookie(ctx, who, &id, None)?;
        let session = ctx.session_mut().ok_or_else(|| unstarted(who))?;
        session.id = id;
        session.dirty = true;
    }
    Ok(ctx.session().ok_or_else(|| unstarted(who))?.id.clone())
}

/// Saves `value` under `key` as a short-lived value of this request's session, for a `Core` class
/// that keeps one visitor's state between two of their requests: `Core\Passkey`'s challenge. `who`
/// is the caller's whole name, as [`first_use`] takes it.
///
/// Only `key` is written back, as [`nvs_core_session_set_short_lived`] writes it.
///
/// # Errors
///
/// As [`nvs_core_session_set_short_lived`]: [`crate::response::head_open`]'s, [`first_use`]'s,
/// [`encoded`]'s and [`write`]'s.
pub(crate) fn put_bytes(
    ctx: &mut Ctx,
    who: &'static str,
    key: &str,
    value: &[u8],
) -> Result<(), Fault> {
    crate::response::head_open(ctx, who)?;
    first_use(ctx, who)?;
    let value = encoded(Value::bytes(NvsStr::new(value)), "setShortLived")?;
    write(
        ctx,
        "setShortLived",
        key.as_bytes(),
        Some(value),
        Lifetime::Short,
    )
}

/// The `bytes` saved under `key` in this request's session, taken out of it: [`put_bytes`]'s other
/// half, and what makes `Core\Passkey`'s challenge single-use.
///
/// The key is removed whatever it held, so a value that is not `bytes` is `None` and is gone
/// too. A key the session does not have is `None` and writes nothing, as
/// [`nvs_core_session_remove`] writes nothing.
///
/// # Errors
///
/// [`first_use`]'s, [`read`]'s and [`write`]'s.
pub(crate) fn take_bytes(
    ctx: &mut Ctx,
    who: &'static str,
    key: &str,
) -> Result<Option<Vec<u8>>, Fault> {
    first_use(ctx, who)?;
    let held = ctx
        .session()
        .ok_or_else(|| unstarted("remove"))?
        .entries
        .contains_key(key.as_bytes());
    if !held {
        return Ok(None);
    }
    let taken = read_bytes(ctx, "remove", key.as_bytes())?;
    write(ctx, "remove", key.as_bytes(), None, Lifetime::Short)?;
    Ok(taken)
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::get(string $key): mixed` — `rule:core-api/session-roster`'s read of a value
    /// of either lifetime, replacing `$_SESSION[$key]`.
    ///
    /// **A key the record does not hold is `null`**, exactly as `Core\Cache\Store::get` answers a
    /// miss: `mixed` already spells absent, so there is no second member to ask whether a key is
    /// there and no `?T` to make nullable. A session that stored a `null` and one that stored
    /// nothing are the same record, which is PHP's answer as well.
    ///
    /// **This member does not mark the record changed**, which is
    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`'s "writing only when the
    /// record changed" written as the thing it buys: a request that only reads its session makes
    /// one round trip and not two. Only the half rule of
    /// `rule:http-server/session-expiry-belongs-to-the-store` makes a read write, at the open.
    ///
    /// # Errors
    ///
    /// [`first_use`]'s, and as [`read`]: a thrown `ParseError` for a value naming a class this
    /// program cannot resolve. Only `$key`'s value is decoded.
    ///
    /// The last slot is the call site's id, which [`crate::registry::RECORDED_SITE_MEMBERS`] puts
    /// last, and [`crate::site::watched`] files the call's `session` event under it.
    fn nvs_core_session_get(ctx, args: [2]) {
        const WHO: &str = r"Core\Session::get";
        let site = args[1];
        let target = args[0].as_text();
        crate::site::watched(ctx, site, TraceKind::Session, WHO, target, |ctx| {
            first_use(ctx, WHO)?;
            let key = crate::string_arg(&args[0], format_args!("{WHO}'s $key"))?;
            // The decoded value's one reference is the answer's.
            Ok(read(ctx, "get", key.as_bytes())?.unwrap_or_else(Value::null))
        })
    }
}

/// `setShortLived` and `setLongLived`: `value` encoded onto the request's own copy of the session
/// under `key`, with `lifetime`. **Consumes nothing**: the argument slot keeps its reference, and
/// the encode takes one of its own.
///
/// **Nothing reaches the store here.** `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`
/// loads the session once and writes back the keys a request changed when it ends, so this encodes
/// the value alone and marks the key touched. Two requests that change different keys keep both;
/// two that change one key keep the last write, which that rule states rather than repairs.
///
/// **It throws where the head is closed, for every visitor**
/// (`rule:concurrency/deferred-work-cannot-write-the-response`), so a call inside
/// `Core\Html::later` fails in development the same way it fails in production.
///
/// # Errors
///
/// [`crate::response::head_open`]'s, [`first_use`]'s, [`encoded`]'s and [`write`]'s.
fn set_lived(
    ctx: &mut Ctx,
    who: &'static str,
    member: &str,
    key: &Value,
    value: Value,
    lifetime: Lifetime,
) -> Result<Value, Fault> {
    crate::response::head_open(ctx, who)?;
    first_use(ctx, who)?;
    let key = crate::string_arg(key, format_args!("{who}'s $key"))?;
    // The encode takes over one reference and the argument slot keeps its own, which is the same
    // split `Core\Cache\Store::put` makes before handing a value to the carrier.
    #[expect(
        unsafe_code,
        reason = "the argument slot holds a live reference for the length of the call, which is \
                  `Value::retain`'s whole obligation"
    )]
    unsafe {
        value.retain();
    }
    let value = encoded(value, member)?;
    write(ctx, member, key.as_bytes(), Some(value), lifetime).map(|()| Value::null())
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::setShortLived(string $key, mixed $value): void` — a value with the short
    /// lifetime (`rule:http-server/a-session-value-is-short-lived-or-long-lived`), visible while the
    /// browser keeps its short cookie and the short clock runs. As [`set_lived`].
    ///
    /// The last slot is the call site's id, as [`nvs_core_session_get`]'s is.
    fn nvs_core_session_set_short_lived(ctx, args: [3]) {
        const WHO: &str = r"Core\Session::setShortLived";
        let site = args[2];
        let target = args[0].as_text();
        crate::site::watched(ctx, site, TraceKind::Session, WHO, target, |ctx| {
            set_lived(ctx, WHO, "setShortLived", &args[0], args[1], Lifetime::Short)
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::setLongLived(string $key, mixed $value): void` — a value with the long
    /// lifetime, kept for `[session] long_lived` and visible with the identifier cookie alone. As
    /// [`set_lived`].
    ///
    /// The last slot is the call site's id, as [`nvs_core_session_get`]'s is.
    fn nvs_core_session_set_long_lived(ctx, args: [3]) {
        const WHO: &str = r"Core\Session::setLongLived";
        let site = args[2];
        let target = args[0].as_text();
        crate::site::watched(ctx, site, TraceKind::Session, WHO, target, |ctx| {
            set_lived(ctx, WHO, "setLongLived", &args[0], args[1], Lifetime::Long)
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::setShortLivedSecret(string $key, secret string $value, array<secret bytes>
    /// $keys): void` — `rule:http-server/a-session-holds-a-secret-only-sealed`'s door for a user's
    /// own secret, with the short lifetime: the access or refresh token a web application holds on
    /// their behalf while they are signed in.
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
    /// # Errors
    ///
    /// [`crate::response::head_open`]'s, [`first_use`]'s, [`sealed_into`]'s, and
    /// [`crate::keyring`]'s `LogicError` for a ring that is empty or holds something that is not a
    /// key.
    ///
    /// The last slot is the call site's id, as [`nvs_core_session_get`]'s is. Its event names no
    /// target, because the key is a secret's.
    fn nvs_core_session_set_short_lived_secret(ctx, args: [4]) {
        const WHO: &str = r"Core\Session::setShortLivedSecret";
        let site = args[3];
        crate::site::watched(ctx, site, TraceKind::Session, WHO, None, |ctx| {
            crate::response::head_open(ctx, WHO)?;
            first_use(ctx, WHO)?;
            let key = crate::string_arg(&args[0], format_args!("{WHO}'s $key"))?;
            let value = secret_at(&args[1], "setShortLivedSecret")?;
            let ring = crate::keyring::borrow(args, 2, WHO)?;
            sealed_into(ctx, key, value.as_bytes(), &ring, WHO, Lifetime::Short)?;
            Ok(Value::null())
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::setLongLivedSecret(string $key, secret string $value, array<secret bytes>
    /// $keys): void` — [`nvs_core_session_set_short_lived_secret`] with the long lifetime.
    ///
    /// **A long-lived secret opens with the identifier cookie alone**, after the browser closed,
    /// for as long as `[session] long_lived` runs. So a secret tied to a login belongs in the short
    /// lifetime, and the card says so.
    ///
    /// # Errors
    ///
    /// As [`nvs_core_session_set_short_lived_secret`].
    ///
    /// The last slot is the call site's id. Its event names no target.
    fn nvs_core_session_set_long_lived_secret(ctx, args: [4]) {
        const WHO: &str = r"Core\Session::setLongLivedSecret";
        let site = args[3];
        crate::site::watched(ctx, site, TraceKind::Session, WHO, None, |ctx| {
            crate::response::head_open(ctx, WHO)?;
            first_use(ctx, WHO)?;
            let key = crate::string_arg(&args[0], format_args!("{WHO}'s $key"))?;
            let value = secret_at(&args[1], "setLongLivedSecret")?;
            let ring = crate::keyring::borrow(args, 2, WHO)?;
            sealed_into(ctx, key, value.as_bytes(), &ring, WHO, Lifetime::Long)?;
            Ok(Value::null())
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::getSecret(string $key, array<secret bytes> $keys): ?secret string` — the
    /// sealed members' door in the other direction, for a secret of either lifetime.
    ///
    /// **A value that does not open is absent rather than an error**, and that is the whole of
    /// what a program sees of a rotated ring, a tampered record or a name nothing sealed under.
    /// The one thing that is not a miss is a ring that cannot key anything, which is the program's
    /// own bug and throws.
    ///
    /// **A secret survives `regenerate` and the clear of the other lifetime**, because what the
    /// value is bound to is the application and the key rather than the identifier — the rule's
    /// own paragraph on why, and the reason a login that hardens its session does not log the user
    /// out of the token it just stored.
    ///
    /// Reading marks nothing changed, as [`nvs_core_session_get`] does not, so a request that only
    /// reads its secrets still makes one round trip.
    ///
    /// # Errors
    ///
    /// [`first_use`]'s, as [`read`] for the sealed value alone, and [`crate::keyring`]'s
    /// `LogicError` for a ring that is empty or holds something that is not a key.
    ///
    /// The last slot is the call site's id, as [`nvs_core_session_get`]'s is. Its event names no
    /// target.
    fn nvs_core_session_get_secret(ctx, args: [3]) -> Value, owns NvsStr {
        const WHO: &str = r"Core\Session::getSecret";
        let site = args[2];
        crate::site::watched(ctx, site, TraceKind::Session, WHO, None, |ctx| {
            first_use(ctx, WHO)?;
            let key = crate::string_arg(&args[0], format_args!("{WHO}'s $key"))?;
            let ring = crate::keyring::borrow(args, 1, WHO)?;
            let answer = opened_from(ctx, key, &ring, WHO)?;
            Ok(answer.unwrap_or_else(Value::null))
        })
    } apply(result, _subject) {
        result
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::remove(string $key): void` — `rule:core-api/session-roster` and
    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`, replacing
    /// `unset($_SESSION[$key])`, for a value of either lifetime.
    ///
    /// **A key the record does not hold is not a refusal, and does not mark it changed either.**
    /// Nothing was written, so there is nothing for the write-back to send, and a `remove` that
    /// dirtied the record regardless would put a request that changed nothing back on the write
    /// path — which is the one thing "writing only when the record changed" is there to prevent.
    /// It is a write member all the same, so it throws where the head is closed.
    ///
    /// # Errors
    ///
    /// [`crate::response::head_open`]'s, [`first_use`]'s and [`write`]'s.
    ///
    /// The last slot is the call site's id, as [`nvs_core_session_get`]'s is.
    fn nvs_core_session_remove(ctx, args: [2]) {
        const WHO: &str = r"Core\Session::remove";
        let site = args[1];
        let target = args[0].as_text();
        crate::site::watched(ctx, site, TraceKind::Session, WHO, target, |ctx| {
            crate::response::head_open(ctx, WHO)?;
            first_use(ctx, WHO)?;
            let key = crate::string_arg(&args[0], format_args!("{WHO}'s $key"))?;
            let session = ctx.session().ok_or_else(|| unstarted("remove"))?;
            if !session.entries.contains_key(key.as_bytes()) {
                return Ok(Value::null());
            }
            write(ctx, "remove", key.as_bytes(), None, Lifetime::Short).map(|()| Value::null())
        })
    }
}

/// `clearShortLived` and `clearLongLived`: every value of `lifetime` deleted, and the session moved
/// to a new identifier (`rule:http-server/a-session-value-is-short-lived-or-long-lived`).
///
/// **The store deletes every value of that lifetime, also one this request did not see**: the
/// session's clear flag for that lifetime makes the write-back delete them all before it sends the
/// keys this request set afterwards. A value another request saved between this request's load and
/// the move would otherwise move with the session, and a logout would keep it. The values of the
/// other lifetime stay, under the new identifier.
///
/// **Then [`moved_to_fresh`]**, so an identifier somebody copied before the clear stops working.
/// A session nobody has written has no row and no values, and nothing happens.
///
/// # Errors
///
/// [`crate::response::head_open`]'s, [`first_use`]'s and [`moved_to_fresh`]'s.
fn cleared(
    ctx: &mut Ctx,
    who: &'static str,
    member: &str,
    lifetime: Lifetime,
) -> Result<Value, Fault> {
    crate::response::head_open(ctx, who)?;
    first_use(ctx, who)?;
    if emptied(ctx, member, lifetime)? {
        moved_to_fresh(ctx, who, member)?;
    }
    Ok(Value::null())
}

/// [`cleared`]'s change to the request's copy: every value of `lifetime` deleted, and the clear
/// flag set for the write-back. `false` for a session nobody has written, which has nothing to
/// clear and nothing to move.
///
/// # Errors
///
/// [`unstarted`] where no session is open.
fn emptied(ctx: &mut Ctx, member: &str, lifetime: Lifetime) -> Result<bool, Fault> {
    let open = ctx.session_mut().ok_or_else(|| unstarted(member))?;
    if open.id.is_empty() {
        return Ok(false);
    }
    let long = lifetime == Lifetime::Long;
    let gone: Vec<Vec<u8>> = open
        .entries
        .keys()
        .filter(|key| open.long.contains(*key) == long)
        .cloned()
        .collect();
    // A touched key stays touched with no value, so the write-back removes it under both
    // lifetimes, as it would any key this request removed.
    for key in gone {
        open.entries.remove(&key);
    }
    if long {
        open.long.clear();
        open.cleared_long = true;
    } else {
        open.cleared_short = true;
    }
    open.dirty = true;
    Ok(true)
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::clearShortLived(): void` — every short-lived value deleted, the session moved
    /// to a new identifier and a new short token drawn: the logout. As [`cleared`].
    fn nvs_core_session_clear_short_lived(ctx, _args: [0]) {
        cleared(ctx, r"Core\Session::clearShortLived", "clearShortLived", Lifetime::Short)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::clearLongLived(): void` — every long-lived value deleted and the session
    /// moved to a new identifier. The identifier cookie loses its `Max-Age`, so the browser keeps
    /// it only until it closes. As [`cleared`].
    fn nvs_core_session_clear_long_lived(ctx, _args: [0]) {
        cleared(ctx, r"Core\Session::clearLongLived", "clearLongLived", Lifetime::Long)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::regenerate(): void` — `rule:core-api/session-roster`'s move to a new
    /// identifier, replacing `session_regenerate_id`. Every value is kept, and a new short token is
    /// drawn. As [`moved_to_fresh`].
    ///
    /// **Immediately rather than at the end of the request** —
    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key` names the move and the
    /// delete as the two exceptions to its own write-back, because both of them are about an
    /// identifier rather than about a record, and an identifier that is only retired when the
    /// request ends is live for the length of the response.
    ///
    /// # Errors
    ///
    /// [`crate::response::head_open`]'s, [`first_use`]'s and [`moved_to_fresh`]'s.
    fn nvs_core_session_regenerate(ctx, _args: [0]) {
        const WHO: &str = r"Core\Session::regenerate";
        crate::response::head_open(ctx, WHO)?;
        first_use(ctx, WHO)?;
        moved_to_fresh(ctx, WHO, "regenerate")?;
        Ok(Value::null())
    }
}

/// The session moved to a new identifier, and a new short token drawn where it has one: what
/// `regenerate`, `clearShortLived` and `clearLongLived` share.
///
/// **One step: the store's `move`.** [`regenerated`] renames the stored session to a fresh
/// identifier, so every key the store holds is kept, also one another request saved after this
/// one loaded. It moves only a session that still exists. The changes this request has not sent
/// yet stay on the request, and the write-back sends them to the fresh identifier.
///
/// **A session nobody has written has no identifier**, and nothing happens: no client holds one
/// for it, and the first write draws a fresh one. **A session this request drew and has not
/// stored yet makes no store call.** A new identifier is drawn and sent, and the write-back creates
/// the row under it.
///
/// # Errors
///
/// A thrown `RuntimeError` for a shared store that is unconfigured or refused by capability, and
/// for a `[session] cookie` that is not a cookie name; a thrown `IOError` for a store that cannot
/// be reached; and [`regenerated`]'s.
fn moved_to_fresh(ctx: &mut Ctx, who: &str, member: &str) -> Result<(), Fault> {
    let open = ctx.session().ok_or_else(|| unstarted(member))?;
    if open.id.is_empty() {
        return Ok(());
    }
    let store = if open.stored {
        Some(opened(
            ctx,
            &format!("{who}()"),
            ", which is where this session's record already lives",
        )?)
    } else {
        None
    };
    moved_on(ctx, store.as_ref(), who, member)
}

/// [`moved_to_fresh`] once the store is known: `store` is where a stored session lives, and
/// `None` is a session this request drew and has not stored yet.
///
/// # Errors
///
/// [`regenerated`]'s, [`issue_cookie`]'s and [`redrawn_short_token`]'s.
fn moved_on(ctx: &mut Ctx, store: Option<&Store>, who: &str, member: &str) -> Result<(), Fault> {
    if let Some(store) = store {
        regenerated(ctx, store, who, member)?;
    } else {
        // This request drew the identifier and the store has no row under it, so nothing moves.
        // The new identifier replaces it, and the write-back creates the row.
        let fresh = mint(ctx);
        let age = identifier_age(ctx);
        issue_cookie(ctx, who, &fresh, age)?;
        if let Some(open) = ctx.session_mut() {
            open.id = fresh;
        }
    }
    redrawn_short_token(ctx, who)
}

/// A new short token for a session that has one, sent in the short cookie, so a short token
/// somebody copied before a move stops working with it
/// (`rule:http-server/a-session-value-is-short-lived-or-long-lived`). The session counts as
/// changed, so the write-back stores the token.
///
/// # Errors
///
/// [`issue_short_cookie`]'s.
fn redrawn_short_token(ctx: &mut Ctx, who: &str) -> Result<(), Fault> {
    let Some(open) = ctx.session() else {
        return Ok(());
    };
    if open.by_hand || open.id.is_empty() || open.short_token.is_none() {
        return Ok(());
    }
    let token = mint(ctx);
    issue_short_cookie(ctx, who, &token)?;
    if let Some(open) = ctx.session_mut() {
        open.short_token = Some(token);
        open.dirty = true;
    }
    Ok(())
}

/// The move of a stored session on `store`: the store's `move` renames it to a fresh identifier
/// in one step, and the cookie names the fresh one.
///
/// **The old one gone means another request deleted the session first.** Nothing moves, no
/// cookie is sent, and the request is left with a fresh, empty session with no identifier, as the
/// first use opens for an absent one. So a session that was signed out elsewhere never moves to a
/// new identifier.
///
/// The request's own keys, and what it has not sent yet, stay as they are: the store holds the
/// same session under the new identifier, and the write-back sends the rest there. What it spends:
/// one round trip, where an issue and a delete took two.
///
/// # Errors
///
/// The store's failure, as [`fetched`]'s; a thrown `RuntimeError` where the store already has a
/// session under the drawn identifier; and [`issue_cookie`]'s.
fn regenerated(ctx: &mut Ctx, store: &Store, who: &str, member: &str) -> Result<(), Fault> {
    let Some(open) = ctx.session() else {
        return Err(unstarted(member));
    };
    let retired = open.id.clone();
    let (by_hand, opened_by) = (open.by_hand, open.opened_by);
    let fresh = mint(ctx);
    let app = crate::cache::application(ctx);
    match moved(store, app, member, &retired, &fresh)? {
        Renamed::Done => {}
        Renamed::Taken => {
            return Err(Fault::thrown(format!(
                "{who}(): the store already has a session under the new identifier. Call \
                 `{member}()` again to draw another one."
            )));
        }
        Renamed::Gone => {
            ctx.open_session(nvs_runtime::Session {
                by_hand,
                opened_by,
                ..fresh_session()
            });
            return Ok(());
        }
    }
    // The identifier changes before the cookie is written, so a cookie that cannot be written
    // still leaves the write-back aimed at the session the store now holds.
    if let Some(open) = ctx.session_mut() {
        open.id.clone_from(&fresh);
    }
    let age = identifier_age(ctx);
    issue_cookie(ctx, who, &fresh, age)
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::clearAll(): void` — `rule:core-api/session-roster`'s delete, replacing
    /// `session_destroy`: the row deleted, both cookies sent with `Max-Age=0`, and the request
    /// left a fresh, empty session with no identifier.
    ///
    /// **The request goes on with an empty session**, so a later write in the same request draws a
    /// new identifier, as a first visit does. The values the request had are gone with the row,
    /// because the end-of-request write-back would otherwise put them straight back.
    ///
    /// **The store is told first**, so a store that cannot be reached throws with the session
    /// still open rather than reporting a sign-out that did not happen. Then both cookies expire,
    /// so the browser forgets an identifier that now names nothing. A session opened by
    /// `openWithIdentifier` has no cookie, and none is sent for it.
    ///
    /// # Errors
    ///
    /// [`crate::response::head_open`]'s and [`first_use`]'s; a thrown `RuntimeError` for a shared
    /// store that is unconfigured or refused by capability, and for a `[session] cookie` that is
    /// not a cookie name; a thrown `IOError` for a store that cannot be reached.
    fn nvs_core_session_clear_all(ctx, _args: [0]) {
        const WHO: &str = r"Core\Session::clearAll";
        crate::response::head_open(ctx, WHO)?;
        first_use(ctx, WHO)?;
        let open = ctx.session().ok_or_else(|| unstarted("clearAll"))?;
        let (held, stored, by_hand, opened_by) =
            (open.id.clone(), open.stored, open.by_hand, open.opened_by);

        // A session nobody stored has no row to delete.
        if !held.is_empty() && stored {
            let store = opened(ctx, &format!("{WHO}()"), ", which is where this session's record lives")?;
            forgot(&store, crate::cache::application(ctx), "clearAll", &held)?;
        }
        if !by_hand {
            let name = cookie(ctx);
            set_cookie(ctx, WHO, &name, "", Some(0))?;
            set_cookie(ctx, WHO, &format!("{name}{SHORT_COOKIE_SUFFIX}"), "", Some(0))?;
        }
        ctx.open_session(nvs_runtime::Session {
            by_hand,
            opened_by,
            ..fresh_session()
        });
        Ok(Value::null())
    }
}

/// How the write-back names itself in a failure: not as a member, because no member called it.
///
/// The request did, by ending. A message reading `Core\Session::save()` would name a member of a
/// class that has none, and the operator reading it is looking for the request rather than for a
/// line of the program.
const SENDER: &str = "Core\\Session's write-back at the end of the request";

/// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`'s write-back, as the function every session this module opens carries.
///
/// Installed on [`nvs_runtime::Session::write_back`] by the open and by `regenerate`, and reached
/// through [`Ctx::end_session`] at the end of the program that opened the record — the isolate an
/// HTTP request is, or `nvs run`'s root task. Only a record something changed gets here: the flag
/// is the whole of `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`'s "writing only
/// when the record changed", and that context method is where it is
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
/// is named after, and the session's last-write-wins already declines to repair a lost write.
fn send_at_end(ctx: &mut Ctx) {
    // No identifier is a session nobody wrote, which `write_back` never marks changed.
    if ctx.session().is_none_or(|open| open.id.is_empty()) {
        return;
    }
    let sent = opened(ctx, SENDER, ", which is where this session's record lives")
        .and_then(|store| sent_to(ctx, &store));
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

/// [`send_at_end`]'s write on `store`.
///
/// **A stored session is saved, and a session this request drew is issued.** A save sends only the
/// keys this request set or removed, and whether it called `clear`, in one atomic step, and changes
/// only a session that still exists. So when another request destroyed the session, or it expired,
/// nothing is written, this request's changes are dropped, and nothing is reported: that is what a
/// destroyed session means. An issue writes every key, because the store has none yet, and sets
/// [`nvs_runtime::Session::stored`], so a second end of the same program saves into the session it
/// made. Once the store has the changes they are no longer owed, so a second end sends only what
/// changed after the first.
///
/// # Errors
///
/// The store's failure, as [`fetched`]'s; a thrown `RuntimeError` where the store already has a
/// session under the identifier this request drew; and [`too_large`]'s, naming the size, where the
/// shared store would not keep the session.
fn sent_to(ctx: &mut Ctx, store: &Store) -> Result<(), Fault> {
    let now = crate::queue::now_millis();
    let (short_lived, long_lived) = (short_lived(ctx), long_lived(ctx));
    let app = crate::cache::application(ctx);
    // Nothing is copied out of the session: the store calls borrow it, and the context is changed
    // only after the borrow has ended.
    let Some(open) = ctx.session().filter(|open| !open.id.is_empty()) else {
        return Ok(());
    };
    // The short clock moves with every write-back. The long clock moves only where the identifier
    // cookie was sent with it, which a member did already, so it is sent as it is.
    let clocks = Clocks {
        now,
        short_until: now.saturating_add(short_lived),
        long_until: if open.long.is_empty() || open.long_until > 0 {
            open.long_until
        } else {
            now.saturating_add(long_lived)
        },
    };
    if open.stored {
        match saved(
            store,
            app,
            "the write-back",
            &open.id,
            &Changes::of(open, clocks),
        )? {
            Saved::Done => {}
            Saved::Gone => return Ok(()),
            Saved::TooLarge(size) => return Err(too_large("the write-back", size)),
        }
    } else if !issued(
        store,
        app,
        "the write-back",
        &open.id,
        &Whole {
            entries: &open.entries,
            long: &open.long,
            token: open.short_token.as_deref().filter(|_| !open.by_hand),
            clocks,
        },
    )? {
        return Err(Fault::thrown(format!(
            "{SENDER}: the store already has a session under the identifier this request drew"
        )));
    }
    if let Some(open) = ctx.session_mut() {
        open.stored = true;
        open.touched.clear();
        open.cleared_short = false;
        open.cleared_long = false;
    }
    Ok(())
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
pub(crate) fn address(symbol: &str) -> Option<nvs_runtime::Entry> {
    Some(nvs_runtime::Entry::borrowed(match symbol {
        "nvs_core_session_open_with_identifier" => {
            (nvs_core_session_open_with_identifier as *const ()).cast()
        }
        "nvs_core_session_get" => (nvs_core_session_get as *const ()).cast(),
        "nvs_core_session_set_short_lived" => {
            (nvs_core_session_set_short_lived as *const ()).cast()
        }
        "nvs_core_session_set_long_lived" => (nvs_core_session_set_long_lived as *const ()).cast(),
        "nvs_core_session_remove" => (nvs_core_session_remove as *const ()).cast(),
        "nvs_core_session_clear_short_lived" => {
            (nvs_core_session_clear_short_lived as *const ()).cast()
        }
        "nvs_core_session_clear_long_lived" => {
            (nvs_core_session_clear_long_lived as *const ()).cast()
        }
        "nvs_core_session_clear_all" => (nvs_core_session_clear_all as *const ()).cast(),
        "nvs_core_session_regenerate" => (nvs_core_session_regenerate as *const ()).cast(),
        "nvs_core_session_set_short_lived_secret" => {
            (nvs_core_session_set_short_lived_secret as *const ()).cast()
        }
        "nvs_core_session_set_long_lived_secret" => {
            (nvs_core_session_set_long_lived_secret as *const ()).cast()
        }
        "nvs_core_session_get_secret" => return Some(nvs_core_session_get_secret::entry()),
        _ => return None,
    }))
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap, HashSet};
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    use nvs_config::session::{BACKENDS, Backend};
    use nvs_runtime::{Ctx, NvsStr, Value};

    use super::{
        Changes, Clocks, Entries, Lifetime, LongKeys, PREFIX, SEAL_DOMAIN, SHORT_FIELD, Saved,
        Store, Whole, backend, cookie, counted, destroy, encoded, fetched, field_of, forgot, issue,
        issued, key_of, load, mint, opened, opened_from, read, read_bytes, regenerated, saved,
        sealed_into, sealed_key, sent_to, short_lived, sqlite, write,
    };
    use crate::cache::redis::Connection;
    use crate::cache::tests::memory_held;
    use crate::cache::{Dial, Target};

    /// The block that chooses the shared tier, which every case driving it through a member writes.
    const SHARED_BLOCK: &str = "[session]\nbackend = \"shared\"\n";

    /// The application of a context with no `[[app]]` block, which is every case's but one.
    const NO_APP: &[u8] = b"";

    /// The id the session two cores exchange lives under.
    const ID: &str = "PmA5tKz1QvR3sYbN";

    /// The session two cores exchange: two keys, each with bytes the store keeps without reading.
    fn record() -> Entries {
        Entries::from([
            (b"user".to_vec(), b"17".to_vec()),
            (b"csrf".to_vec(), b"aa".to_vec()),
        ])
    }

    /// No long-lived key.
    static SHORT_ONLY: LongKeys = LongKeys::new();

    /// The clocks every write a case makes directly sends: taken once per test process, with the
    /// shipped short clock and no long one, so a hash a case expects has the same `x`.
    fn clocks() -> Clocks {
        static TAKEN: std::sync::OnceLock<Clocks> = std::sync::OnceLock::new();
        *TAKEN.get_or_init(|| {
            let now = crate::queue::now_millis();
            Clocks {
                now,
                short_until: now
                    + i64::try_from(super::DEFAULT_SHORT_LIVED.as_millis()).expect("two hours"),
                long_until: 0,
            }
        })
    }

    /// `entries` as `issue` writes them: every key short-lived, no short token, and [`clocks`].
    fn whole(entries: &Entries) -> Whole<'_> {
        Whole {
            entries,
            long: &SHORT_ONLY,
            token: None,
            clocks: clocks(),
        }
    }

    /// The keys of the session `load` found, or absent.
    fn keys_of(record: Option<super::Record>) -> Option<Entries> {
        record.map(|record| record.entries)
    }

    /// One session's hash, as the fake shared store keeps it: each field and its value.
    type Hash = BTreeMap<Vec<u8>, Vec<u8>>;

    /// The hash [`super::ISSUE`] writes for [`whole`] of `entries`: `m` with the counted size, `x`
    /// with the short clock, and one `s:` field per key.
    fn hash_of(entries: &Entries) -> Hash {
        let mut hash: Hash = entries
            .iter()
            .map(|(key, value)| (field_of(key, false), value.clone()))
            .collect();
        hash.insert(b"m".to_vec(), counted(entries).to_string().into_bytes());
        hash.insert(b"x".to_vec(), clocks().short_until.to_string().into_bytes());
        hash
    }

    /// The session keys a hash holds, as [`load`] reads them: every `s:` and `l:` field, and none
    /// of the store's own.
    fn entries_in(hash: &Hash) -> Entries {
        hash.iter()
            .filter_map(|(field, value)| {
                field
                    .strip_prefix(SHORT_FIELD)
                    .or_else(|| field.strip_prefix(super::LONG_FIELD))
                    .map(|key| (key.to_vec(), value.clone()))
            })
            .collect()
    }

    /// A stored session under [`ID`] with `entries`, nothing changed yet, written back to the
    /// configured store.
    fn loaded(entries: Entries) -> nvs_runtime::Session {
        nvs_runtime::Session {
            id: ID.to_owned(),
            entries,
            stored: true,
            ..nvs_runtime::Session::empty(super::send_at_end)
        }
    }

    /// `value` written under `key` as `Core\Session::set` writes it. Consumes one reference.
    fn put(ctx: &mut Ctx, key: &[u8], value: Value) {
        let value = encoded(value, "set").expect("the value encodes");
        write(ctx, "set", key, Some(value), Lifetime::Short)
            .expect("a started session takes the value");
    }

    /// The integer the session holds under `key`, read as `Core\Session::get` reads it.
    fn int_at(ctx: &Ctx, key: &[u8]) -> Option<i64> {
        let value = read(ctx, "get", key).expect("the value decodes")?;
        let int = value.as_int();
        dropped(value);
        int
    }

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
    /// Dropping it stops the accept loop and joins every thread it started, so a test ends with
    /// no thread of its own still running. It closes this thread's `crate::cache` connection
    /// first, which otherwise stays open until the thread exits. A test declares it before its
    /// own connections, so those close first too.
    struct Serving {
        address: SocketAddr,
        stop: Arc<AtomicBool>,
        accepting: Option<thread::JoinHandle<()>>,
        /// The keys whose expiry has passed. The store still has them until a command names one,
        /// and then deletes it first, as a real store expires a key when it is read.
        lapsed: Arc<Mutex<HashSet<Vec<u8>>>>,
        /// How many `EVAL` commands the store received, which are its only writes: one per
        /// `issue` and one per `save`, written or not.
        writes: Arc<AtomicUsize>,
        /// How many commands of any kind the store received, reads included.
        commands: Arc<AtomicUsize>,
    }

    impl Serving {
        fn start(listener: TcpListener, held: Rows) -> Self {
            let address = listener.local_addr().expect("the port it took");
            let stop = Arc::new(AtomicBool::new(false));
            let stopping = Arc::clone(&stop);
            let lapsed = Arc::new(Mutex::new(HashSet::new()));
            let expired = Arc::clone(&lapsed);
            let writes = Arc::new(AtomicUsize::new(0));
            let counting = Arc::clone(&writes);
            let commands = Arc::new(AtomicUsize::new(0));
            let received = Arc::clone(&commands);
            let accepting = thread::spawn(move || {
                serving(&listener, &held, &expired, &counting, &received, &stopping);
            });
            Self {
                address,
                stop,
                accepting: Some(accepting),
                lapsed,
                writes,
                commands,
            }
        }
    }

    impl Drop for Serving {
        fn drop(&mut self) {
            crate::cache::tests::close_shared();
            self.stop.store(true, Ordering::SeqCst);
            // The accept loop is blocked in `accept`, so one connection wakes it to see `stop`.
            drop(TcpStream::connect(self.address));
            if let Some(accepting) = self.accepting.take() {
                let joined = accepting.join();
                if !thread::panicking() {
                    joined.expect("the store's threads do not panic");
                }
            }
        }
    }

    /// [`Serving`]'s accept loop, which joins each connection's thread once `stop` is set.
    ///
    /// `HGETALL`, `RENAMENX`, `DEL`, and `EVAL` of [`super::ISSUE`] or [`super::SAVE`] only,
    /// because those are the commands [`load`], [`super::relocate`], [`destroy`], [`issue`] and
    /// [`super::save`] send. Each script is run as the same steps in Rust, over the same hash.
    /// Any other command or script is a panic rather than a silent `+OK`, so one added upstream
    /// fails here instead of passing untested. A key in `lapsed` is deleted before any command
    /// that names it runs. Each `EVAL` adds one to `writes`, and every command adds one to
    /// `commands` before its reply is sent.
    fn serving(
        listener: &TcpListener,
        held: &Rows,
        lapsed: &Arc<Mutex<HashSet<Vec<u8>>>>,
        writes: &Arc<AtomicUsize>,
        commands: &Arc<AtomicUsize>,
        stop: &AtomicBool,
    ) {
        let mut connections = Vec::new();
        while let Ok((stream, _)) = listener.accept() {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let held = Arc::clone(held);
            let lapsed = Arc::clone(lapsed);
            let writes = Arc::clone(writes);
            let commands = Arc::clone(commands);
            let handler = thread::spawn(move || {
                let mut writing = stream.try_clone().expect("the write half");
                let mut reading = BufReader::new(stream);
                while let Some(parts) = command(&mut reading) {
                    commands.fetch_add(1, Ordering::SeqCst);
                    let mut store = held.lock().expect("the store");
                    {
                        let mut expired = lapsed.lock().expect("the lapsed keys");
                        for part in &parts[1..] {
                            if expired.remove(part) {
                                store.remove(part);
                            }
                        }
                    }
                    let reply = match parts[0].as_slice() {
                        b"HGETALL" => match store.get(&parts[1]) {
                            Some(hash) => {
                                let mut array = format!("*{}\r\n", hash.len() * 2).into_bytes();
                                for (field, value) in hash {
                                    for part in [field, value] {
                                        array.extend_from_slice(
                                            format!("${}\r\n", part.len()).as_bytes(),
                                        );
                                        array.extend_from_slice(part);
                                        array.extend_from_slice(b"\r\n");
                                    }
                                }
                                array
                            }
                            None => b"*0\r\n".to_vec(),
                        },
                        b"RENAMENX" => {
                            if !store.contains_key(&parts[1]) {
                                b"-ERR no such key\r\n".to_vec()
                            } else if store.contains_key(&parts[2]) {
                                b":0\r\n".to_vec()
                            } else {
                                let hash = store.remove(&parts[1]).expect("the old key");
                                store.insert(parts[2].clone(), hash);
                                b":1\r\n".to_vec()
                            }
                        }
                        b"DEL" => {
                            format!(":{}\r\n", usize::from(store.remove(&parts[1]).is_some()))
                                .into_bytes()
                        }
                        b"EVAL" => {
                            writes.fetch_add(1, Ordering::SeqCst);
                            assert_eq!(parts[2], b"1", "every script names one key");
                            let (key, argv) = (&parts[3], &parts[4..]);
                            assert!(
                                number(&argv[0]) > 0,
                                "a session is always written with an expiry"
                            );
                            let answer = if parts[1] == super::ISSUE.as_bytes() {
                                issued_in(&mut store, key, argv)
                            } else if parts[1] == super::SAVE.as_bytes() {
                                saved_in(&mut store, key, argv)
                            } else {
                                panic!(
                                    "the session store sends no script {:?}",
                                    String::from_utf8_lossy(&parts[1])
                                )
                            };
                            let mut array = format!("*{}\r\n", answer.len()).into_bytes();
                            for integer in answer {
                                array.extend_from_slice(format!(":{integer}\r\n").as_bytes());
                            }
                            array
                        }
                        other => panic!(
                            "the session store sends no {:?}",
                            String::from_utf8_lossy(other)
                        ),
                    };
                    drop(store);
                    writing.write_all(&reply).expect("the reply");
                }
            });
            connections.push(handler);
        }
        for handler in connections {
            handler
                .join()
                .expect("a connection's thread does not panic");
        }
    }

    /// An argument a script reads with `tonumber`.
    fn number(argument: &[u8]) -> i64 {
        std::str::from_utf8(argument)
            .ok()
            .and_then(|text| text.parse().ok())
            .unwrap_or_else(|| panic!("{argument:?} is a number"))
    }

    /// The bytes one field costs in the counted size, as [`super::SAVE`] counts it.
    fn cost(field: &[u8], value: &[u8]) -> i64 {
        i64::try_from(field.len() + value.len() + 32).expect("a small field")
    }

    /// [`super::ISSUE`]'s steps over `store`: `{0}` for a key that exists, and otherwise the hash
    /// written with `m`, the short clock `x`, the long clock `y` where it is set, the token `t`
    /// where there is one, and every pair, and `{1}`. The expiry is `lapsed`'s to model.
    fn issued_in(store: &mut HashMap<Vec<u8>, Hash>, key: &[u8], argv: &[Vec<u8>]) -> Vec<i64> {
        if store.contains_key(key) {
            return vec![0];
        }
        let mut hash = Hash::new();
        hash.insert(b"m".to_vec(), argv[3].clone());
        hash.insert(b"x".to_vec(), argv[1].clone());
        if number(&argv[2]) > 0 {
            hash.insert(b"y".to_vec(), argv[2].clone());
        }
        if !argv[4].is_empty() {
            hash.insert(b"t".to_vec(), argv[4].clone());
        }
        for pair in argv[5..].chunks(2) {
            hash.insert(pair[0].clone(), pair[1].clone());
        }
        store.insert(key.to_vec(), hash);
        vec![1]
    }

    /// Whether `field` is a session key's, under either lifetime's prefix, and not the store's own.
    fn is_key_field(field: &[u8]) -> bool {
        field.starts_with(b"s:") || field.starts_with(b"l:")
    }

    /// `bare` under both lifetimes' prefixes.
    fn both_fields(bare: &[u8]) -> [Vec<u8>; 2] {
        [field_of(bare, false), field_of(bare, true)]
    }

    /// [`super::SAVE`]'s steps over `store`: `{0}` for a gone key, `{-1, size}` past a ceiling, and
    /// otherwise the changes applied and `{1, size}`.
    fn saved_in(store: &mut HashMap<Vec<u8>, Hash>, key: &[u8], argv: &[Vec<u8>]) -> Vec<i64> {
        let Some(hash) = store.get_mut(key) else {
            return vec![0];
        };
        let clear = argv[4].clone();
        let dropped = |field: &[u8]| {
            (field.starts_with(b"s:") && clear.contains(&b's'))
                || (field.starts_with(b"l:") && clear.contains(&b'l'))
        };
        let first = 8 + usize::try_from(number(&argv[7])).expect("a count");
        let (removed, changed) = (&argv[8..first], &argv[first..]);
        assert!(
            changed.len().is_multiple_of(2),
            "each changed field has a value"
        );
        let mut size = hash
            .get(b"m".as_slice())
            .and_then(|held| std::str::from_utf8(held).ok()?.parse::<i64>().ok())
            .unwrap_or(0);
        let mut count = i64::try_from(hash.keys().filter(|field| is_key_field(field)).count())
            .expect("a count");
        for (field, value) in hash.iter().filter(|(field, _)| dropped(field)) {
            size -= cost(field, value);
            count -= 1;
        }
        let bare = removed
            .iter()
            .map(Vec::as_slice)
            .chain(changed.iter().step_by(2).map(|field| &field[2..]));
        for bare in bare {
            for field in both_fields(bare) {
                if let Some(old) = hash.get(&field).filter(|_| !dropped(&field)) {
                    size -= cost(&field, old);
                    count -= 1;
                }
            }
        }
        for pair in changed.chunks(2) {
            size += cost(&pair[0], &pair[1]);
            count += 1;
        }
        if size > number(&argv[5]) || count > number(&argv[6]) {
            return vec![-1, size];
        }
        hash.retain(|field, _| !dropped(field));
        for bare in removed {
            for field in both_fields(bare) {
                hash.remove(&field);
            }
        }
        for pair in changed.chunks(2) {
            for field in both_fields(&pair[0][2..]) {
                hash.remove(&field);
            }
            hash.insert(pair[0].clone(), pair[1].clone());
        }
        hash.insert(b"m".to_vec(), size.to_string().into_bytes());
        for (field, clock) in [(b"x", &argv[1]), (b"y", &argv[2])] {
            let held = hash.get(field.as_slice()).map_or(0, |held| number(held));
            if number(clock) > held {
                hash.insert(field.to_vec(), clock.clone());
            }
        }
        if !argv[3].is_empty() {
            hash.insert(b"t".to_vec(), argv[3].clone());
        }
        vec![1, size]
    }

    /// `rule:concurrency/the-memory-tier-cannot-hold-what-must-be-coherent`, as the roster and as the write.
    ///
    /// Three claims, because each alone passes something broken. The roster admits no memory tier —
    /// so no later reader can select one — and it is read from `nvs_config` rather than spelled
    /// here, so adding a variant there fails this. The word is refused by `Backend::of` rather than
    /// resolving to something. And a session actually written through [`issue`] is **absent from
    /// the process's map** afterwards: a module that wrote into the memory tier on some branch would
    /// satisfy the first two and fail this one, which is the failure the section is about.
    #[test]
    fn a_session_is_never_backed_by_the_memory_cache_tier() {
        assert!(
            BACKENDS.iter().all(|(word, _)| *word != "memory"),
            "`rule:concurrency/the-memory-tier-cannot-hold-what-must-be-coherent` is enforced by the roster having no memory entry: {BACKENDS:?}"
        );
        assert_eq!(Backend::of("memory"), None);

        let (listener, address) = listening();
        let held: Rows = Arc::new(Mutex::new(HashMap::new()));
        let store = Arc::clone(&held);
        let _serving = Serving::start(listener, store);

        let mut open = Connection::new(
            Dial::configured(Target::Tcp(address), None, None),
            Duration::from_secs(5),
        );
        let written = record();
        assert!(
            issue(&mut open, NO_APP, ID, &whole(&written), counted(&written))
                .expect("the session reaches the configured store")
        );

        assert_eq!(
            held.lock()
                .expect("the store")
                .get(&key_of(NO_APP, ID))
                .cloned(),
            Some(hash_of(&written)),
            "the session is in the shared store, with its counted size"
        );
        assert_eq!(
            memory_held(&key_of(NO_APP, ID)),
            None,
            "and nowhere in the memory tier `rule:concurrency/the-memory-tier-cannot-hold-what-must-be-coherent` refuses it"
        );
    }

    /// `rule:http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot`'s reason,
    /// asserted as where the record is rather than as a round trip.
    ///
    /// Two threads are two cores: `crate::cache`'s shared connection is `thread_local`, so a second
    /// thread is exactly what a request landing on another core sees. The session record crosses —
    /// saved on one, loaded on the other through a connection the first never touched. A round
    /// trip on one thread would pass for a store that is merely reachable; the second connection is
    /// what makes this a test of the store every core and every process reads.
    #[test]
    fn a_session_survives_a_request_landing_on_another_core() {
        let (listener, address) = listening();
        let held: Rows = Arc::new(Mutex::new(HashMap::new()));
        let _serving = Serving::start(listener, held);

        let wrote = thread::spawn(move || {
            let mut open = Connection::new(
                Dial::configured(Target::Tcp(address), None, None),
                Duration::from_secs(5),
            );
            let written = record();
            assert!(
                issue(&mut open, NO_APP, ID, &whole(&written), counted(&written))
                    .expect("the session is written")
            );
        });
        wrote.join().expect("the first core");

        let read = thread::spawn(move || {
            let mut open = Connection::new(
                Dial::configured(Target::Tcp(address), None, None),
                Duration::from_secs(5),
            );
            keys_of(load(&mut open, NO_APP, ID).expect("the store answers"))
        });
        let shared = read.join().expect("the second core");

        assert_eq!(
            shared,
            Some(record()),
            "the session the first core wrote is the session the second core reads"
        );
    }

    /// `rule:core-api/session-roster`'s read and write over one session, as
    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key` keeps it: what `set` put
    /// there, `get` reads back, and `remove` takes out again — with the session's own entries and
    /// touched keys asserted beside the answers.
    ///
    /// **Here rather than in a `.nvst` case**, because no conformance case can reach a session
    /// store and none of this needs one: the session is loaded once and every change until the end
    /// of the request happens in the request's own heap, so the members below never touch a socket.
    /// The corpus asks the *language* rule instead — that the first call opens the session.
    ///
    /// **The entries are asserted, not only the answers.** A member that kept a decoded array of
    /// its own on the side would satisfy every assertion about what `get` said and leave
    /// `nvs_runtime::Session::entries` empty, so the write-back at the end of the request would send
    /// nothing. And a removed key stays touched, so the write-back sends its removal.
    #[test]
    fn the_entries_carry_what_set_wrote_and_lose_what_remove_took() {
        let mut ctx = Ctx::buffered();
        ctx.open_session(loaded(Entries::new()));

        assert_eq!(int_at(&ctx, b"cart"), None, "an empty session has no keys");

        put(&mut ctx, b"cart", Value::int(17));
        let after = ctx.session().expect("the session is still open");
        assert!(
            after.dirty,
            "the end-of-request write-back is what the flag earns, and a `set` that left it clear would be a \
             change the store never hears about"
        );
        assert!(after.entries.contains_key(b"cart".as_slice()));
        assert!(
            after.touched.contains(b"cart".as_slice()),
            "the key is owed to the store"
        );
        assert_eq!(
            int_at(&ctx, b"cart"),
            Some(17),
            "what `set` wrote is what `get` reads, across the encode and the decode between them"
        );

        write(&mut ctx, "remove", b"cart", None, Lifetime::Short).expect("a held key is removed");
        let emptied = ctx.session().expect("the session is still open");
        assert!(emptied.entries.is_empty(), "the session has no keys left");
        assert!(
            emptied.touched.contains(b"cart".as_slice()),
            "and the removal is still owed to the store"
        );
    }

    /// `clearShortLived` deletes the short-lived keys and keeps the long-lived ones, marks the
    /// session cleared so the write-back deletes every short-lived key the store holds, and moves
    /// it to a new identifier. The session here was drawn by this request, so the move needs no
    /// store. A session with no identifier has no row, so clearing it changes nothing.
    ///
    /// A value starts as bytes no decode accepts, because a clear does not read the session, and a
    /// value this unit cannot decode is exactly what it recovers from. A program answering no
    /// request throws.
    // covers: Core\Session::clearShortLived
    #[test]
    fn clear_short_lived_keeps_long_lived_keys_and_moves_the_session() {
        let mut ctx = Ctx::buffered();
        assert!(
            nvs_runtime::call(super::nvs_core_session_clear_short_lived, &mut ctx, &[]).is_err(),
            "a program answering no request has no session to clear"
        );
        drop(ctx.take_pending());

        ctx.open_session(nvs_runtime::Session {
            stored: false,
            long: [b"consent".to_vec()].into(),
            ..loaded(Entries::from([
                (b"cart".to_vec(), b"no decode accepts this".to_vec()),
                (b"consent".to_vec(), int_bytes(1)),
            ]))
        });
        ctx.session_mut()
            .expect("the session is open")
            .touched
            .insert(b"cart".to_vec());
        nvs_runtime::call(super::nvs_core_session_clear_short_lived, &mut ctx, &[])
            .expect("a value this unit cannot decode is still clearable");

        let cleared = ctx.session().expect("the session is still open");
        assert_ne!(
            cleared.id, ID,
            "clearing moves the session to a new identifier"
        );
        assert_eq!(
            cleared.entries,
            Entries::from([(b"consent".to_vec(), int_bytes(1))]),
            "only the long-lived key is left"
        );
        assert!(
            cleared.touched.contains(b"cart".as_slice()),
            "a key the request set is owed as a removal"
        );
        assert!(
            cleared.cleared_short && !cleared.cleared_long,
            "the write-back deletes every short-lived key in the store, and no long-lived one"
        );
        assert!(
            cleared.dirty,
            "a cleared session is a change the store must hear about"
        );
        assert!(
            ctx.take_headers().iter().any(|header| header
                .value
                .starts_with(&format!("nvsid={};", cleared_id(&ctx)))),
            "the response sends the new identifier"
        );

        ctx.open_session(super::fresh_session());
        nvs_runtime::call(super::nvs_core_session_clear_short_lived, &mut ctx, &[])
            .expect("a session nobody wrote is clearable");
        let fresh = ctx.session().expect("the session is still open");
        assert!(
            !fresh.dirty && !fresh.cleared_short,
            "a session with no identifier has no row to clear, so it earns no write"
        );
    }

    /// The identifier of the session `ctx` has open.
    fn cleared_id(ctx: &Ctx) -> String {
        ctx.session().expect("the session is open").id.clone()
    }

    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`'s write-back, asserted **both
    /// ways**: the send a changed record earns, and the round
    /// trip a read-only request does not make.
    ///
    /// One direction alone passes something broken. A build that sent unconditionally would
    /// satisfy "a `set` reaches the store" while spending a write on every request that read its
    /// session — the cost that rule names when it says only a changed record is written — and a build
    /// that never sent would satisfy "a read-only request makes one round trip" perfectly.
    ///
    /// **The flag is earned through the real member path**, by the same [`write`] `set` calls,
    /// rather than written by hand: a `set` that stopped marking the record would pass a test that
    /// set `dirty` itself and lose every write in production.
    ///
    /// **The send is a test's own function pointer**, which is what the seam is for: whether the
    /// bytes reach a store is [`issue`]'s question and three cases above already ask it, while what
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
            write_back: counting,
            ..loaded(Entries::new())
        });

        // The read-only request, made of the one member that reads.
        assert_eq!(int_at(&ctx, b"cart"), None);
        ctx.end_session();
        assert_eq!(
            SENT.with(std::cell::Cell::get),
            0,
            "a request that only read its session makes one round trip, not two — the whole \
             reason for the flag"
        );

        put(&mut ctx, b"cart", Value::int(17));

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

    /// The fourth operation of `rule:http-server/a-session-store-answers-five-operations`: after
    /// `destroy` the store answers absent, which is the same answer
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
        let held: Rows = Arc::new(Mutex::new(HashMap::new()));
        let _serving = Serving::start(listener, held);

        let answered = thread::spawn(move || {
            let mut open = Connection::new(
                Dial::configured(Target::Tcp(address), None, None),
                Duration::from_secs(5),
            );
            let written = record();
            assert!(
                issue(&mut open, NO_APP, ID, &whole(&written), counted(&written))
                    .expect("the session is written")
            );
            let before = keys_of(load(&mut open, NO_APP, ID).expect("the store answers"));
            let first = destroy(&mut open, NO_APP, ID).expect("the record is forgotten");
            let after = keys_of(load(&mut open, NO_APP, ID).expect("the store answers"));
            let second = destroy(&mut open, NO_APP, ID)
                .expect("an id with no record under it is already forgotten");
            (before, after, first, second)
        });
        let (before, after, first, second) = answered.join().expect("the core");

        assert_eq!(before, Some(record()));
        assert_eq!(after, None);
        assert!(first, "the first destroy removed the record");
        assert!(!second, "and the second found nothing to remove");
    }

    thread_local! {
        /// The store [`to_the_case_store`] sends to: the one the running case built.
        static CASE_STORE: std::cell::RefCell<Option<Store>> =
            const { std::cell::RefCell::new(None) };
    }

    /// A write-back that sends to the case's own store, through the same [`sent_to`]
    /// [`super::send_at_end`] uses for the configured one.
    fn to_the_case_store(ctx: &mut Ctx) {
        let store = CASE_STORE
            .with_borrow(Clone::clone)
            .expect("the case set its store");
        sent_to(ctx, &store).expect("the write-back reaches the store");
    }

    /// What keeps one store of [`BACKENDS`] alive for a case.
    enum Kept {
        /// The fake shared store, and the map it keeps.
        Shared(Serving, Rows),
        /// The SQLite file, and the scratch folder that deletes it when the case ends.
        Db {
            path: std::path::PathBuf,
            _folder: nvs_repo::Scratch,
        },
    }

    impl Kept {
        /// How many rows the store holds, expired or not.
        fn rows(&self) -> usize {
            match self {
                Self::Shared(_, held) => held.lock().expect("the store").len(),
                Self::Db { path, .. } => usize::try_from(sqlite::rows(path)).expect("a count"),
            }
        }

        /// Makes the row under `id` expire now.
        fn lapse(&self, id: &str) {
            match self {
                Self::Shared(serving, _) => {
                    serving
                        .lapsed
                        .lock()
                        .expect("the lapsed keys")
                        .insert(key_of(NO_APP, id));
                }
                Self::Db { path, .. } => sqlite::lapse(path, NO_APP, id),
            }
        }
    }

    impl Drop for Kept {
        fn drop(&mut self) {
            if let Self::Db { .. } = self {
                sqlite::closed();
            }
        }
    }

    /// An empty store of the kind `backend` names, with `ctx` configured to reach it. The `match`
    /// has no wildcard arm, so a store added to [`BACKENDS`] does not compile here until the
    /// cases that loop over every store can build it.
    fn a_store(ctx: &mut Ctx, case: &str, backend: Backend) -> (Store, Kept) {
        match backend {
            Backend::Shared => {
                let (listener, address) = listening();
                let held = Arc::new(Mutex::new(HashMap::new()));
                let serving = Serving::start(listener, Arc::clone(&held));
                ctx.set_config(crate::tests::granting(&format!(
                    "{SHARED_BLOCK}[capabilities.cache]\nshared = true\n\
                     [cache.shared]\nurl = \"redis://{address}\"\n"
                )));
                let store = opened(ctx, "a case", "").expect("the shared store is configured");
                (store, Kept::Shared(serving, held))
            }
            Backend::Db => {
                let dir = nvs_repo::scratch(&format!("{case}-db"));
                let path = dir.join("session.sqlite");
                let kept = Kept::Db {
                    path: path.clone(),
                    _folder: dir,
                };
                (Store::File(path), kept)
            }
        }
    }

    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`: a request loads a stored
    /// session and changes it, another request destroys the session, and then the first request
    /// ends. On every store the end writes nothing, so the session stays destroyed.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn a_destroyed_session_is_not_written_back_on_any_store() {
        for (word, backend) in BACKENDS {
            let mut ctx = Ctx::buffered();
            let (store, kept) = a_store(&mut ctx, "session-destroyed-stays", *backend);
            assert!(
                issued(&store, NO_APP, "a case", ID, &whole(&Entries::new()))
                    .expect("the row is issued")
            );
            let entries =
                keys_of(fetched(&store, NO_APP, "a case", ID).expect("the store answers"))
                    .expect("the row the case issued");
            CASE_STORE.with_borrow_mut(|slot| *slot = Some(store.clone()));
            ctx.open_session(nvs_runtime::Session {
                write_back: to_the_case_store,
                ..loaded(entries)
            });
            put(&mut ctx, b"cart", Value::int(17));

            assert!(
                forgot(&store, NO_APP, "a case", ID).expect("another request destroys the session"),
                "{word}: the row was there to destroy"
            );
            ctx.end_session();
            CASE_STORE.with_borrow_mut(Option::take);

            assert_eq!(
                fetched(&store, NO_APP, "a case", ID).expect("the store answers"),
                None,
                "{word}: the destroyed session stays destroyed"
            );
            assert_eq!(kept.rows(), 0, "{word}: and the store holds no row");
        }
    }

    /// `rule:core-api/session-roster`'s `regenerate`, when another request destroyed the session
    /// first: on every store nothing moves to a new identifier, and the request is left with an
    /// empty session that has no identifier.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn a_regenerated_session_moves_only_a_row_that_still_exists() {
        for (word, backend) in BACKENDS {
            let mut ctx = Ctx::buffered();
            let (store, kept) = a_store(&mut ctx, "session-regenerate-gone", *backend);
            assert!(
                issued(&store, NO_APP, "a case", ID, &whole(&record())).expect("the row is issued")
            );
            ctx.open_session(loaded(record()));
            assert!(
                forgot(&store, NO_APP, "a case", ID).expect("another request destroys the session"),
                "{word}: the row was there to destroy"
            );

            regenerated(&mut ctx, &store, r"Core\Session::regenerate", "regenerate")
                .unwrap_or_else(|why| {
                    panic!("{word}: a session that is gone does not throw: {why:?}")
                });
            assert_eq!(kept.rows(), 0, "{word}: no row moved to a new identifier");
            let left = ctx.session().expect("the request keeps a session");
            assert!(left.id.is_empty(), "{word}: the session has no identifier");
            assert!(left.entries.is_empty(), "{word}: and no keys");
            assert!(!left.stored && !left.dirty, "{word}: and nothing to write");
            assert!(
                ctx.take_headers().is_empty(),
                "{word}: and no cookie names the identifier that was removed"
            );
        }
    }

    /// A request that loaded the session under [`ID`] from `store`, and whose write-back sends there
    /// once [`ended`] runs it.
    fn a_request(store: &Store) -> Ctx {
        let entries = keys_of(fetched(store, NO_APP, "a case", ID).expect("the store answers"))
            .expect("the session the case issued");
        let mut ctx = Ctx::buffered();
        ctx.open_session(nvs_runtime::Session {
            write_back: to_the_case_store,
            ..loaded(entries)
        });
        ctx
    }

    /// `ctx`'s request ended, with its write-back sent to `store`.
    fn ended(mut ctx: Ctx, store: &Store) {
        CASE_STORE.with_borrow_mut(|slot| *slot = Some(store.clone()));
        ctx.end_session();
        CASE_STORE.with_borrow_mut(Option::take);
    }

    /// The keys `store` keeps under `id`, or `None` for a session that is gone.
    fn stored_under(store: &Store, id: &str) -> Option<Entries> {
        keys_of(fetched(store, NO_APP, "a case", id).expect("the store answers"))
    }

    /// `n` encoded as `Core\Session::set` encodes it, as the store keeps it.
    fn int_bytes(n: i64) -> Vec<u8> {
        encoded(Value::int(n), "set").expect("an integer encodes")
    }

    /// An empty store of the kind `backend` names, holding [`record`] under [`ID`].
    fn a_store_with_record(case: &str, backend: Backend) -> (Store, Kept) {
        let mut ctx = Ctx::buffered();
        let (store, kept) = a_store(&mut ctx, case, backend);
        assert!(
            issued(&store, NO_APP, "a case", ID, &whole(&record())).expect("the row is issued")
        );
        (store, kept)
    }

    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`: two requests load one
    /// session, one sets `cart` and the other `role`, and both end. On every store the session
    /// keeps both keys, and the keys neither request touched.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn two_requests_that_change_different_keys_keep_both_on_any_store() {
        for (word, backend) in BACKENDS {
            let (store, _kept) = a_store_with_record("session-different-keys", *backend);
            let mut first = a_request(&store);
            let mut second = a_request(&store);
            put(&mut first, b"cart", Value::int(3));
            put(&mut second, b"role", Value::int(7));
            ended(first, &store);
            ended(second, &store);

            let mut want = record();
            want.insert(b"cart".to_vec(), int_bytes(3));
            want.insert(b"role".to_vec(), int_bytes(7));
            assert_eq!(
                stored_under(&store, ID),
                Some(want),
                "{word}: the session keeps both requests' keys"
            );
        }
    }

    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`: two requests set one
    /// key, and the one whose write-back runs last is the value kept, in either order.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn two_requests_that_change_one_key_keep_the_last_on_any_store() {
        for (word, backend) in BACKENDS {
            for first_ends_first in [true, false] {
                let (store, _kept) = a_store_with_record("session-one-key", *backend);
                let mut first = a_request(&store);
                let mut second = a_request(&store);
                put(&mut first, b"cart", Value::int(1));
                put(&mut second, b"cart", Value::int(2));
                let last = if first_ends_first {
                    ended(first, &store);
                    ended(second, &store);
                    2
                } else {
                    ended(second, &store);
                    ended(first, &store);
                    1
                };
                let kept = stored_under(&store, ID).expect("the session is kept");
                assert_eq!(
                    kept.get(b"cart".as_slice()),
                    Some(&int_bytes(last)),
                    "{word}: the last write-back's value is kept (first ends first: \
                     {first_ends_first})"
                );
            }
        }
    }

    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`: a request reads `user`
    /// and sets `cart`, and meanwhile another request sets `user`. The first request's end sends
    /// `cart` alone, so the newer `user` stays.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn a_key_a_request_only_read_is_not_written_back_on_any_store() {
        for (word, backend) in BACKENDS {
            let (store, _kept) = a_store_with_record("session-only-read", *backend);
            let mut first = a_request(&store);
            put(&mut first, b"user", Value::int(1));
            ended(first, &store);

            let mut reader = a_request(&store);
            let mut writer = a_request(&store);
            put(&mut writer, b"user", Value::int(42));
            ended(writer, &store);

            assert_eq!(
                int_at(&reader, b"user"),
                Some(1),
                "{word}: the reader reads the value it loaded"
            );
            put(&mut reader, b"cart", Value::int(5));
            assert_eq!(
                reader.session().expect("a session").touched,
                std::collections::BTreeSet::from([b"cart".to_vec()]),
                "{word}: reading marks nothing"
            );
            ended(reader, &store);

            let kept = stored_under(&store, ID).expect("the session is kept");
            assert_eq!(
                kept.get(b"user".as_slice()),
                Some(&int_bytes(42)),
                "{word}: the other request's newer `user` stays"
            );
            assert_eq!(kept.get(b"cart".as_slice()), Some(&int_bytes(5)), "{word}");
        }
    }

    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`: `clear` is a
    /// act on one lifetime. A request clears its short-lived values, and its end deletes every
    /// short-lived key the store holds, also `late`, which another request saved after this one
    /// loaded. The long-lived `consent` another request saved stays, and so does the key the
    /// request sets after the clear.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn a_short_clear_deletes_every_short_key_the_store_holds_when_the_request_ends() {
        for (word, backend) in BACKENDS {
            let (store, _kept) = a_store_with_record("session-clear", *backend);
            let mut clearing = a_request(&store);
            let mut other = a_request(&store);
            put(&mut other, b"late", Value::int(9));
            let consent = int_bytes(2);
            write(
                &mut other,
                "setLongLived",
                b"consent",
                Some(consent.clone()),
                Lifetime::Long,
            )
            .expect("a long-lived value is written");
            ended(other, &store);

            let open = clearing.session_mut().expect("the session is open");
            open.entries.clear();
            open.cleared_short = true;
            open.dirty = true;
            put(&mut clearing, b"after", Value::int(1));
            ended(clearing, &store);

            assert_eq!(
                stored_under(&store, ID),
                Some(Entries::from([
                    (b"after".to_vec(), int_bytes(1)),
                    (b"consent".to_vec(), consent),
                ])),
                "{word}: the long-lived key and the key set after the clear are left"
            );
        }
    }

    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`: a session with no keys
    /// still exists. A request removes both of its keys, and the store still has the session, so
    /// nobody is signed out.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn a_session_whose_last_key_is_removed_still_exists_on_any_store() {
        for (word, backend) in BACKENDS {
            let (store, kept) = a_store_with_record("session-last-key", *backend);
            let mut ctx = a_request(&store);
            write(&mut ctx, "remove", b"user", None, Lifetime::Short).expect("the key is removed");
            write(&mut ctx, "remove", b"csrf", None, Lifetime::Short).expect("the key is removed");
            ended(ctx, &store);

            assert_eq!(
                stored_under(&store, ID),
                Some(Entries::new()),
                "{word}: the session is still there, with no keys"
            );
            assert_eq!(kept.rows(), 1, "{word}: and the store holds its one row");
        }
    }

    /// `rule:core-api/session-roster`'s `regenerate`: a request loads a session, another request
    /// saves `role`, and then the first request regenerates and sets `cart`. On every store the new
    /// identifier holds every key, also `role`, and the old one names nothing.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn regenerate_moves_a_key_another_request_wrote_on_any_store() {
        for (word, backend) in BACKENDS {
            let (store, kept) = a_store_with_record("session-regenerate-moves", *backend);
            let mut regenerating = a_request(&store);
            let mut other = a_request(&store);
            put(&mut other, b"role", Value::int(7));
            ended(other, &store);

            regenerated(
                &mut regenerating,
                &store,
                r"Core\Session::regenerate",
                "regenerate",
            )
            .unwrap_or_else(|why| panic!("{word}: the session moves: {why:?}"));
            put(&mut regenerating, b"cart", Value::int(3));
            let fresh = regenerating.session().expect("a session").id.clone();
            assert_ne!(fresh, ID, "{word}: the identifier is new");
            ended(regenerating, &store);

            let mut want = record();
            want.insert(b"role".to_vec(), int_bytes(7));
            want.insert(b"cart".to_vec(), int_bytes(3));
            assert_eq!(
                stored_under(&store, &fresh),
                Some(want),
                "{word}: the new identifier holds every key"
            );
            assert_eq!(
                stored_under(&store, ID),
                None,
                "{word}: the old one names nothing"
            );
            assert_eq!(kept.rows(), 1, "{word}: and the store holds one row");
        }
    }

    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`: `get` decodes one key's
    /// value and `set` encodes one. The session holds a large value under `big` whose bytes cannot
    /// be decoded. `get` of `small` still reads, and `set` of `small` leaves `big`'s bytes as they
    /// were and marks only `small`. A `get` that decoded the whole session would throw here.
    #[test]
    fn get_and_set_decode_and_encode_only_their_own_key() {
        let mut big =
            encoded(Value::text(&"x".repeat(64 * 1024)), "set").expect("a string encodes");
        big.truncate(big.len() - 1);
        let mut ctx = Ctx::buffered();
        ctx.open_session(loaded(Entries::from([
            (b"big".to_vec(), big.clone()),
            (b"small".to_vec(), int_bytes(3)),
        ])));

        assert!(
            read(&ctx, "get", b"big").is_err(),
            "the cut value does not decode, so a read of it throws"
        );
        assert_eq!(
            int_at(&ctx, b"small"),
            Some(3),
            "a read of another key decodes that key alone"
        );

        put(&mut ctx, b"small", Value::int(4));
        let session = ctx.session().expect("a session");
        assert_eq!(
            session.entries.get(b"big".as_slice()),
            Some(&big),
            "a set leaves every other key's bytes as they were"
        );
        assert_eq!(
            session.touched,
            std::collections::BTreeSet::from([b"small".to_vec()]),
            "and marks only its own key"
        );
        assert_eq!(int_at(&ctx, b"small"), Some(4));
    }

    /// A connection to the real store `tests/db/compose.yaml` starts, and an application name no
    /// other run uses, with the session under [`ID`] deleted. [`None`] when no store answers, after
    /// printing the command that starts one: the shape of `crate::cache::redis`'s `lease_case`.
    // The one line a developer without the store reads, so a case that asserted nothing does not
    // read like one that passed.
    #[expect(
        clippy::print_stderr,
        reason = "the line names the command that starts the store"
    )]
    fn real_store(label: &str) -> Option<(Connection, Vec<u8>)> {
        use crate::cache::redis::tests::{REDIS, STORE_WAIT};

        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, REDIS));
        let mut open = Connection::new(
            Dial::configured(Target::Tcp(address), None, None),
            STORE_WAIT,
        );
        let app = format!("test-{}-{label}", std::process::id()).into_bytes();
        if let Err(why) = open.ensure().and_then(|()| destroy(&mut open, &app, ID)) {
            eprintln!(
                "the {label} session case asserted nothing: no store at {address} ({why}). \
                 `docker compose -f tests/db/compose.yaml up -d --wait redis` starts the one \
                 these cases are written against."
            );
            return None;
        }
        Some((open, app))
    }

    /// Every field of the hash under `app`'s session [`ID`], in field order, or `None`.
    fn real_hash(open: &mut Connection, app: &[u8]) -> Option<Hash> {
        open.hash_all(&key_of(app, ID))
            .expect("the store answers")
            .map(|fields| fields.into_iter().collect())
    }

    /// `rule:http-server/a-session-store-answers-five-operations`'s `save` on a real store: once
    /// another request destroyed the session, the write-back script answers gone and creates no
    /// hash.
    #[cfg_attr(miri, ignore = "a real store is a socket, which Miri cannot open")]
    #[test]
    fn the_write_back_script_writes_nothing_once_the_session_is_gone() {
        let Some((mut open, app)) = real_store("save-gone") else {
            return;
        };
        let size = super::counted(&record());
        assert!(issue(&mut open, &app, ID, &whole(&record()), size).expect("issued"));
        assert!(destroy(&mut open, &app, ID).expect("destroyed"));

        let value = int_bytes(3);
        let changes = Changes {
            changed: vec![(b"cart".as_slice(), value.as_slice(), false)],
            clocks: clocks(),
            ..Changes::default()
        };
        assert_eq!(
            super::save(&mut open, &app, ID, &changes).expect("the store answers"),
            Saved::Gone
        );
        assert_eq!(real_hash(&mut open, &app), None, "and nothing was written");
    }

    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`'s session with no keys,
    /// on a real store: a write-back that removes the last key keeps the hash with its `m` field,
    /// so `load` still finds the session.
    #[cfg_attr(miri, ignore = "a real store is a socket, which Miri cannot open")]
    #[test]
    fn the_write_back_script_keeps_a_session_with_no_keys() {
        let Some((mut open, app)) = real_store("save-empty") else {
            return;
        };
        let size = super::counted(&record());
        assert!(issue(&mut open, &app, ID, &whole(&record()), size).expect("issued"));

        let changes = Changes {
            removed: vec![b"user".as_slice(), b"csrf".as_slice()],
            ..Changes::default()
        };
        assert_eq!(
            super::save(&mut open, &app, ID, &changes).expect("the store answers"),
            Saved::Done
        );
        let hash = real_hash(&mut open, &app).expect("the hash is kept");
        assert_eq!(
            hash.get(b"m".as_slice()).map(Vec::as_slice),
            Some(b"0".as_slice()),
            "the hash keeps `m`, at size 0"
        );
        assert!(
            !hash.keys().any(|field| is_key_field(field)),
            "and no key's field: {hash:?}"
        );
        assert_eq!(
            keys_of(load(&mut open, &app, ID).expect("the store answers")),
            Some(Entries::new()),
            "and `load` finds a session with no keys"
        );
        destroy(&mut open, &app, ID).expect("the case cleans up");
    }

    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`'s reply ceiling, on a
    /// real store: the write-back script is given a ceiling of 200 bytes, and a value that takes
    /// the session past it answers `{-1, size}` and writes nothing. A value under it is written.
    #[cfg_attr(miri, ignore = "a real store is a socket, which Miri cannot open")]
    #[test]
    fn the_write_back_script_refuses_a_session_past_the_reply_ceiling() {
        let Some((mut open, app)) = real_store("save-ceiling") else {
            return;
        };
        let size = super::counted(&record());
        assert!(issue(&mut open, &app, ID, &whole(&record()), size).expect("issued"));
        let before = real_hash(&mut open, &app);

        let field = field_of(b"cart", false);
        let now = crate::queue::now_millis();
        let (now_text, short_text) = (super::decimal(now), super::decimal(now + 60_000));
        let saved_with = |open: &mut Connection, value: &[u8]| {
            let argv: [&[u8]; 10] = [
                &now_text,
                &short_text,
                b"0",
                b"",
                b"0",
                b"200",
                b"100",
                b"0",
                &field,
                value,
            ];
            open.eval(
                super::SAVE,
                &key_of(&app, ID),
                &argv,
                crate::cache::redis::Replay::Idempotent,
            )
            .expect("the store answers")
        };

        let large = vec![b'x'; 200];
        let past = i64::try_from(size + field.len() + large.len() + super::FRAMING).expect("small");
        assert_eq!(
            saved_with(&mut open, &large),
            vec![-1, past],
            "past the ceiling"
        );
        assert_eq!(
            real_hash(&mut open, &app),
            before,
            "and nothing was written"
        );

        let small = b"ok".as_slice();
        let within =
            i64::try_from(size + field.len() + small.len() + super::FRAMING).expect("small");
        assert_eq!(
            saved_with(&mut open, small),
            vec![1, within],
            "under it, written"
        );
        destroy(&mut open, &app, ID).expect("the case cleans up");
    }

    /// `rule:http-server/a-session-store-answers-five-operations`'s replay of `save`, on a real
    /// store: the write-back script run twice with the same changes leaves the same hash as once,
    /// with and without `clear`.
    #[cfg_attr(miri, ignore = "a real store is a socket, which Miri cannot open")]
    #[test]
    fn the_write_back_script_run_twice_leaves_the_same_hash() {
        for cleared in [false, true] {
            let Some((mut open, app)) = real_store("save-twice") else {
                return;
            };
            let size = super::counted(&record());
            assert!(issue(&mut open, &app, ID, &whole(&record()), size).expect("issued"));

            let value = int_bytes(3);
            let changes = Changes {
                cleared_short: cleared,
                cleared_long: cleared,
                removed: vec![b"user".as_slice()],
                changed: vec![(b"cart".as_slice(), value.as_slice(), false)],
                token: None,
                clocks: clocks(),
            };
            let save = |open: &mut Connection| {
                super::save(open, &app, ID, &changes).expect("the store answers")
            };
            assert_eq!(save(&mut open), Saved::Done);
            let once = real_hash(&mut open, &app);
            assert_eq!(save(&mut open), Saved::Done);
            assert_eq!(
                real_hash(&mut open, &app),
                once,
                "a second run leaves the same hash (clear: {cleared})"
            );

            let mut want = if cleared { Entries::new() } else { record() };
            want.remove(b"user".as_slice());
            want.insert(b"cart".to_vec(), value.clone());
            assert_eq!(
                keys_of(load(&mut open, &app, ID).expect("the store answers")),
                Some(want),
                "(clear: {cleared})"
            );
            destroy(&mut open, &app, ID).expect("the case cleans up");
        }
    }

    /// The map the fake shared store keeps: each session's hash, by key.
    type Rows = Arc<Mutex<HashMap<Vec<u8>, Hash>>>;

    /// A context configured with the fake shared store as its session store and a CSRF key, the
    /// server, and the map it keeps.
    fn tokened() -> (Ctx, Serving, Rows) {
        let (listener, address) = listening();
        let held = Arc::new(Mutex::new(HashMap::new()));
        let serving = Serving::start(listener, Arc::clone(&held));
        let mut ctx = Ctx::buffered();
        ctx.set_random_state(31);
        ctx.set_config(crate::tests::granting(&format!(
            "{SHARED_BLOCK}[capabilities.cache]\nshared = true\n\
             [cache.shared]\nurl = \"redis://{address}\"\n\
             [http]\ncsrf_key = \"BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc\"\n"
        )));
        opened(&ctx, "a case", "").expect("the shared store is configured");
        (ctx, serving, held)
    }

    /// `Core\Csrf::token` called the way a program calls it.
    fn token(ctx: &mut Ctx) {
        let token = nvs_runtime::call(crate::csrf::nvs_core_csrf_token, ctx, &[])
            .expect("a started session with a key gets a token");
        dropped(token);
    }

    /// `rule:security/csrf-is-on-by-default`: a token is sealed over the session's id and never
    /// stored, so two tokens for a loaded session send no store write. The same request with one
    /// `Core\Session::set` sends one.
    #[test]
    fn a_token_for_a_loaded_session_makes_no_store_write() {
        for changes in [false, true] {
            let (mut ctx, serving, held) = tokened();
            held.lock()
                .expect("the store")
                .insert(key_of(NO_APP, ID), hash_of(&Entries::new()));
            ctx.open_session(loaded(Entries::new()));
            token(&mut ctx);
            token(&mut ctx);
            if changes {
                let key = Value::text("cart");
                let value = Value::text("17");
                let site = Value::int(0);
                nvs_runtime::call(
                    super::nvs_core_session_set_short_lived,
                    &mut ctx,
                    &[key, value, site],
                )
                .expect("the value is saved");
                dropped(key);
                dropped(value);
            }
            ctx.end_session();
            assert_eq!(
                serving.writes.load(Ordering::SeqCst),
                usize::from(changes),
                "a token writes nothing, and a set writes once (set called: {changes})"
            );
        }
    }

    /// `rule:security/csrf-is-on-by-default`: a new visitor's first token draws the session's
    /// identifier and sends its cookie, and the request's end creates the row once under that
    /// identifier. A second token in the same request writes nothing more.
    #[test]
    fn a_token_for_a_new_visitor_creates_the_row_once() {
        let (mut ctx, serving, held) = tokened();
        ctx.open_session(super::fresh_session());
        token(&mut ctx);
        token(&mut ctx);
        let id = ctx.session().expect("the session is open").id.clone();
        assert!(
            super::issuable(&id),
            "the first token drew an identifier: {id:?}"
        );
        let headers = ctx.take_headers();
        assert!(
            headers
                .iter()
                .any(|header| header.value.contains(id.as_str())),
            "the cookie carries the drawn identifier: {headers:?}"
        );
        ctx.end_session();
        assert_eq!(
            serving.writes.load(Ordering::SeqCst),
            1,
            "one row is created"
        );
        assert!(
            held.lock()
                .expect("the store")
                .contains_key(&key_of(NO_APP, &id)),
            "the row is under the identifier the cookie carries"
        );
    }

    /// `ctx` under the one `[[app]]` block at `app`, with the store configuration it already had.
    fn under(ctx: &mut Ctx, app: &str) {
        let mut snapshot = ctx
            .config()
            .map(|config| nvs_config::Snapshot::clone(config.snapshot()))
            .unwrap_or_default();
        snapshot.blocks = vec![std::path::PathBuf::from(app)];
        ctx.set_config(Arc::new(snapshot));
    }

    /// `rule:security/a-session-id-the-store-did-not-issue-is-rejected`: on every store, a session
    /// one application wrote is absent for a second application that presents its id. The second
    /// application's own session under that id is a second row, and the first application still
    /// loads its own record.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn a_session_one_application_wrote_is_absent_in_another_on_every_store() {
        for (word, backend) in BACKENDS {
            let mut blog = Ctx::buffered();
            let (store, kept) = a_store(&mut blog, "session-per-application", *backend);
            under(&mut blog, "/srv/blog");
            CASE_STORE.with_borrow_mut(|slot| *slot = Some(store.clone()));
            blog.open_session(nvs_runtime::Session {
                dirty: true,
                stored: false,
                write_back: to_the_case_store,
                ..loaded(record())
            });
            blog.end_session();
            assert_eq!(
                kept.rows(),
                1,
                "{word}: the first application wrote its row"
            );

            let mut shop = Ctx::buffered();
            if let Some(config) = blog.config() {
                shop.set_config(Arc::clone(config.snapshot()));
            }
            under(&mut shop, "/srv/shop");
            let shop_app = crate::cache::application(&shop);
            assert_eq!(
                fetched(&store, shop_app, "start", ID).expect("the store answers"),
                None,
                "{word}: the second application finds no session under the id"
            );
            let taking = Changes {
                changed: vec![(b"user".as_slice(), b"taken".as_slice(), false)],
                clocks: clocks(),
                ..Changes::default()
            };
            assert_eq!(
                saved(&store, shop_app, "a case", ID, &taking).expect("the store answers"),
                Saved::Gone,
                "{word}: and cannot write over the first application's row"
            );
            let its_own = Entries::from([(b"user".to_vec(), b"its own".to_vec())]);
            assert!(
                issued(&store, shop_app, "a case", ID, &whole(&its_own))
                    .expect("the store answers"),
                "{word}: its own session under that id is a row of its own"
            );
            assert_eq!(kept.rows(), 2, "{word}: so the store holds two rows");

            let mut again = Ctx::buffered();
            if let Some(config) = blog.config() {
                again.set_config(Arc::clone(config.snapshot()));
            }
            assert_eq!(
                keys_of(
                    fetched(&store, crate::cache::application(&again), "start", ID)
                        .expect("the store answers")
                ),
                Some(record()),
                "{word}: the first application still loads its record"
            );
            CASE_STORE.with_borrow_mut(Option::take);
        }
    }

    /// `rule:http-server/a-session-store-answers-five-operations`: on every store, `save`
    /// writes nothing for an absent id or an expired row, and `issue` on an id that has a row
    /// fails and keeps that row's keys.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn only_issue_inserts_a_session_row() {
        let changes = Changes {
            changed: vec![(b"user".as_slice(), b"late".as_slice(), false)],
            clocks: clocks(),
            ..Changes::default()
        };
        for (word, backend) in BACKENDS {
            let mut ctx = Ctx::buffered();
            let (store, kept) = a_store(&mut ctx, "session-only-create", *backend);
            assert_eq!(
                saved(&store, NO_APP, "a case", ID, &changes).expect("the store answers"),
                Saved::Gone,
                "{word}: a save of an absent id writes nothing"
            );
            assert_eq!(kept.rows(), 0, "{word}: and the store holds no row");

            assert!(
                issued(&store, NO_APP, "a case", ID, &whole(&record())).expect("the store answers"),
                "{word}: an issue of an absent id inserts the row"
            );
            let another = Entries::from([(b"user".to_vec(), b"another".to_vec())]);
            assert!(
                !issued(&store, NO_APP, "a case", ID, &whole(&another)).expect("the store answers"),
                "{word}: an issue of a held id fails"
            );
            assert_eq!(
                keys_of(fetched(&store, NO_APP, "a case", ID).expect("the store answers")),
                Some(record()),
                "{word}: and the held keys stay"
            );

            kept.lapse(ID);
            assert_eq!(
                saved(&store, NO_APP, "a case", ID, &changes).expect("the store answers"),
                Saved::Gone,
                "{word}: a save of an expired row writes nothing"
            );
            assert_eq!(
                fetched(&store, NO_APP, "a case", ID).expect("the store answers"),
                None,
                "{word}: and the row stays absent"
            );
        }
    }

    /// `Core\Session::clearAll` as a program reaches it: the store is told first, and only then
    /// does the request forget.
    ///
    /// Both orders are asserted. With no store configured the member throws and the session is
    /// still open, holding the id and the record it had, so a sign-out that did not happen is
    /// never reported as one. With the store configured, over [`serving`], the entry is gone from
    /// the map both cores share, both cookies expire, and the request goes on with a fresh, empty
    /// session with no identifier.
    // covers: Core\Session::clearAll
    #[test]
    fn clear_all_forgets_the_record_before_it_empties_the_session() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(SHARED_BLOCK));
        assert!(
            nvs_runtime::call(super::nvs_core_session_clear_all, &mut ctx, &[]).is_err(),
            "a program answering no request has no session to delete"
        );
        drop(ctx.take_pending());

        ctx.open_session(loaded(record()));
        assert!(
            nvs_runtime::call(super::nvs_core_session_clear_all, &mut ctx, &[]).is_err(),
            "no `[cache.shared] url`, so no store heard the sign-out"
        );
        drop(ctx.take_pending());
        let kept = ctx
            .session()
            .expect("a refused clear leaves the session open");
        assert_eq!(kept.id, ID);
        assert_eq!(kept.entries, record());

        let (listener, address) = listening();
        let held: Rows = Arc::new(Mutex::new(HashMap::new()));
        held.lock()
            .expect("the store")
            .insert(key_of(NO_APP, ID), hash_of(&record()));
        let store = Arc::clone(&held);
        let _serving = Serving::start(listener, store);
        ctx.set_config(crate::tests::granting(&format!(
            "{SHARED_BLOCK}[capabilities.cache]\nshared = true\n\
             [cache.shared]\nurl = \"redis://{address}\"\n"
        )));

        nvs_runtime::call(super::nvs_core_session_clear_all, &mut ctx, &[])
            .expect("a configured store forgets the record");
        assert!(
            !held
                .lock()
                .expect("the store")
                .contains_key(&key_of(NO_APP, ID)),
            "the entry is gone from the store"
        );
        let left = ctx.session().expect("the request keeps a fresh session");
        assert!(
            left.id.is_empty() && left.entries.is_empty() && !left.stored,
            "the session has no identifier, no keys and no row"
        );
        let headers = ctx.take_headers();
        for name in ["nvsid=", "nvsid-short="] {
            assert!(
                headers
                    .iter()
                    .any(|header| header.value.starts_with(name)
                        && header.value.contains("Max-Age=0")),
                "{headers:?} expires {name}"
            );
        }
    }

    /// `Core\Session::regenerate` as a program reaches it: the session moves to a new identifier
    /// in the store and the old entry is forgotten, before the request's own session changes.
    ///
    /// With no store configured the member throws and the request keeps the id and keys it had,
    /// so a login that did not harden is never reported as one. Over [`serving`] the store holds
    /// the session under the new id alone, and the request holds the same keys, not dirty, because
    /// the move changed no key.
    // covers: Core\Session::regenerate
    #[test]
    fn regenerate_moves_the_record_to_a_new_id_and_forgets_the_old_one() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(SHARED_BLOCK));
        assert!(
            nvs_runtime::call(super::nvs_core_session_regenerate, &mut ctx, &[]).is_err(),
            "a program answering no request has no session to move"
        );
        drop(ctx.take_pending());

        ctx.open_session(loaded(record()));
        assert!(
            nvs_runtime::call(super::nvs_core_session_regenerate, &mut ctx, &[]).is_err(),
            "no `[cache.shared] url`, so no store can take the record"
        );
        drop(ctx.take_pending());
        let kept = ctx
            .session()
            .expect("a refused regenerate leaves the session open");
        assert_eq!(kept.id, ID);
        assert_eq!(kept.entries, record());

        let (listener, address) = listening();
        let held: Rows = Arc::new(Mutex::new(HashMap::new()));
        held.lock()
            .expect("the store")
            .insert(key_of(NO_APP, ID), hash_of(&record()));
        let store = Arc::clone(&held);
        let _serving = Serving::start(listener, store);
        ctx.set_config(crate::tests::granting(&format!(
            "{SHARED_BLOCK}[capabilities.cache]\nshared = true\n\
             [cache.shared]\nurl = \"redis://{address}\"\n"
        )));

        nvs_runtime::call(super::nvs_core_session_regenerate, &mut ctx, &[])
            .expect("a configured store takes the record under a new id");
        let moved = ctx.session().expect("the request keeps a session");
        assert_ne!(moved.id, ID, "the identifier is new");
        assert_eq!(moved.entries, record(), "and the keys are the ones it had");
        assert!(
            !moved.dirty,
            "the move changed no key, so nothing is left to write"
        );

        let entries = held.lock().expect("the store");
        assert!(
            !entries.contains_key(&key_of(NO_APP, ID)),
            "the old entry is gone from the store"
        );
        assert_eq!(
            entries.get(&key_of(NO_APP, &moved.id)).map(entries_in),
            Some(record()),
            "and the keys are under the new id"
        );
    }

    /// The first use as a program reaches it: an identifier the store holds opens that record and
    /// writes no cookie, and one it does not hold opens a fresh, empty record with no identifier
    /// yet: nothing is written to the store and no `Set-Cookie` is sent.
    ///
    /// Both refusals come first, and neither leaves a session open: a context answering no
    /// request, and a request whose tree chose the shared tier and gave it no address.
    /// `openWithIdentifier` after the first use throws naming the member that opened the session,
    /// and keeps that session. The held identifier arrives in the `nvsid` cookie and the forged one
    /// through `openWithIdentifier`, so both readings are driven.
    // covers: Core\Session::openWithIdentifier
    #[test]
    fn the_first_use_opens_a_held_record_and_a_fresh_one_for_an_id_the_store_does_not_hold() {
        const HELD: &str = "PmA5tKz1QvR3sYbNcW8xLq";
        const FORGED: &str = "AAAAAAAAAAAAAAAAAAAAAA";
        const TOKEN: &str = "Tk4w9QbL2sYcXr7uVn1pMa";
        const WHO: &str = r"Core\Session::get";

        let mut ctx = Ctx::buffered();
        assert!(
            super::first_use(&mut ctx, WHO).is_err(),
            "a program answering no request has no session to open"
        );
        assert!(ctx.session().is_none());

        let mut inbound = nvs_runtime::Inbound::new("GET", "/", "");
        inbound.push_header(
            "cookie",
            format!("nvsid={HELD}; nvsid-short={TOKEN}").as_bytes(),
        );
        ctx.set_inbound(inbound);
        ctx.set_config(crate::tests::granting(SHARED_BLOCK));
        assert!(
            super::first_use(&mut ctx, WHO).is_err(),
            "the shared tier with no `[cache.shared] url`, so there is no store to open a record in"
        );
        assert!(ctx.session().is_none());

        let (listener, address) = listening();
        let held: Rows = Arc::new(Mutex::new(HashMap::new()));
        // The record's short-lived keys are visible only to the browser that holds its token,
        // while the short clock runs. Two hours left is the whole default clock, so no refresh.
        let mut hash = hash_of(&record());
        hash.insert(b"t".to_vec(), TOKEN.as_bytes().to_vec());
        let short_until = crate::queue::now_millis() + 7_200_000;
        hash.insert(b"x".to_vec(), short_until.to_string().into_bytes());
        held.lock()
            .expect("the store")
            .insert(key_of(NO_APP, HELD), hash);
        let store = Arc::clone(&held);
        let _serving = Serving::start(listener, store);
        let config = crate::tests::granting(&format!(
            "[capabilities.cache]\nshared = true\n[session]\nbackend = \"shared\"\n\
             [cache.shared]\nurl = \"redis://{address}\"\n"
        ));
        ctx.set_config(Arc::clone(&config));

        super::first_use(&mut ctx, WHO).expect("the cookie names a record the store holds");
        let opened = ctx.session().expect("the request has a session");
        assert_eq!(opened.id, HELD, "the presented identifier is kept");
        assert_eq!(opened.entries, record(), "and its keys are the stored ones");
        assert!(opened.stored, "the session is the stored one");
        assert!(!opened.dirty, "opening a record is not a change");
        assert!(
            ctx.take_headers().is_empty(),
            "the client already has this identifier, so no cookie is sent"
        );

        let late = [Value::text(FORGED)];
        let answer = nvs_runtime::call(
            super::nvs_core_session_open_with_identifier,
            &mut ctx,
            &late,
        );
        late.into_iter().for_each(dropped);
        assert!(
            answer.is_err(),
            "`openWithIdentifier` after the first use throws"
        );
        let thrown = ctx
            .take_pending()
            .expect("the throw is pending")
            .into_owned();
        assert!(
            thrown.contains(WHO),
            "{thrown} names the member that opened the session"
        );
        assert_eq!(
            ctx.session().expect("and the first session stays open").id,
            HELD
        );

        let mut fresh = Ctx::buffered();
        fresh.set_inbound(nvs_runtime::Inbound::new("GET", "/", ""));
        fresh.set_config(config);
        let forged = [Value::text(FORGED)];
        let answer = nvs_runtime::call(
            super::nvs_core_session_open_with_identifier,
            &mut fresh,
            &forged,
        );
        forged.into_iter().for_each(dropped);
        answer.expect("an identifier the store does not hold opens a fresh session");
        let issued = fresh.session().expect("the request has a session");
        assert!(
            issued.id.is_empty(),
            "the forged identifier is not used, and none is drawn before the first write"
        );
        assert!(issued.entries.is_empty(), "the fresh session has no keys");
        assert!(
            fresh.take_headers().is_empty(),
            "no cookie is sent for a session nobody wrote"
        );

        let entries = held.lock().expect("the store");
        assert_eq!(entries.len(), 1, "nothing is written for the fresh session");
        assert!(
            !entries.contains_key(&key_of(NO_APP, FORGED)),
            "and nothing is written under the forged identifier"
        );
    }

    /// Attack: 20,000 requests with no cookie each call `Core\Session::get` and only read.
    /// None of them writes a row or sends a cookie, so the store grows with sessions somebody
    /// wrote, never with requests served. The first write then sends one cookie, and the
    /// write-back stores the record under the identifier that cookie carries. Under Miri there
    /// are 20 requests, because each one is a full session start and 200 outlast nextest's
    /// five minutes.
    #[test]
    fn a_session_nobody_wrote_leaves_no_row_and_sends_no_cookie() {
        let (listener, address) = listening();
        let held: Rows = Arc::new(Mutex::new(HashMap::new()));
        let _serving = Serving::start(listener, Arc::clone(&held));
        let config = crate::tests::granting(&format!(
            "[capabilities.cache]\nshared = true\n[session]\nbackend = \"shared\"\n\
             [cache.shared]\nurl = \"redis://{address}\"\n"
        ));
        let request = || {
            let mut ctx = Ctx::buffered();
            ctx.set_inbound(nvs_runtime::Inbound::new("GET", "/", ""));
            ctx.set_config(Arc::clone(&config));
            ctx
        };

        let requests = if cfg!(miri) { 20 } else { 20_000 };
        for _ in 0..requests {
            let mut ctx = request();
            let read = [Value::text("user"), Value::int(0)];
            let answer = nvs_runtime::call(super::nvs_core_session_get, &mut ctx, &read);
            read.into_iter().for_each(dropped);
            dropped(answer.expect("an empty session reads"));
            ctx.end_session();
            assert!(
                ctx.take_headers().is_empty(),
                "a session nobody wrote sent a cookie"
            );
        }
        assert!(
            held.lock().expect("the store").is_empty(),
            "a session nobody wrote left a row"
        );

        let mut ctx = request();
        let write = [Value::text("user"), Value::text("ada"), Value::int(0)];
        let answer = nvs_runtime::call(super::nvs_core_session_set_short_lived, &mut ctx, &write);
        write.into_iter().for_each(dropped);
        dropped(answer.expect("the first write is kept"));
        let headers = ctx.take_headers();
        assert_eq!(
            headers.len(),
            2,
            "the first short-lived write sends the identifier and the short token"
        );
        let id = ctx.session().expect("the session stays open").id.clone();
        assert!(
            headers
                .iter()
                .any(|header| header.value.starts_with(&format!("nvsid={id};"))),
            "{headers:?} carries the identifier"
        );
        assert!(
            headers
                .iter()
                .any(|header| header.value.starts_with("nvsid-short=")),
            "{headers:?} carries the short token"
        );
        ctx.end_session();
        let entries = held.lock().expect("the store");
        assert_eq!(
            entries.len(),
            1,
            "one row, for the one session somebody wrote"
        );
        assert!(
            entries.contains_key(&key_of(NO_APP, &id)),
            "under the identifier the cookie carries"
        );
    }

    /// `Core\Session::get` through the member: a held key's value, `null` for a key the record
    /// does not hold and for one that holds `null`, and no write earned by any of them.
    ///
    /// The flag is asserted because it is the member's other half: a `get` that marked the record
    /// changed would answer every key correctly and cost a read-only request a second round trip.
    /// A value no decode accepts throws rather than answering `null`, so a value this unit cannot
    /// read is never mistaken for an absent one.
    // covers: Core\Session::get
    #[test]
    fn get_answers_a_held_key_and_null_for_the_rest_and_earns_no_write() {
        fn got(ctx: &mut Ctx, key: &[u8]) -> Result<Value, i32> {
            let key = Value::text(std::str::from_utf8(key).expect("the test writes UTF-8"));
            let args = [key, Value::int(0)];
            let answer = nvs_runtime::call(super::nvs_core_session_get, ctx, &args);
            dropped(key);
            answer
        }

        let mut ctx = Ctx::buffered();
        assert!(
            got(&mut ctx, b"cart").is_err(),
            "a request that never started a session has no record to read"
        );
        drop(ctx.take_pending());

        ctx.open_session(loaded(Entries::new()));
        put(&mut ctx, b"cart", Value::int(17));
        put(&mut ctx, b"note", Value::null());
        ctx.session_mut().expect("the session is open").dirty = false;

        let cart = got(&mut ctx, b"cart").expect("a held key is read");
        assert_eq!(cart.as_int(), Some(17));
        dropped(cart);
        for key in [&b"note"[..], b"missing"] {
            let answer = got(&mut ctx, key).expect("an absent or null key is still an answer");
            assert_eq!(
                answer.tag(),
                Some(nvs_runtime::Tag::Null),
                "{} reads as null",
                String::from_utf8_lossy(key)
            );
        }
        assert!(
            !ctx.session().expect("the session is open").dirty,
            "reading is not a change, so it earns no write"
        );

        ctx.session_mut()
            .expect("the session is open")
            .entries
            .insert(b"cart".to_vec(), b"no decode accepts this".to_vec());
        assert!(
            got(&mut ctx, b"cart").is_err(),
            "a value this unit cannot decode throws rather than reading as absent"
        );
        drop(ctx.take_pending());
    }

    /// `Core\Session::remove` through the member: a held key goes and earns a write, the other
    /// keys stay, and a key the record does not hold earns no write at all.
    ///
    /// The flag is asserted on both sides because the second half is the member's own branch: a
    /// `remove` that re-encoded the record regardless would leave every key correct and still put
    /// a request that changed nothing back on the write path.
    // covers: Core\Session::remove
    #[test]
    fn remove_takes_a_held_key_out_and_earns_no_write_for_an_absent_one() {
        fn removed(ctx: &mut Ctx, key: &[u8]) -> Result<Value, i32> {
            let key = Value::text(std::str::from_utf8(key).expect("the test writes UTF-8"));
            let args = [key, Value::int(0)];
            let answer = nvs_runtime::call(super::nvs_core_session_remove, ctx, &args);
            dropped(key);
            answer
        }

        let mut ctx = Ctx::buffered();
        assert!(
            removed(&mut ctx, b"cart").is_err(),
            "a request that never started a session has no record to change"
        );
        drop(ctx.take_pending());

        ctx.open_session(loaded(Entries::new()));
        put(&mut ctx, b"cart", Value::int(17));
        put(&mut ctx, b"user", Value::int(4));
        let session = ctx.session_mut().expect("the session is open");
        session.dirty = false;
        session.touched.clear();

        let before = ctx.session().expect("the session is open").entries.clone();
        dropped(removed(&mut ctx, b"missing").expect("an absent key is not an error"));
        let session = ctx.session().expect("the session is open");
        assert!(!session.dirty, "removing an absent key is not a change");
        assert!(session.touched.is_empty(), "and owes the store nothing");
        assert_eq!(session.entries, before, "and leaves the keys as they were");

        dropped(removed(&mut ctx, b"cart").expect("a held key is removed"));
        assert!(
            ctx.session().expect("the session is open").dirty,
            "removing a held key is a change, so it earns a write"
        );
        assert_eq!(int_at(&ctx, b"cart"), None, "the removed key is gone");
        assert_eq!(int_at(&ctx, b"user"), Some(4), "and the other key stays");
    }

    /// `Core\Session::setShortLived` through the member: the value is saved and earns a write, a
    /// second call replaces it, and the caller's own reference to a heap value survives the call.
    ///
    /// The string is released by this test after each call, so a member that took the argument's
    /// reference instead of retaining its own would free it twice here and panic.
    // covers: Core\Session::setShortLived
    #[test]
    fn set_short_lived_saves_a_value_replaces_it_and_earns_a_write() {
        fn set(ctx: &mut Ctx, key: &[u8], value: &[u8]) -> Result<Value, i32> {
            let key = Value::text(std::str::from_utf8(key).expect("the test writes UTF-8"));
            let value = Value::text(std::str::from_utf8(value).expect("the test writes UTF-8"));
            let args = [key, value, Value::int(0)];
            let answer = nvs_runtime::call(super::nvs_core_session_set_short_lived, ctx, &args);
            dropped(key);
            dropped(value);
            answer
        }
        fn text_at(ctx: &Ctx, key: &[u8]) -> Option<String> {
            let value = read(ctx, "get", key).expect("the value decodes")?;
            let text = value.as_text().map(str::to_owned);
            dropped(value);
            text
        }

        let mut ctx = Ctx::buffered();
        assert!(
            set(&mut ctx, b"language", b"en").is_err(),
            "a program answering no request has no session to write"
        );
        drop(ctx.take_pending());

        ctx.open_session(loaded(Entries::new()));
        dropped(set(&mut ctx, b"language", b"en").expect("a started session saves"));
        assert!(
            ctx.session().expect("the session is open").dirty,
            "saving is a change, so it earns a write"
        );
        assert_eq!(text_at(&ctx, b"language").as_deref(), Some("en"));

        dropped(set(&mut ctx, b"language", b"de").expect("a second value replaces the first"));
        assert_eq!(text_at(&ctx, b"language").as_deref(), Some("de"));
    }

    /// A `get` in a recorded request files one `session` event under the member's name.
    #[test]
    fn a_recorded_session_read_files_one_session_event() {
        let mut ctx = started();
        ctx.set_debug_flags(nvs_runtime::DebugFlags::TRACE);
        let mut sites = nvs_runtime::QuerySites::new();
        sites.push("shop.nvs", 4, "aa");
        ctx.set_query_sites(Arc::new(sites));

        let args = [Value::text("cart"), Value::int(1)];
        let answer = nvs_runtime::call(super::nvs_core_session_get, &mut ctx, &args);
        args.into_iter().for_each(dropped);
        dropped(answer.expect("an empty record reads"));

        let filed = ctx.trace().iter().any(|event| {
            event.kind == nvs_runtime::TraceKind::Session && event.callee == r"Core\Session::get"
        });
        assert!(
            filed,
            "the read files one `session` event named after the member"
        );
    }

    /// A context with a session open over an empty record, as the first use leaves one.
    fn started() -> Ctx {
        let mut ctx = Ctx::buffered();
        ctx.set_random_state(11);
        ctx.open_session(loaded(Entries::new()));
        ctx
    }

    /// Calls `setShortLivedSecret(key, value, ring)` the way a program does, releasing what it
    /// built.
    fn secret_set(ctx: &mut Ctx, key: &[u8], value: &[u8], ring: &[&[u8]]) -> Result<Value, i32> {
        let args = [
            Value::text(std::str::from_utf8(key).expect("the test writes UTF-8")),
            Value::text(std::str::from_utf8(value).expect("the test writes UTF-8")),
            crate::keyring::tests::ring_of(ring),
            Value::int(0),
        ];
        let answer = nvs_runtime::call(super::nvs_core_session_set_short_lived_secret, ctx, &args);
        args.into_iter().for_each(dropped);
        answer
    }

    /// Calls `getSecret(key, ring)` and reads the answer as text, `None` for `null`.
    fn secret_got(ctx: &mut Ctx, key: &[u8], ring: &[&[u8]]) -> Result<Option<String>, i32> {
        let args = [
            Value::text(std::str::from_utf8(key).expect("the test writes UTF-8")),
            crate::keyring::tests::ring_of(ring),
            Value::int(0),
        ];
        let answer = nvs_runtime::call(super::nvs_core_session_get_secret, ctx, &args);
        args.into_iter().for_each(dropped);
        answer.map(|value| {
            let text = value.as_text().map(str::to_owned);
            dropped(value);
            text
        })
    }

    const OLD: &[u8] = &[7; crate::crypto::KEY_LEN];
    const NEW: &[u8] = &[9; crate::crypto::KEY_LEN];

    /// `Core\Session::setShortLivedSecret` through the member: the value lands sealed and earns a
    /// write, the
    /// plain key stays empty so `get` cannot read it, and the stored bytes are not the plaintext.
    ///
    /// Both refusals are asserted before the write, because a member that checked the ring after
    /// sealing would leave a half-written record behind a `LogicError`.
    // covers: Core\Session::setShortLivedSecret
    #[test]
    fn set_short_lived_secret_seals_the_value_out_of_gets_reach_and_earns_a_write() {
        let mut unstarted = Ctx::buffered();
        assert!(
            secret_set(&mut unstarted, b"refresh", b"sk-live-7", &[NEW]).is_err(),
            "a program answering no request has no session to write"
        );
        drop(unstarted.take_pending());

        let mut ctx = started();
        assert!(
            secret_set(&mut ctx, b"refresh", b"sk-live-7", &[]).is_err(),
            "an empty ring seals nothing"
        );
        drop(ctx.take_pending());
        assert!(
            !ctx.session().expect("the session is open").dirty,
            "a refused ring changes nothing"
        );

        dropped(
            secret_set(&mut ctx, b"refresh", b"sk-live-7", &[NEW])
                .expect("a started session seals"),
        );
        assert!(
            ctx.session().expect("the session is open").dirty,
            "sealing a value is a change, so it earns a write"
        );
        assert!(
            !ctx.session()
                .expect("the session is open")
                .entries
                .contains_key(b"refresh".as_slice()),
            "`get` finds nothing under the plain key"
        );
        let sealed = read_bytes(&ctx, "get", &sealed_key(b"refresh"))
            .expect("the sealed entry decodes")
            .expect("the sealed entry is bytes");
        assert!(
            !sealed.windows(9).any(|window| window == b"sk-live-7"),
            "what the store receives is ciphertext"
        );
    }

    /// `Core\Session::getSecret` through the member: the ring that sealed a value opens it, a ring
    /// rotated with that key behind a newer one still does, and another ring or another name is
    /// `null`. Reading is not a change, so it earns no write.
    // covers: Core\Session::getSecret
    #[test]
    fn get_secret_opens_under_its_ring_after_a_rotation_and_misses_elsewhere() {
        let mut unstarted = Ctx::buffered();
        assert!(
            secret_got(&mut unstarted, b"refresh", &[NEW]).is_err(),
            "a request that never started a session has no record to read"
        );
        drop(unstarted.take_pending());

        let mut ctx = started();
        dropped(
            secret_set(&mut ctx, b"refresh", b"sk-live-7", &[OLD])
                .expect("a started session seals"),
        );
        ctx.session_mut().expect("the session is open").dirty = false;

        assert_eq!(
            secret_got(&mut ctx, b"refresh", &[OLD])
                .expect("a key ring reads")
                .as_deref(),
            Some("sk-live-7"),
            "the ring that sealed it opens it"
        );
        assert_eq!(
            secret_got(&mut ctx, b"refresh", &[NEW, OLD])
                .expect("a key ring reads")
                .as_deref(),
            Some("sk-live-7"),
            "a rotated ring still holds the older key, and every key is tried"
        );
        assert_eq!(
            secret_got(&mut ctx, b"refresh", &[NEW]).expect("a miss is not an error"),
            None,
            "a ring without the sealing key opens nothing"
        );
        assert_eq!(
            secret_got(&mut ctx, b"never-written", &[OLD]).expect("a miss is not an error"),
            None,
            "a name nothing was sealed under is a miss"
        );
        assert!(
            secret_got(&mut ctx, b"refresh", &[]).is_err(),
            "an empty ring is the program's bug, and throws rather than missing"
        );
        drop(ctx.take_pending());
        assert!(
            !ctx.session().expect("the session is open").dirty,
            "reading a secret changes nothing, so it earns no write"
        );
    }

    /// The key is prefixed and carries the id, so one store holding a cache, a limiter and a
    /// session store keeps the three apart — and an application-named limiter key cannot be
    /// spelled to read a session record. The prefix names the layout with two lifetimes, so a key
    /// an earlier build wrote, as one string under `nvs:session:` or as a hash under
    /// `nvs:session-keys:`, is never read.
    #[test]
    fn a_record_is_keyed_under_its_own_prefix() {
        let key = key_of(b"/srv/blog", ID);
        assert_eq!(PREFIX, "nvs:session-lives:");
        assert!(key.starts_with(PREFIX.as_bytes()));
        for earlier in [b"nvs:session:".as_slice(), b"nvs:session-keys:"] {
            assert!(
                !key.starts_with(earlier),
                "a key of an earlier layout is never read"
            );
        }
        assert_eq!(
            &key[PREFIX.len()..],
            b"/srv/blog\0PmA5tKz1QvR3sYbN".as_slice()
        );
        assert_eq!(&key[key.len() - ID.len()..], ID.as_bytes());
        assert_ne!(
            key,
            key_of(b"/srv/shop", ID),
            "another application's key differs"
        );
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
    /// `rule:http-server/session-expiry-belongs-to-the-store`'s lifetime at both ends — the shipped
    /// one with nothing configured, and a written one preferred to it — and
    /// `rule:http-server/no-session-block-means-the-data-folder-store`: a tree with no `[session]`
    /// block gets the data folder's SQLite file, and a word the roster does not spell throws
    /// naming the two it does.
    #[test]
    fn the_block_decides_the_store_and_absence_selects_the_data_folder_file() {
        let mut ctx = Ctx::buffered();
        assert_eq!(
            short_lived(&ctx),
            i64::try_from(super::DEFAULT_SHORT_LIVED.as_millis()).expect("two hours")
        );
        assert_eq!(cookie(&ctx), "nvsid");
        assert_eq!(
            backend(&ctx, "start").expect("no block is the default store"),
            Backend::Db
        );
        if let Some(folder) = nvs_config::data::current() {
            assert_eq!(
                opened(&ctx, "start", "").expect("the data folder's file"),
                Store::File(folder.sessions())
            );
        }

        ctx.set_config(crate::tests::granting("[session]\nbackend = \"redis\"\n"));
        let refused = backend(&ctx, "start").expect_err("a word the roster does not spell");
        let nvs_runtime::Fault::Thrown(_, said) = refused else {
            panic!("an unknown store is a throw the program can catch");
        };
        assert!(
            said.contains("backend = \"db\"") && said.contains("backend = \"shared\""),
            "the throw names the two stores: {said}"
        );

        ctx.set_config(crate::tests::granting(
            "[session]\nbackend = \"shared\"\nshort_lived = \"30m\"\ncookie = \"sid\"\n",
        ));
        assert_eq!(
            backend(&ctx, "start").expect("a store the roster admits"),
            Backend::Shared
        );
        assert_eq!(short_lived(&ctx), 30 * 60 * 1000);
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
    /// Asserted over the **encoded** entries as well as the decoded value, because a member that
    /// sealed the value and then also left it somewhere in the clear would pass on the value alone —
    /// and what crosses to the store is those bytes, not the value this process decoded.
    #[test]
    fn session_secret_is_stored_as_ciphertext_in_the_record() {
        const TOKEN: &[u8] = b"sk-live-7";

        let mut ctx = Ctx::buffered();
        ctx.set_random_state(23);
        ctx.open_session(loaded(Entries::new()));

        let ring = crate::keyring::tests::ring_of(&[&[7_u8; 32]]);
        let keys = crate::keyring::tests::borrowed(&ring);

        sealed_into(&mut ctx, "token", TOKEN, &keys, "test", Lifetime::Short)
            .expect("a ring of one key seals a short value");

        let session = ctx.session().expect("the session is still open");
        assert!(
            session.entries.iter().all(|(key, value)| {
                ![key, value]
                    .iter()
                    .any(|bytes| bytes.windows(TOKEN.len()).any(|window| window == TOKEN))
            }),
            "the bytes the store is handed carry the secret nowhere in the clear"
        );

        let entry = read_bytes(&ctx, "getSecret", &sealed_key(b"token"))
            .expect("the sealed entry decodes")
            .expect("the sealed entry is under the sealed name");
        assert_ne!(entry, TOKEN, "and neither does the entry itself");
        assert!(
            read(&ctx, "get", b"token")
                .expect("an absent key reads")
                .is_none(),
            "`get` is blind to it, so the sealed door is the only door here too"
        );

        let opened = opened_from(&ctx, "token", &keys, "test")
            .expect("nothing here is unaffordable")
            .expect("its own ring opens what it sealed");
        assert_eq!(
            opened.as_text(),
            Some("sk-live-7"),
            "what went in is what comes back out, across the encode and the decode between them"
        );

        dropped(opened);
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

        let mut ctx = started();
        let ring = crate::keyring::tests::ring_of(&[&[9_u8; 32]]);
        let keys = crate::keyring::tests::borrowed(&ring);
        let (slot, newest) = crate::keyring::newest(&keys);
        let cipher =
            crate::keyring::cipher_at(&newest, slot, "test").expect("the fixture is a key");

        let app = crate::cache::application(&ctx).to_vec();
        let cached = crate::cache::bound(crate::cache::SEAL_DOMAIN, &app, KEY);
        let sessioned = crate::cache::bound(SEAL_DOMAIN, &app, KEY);

        let for_the_cache = crate::crypto::seal_under(&mut ctx, &cipher, &cached, TOKEN, "test")
            .expect("a short value seals");
        put(
            &mut ctx,
            &sealed_key(KEY),
            Value::bytes(NvsStr::new(&for_the_cache)),
        );
        assert!(
            opened_from(&ctx, "token", &keys, "test")
                .expect("nothing here is unaffordable")
                .is_none(),
            "a ciphertext an operator moved out of a cache and into a record is a miss"
        );

        sealed_into(&mut ctx, "token", TOKEN, &keys, "test", Lifetime::Short)
            .expect("a ring of one key seals a short value");
        let sealed = read_bytes(&ctx, "getSecret", &sealed_key(KEY))
            .expect("the sealed entry decodes")
            .expect("a sealed entry is bytes");
        let sealed = sealed.as_slice();
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

        dropped(ring);
    }

    /// The identifier the cases below store and present. It has the width [`super::issuable`]
    /// checks, so a cookie that carries it reaches the store.
    const LIVE: &str = "PmA5tKz1QvR3sYbNcW8xLq";

    /// The short token the cases below store and present in the short cookie.
    const SHORT_TOKEN: &str = "Tk4w9QbL2sYcXr7uVn1pMa";

    /// The shipped short and long lifetimes, in milliseconds.
    fn lifetimes() -> (i64, i64) {
        let ctx = Ctx::buffered();
        (short_lived(&ctx), super::long_lived(&ctx))
    }

    /// The `Max-Age` attribute the identifier cookie carries when the long clock starts again.
    fn long_max_age() -> String {
        format!("Max-Age={}", lifetimes().1 / 1000)
    }

    /// Integer values under their keys, encoded as the store keeps them.
    fn ints(pairs: &[(&str, i64)]) -> Entries {
        pairs
            .iter()
            .map(|(key, n)| (key.as_bytes().to_vec(), int_bytes(*n)))
            .collect()
    }

    /// The names of long-lived keys.
    fn longs(keys: &[&str]) -> LongKeys {
        keys.iter().map(|key| key.as_bytes().to_vec()).collect()
    }

    /// `entries` stored under `id` on `store`, with `long` long-lived, the short `token`, and the
    /// short and long clocks ending at `ends`. A case moves time by choosing those ends.
    fn stored_with(
        store: &Store,
        id: &str,
        entries: &Entries,
        long: &LongKeys,
        token: Option<&str>,
        (short_until, long_until): (i64, i64),
    ) {
        let clocks = Clocks {
            now: crate::queue::now_millis(),
            short_until,
            long_until,
        };
        let whole = Whole {
            entries,
            long,
            token,
            clocks,
        };
        assert!(
            issued(store, NO_APP, "a case", id, &whole).expect("the store answers"),
            "the row is issued"
        );
    }

    /// The whole record `store` keeps under `id`, or `None` for a session that is gone.
    fn record_under(store: &Store, id: &str) -> Option<super::Record> {
        fetched(store, NO_APP, "a case", id).expect("the store answers")
    }

    /// A request from a visitor with no session yet, whose write-back sends to the case's store
    /// once [`ended`] runs it.
    fn a_visitor() -> Ctx {
        let mut ctx = Ctx::buffered();
        ctx.open_session(nvs_runtime::Session {
            write_back: to_the_case_store,
            ..super::fresh_session()
        });
        ctx
    }

    /// A request whose first use finds the session under `id` on `store`, opened as
    /// [`super::opened_under`] opens it after the load: `short` is the short cookie the browser
    /// presents, and `by_hand` is a session opened by `openWithIdentifier`. Its write-back sends to
    /// `store` once [`ended`] runs it.
    fn returning(store: &Store, id: &str, short: Option<&str>, by_hand: bool) -> Ctx {
        let mut ctx = Ctx::buffered();
        let record = record_under(store, id).expect("the session the case stored");
        let session = super::opened_record(&ctx, id.to_owned(), record, short, by_hand);
        ctx.open_session(nvs_runtime::Session {
            write_back: to_the_case_store,
            ..session
        });
        super::refreshed_long_clock(&mut ctx, "a case").expect("the head is open");
        ctx
    }

    /// The shared store with nothing in it, the configuration a request needs to reach it, and
    /// what keeps it alive.
    fn a_shared_store(case: &str) -> (Store, Arc<nvs_config::Snapshot>, Kept) {
        let mut server = Ctx::buffered();
        let (store, kept) = a_store(&mut server, case, Backend::Shared);
        let config = Arc::clone(
            server
                .config()
                .expect("the case configured the store")
                .snapshot(),
        );
        (store, config, kept)
    }

    /// A request under `config` whose `Cookie` header carries `cookies`, or that has none where
    /// `cookies` is empty.
    fn a_browser(config: &Arc<nvs_config::Snapshot>, cookies: &str) -> Ctx {
        let mut inbound = nvs_runtime::Inbound::new("GET", "/", "");
        if !cookies.is_empty() {
            inbound.push_header("cookie", cookies.as_bytes());
        }
        let mut ctx = Ctx::buffered();
        ctx.set_inbound(inbound);
        ctx.set_config(Arc::clone(config));
        ctx
    }

    /// `member(key, n)` called the way a program calls `setShortLived` or `setLongLived`.
    fn set_with(ctx: &mut Ctx, member: nvs_runtime::NvsFn, key: &str, n: i64) {
        let args = [Value::text(key), Value::int(n), Value::int(0)];
        let answer = nvs_runtime::call(member, ctx, &args);
        args.into_iter().for_each(dropped);
        dropped(answer.expect("the value is saved"));
    }

    /// `Core\Session::get(key)` called the way a program calls it, read as an integer.
    fn got_int(ctx: &mut Ctx, key: &str) -> Option<i64> {
        let args = [Value::text(key), Value::int(0)];
        let answer = nvs_runtime::call(super::nvs_core_session_get, ctx, &args);
        args.into_iter().for_each(dropped);
        let value = answer.expect("the session reads");
        let int = value.as_int();
        dropped(value);
        int
    }

    /// `Core\Session::remove(key)` called the way a program calls it.
    fn removed(ctx: &mut Ctx, key: &str) {
        let args = [Value::text(key), Value::int(0)];
        let answer = nvs_runtime::call(super::nvs_core_session_remove, ctx, &args);
        args.into_iter().for_each(dropped);
        dropped(answer.expect("the key is removed"));
    }

    /// A member with no arguments, such as `clearAll`, called the way a program calls it.
    fn called(ctx: &mut Ctx, member: nvs_runtime::NvsFn) {
        dropped(nvs_runtime::call(member, ctx, &[]).expect("the member runs"));
    }

    /// `member(key, value, ring)` called the way a program calls `setShortLivedSecret` or
    /// `setLongLivedSecret`.
    fn sealed_with(ctx: &mut Ctx, member: nvs_runtime::NvsFn, key: &str, value: &str) {
        let args = [
            Value::text(key),
            Value::text(value),
            crate::keyring::tests::ring_of(&[NEW]),
            Value::int(0),
        ];
        let answer = nvs_runtime::call(member, ctx, &args);
        args.into_iter().for_each(dropped);
        dropped(answer.expect("the secret is sealed"));
    }

    /// The value and the whole line of the last `Set-Cookie` in `headers` for the cookie `name`.
    fn sent_cookie(
        headers: &[nvs_runtime::DeclaredHeader],
        name: &str,
    ) -> Option<(String, String)> {
        let start = format!("{name}=");
        headers.iter().rev().find_map(|header| {
            if !header.name.eq_ignore_ascii_case("set-cookie") {
                return None;
            }
            let rest = header.value.strip_prefix(start.as_str())?;
            let value = rest.split(';').next().unwrap_or_default();
            Some((value.to_owned(), header.value.to_string()))
        })
    }

    /// `rule:http-server/a-session-value-is-short-lived-or-long-lived`: a short-lived value is
    /// bound to the short cookie. A request that presents the identifier without it sees no
    /// short-lived value, and its write-back deletes the value, so the old short cookie does not
    /// bring it back.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn a_short_lived_value_is_gone_once_the_browser_drops_its_short_cookie_on_any_store() {
        for (word, backend) in BACKENDS {
            let mut server = Ctx::buffered();
            let (store, _kept) = a_store(&mut server, "session-short-cookie-dropped", *backend);
            let mut first = a_visitor();
            set_with(
                &mut first,
                super::nvs_core_session_set_short_lived,
                "cart",
                3,
            );
            set_with(
                &mut first,
                super::nvs_core_session_set_long_lived,
                "consent",
                1,
            );
            let headers = first.take_headers();
            let (id, _) = sent_cookie(&headers, "nvsid").expect("the identifier cookie is sent");
            let (token, line) =
                sent_cookie(&headers, "nvsid-short").expect("the short cookie is sent");
            assert!(
                !line.contains("Max-Age"),
                "{word}: the short cookie ends when the browser closes: {line}"
            );
            ended(first, &store);

            let mut same_browser = returning(&store, &id, Some(&token), false);
            assert_eq!(
                got_int(&mut same_browser, "cart"),
                Some(3),
                "{word}: the browser that has the short cookie sees the value"
            );
            ended(same_browser, &store);

            let mut restarted = returning(&store, &id, None, false);
            assert_eq!(
                got_int(&mut restarted, "cart"),
                None,
                "{word}: without the short cookie the value is gone"
            );
            assert_eq!(
                got_int(&mut restarted, "consent"),
                Some(1),
                "{word}: and the long-lived value is still there"
            );
            ended(restarted, &store);

            assert_eq!(
                stored_under(&store, &id),
                Some(ints(&[("consent", 1)])),
                "{word}: the write-back deleted the short-lived value from the store"
            );
            let mut copied = returning(&store, &id, Some(&token), false);
            assert_eq!(
                got_int(&mut copied, "cart"),
                None,
                "{word}: so the old short cookie does not bring it back"
            );
        }
    }

    /// `rule:http-server/a-session-value-is-short-lived-or-long-lived`: a long-lived value needs
    /// the identifier cookie alone. That cookie has a `Max-Age`, so the browser keeps it when it
    /// closes, and every request without the short cookie sees the value and keeps it.
    // covers: Core\Session::setLongLived
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn a_long_lived_value_survives_a_new_browser_session_on_any_store() {
        for (word, backend) in BACKENDS {
            let mut server = Ctx::buffered();
            let (store, _kept) = a_store(&mut server, "session-long-survives", *backend);
            let mut first = a_visitor();
            set_with(
                &mut first,
                super::nvs_core_session_set_long_lived,
                "consent",
                1,
            );
            let headers = first.take_headers();
            let (id, line) = sent_cookie(&headers, "nvsid").expect("the identifier cookie is sent");
            assert!(
                line.contains(&long_max_age()),
                "{word}: the identifier cookie outlives the browser: {line}"
            );
            assert!(
                sent_cookie(&headers, "nvsid-short").is_none(),
                "{word}: a session with no short-lived value sends no short cookie"
            );
            ended(first, &store);

            for visit in 1..=2 {
                let mut restarted = returning(&store, &id, None, false);
                assert_eq!(
                    got_int(&mut restarted, "consent"),
                    Some(1),
                    "{word}: new browser session {visit} sees the value"
                );
                ended(restarted, &store);
            }
            let record = record_under(&store, &id).expect("the session is kept");
            assert!(
                record.long.contains(b"consent".as_slice()),
                "{word}: and the store keeps the value long-lived"
            );
        }
    }

    /// `rule:http-server/session-expiry-belongs-to-the-store`'s short clock: once it has ended, a
    /// short-lived value is gone, also for the browser that has the short cookie. A long-lived value
    /// keeps the record, and the write-back deletes the short-lived one.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn a_short_lived_value_is_gone_after_the_short_clock_on_any_store() {
        let (_, long_lived) = lifetimes();
        for (word, backend) in BACKENDS {
            let mut server = Ctx::buffered();
            let (store, _kept) = a_store(&mut server, "session-short-clock-ended", *backend);
            let now = crate::queue::now_millis();
            stored_with(
                &store,
                LIVE,
                &ints(&[("cart", 3), ("consent", 1)]),
                &longs(&["consent"]),
                Some(SHORT_TOKEN),
                (now - 1, now + long_lived),
            );

            let mut late = returning(&store, LIVE, Some(SHORT_TOKEN), false);
            assert_eq!(
                got_int(&mut late, "cart"),
                None,
                "{word}: the short clock ended 1 ms ago, so the value is gone"
            );
            assert_eq!(
                got_int(&mut late, "consent"),
                Some(1),
                "{word}: the long-lived value is not on the short clock"
            );
            ended(late, &store);
            assert_eq!(
                stored_under(&store, LIVE),
                Some(ints(&[("consent", 1)])),
                "{word}: the write-back deleted the short-lived value"
            );
        }
    }

    /// `rule:http-server/session-expiry-belongs-to-the-store`'s half rule: a request that only
    /// reads moves a clock when less than half of its time is left, and writes nothing while more
    /// than half is left. Moving the long clock sends the identifier cookie again.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn a_read_refreshes_a_clock_only_when_less_than_half_is_left_on_any_store() {
        let (short_lived, long_lived) = lifetimes();
        for (word, backend) in BACKENDS {
            let mut server = Ctx::buffered();
            let (store, _kept) = a_store(&mut server, "session-half-rule", *backend);
            for (left, refreshed) in [(60, false), (40, true)] {
                let now = crate::queue::now_millis();

                let short_id = format!("short-clock-{left}");
                let short_until = now + short_lived * left / 100;
                stored_with(
                    &store,
                    &short_id,
                    &ints(&[("cart", 3)]),
                    &LongKeys::new(),
                    Some(SHORT_TOKEN),
                    (short_until, 0),
                );
                let mut reader = returning(&store, &short_id, Some(SHORT_TOKEN), false);
                assert_eq!(got_int(&mut reader, "cart"), Some(3), "{word}");
                assert_eq!(
                    reader.session().expect("the session is open").dirty,
                    refreshed,
                    "{word}: {left}% of the short clock is left"
                );
                assert!(
                    reader.take_headers().is_empty(),
                    "{word}: the short clock sends no cookie"
                );
                ended(reader, &store);
                let moved = record_under(&store, &short_id)
                    .expect("the session is kept")
                    .short_until;
                assert_eq!(
                    moved > short_until,
                    refreshed,
                    "{word}: with {left}% of the short clock left, the read moves it: {refreshed}"
                );

                let long_id = format!("long-clock-{left}");
                let long_until = now + long_lived * left / 100;
                stored_with(
                    &store,
                    &long_id,
                    &ints(&[("consent", 1)]),
                    &longs(&["consent"]),
                    None,
                    (now + short_lived, long_until),
                );
                let mut reader = returning(&store, &long_id, None, false);
                assert_eq!(got_int(&mut reader, "consent"), Some(1), "{word}");
                assert_eq!(
                    reader.session().expect("the session is open").dirty,
                    refreshed,
                    "{word}: {left}% of the long clock is left"
                );
                let headers = reader.take_headers();
                assert_eq!(
                    sent_cookie(&headers, "nvsid")
                        .is_some_and(|(id, line)| id == long_id && line.contains(&long_max_age())),
                    refreshed,
                    "{word}: with {left}% of the long clock left, the cookie is sent again: \
                     {headers:?}"
                );
                ended(reader, &store);
                let moved = record_under(&store, &long_id)
                    .expect("the session is kept")
                    .long_until;
                assert_eq!(
                    moved > long_until,
                    refreshed,
                    "{word}: with {left}% of the long clock left, the read moves it: {refreshed}"
                );
            }
        }
    }

    /// `rule:http-server/a-session-value-is-short-lived-or-long-lived`: all values share one set of
    /// keys, so the last write of a key sets its lifetime. The store keeps the key once, with the
    /// lifetime of that write.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn writing_a_key_with_the_other_lifetime_replaces_it_on_any_store() {
        let (short_lived, _) = lifetimes();
        for (word, backend) in BACKENDS {
            let mut server = Ctx::buffered();
            let (store, kept) = a_store(&mut server, "session-lifetime-replaced", *backend);
            let now = crate::queue::now_millis();
            stored_with(
                &store,
                LIVE,
                &ints(&[("pref", 1)]),
                &LongKeys::new(),
                Some(SHORT_TOKEN),
                (now + short_lived, 0),
            );

            let mut ctx = returning(&store, LIVE, Some(SHORT_TOKEN), false);
            set_with(&mut ctx, super::nvs_core_session_set_long_lived, "pref", 2);
            ended(ctx, &store);
            let record = record_under(&store, LIVE).expect("the session is kept");
            assert_eq!(
                record.entries,
                ints(&[("pref", 2)]),
                "{word}: one value under the key"
            );
            assert!(
                record.long.contains(b"pref".as_slice()),
                "{word}: and it is long-lived now"
            );
            let mut restarted = returning(&store, LIVE, None, false);
            assert_eq!(
                got_int(&mut restarted, "pref"),
                Some(2),
                "{word}: so a new browser session sees it"
            );

            let mut ctx = returning(&store, LIVE, Some(SHORT_TOKEN), false);
            set_with(&mut ctx, super::nvs_core_session_set_short_lived, "pref", 3);
            ended(ctx, &store);
            let record = record_under(&store, LIVE).expect("the session is kept");
            assert_eq!(
                record.entries,
                ints(&[("pref", 3)]),
                "{word}: one value under the key"
            );
            assert!(
                record.long.is_empty(),
                "{word}: and it is short-lived again"
            );
            if let Kept::Shared(_, held) = &kept {
                let fields: Vec<Vec<u8>> = held
                    .lock()
                    .expect("the store")
                    .get(&key_of(NO_APP, LIVE))
                    .expect("the hash")
                    .keys()
                    .filter(|field| is_key_field(field))
                    .cloned()
                    .collect();
                assert_eq!(
                    fields,
                    vec![field_of(b"pref", false)],
                    "{word}: the hash keeps the key under one field"
                );
            }
        }
    }

    /// `rule:http-server/a-session-value-is-short-lived-or-long-lived`'s `clearShortLived`, the
    /// logout: the short-lived values are deleted, and the session moves to a new identifier and a
    /// new short token. The long-lived values stay under the new identifier. The member's two
    /// steps run with the case's store in place of the configured one.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn clear_short_lived_keeps_long_lived_values_under_a_new_identifier_on_any_store() {
        const WHO: &str = r"Core\Session::clearShortLived";
        let (short_lived, long_lived) = lifetimes();
        for (word, backend) in BACKENDS {
            let mut server = Ctx::buffered();
            let (store, _kept) = a_store(&mut server, "session-clear-short", *backend);
            let now = crate::queue::now_millis();
            stored_with(
                &store,
                LIVE,
                &ints(&[("cart", 3), ("consent", 1)]),
                &longs(&["consent"]),
                Some(SHORT_TOKEN),
                (now + short_lived, now + long_lived),
            );

            let mut ctx = returning(&store, LIVE, Some(SHORT_TOKEN), false);
            assert!(
                super::emptied(&mut ctx, "clearShortLived", Lifetime::Short)
                    .expect("the session is open"),
                "{word}: a stored session has values to clear"
            );
            super::moved_on(&mut ctx, Some(&store), WHO, "clearShortLived")
                .unwrap_or_else(|why| panic!("{word}: the session moves: {why:?}"));
            let headers = ctx.take_headers();
            let (fresh, line) = sent_cookie(&headers, "nvsid").expect("the new identifier is sent");
            assert_ne!(fresh, LIVE, "{word}: the identifier is new");
            assert!(
                line.contains("Max-Age="),
                "{word}: its cookie outlives the browser, for the long-lived value: {line}"
            );
            let (token, _) = sent_cookie(&headers, "nvsid-short").expect("a new short token");
            assert_ne!(token, SHORT_TOKEN, "{word}: the short token is new");
            assert_eq!(got_int(&mut ctx, "cart"), None, "{word}");
            ended(ctx, &store);

            assert_eq!(
                stored_under(&store, LIVE),
                None,
                "{word}: the old identifier names nothing"
            );
            assert_eq!(
                stored_under(&store, &fresh),
                Some(ints(&[("consent", 1)])),
                "{word}: the new one keeps the long-lived value alone"
            );
            let mut next = returning(&store, &fresh, Some(&token), false);
            assert_eq!(got_int(&mut next, "consent"), Some(1), "{word}");
            assert_eq!(got_int(&mut next, "cart"), None, "{word}");
        }
    }

    /// `rule:http-server/a-session-value-is-short-lived-or-long-lived`'s `clearLongLived`: the
    /// long-lived values are deleted and the session moves to a new identifier, whose cookie has no
    /// `Max-Age`. The short-lived values stay under the new identifier and a new short token. The
    /// member's two steps run with the case's store in place of the configured one.
    // covers: Core\Session::clearLongLived
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn clear_long_lived_keeps_short_lived_values_under_a_new_identifier_on_any_store() {
        const WHO: &str = r"Core\Session::clearLongLived";
        let (short_lived, long_lived) = lifetimes();
        for (word, backend) in BACKENDS {
            let mut server = Ctx::buffered();
            let (store, _kept) = a_store(&mut server, "session-clear-long", *backend);
            let now = crate::queue::now_millis();
            stored_with(
                &store,
                LIVE,
                &ints(&[("cart", 3), ("consent", 1)]),
                &longs(&["consent"]),
                Some(SHORT_TOKEN),
                (now + short_lived, now + long_lived),
            );

            let mut ctx = returning(&store, LIVE, Some(SHORT_TOKEN), false);
            assert!(
                super::emptied(&mut ctx, "clearLongLived", Lifetime::Long)
                    .expect("the session is open"),
                "{word}: a stored session has values to clear"
            );
            super::moved_on(&mut ctx, Some(&store), WHO, "clearLongLived")
                .unwrap_or_else(|why| panic!("{word}: the session moves: {why:?}"));
            let headers = ctx.take_headers();
            let (fresh, line) = sent_cookie(&headers, "nvsid").expect("the new identifier is sent");
            assert_ne!(fresh, LIVE, "{word}: the identifier is new");
            assert!(
                !line.contains("Max-Age"),
                "{word}: no long-lived value is left, so the cookie ends with the browser: {line}"
            );
            let (token, _) = sent_cookie(&headers, "nvsid-short").expect("a new short token");
            ended(ctx, &store);

            assert_eq!(
                stored_under(&store, LIVE),
                None,
                "{word}: the old identifier names nothing"
            );
            let record = record_under(&store, &fresh).expect("the new identifier is kept");
            assert_eq!(
                record.entries,
                ints(&[("cart", 3)]),
                "{word}: the new one keeps the short-lived value alone"
            );
            assert!(record.long.is_empty(), "{word}: and no long-lived key");
            let mut next = returning(&store, &fresh, Some(&token), false);
            assert_eq!(got_int(&mut next, "cart"), Some(3), "{word}");
            assert_eq!(got_int(&mut next, "consent"), None, "{word}");
        }
    }

    /// `rule:core-api/session-roster`'s `clearAll`, from the first use to the end of the request:
    /// the row is deleted, both cookies expire, and the end of the request writes nothing. A
    /// request that presents the old cookies afterwards gets a fresh session and leaves no row.
    #[test]
    fn clear_all_leaves_no_row_and_expires_both_cookies() {
        let (short_lived, _) = lifetimes();
        let (store, config, kept) = a_shared_store("session-clear-all");
        let now = crate::queue::now_millis();
        stored_with(
            &store,
            LIVE,
            &ints(&[("cart", 3)]),
            &LongKeys::new(),
            Some(SHORT_TOKEN),
            (now + short_lived, 0),
        );
        let cookies = format!("nvsid={LIVE}; nvsid-short={SHORT_TOKEN}");

        let mut ctx = a_browser(&config, &cookies);
        called(&mut ctx, super::nvs_core_session_clear_all);
        assert_eq!(kept.rows(), 0, "the row is deleted");
        let headers = ctx.take_headers();
        for name in ["nvsid", "nvsid-short"] {
            let (value, line) =
                sent_cookie(&headers, name).unwrap_or_else(|| panic!("{headers:?} sends {name}"));
            assert!(
                value.is_empty() && line.contains("Max-Age=0"),
                "{line} expires {name}"
            );
        }
        assert_eq!(
            got_int(&mut ctx, "cart"),
            None,
            "the request goes on with an empty session"
        );
        ctx.end_session();
        assert_eq!(kept.rows(), 0, "and its end writes nothing back");

        let mut after = a_browser(&config, &cookies);
        assert_eq!(
            got_int(&mut after, "cart"),
            None,
            "the old cookies name nothing"
        );
        assert!(
            after.session().expect("the session is open").id.is_empty(),
            "so the request has a fresh session with no identifier"
        );
        after.end_session();
        assert_eq!(kept.rows(), 0, "and leaves no row");
    }

    /// `rule:http-server/a-session-value-is-short-lived-or-long-lived`: the identifier cookie has a
    /// `Max-Age` while the session has a long-lived value, and none otherwise. It is sent again
    /// when the first long-lived value arrives, when the last one goes, and by `clearLongLived`.
    #[test]
    fn the_identifier_cookie_has_max_age_only_while_a_long_lived_value_exists() {
        let (_store, config, _kept) = a_shared_store("session-identifier-max-age");

        let mut first = a_browser(&config, "");
        set_with(
            &mut first,
            super::nvs_core_session_set_short_lived,
            "cart",
            3,
        );
        let (id, line) =
            sent_cookie(&first.take_headers(), "nvsid").expect("the first write sends it");
        assert!(
            !line.contains("Max-Age"),
            "only a short-lived value, so the cookie ends with the browser: {line}"
        );
        set_with(
            &mut first,
            super::nvs_core_session_set_long_lived,
            "consent",
            1,
        );
        let (again, line) = sent_cookie(&first.take_headers(), "nvsid")
            .expect("the first long-lived value sends it again");
        assert_eq!(again, id, "under the same identifier");
        assert!(
            line.contains(&long_max_age()),
            "a long-lived value, so the cookie outlives the browser: {line}"
        );
        let token = first
            .session()
            .expect("the session is open")
            .short_token
            .clone()
            .expect("the short-lived write drew a token");
        first.end_session();

        let mut second = a_browser(&config, &format!("nvsid={id}; nvsid-short={token}"));
        assert_eq!(got_int(&mut second, "consent"), Some(1));
        assert!(
            second.take_headers().is_empty(),
            "a long clock with more than half left is not sent again"
        );
        removed(&mut second, "consent");
        let (_, line) = sent_cookie(&second.take_headers(), "nvsid")
            .expect("the last long-lived value going sends it again");
        assert!(
            !line.contains("Max-Age"),
            "no long-lived value is left: {line}"
        );
        set_with(
            &mut second,
            super::nvs_core_session_set_long_lived,
            "consent",
            2,
        );
        let (_, line) = sent_cookie(&second.take_headers(), "nvsid")
            .expect("a long-lived value arriving sends it again");
        assert!(line.contains(&long_max_age()), "{line}");
        called(&mut second, super::nvs_core_session_clear_long_lived);
        let (moved, line) = sent_cookie(&second.take_headers(), "nvsid")
            .expect("`clearLongLived` sends the new identifier");
        assert_ne!(moved, id, "the identifier is new");
        assert!(
            !line.contains("Max-Age"),
            "the long-lived values are gone: {line}"
        );
    }

    /// `rule:core-classes/a-session-opens-on-its-first-use`: the first call of a member loads the
    /// session with one read, and later calls use the request's copy. A request that presents the
    /// cookies and calls no member sends the store nothing.
    #[test]
    fn the_first_call_opens_the_session_and_a_request_that_never_calls_one_reaches_no_store() {
        let (short_lived, _) = lifetimes();
        let (store, config, kept) = a_shared_store("session-first-call");
        let now = crate::queue::now_millis();
        stored_with(
            &store,
            LIVE,
            &ints(&[("cart", 3)]),
            &LongKeys::new(),
            Some(SHORT_TOKEN),
            (now + short_lived, 0),
        );
        let Kept::Shared(serving, _) = &kept else {
            panic!("the case built the shared store");
        };
        let commands = || serving.commands.load(Ordering::SeqCst);
        let writes = || serving.writes.load(Ordering::SeqCst);
        let (before, written) = (commands(), writes());
        let cookies = format!("nvsid={LIVE}; nvsid-short={SHORT_TOKEN}");

        let mut idle = a_browser(&config, &cookies);
        idle.end_session();
        assert!(
            idle.session().is_none(),
            "a request that calls no member opens no session"
        );
        assert!(idle.take_headers().is_empty(), "and sends no cookie");
        assert_eq!(commands(), before, "and sends the store nothing");

        let mut reader = a_browser(&config, &cookies);
        assert!(
            reader.session().is_none(),
            "nothing is open before the first call"
        );
        assert_eq!(
            got_int(&mut reader, "cart"),
            Some(3),
            "the first call reads the stored value"
        );
        let open = reader.session().expect("the first call opened the session");
        assert_eq!(open.id, LIVE);
        assert_eq!(
            open.opened_by, r"Core\Session::get",
            "the session names the member whose call opened it"
        );
        assert_eq!(commands(), before + 1, "the open is one read");
        assert_eq!(got_int(&mut reader, "cart"), Some(3));
        assert_eq!(
            commands(),
            before + 1,
            "a second call reads the request's copy"
        );
        reader.end_session();
        assert_eq!(
            (commands(), writes()),
            (before + 1, written),
            "and a request that only read writes nothing"
        );
    }

    /// `rule:core-classes/a-session-opens-on-its-first-use`'s `openWithIdentifier`: a session the
    /// program opens under an identifier it read itself has no browser, so its short-lived values
    /// are bound to the identifier alone, and a write sends no cookie. A browser that presents the
    /// identifier cookie without the short cookie still sees no short-lived value.
    #[test]
    fn an_identifier_opened_by_hand_sees_its_short_lived_values_without_a_short_cookie() {
        let (short_lived, _) = lifetimes();
        let (store, config, _kept) = a_shared_store("session-opened-by-hand");
        let now = crate::queue::now_millis();
        stored_with(
            &store,
            LIVE,
            &ints(&[("cart", 3)]),
            &LongKeys::new(),
            Some(SHORT_TOKEN),
            (now + short_lived, 0),
        );
        let by_hand = || {
            let mut ctx = a_browser(&config, "");
            let id = [Value::text(LIVE)];
            let answer =
                nvs_runtime::call(super::nvs_core_session_open_with_identifier, &mut ctx, &id);
            id.into_iter().for_each(dropped);
            dropped(answer.expect("the store has the identifier"));
            ctx
        };

        let mut api = by_hand();
        assert_eq!(
            got_int(&mut api, "cart"),
            Some(3),
            "no short cookie, and the short-lived value is there"
        );
        set_with(&mut api, super::nvs_core_session_set_short_lived, "more", 4);
        assert!(
            api.take_headers().is_empty(),
            "no browser, so a write sends no cookie"
        );
        api.end_session();

        let mut again = by_hand();
        assert_eq!(got_int(&mut again, "cart"), Some(3));
        assert_eq!(
            got_int(&mut again, "more"),
            Some(4),
            "the next request opened by hand sees the value the first one wrote"
        );

        let mut browser = a_browser(&config, &format!("nvsid={LIVE}"));
        assert_eq!(
            got_int(&mut browser, "cart"),
            None,
            "a browser without the short cookie sees no short-lived value"
        );
    }

    /// `rule:http-server/a-session-holds-a-secret-only-sealed`: a long-lived secret opens with the
    /// identifier cookie alone after the browser closes, and the store has only its ciphertext. A
    /// short-lived secret saved beside it is gone in the new browser session.
    // covers: Core\Session::setLongLivedSecret
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn a_long_lived_secret_survives_a_new_browser_session_and_stays_sealed_on_any_store() {
        const PLAIN: &str = "sk-live-7";
        for (word, backend) in BACKENDS {
            let mut server = Ctx::buffered();
            let (store, _kept) = a_store(&mut server, "session-long-secret", *backend);
            let mut first = a_visitor();
            sealed_with(
                &mut first,
                super::nvs_core_session_set_long_lived_secret,
                "refresh",
                PLAIN,
            );
            sealed_with(
                &mut first,
                super::nvs_core_session_set_short_lived_secret,
                "login",
                "sess-9",
            );
            let id = first.session().expect("the session is open").id.clone();
            ended(first, &store);

            let kept = stored_under(&store, &id).expect("the session is kept");
            assert!(
                kept.iter().all(|(key, value)| {
                    ![key, value].iter().any(|bytes| {
                        bytes
                            .windows(PLAIN.len())
                            .any(|window| window == PLAIN.as_bytes())
                    })
                }),
                "{word}: the store has no plaintext"
            );

            for visit in 1..=2 {
                let mut restarted = returning(&store, &id, None, false);
                assert_eq!(
                    secret_got(&mut restarted, b"refresh", &[NEW]).expect("the secret reads"),
                    Some(PLAIN.to_owned()),
                    "{word}: new browser session {visit} opens the long-lived secret"
                );
                assert_eq!(
                    secret_got(&mut restarted, b"login", &[NEW]).expect("the secret reads"),
                    None,
                    "{word}: the short-lived secret is gone"
                );
                assert!(
                    read(&restarted, "get", b"refresh")
                        .expect("an absent key reads")
                        .is_none(),
                    "{word}: `get` cannot read the secret"
                );
                ended(restarted, &store);
            }
        }
    }

    thread_local! {
        /// What each write member threw inside [`writes_the_session`]: its name, and the class of
        /// the throw or `None` where it did not throw.
        static THROWN: std::cell::RefCell<Vec<(&'static str, Option<String>)>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }

    /// Calls every member that writes the session on `ctx`, and notes in [`THROWN`] what each
    /// threw.
    fn every_write(ctx: &mut Ctx) {
        let ring = crate::keyring::tests::ring_of(&[NEW]);
        let (key, value, site) = (Value::text("cart"), Value::text("9"), Value::int(0));
        let calls: [(&'static str, nvs_runtime::NvsFn, &[Value]); 9] = [
            (
                "setShortLived",
                super::nvs_core_session_set_short_lived,
                &[key, value, site],
            ),
            (
                "setLongLived",
                super::nvs_core_session_set_long_lived,
                &[key, value, site],
            ),
            ("remove", super::nvs_core_session_remove, &[key, site]),
            (
                "setShortLivedSecret",
                super::nvs_core_session_set_short_lived_secret,
                &[key, value, ring, site],
            ),
            (
                "setLongLivedSecret",
                super::nvs_core_session_set_long_lived_secret,
                &[key, value, ring, site],
            ),
            (
                "clearShortLived",
                super::nvs_core_session_clear_short_lived,
                &[],
            ),
            (
                "clearLongLived",
                super::nvs_core_session_clear_long_lived,
                &[],
            ),
            ("clearAll", super::nvs_core_session_clear_all, &[]),
            ("regenerate", super::nvs_core_session_regenerate, &[]),
        ];
        for (name, member, args) in calls {
            let class = match nvs_runtime::call(member, ctx, args) {
                Ok(answer) => {
                    dropped(answer);
                    None
                }
                Err(_) => {
                    let class = ctx.pending_class().unwrap_or_default();
                    drop(ctx.take_pending());
                    Some(class)
                }
            };
            THROWN.with_borrow_mut(|thrown| thrown.push((name, class)));
        }
        [key, value, ring].into_iter().for_each(dropped);
    }

    /// A deferred function's entry point, as compiled code has one: it runs [`every_write`].
    #[expect(
        unsafe_code,
        reason = "`call_callable` passes one live value this callee owes a release, and the \
                  address of a live `Value` for the result"
    )]
    unsafe extern "C" fn writes_the_session(
        ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        unsafe {
            (*args).release();
            every_write(&mut *ctx);
            *out = Value::null();
        }
        nvs_runtime::OK
    }

    /// A callable with no parameters that calls `invoke`, owned by the caller:
    /// `crates/nvs-stdlib/src/task.rs`'s `callable_of`, whose doc comment says why the table leaks.
    fn callable_of(invoke: nvs_runtime::NvsFn) -> Value {
        let mut table = nvs_runtime::ClassTable::new();
        let id = table.define("{callable}", &["arity", "params"], &[]);
        table.set_methods(
            id,
            vec![nvs_runtime::MethodRow {
                name: nvs_runtime::CALLABLE_INVOKE.to_owned(),
                code: invoke as *const u8,
                arity: 0,
                param_tags: 0,
                param_names: Vec::new(),
                param_types: Vec::new(),
                public: true,
                protected: false,
                native: false,
                param_classes: Vec::new(),
                param_checks: Vec::new(),
                is_static: false,
                by_reference: false,
            }],
        );
        table.set_callable(id);
        let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table is leaked, so the descriptor outlives every instance made from \
                      it, which is `NvsObj::new`'s whole obligation"
        )]
        let object = unsafe { nvs_runtime::NvsObj::new(table.desc(id)) };
        object.set_field(nvs_runtime::CALLABLE_ARITY_SLOT, Value::int(0));
        object.set_field(nvs_runtime::CALLABLE_PARAM_TAGS_SLOT, Value::int(0));
        Value::object(object)
    }

    /// `rule:concurrency/deferred-work-cannot-write-the-response`: every member that writes the
    /// session throws `LogicError` inside a `Core\Html::later` function and after the response.
    /// The session already has an identifier and a short token, so no write would draw a cookie,
    /// and it is left as it was.
    #[test]
    fn a_session_write_inside_later_or_after_response_throws_for_an_existing_session_too() {
        let (_, long_lived) = lifetimes();
        for place in ["later", "afterResponse"] {
            let mut ctx = Ctx::buffered();
            ctx.open_session(nvs_runtime::Session {
                long: longs(&["consent"]),
                short_token: Some(SHORT_TOKEN.to_owned()),
                long_until: crate::queue::now_millis() + long_lived,
                ..loaded(ints(&[("cart", 3), ("consent", 1)]))
            });
            THROWN.with_borrow_mut(Vec::clear);
            if place == "later" {
                let marker = ctx.register_later(callable_of(writes_the_session), b"", b"", None);
                ctx.write_output(&marker).expect("a buffer");
                nvs_runtime::later::run_later(&mut ctx);
            } else {
                ctx.defer(callable_of(writes_the_session), 0)
                    .expect("the request may defer work");
                nvs_runtime::deferred::run_deferred(&mut ctx);
            }

            let thrown = THROWN.with_borrow_mut(std::mem::take);
            assert_eq!(thrown.len(), 9, "{place}: every write member ran");
            for (member, class) in thrown {
                assert_eq!(
                    class.as_deref(),
                    Some("LogicError"),
                    "{place}: `{member}` throws `LogicError`"
                );
            }
            let open = ctx.session().expect("the session stays open");
            assert_eq!(open.id, ID, "{place}: the identifier is the same");
            assert_eq!(
                open.entries,
                ints(&[("cart", 3), ("consent", 1)]),
                "{place}: the values are the same"
            );
            assert!(
                !open.dirty && open.touched.is_empty(),
                "{place}: nothing is owed to the store"
            );
            assert!(ctx.take_headers().is_empty(), "{place}: no cookie is sent");
        }
    }
}
