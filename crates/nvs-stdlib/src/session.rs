//! `rule:http-server/a-session-store-answers-four-operations`'s session store:
//! the identifier a store issues, the record it keeps under it, and the two directives that decide
//! where that store is and how long a record survives.
//!
//! **What is on disk is the store `rule:http-server/a-session-store-answers-four-operations` names
//! and all seven members of `rule:core-api/session-roster`.** The four operations a backend
//! answers are here — [`mint`], [`load`], [`create`], [`replace`], [`destroy`] and the key they
//! share — over the shared tier's wire, and [`sqlite`]'s over the data folder's SQLite file.
//! `create` is the only write that inserts a row, and `replace` changes only a row that still
//! exists, so a destroyed session stays destroyed. [`opened`] picks
//! one as a [`Store`], and [`nvs_core_session_start`] is the member that reaches it: it takes the
//! identifier the client presented, loads the record the store issued it for, and opens a fresh,
//! empty one where there is none, which reaches the store only once something is written to it.
//! The other six operate on the record `start` left on the request
//! ([`nvs_runtime::Session`]), and each of them throws until `start` has run — the whole benefit
//! `rule:core-classes/session-is-started-explicitly` was buying, and worth nothing if
//! the first `get` can silently start one.
//!
//! **The write-back of `rule:http-server/a-session-is-loaded-once-and-written-whole` is on disk
//! too.** [`write_back`] marks the record changed on the request,
//! [`send_at_end`] sends it when the program that opened it ends, and
//! [`nvs_core_session_regenerate`] and [`nvs_core_session_destroy`] reach the store as they land
//! because that rule makes those two immediate. The section below is where the send is decided.
//!
//! **`db` is the default store.** A tree that writes no `[session] backend` gets the data folder's
//! `session.sqlite` (`rule:http-server/no-session-block-means-the-data-folder-store`), and a
//! process with no usable data folder throws at `start` naming `[session]`. [`sqlite`]'s module
//! doc owns the file's schema, its sweep, and why a request never waits on another.
//!
//! # Decision: a dirty record is sent by the *program's* end, and it travels there itself
//!
//! `rule:http-server/a-session-is-loaded-once-and-written-whole` writes the record back when the
//! request ends and leaves open who does it. Three
//! crates have a claim and none of them can hold it alone: this one owns the store and cannot see
//! a request end; `nvs-host` ends every isolate and may not name `nvs-stdlib`; `nvs-server` ends
//! the HTTP request and names neither. So the send is a `fn` pointer that rides on the record —
//! [`nvs_runtime::Session::write_back`], filled in here by `start` and `regenerate`, called
//! through `Ctx::end_session` by whoever ends the program. That method's own docs own the
//! mechanism; what is decided here is the *where*, and it is three refusals:
//!
//! **Not the door.** The obvious reading of that rule puts the send in `nvs-server`, at the line where a
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
//! `rule:http-server/a-session-store-answers-four-operations` refuses a separate `validateId`,
//! because a second question is a second thing that can
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
//! a request that changed the record (`rule:http-server/a-session-is-loaded-once-and-written-whole`).
//! A request that presents no identifier spends a draw
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
use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

mod sqlite;

/// The class name, once, for the messages that all name it.
pub(crate) const NAME: &str = r"Core\Session";

/// `Core\Session`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Keeps values for one visitor from one request to the next. Call `start()` first, then \
            use `get()` and `set()`. By default, the values are saved in a SQLite file in the data \
            folder. `[session] backend = \"shared\"` saves them in the shared cache instead.",
};

/// `rule:core-api/session-roster`'s roster, of which `start` is the member that talks to the store.
///
/// One class and no instance side: a session is the request's, not an object a program holds, so
/// there is nothing for a handle to be and nothing to hand back. Every member is static for that
/// reason, and this is that rule's list in its order, with the sealed pair
/// `rule:http-server/a-session-holds-a-secret-only-sealed` adds standing after it.
///
/// The third half of one request: what arrived, what goes back, and
/// what is remembered between the two. `rule:core-api/session-roster`'s roster, of which
/// `start` is the member that talks to the store — [`crate::session`]'s own
/// module doc owns which of
/// `rule:http-server/a-session-store-answers-four-operations`'s four operations are on disk, why the local
/// tier is unreachable from there rather than merely unselected, and why a
/// presented identifier this store cannot have issued is *absent* rather
/// than a second question about validity.
// rank: 780
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "start",
            names: &["presented"],
            // `?tainted string` as this registry spells it: `Nullable` for the `?`, and
            // `Text(Qual::Neutral)` for the rest — `CoreTy::TaintedStr` is return position only,
            // and a `Qual` is what says a `tainted` argument is admitted here (`rule:security/unclassified-parameter-refuses-tainted`). It
            // is admitted because the strict-id rule
            // (`rule:http-server/a-session-store-answers-four-operations`) makes the value a lookup key and never an
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
    short: "Opens the session of this request. Call it once, before any other method of this \
            class. The session identifier is read from the session cookie.",
    params: &[ParamDoc {
        name: "presented",
        desc: "The identifier to open. Leave it out to read it from the cookie named by \
               `[session] cookie`, which is `nvsid` by default. If the store has no session \
               under the identifier, a new, empty session is opened. This happens when the \
               identifier expired, and when somebody made it up.",
        shape: &[],
    }],
    ret: "Nothing. After it, the other methods of this class can read and change the session. \
          A new session is not saved, and no cookie is sent, until the program saves a value in \
          it. Then the response sends its identifier in the session cookie.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The program is not handling a request. A command-line program, a job and a \
                   test have no session.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request already called `start()`. It is also thrown when the server has \
                   no data folder for the default store. With `[session] backend = \"shared\"`, \
                   it is thrown when `[cache.shared] url` is not set, or the program does not have \
                   the `cache.shared` capability.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The store cannot be reached. The user may still have a session there, so do \
                   not treat them as signed out.",
        },
    ],
};

/// `Core\Session::get`'s reference card — `rule:core-api/reference-card`.
const GET_DOC: MethodDoc = MethodDoc {
    short: "Returns the value saved under one key in the session of this request.",
    params: &[ParamDoc {
        name: "key",
        desc: "The key to read. If the session has no value under it, the result is `null`. A key \
               that was set to `null` gives the same result.",
        shape: &[],
    }],
    ret: "The value saved under `$key`, or `null`. Reading does not change the session, so a \
          request that only reads its session does not write it back to the store.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, or it called `destroy()`. There is no \
                   session to read.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "The saved session contains an object of a class this program does not declare. \
                   This happens when another program saved it.",
        },
    ],
};

/// `Core\Session::set`'s reference card — `rule:core-api/reference-card`.
const SET_DOC: MethodDoc = MethodDoc {
    short: "Saves a value under one key in the session of this request. If the key already has a \
            value, it is replaced.",
    params: &[
        ParamDoc {
            name: "key",
            desc: "The key to save the value under.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The value to save: a string, a number, a bool, `null`, an array or an object. \
                   A `tainted` value (text that came from the request) is not allowed here, so \
                   the call does not compile.",
            shape: &[],
        },
    ],
    ret: "Nothing. The session is saved to the store once, when the request ends. The first \
          value saved in a new session also sends the session cookie.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, or it called `destroy()`. There is no \
                   session to save into. It is also thrown on the first save in a new session \
                   when `[session] cookie` is not a valid cookie name.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$value` cannot be saved: it is a callable or a resource, or it contains one. \
                   It is also thrown on the first save in a new session after the response was \
                   sent, because the cookie cannot be sent then.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "As `get()`, because `set()` reads the whole session first.",
        },
    ],
};

/// `Core\Session::remove`'s reference card — `rule:core-api/reference-card`.
const REMOVE_DOC: MethodDoc = MethodDoc {
    short: "Deletes one key and its value from the session of this request.",
    params: &[ParamDoc {
        name: "key",
        desc: "The key to delete. If the session has no value under it, `remove()` does not throw \
               an error and the session does not change.",
        shape: &[],
    }],
    ret: "Nothing. After it, `get()` returns `null` for `$key`. The other keys keep their values. \
          To delete every key, use `clear()`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, or it called `destroy()`. There is no \
                   session to change.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "As `set()`. The values that are left are saved again, and a value `set()` does \
                   not allow cannot be saved.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "As `get()`, because `remove()` reads the whole session first.",
        },
    ],
};

/// `Core\Session::clear`'s reference card — `rule:core-api/reference-card`.
const CLEAR_DOC: MethodDoc = MethodDoc {
    short: "Removes every value from the session of this request. The session stays open and \
            keeps its identifier.",
    params: &[],
    ret: "Nothing. The client keeps the same session cookie, and `set()` works again at once. To \
          end the session completely, for example when a user signs out, use `destroy()`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "This request has not called `start()`, or it called `destroy()`. There is no \
               session to empty.",
    }],
};

/// `Core\Session::regenerate`'s reference card — `rule:core-api/reference-card`.
const REGENERATE_DOC: MethodDoc = MethodDoc {
    short: "Gives the session a new identifier and deletes the old one from the store. Call it \
            when a user signs in, so an identifier that somebody else knew stops working.",
    params: &[],
    ret: "Nothing. The data in the session does not change. The response sends the new identifier \
          in the session cookie. The old identifier no longer finds the session.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, or it called `destroy()`. With \
                   `[session] backend = \"shared\"`, it is also thrown when `[cache.shared] url` \
                   is not set or the program does not have the `cache.shared` capability. It is \
                   also thrown when `[session] cookie` is not a valid cookie name.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The store cannot be reached. The old identifier may still work, so do not \
                   treat the user as signed in.",
        },
    ],
};

/// `Core\Session::destroy`'s reference card — `rule:core-api/reference-card`.
const DESTROY_DOC: MethodDoc = MethodDoc {
    short: "Ends the session of this request and deletes its data from the store. Call it when a \
            user signs out.",
    params: &[],
    ret: "Nothing. After it, this request has no session. The other methods of this class throw \
          a `RuntimeError` until you call `start()` again, and that call opens a new, empty session.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, or it already called `destroy()`. With \
                   `[session] backend = \"shared\"`, it is also thrown when `[cache.shared] url` \
                   is not set or the program does not have the `cache.shared` capability.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The store cannot be reached. The data is still in the store, and the session \
                   stays open, so you can try again.",
        },
    ],
};

/// `Core\Session::setSecret`'s reference card — `rule:core-api/reference-card`.
const SET_SECRET_DOC: MethodDoc = MethodDoc {
    short: "Saves a secret, such as an access token, in the session of this request. The secret \
            is encrypted with a key from `$keys`, so the store only sees encrypted bytes.",
    params: &[
        ParamDoc {
            name: "key",
            desc: "The key to save the secret under. `get()` returns `null` for this key. Only \
                   `getSecret()` with the same `$keys` can read the secret.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The secret to save, for example an access token or a refresh token of the user.",
            shape: &[],
        },
        ParamDoc {
            name: "keys",
            desc: "A list of encryption keys, newest first. `Core\\Crypto::generateKey()` creates \
                   one. The first key encrypts the secret.",
            shape: &[],
        },
    ],
    ret: "Nothing. The session is saved to the store once, when the request ends.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, or it called `destroy()`. There is no \
                   session to save into.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$keys` is empty, or its first key is not 32 bytes long.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "As `get()`, because `setSecret()` reads the whole session first.",
        },
    ],
};

/// `Core\Session::getSecret`'s reference card — `rule:core-api/reference-card`.
const GET_SECRET_DOC: MethodDoc = MethodDoc {
    short: "Reads a secret that `setSecret()` saved in the session of this request. If the \
            secret cannot be decrypted with `$keys`, the result is `null`.",
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
          changed. Reading does not change the session.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This request has not called `start()`, or it called `destroy()`. There is no \
                   session to read.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$keys` is empty, or a key in it is not 32 bytes long.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "As `get()`, because `getSecret()` reads the whole session first.",
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

/// `[session] backend` — `rule:http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot`'s store, as `nvs_config::session::Backend` spells it.
const BACKEND: &str = "session.backend";

/// `[session] ttl` — how long an untouched record survives.
const TTL: &str = "session.ttl";

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
const DEFAULT_TTL: Duration = nvs_config::defaults::SESSION_TTL;

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

/// The store this deployment's records live in, ready for one of the four operations.
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
fn fetched(store: &Store, app: &[u8], member: &str, id: &str) -> Result<Option<Vec<u8>>, Fault> {
    match store {
        Store::Shared => on_shared(NAME, member, |open| load(open, app, id)),
        Store::File(path) => on_file(member, sqlite::load(path, app, id)),
    }
}

/// `create` on `store`, as [`fetched`]: `true` once a new row holds `record`, and `false` for an
/// `id` that already has a live row, which keeps its record.
///
/// # Errors
///
/// As [`fetched`].
fn created(
    store: &Store,
    app: &[u8],
    member: &str,
    id: &str,
    record: &[u8],
    ttl: Duration,
) -> Result<bool, Fault> {
    match store {
        Store::Shared => on_shared(NAME, member, |open| create(open, app, id, record, ttl)),
        Store::File(path) => on_file(member, sqlite::create(path, app, id, record, ttl)),
    }
}

/// `replace` on `store`, as [`fetched`]: `true` once the live row under `id` holds `record`, and
/// `false` for an `id` with no live row, which stays absent.
///
/// # Errors
///
/// As [`fetched`].
fn replaced(
    store: &Store,
    app: &[u8],
    member: &str,
    id: &str,
    record: &[u8],
    ttl: Duration,
) -> Result<bool, Fault> {
    match store {
        Store::Shared => on_shared(NAME, member, |open| replace(open, app, id, record, ttl)),
        Store::File(path) => on_file(member, sqlite::replace(path, app, id, record, ttl)),
    }
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

/// `rule:http-server/a-session-store-answers-four-operations`'s `load` — the record under `id`, or
/// **absent**.
///
/// Absent is the answer to all three of
/// `rule:security/a-session-id-the-store-did-not-issue-is-rejected`
/// 's cases at once: an id no store issued, one that has expired, and one an attacker minted.
/// `Core\Session::start()` responds to it by issuing a fresh id, which is why there is no separate
/// `validateId` for the two of them to disagree about.
///
/// # Errors
///
/// The store's own failure as text, for [`crate::cache::on_shared`] to classify. Never an answer
/// that looks like absence: a store that cannot be reached must not read as a forged id, because
/// the two have opposite responses.
pub(crate) fn load(open: &mut Connection, app: &[u8], id: &str) -> Result<Option<Vec<u8>>, String> {
    open.get(&key_of(app, id))
}

/// `rule:http-server/a-session-store-answers-four-operations`'s `destroy` — the record under `id`,
/// forgotten, and whether there was one.
///
/// One key, as [`create`] writes one: a session is one entry, so there is nothing here that a
/// store with no multi-key atomic step could not do. `false` tells `regenerate` that another
/// request destroyed the session first.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn destroy(open: &mut Connection, app: &[u8], id: &str) -> Result<bool, String> {
    open.del(&key_of(app, id))
}

/// `rule:http-server/a-session-store-answers-four-operations`'s `issue` — a new entry under `id`,
/// and whether it was written. `SET … NX PX`: `false` is a key that already holds a record, which
/// keeps it.
///
/// The whole record and not one key of it, per
/// `rule:http-server/a-session-is-loaded-once-and-written-whole`: one session is one entry, which
/// is what keeps `clear`, `regenerate` and `destroy` single-key operations over a store
/// `rule:core-classes/ratelimit-two-members` gives no multi-key atomic step.
///
/// The expiry is the store's own, per `rule:http-server/session-expiry-belongs-to-the-store`: a
/// record written with no expiry at all is the sweeper that rule declines coming back.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn create(
    open: &mut Connection,
    app: &[u8],
    id: &str,
    record: &[u8],
    ttl: Duration,
) -> Result<bool, String> {
    open.set_if_absent(&key_of(app, id), record, ttl)
}

/// `rule:http-server/a-session-store-answers-four-operations`'s `save` — the record under `id`
/// replaced, with its expiry refreshed, **only while the entry still exists**, and whether it did.
/// `SET … XX PX`: `false` is a key that is gone, and it stays gone.
///
/// That is what keeps a destroyed session destroyed: a request that loaded the session before
/// another request destroyed it cannot write the row back at its end. As [`create`] for the rest.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn replace(
    open: &mut Connection,
    app: &[u8],
    id: &str,
    record: &[u8],
    ttl: Duration,
) -> Result<bool, String> {
    open.set_if_present(&key_of(app, id), record, ttl)
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
    /// **Absent is one answer with one response.**
    /// `rule:http-server/a-session-store-answers-four-operations` gives `load` three ways to answer absent —
    /// an identifier the store never issued, one it issued and has since expired, and one an
    /// attacker minted — and this member does not distinguish them: each discards the presented
    /// identifier and opens a fresh, empty session. There is no `validateId` for the two to disagree about,
    /// and the pre-store shape check ([`issuable`]) is not one, for the reason the module doc
    /// gives.
    ///
    /// **A store that cannot be reached throws.** [`on_shared`] classifies it as an `IOError`
    /// rather than answering as though the record were absent, because absence means *issue a new
    /// session* — so a store that is down would silently sign every user out and look like a
    /// forged identifier while doing it.
    ///
    /// **A fresh session has no identifier until something is kept in it.** `start` writes no
    /// row and sends no cookie for one; [`write_back`] draws the identifier and sends the cookie
    /// on the first write that leaves the record non-empty, and the end-of-request write-back
    /// creates the row. So the store holds sessions somebody wrote, and a flood of cookieless
    /// requests that only read leaves nothing behind (`rule:core-classes/session-is-started-explicitly`).
    /// `regenerate` on such a session does nothing, and `destroy` reaches no store.
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
    /// `RuntimeError` for a second `start` on one request, for the `db` store in a process with
    /// no usable data folder, and for a shared store that is unconfigured or refused by capability;
    /// a thrown `IOError` for a store that cannot be reached.
    fn nvs_core_session_start(ctx, args: [1]) {
        crate::request::served(ctx, &format!("{NAME}::start"))?;

        if ctx.session().is_some() {
            return Err(Fault::thrown(format!(
                "{NAME}::start(): this request has already started a session — a second `start()` \
                 would discard whatever the first one's record has collected since, so the member \
                 that deliberately replaces a session is `regenerate()`"
            )));
        }

        let member = format!("{NAME}::start()");
        let store = opened(
            ctx,
            &member,
            ". A session has to be in a store that every server process can read",
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
            Some(id) => fetched(&store, crate::cache::application(ctx), "start", &id)?
                .map(|record| nvs_runtime::Session {
                    id,
                    record,
                    dirty: false,
                    stored: true,
                    write_back: send_at_end,
                }),
            None => None,
        };

        // A visitor with no record gets an empty session with no identifier yet. Nothing reaches
        // the store and no cookie is sent until [`write_back`] keeps something in it, so a
        // request that only reads costs the store nothing.
        let session = opened.unwrap_or_else(fresh_session);
        ctx.open_session(session);
        Ok(Value::null())
    }
}

/// An empty session with no identifier and no row in the store: what `start` opens for a visitor
/// with no record, and what `regenerate` leaves when another request destroyed the session first.
fn fresh_session() -> nvs_runtime::Session {
    nvs_runtime::Session {
        id: String::new(),
        record: Vec::new(),
        dirty: false,
        stored: false,
        write_back: send_at_end,
    }
}

/// The `Set-Cookie` line carrying `id`, under `[session] cookie` and `[http.cookies]`' policy.
///
/// `who` is the member a refusal names, written whole (`Core\Session::set`), because
/// `Core\Csrf::token` draws an identifier too.
///
/// # Errors
///
/// A thrown `RuntimeError` for a `[session] cookie` that is not a cookie name. That is an
/// operator's typo rather than anything a program did, and it is refused here because the value
/// reaches a header line: a name carrying a `;` or a newline would end the line and begin one
/// nobody wrote. The identifier needs no such check — it is 22 characters this core drew.
fn issue_cookie(ctx: &mut Ctx, who: &str, id: &str) -> Result<(), Fault> {
    let name = cookie(ctx);
    if !crate::response::nameable(&name) {
        return Err(Fault::thrown(format!(
            "{who}(): `[session] cookie = \"{name}\"` is not a cookie name — a name is \
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
        "{NAME}::{member}(): this request has not started a session. Call `{NAME}::start()` \
         first."
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
/// and what it finds there is a value no ring opens. A plaintext that opens but is not UTF-8 is the
/// same miss too, because a `string` must be UTF-8.
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
        if let Some(plain) = crate::crypto::open_under(&cipher, &aad, sealed, who)?
            && let Ok(text) = std::str::from_utf8(&plain)
        {
            return Ok(Some(Value::text(text)));
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

    let decoded = nvs_runtime::decode(&session.record, &ctx.receiving()).map_err(|why| {
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

/// `rule:http-server/a-session-is-loaded-once-and-written-whole`'s "mutate the copy in the request's
/// own heap": `record` encoded back onto the request, and
/// marked as owing the store a write.
///
/// **The record is written whole**, per that rule — one session is one entry, so there is no key-wise
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
    if session.id.is_empty() {
        if payload.is_empty() {
            // Still empty, so there is still nothing to keep and nothing to send.
            session.record = payload;
            return Ok(Value::null());
        }
        // The first write to a session `start` opened with no record: the identifier is drawn
        // and sent now, before the record changes, so a cookie that cannot be sent leaves the
        // session as it was.
        crate::response::head_open(ctx, &format!("{NAME}::{member}"))?;
        let id = mint(ctx);
        issue_cookie(ctx, &format!("{NAME}::{member}"), &id)?;
        let session = ctx.session_mut().ok_or_else(|| unstarted(member))?;
        session.id = id;
        session.record = payload;
        session.dirty = true;
        return Ok(Value::null());
    }
    session.record = payload;
    session.dirty = true;
    Ok(Value::null())
}

/// The identifier the response sends for this request's session, with the session marked
/// changed. `Core\Csrf::token` binds its token to it.
///
/// **A session `start` opened with no identifier draws one here**, and sends its cookie, as its
/// first [`write_back`] would. So the token and the cookie name the same session, and the door
/// accepts the token on the next request (`rule:security/csrf-is-on-by-default`).
///
/// **The record is marked changed even while it is empty**, so the write-back stores a row under
/// the identifier and the next request's `start` finds it rather than opening a fresh one. What
/// it spends is one row per visitor who is shown a token, which `[session] ttl` expires, and one
/// store write per such request.
///
/// # Errors
///
/// A thrown `RuntimeError` naming `Core\Session::start` while this request has started no session,
/// and, for a session with no identifier yet, [`crate::response::head_open`]'s and
/// [`issue_cookie`]'s.
pub(crate) fn sent_id(ctx: &mut Ctx, who: &str) -> Result<String, Fault> {
    let unstarted = || {
        Fault::thrown(format!(
            "{who}(): this request has not started a session. Call `{NAME}::start()` first, \
             because a token is bound to the session."
        ))
    };
    let fresh = ctx.session().ok_or_else(unstarted)?.id.is_empty();
    if fresh {
        // Drawn and sent before the session changes, so a cookie that cannot be sent leaves the
        // session as it was.
        crate::response::head_open(ctx, who)?;
        let id = mint(ctx);
        issue_cookie(ctx, who, &id)?;
        ctx.session_mut().ok_or_else(unstarted)?.id = id;
    }
    let session = ctx.session_mut().ok_or_else(unstarted)?;
    session.dirty = true;
    Ok(session.id.clone())
}

/// Saves `value` under `key` in the started session's record, for a `Core` class that keeps one
/// visitor's state between two of their requests: `Core\Passkey`'s challenge.
///
/// The caller checks that a session is started first and throws its own sentence, so this never
/// meets [`unstarted`] from a program. The record is written whole, as [`nvs_core_session_set`]
/// writes it.
///
/// # Errors
///
/// As [`nvs_core_session_set`]: [`record`]'s and [`write_back`]'s.
pub(crate) fn put_bytes(ctx: &mut Ctx, key: &str, value: &[u8]) -> Result<(), Fault> {
    let mut held = record(ctx, "set")?;
    held.set(
        NvsStr::new(key.as_bytes()),
        Value::bytes(NvsStr::new(value)),
    );
    write_back(ctx, held, "set").map(drop)
}

/// The `bytes` saved under `key` in the started session's record, taken out of it: [`put_bytes`]'s
/// other half, and what makes `Core\Passkey`'s challenge single-use.
///
/// The key is removed whatever it held, so a value that is not `bytes` is `None` and is gone
/// too. A key the record does not have is `None` and writes nothing, as
/// [`nvs_core_session_remove`] writes nothing.
///
/// # Errors
///
/// As [`nvs_core_session_remove`].
pub(crate) fn take_bytes(ctx: &mut Ctx, key: &str) -> Result<Option<Vec<u8>>, Fault> {
    let mut held = record(ctx, "remove")?;
    let Some(found) = held.get(key.as_bytes()) else {
        drop(held);
        return Ok(None);
    };
    let taken = found.as_bytes().map(<[u8]>::to_vec);
    held.unset(key.as_bytes());
    write_back(ctx, held, "remove")?;
    Ok(taken)
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
    /// **This member does not mark the record changed**, which is
    /// `rule:http-server/a-session-is-loaded-once-and-written-whole`'s "writing only when the
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
    /// **Nothing reaches the store here.** `rule:http-server/a-session-is-loaded-once-and-written-whole`
    /// loads the record once and writes it whole when the
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
    /// Nothing was written, so there is nothing for the write-back to send, and a `remove` that
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
        // An empty record cleared is not a change, so it does not earn the write the dirty flag is
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
    /// **In `rule:core-api/session-roster`'s order: issue, move the record, forget the old entry.**
    /// A failure between the
    /// second step and the third leaves the record readable under two identifiers until
    /// `[session] ttl` expires the old one; the same failure in the other order leaves it readable
    /// under none, which is a signed-out user. The first is the recoverable one, so it is the one
    /// this member risks. [`regenerated`] is the move, and it moves only a session whose old row
    /// still exists.
    ///
    /// **A session this request drew and has not stored yet makes no store call.** A new
    /// identifier is drawn and sent, and the write-back creates the row under it.
    ///
    /// **Immediately rather than at the end of the request** —
    /// `rule:http-server/a-session-is-loaded-once-and-written-whole` names this member and `destroy`
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
        crate::response::head_open(ctx, "Core\\Session::regenerate")?;
        let Some(open) = ctx.session() else {
            return Err(unstarted("regenerate"));
        };
        if open.id.is_empty() {
            // Nobody has written this session, so no client holds an identifier for it. The
            // first write draws a fresh one.
            return Ok(Value::null());
        }
        if !open.stored {
            // This request drew the identifier and the store has no row under it, so nothing
            // moves. The new identifier replaces it, and the write-back creates the row.
            let fresh = mint(ctx);
            issue_cookie(ctx, &format!("{NAME}::regenerate"), &fresh)?;
            if let Some(open) = ctx.session_mut() {
                open.id = fresh;
            }
            return Ok(Value::null());
        }

        let member = format!("{NAME}::regenerate()");
        let store = opened(ctx, &member, ", which is where this session's record already lives")?;
        regenerated(ctx, &store)?;
        Ok(Value::null())
    }
}

/// `regenerate`'s move of a stored session on `store`: the row is created under a fresh
/// identifier, then the old row is destroyed, and the cookie names the fresh one.
///
/// **A destroy that finds no old row means another request destroyed the session first.** The
/// fresh row is destroyed too, no cookie is sent, and the request is left with a fresh, empty
/// session with no identifier, as `start` opens for an absent one. So a session that was signed
/// out elsewhere never moves to a new identifier. What it spends: one more delete, in that race
/// alone.
///
/// # Errors
///
/// [`unstarted`] before `start`; the store's failure, as [`fetched`]'s; a thrown `RuntimeError`
/// where the store already has a row under the drawn identifier; and [`issue_cookie`]'s.
fn regenerated(ctx: &mut Ctx, store: &Store) -> Result<(), Fault> {
    let Some(open) = ctx.session() else {
        return Err(unstarted("regenerate"));
    };
    // Copied rather than taken, so a store that fails halfway leaves the request holding the
    // session it already had. O(record), which this member spends once and no other does.
    let (retired, record) = (open.id.clone(), open.record.clone());
    let fresh = mint(ctx);
    let lifetime = ttl(ctx);
    let app = crate::cache::application(ctx);
    if !created(store, app, "regenerate", &fresh, &record, lifetime)? {
        return Err(Fault::thrown(format!(
            "{NAME}::regenerate(): the store already has a session under the new identifier. \
             Call `regenerate()` again to draw another one."
        )));
    }
    if !forgot(store, app, "regenerate", &retired)? {
        forgot(store, app, "regenerate", &fresh)?;
        ctx.open_session(fresh_session());
        return Ok(());
    }
    issue_cookie(ctx, &format!("{NAME}::regenerate"), &fresh)?;

    // Not dirty: the record was written whole a line ago, so the write-back has nothing left
    // to send. `Ctx::open_session` replaces rather than refuses for exactly this call.
    ctx.open_session(nvs_runtime::Session {
        id: fresh,
        record,
        dirty: false,
        stored: true,
        write_back: send_at_end,
    });
    Ok(())
}

nvs_runtime::nvs_helper! {
    /// `Core\Session::destroy(): void` — `rule:core-api/session-roster` and `rule:http-server/a-session-is-loaded-once-and-written-whole`, replacing `session_destroy`.
    ///
    /// **The request is left with no session**, so every member of this class throws again
    /// afterwards — the same answer as before the first `start()`, because it is the same state.
    /// PHP leaves `$_SESSION` populated after `session_destroy()` and that is the divergence: a
    /// record that outlives the entry it came from is a copy of something that no longer exists,
    /// and the end-of-request write-back would put it straight back.
    ///
    /// **The store is told before the request forgets**, so a store that cannot be reached throws
    /// with the session still open rather than reporting a sign-out that did not happen.
    ///
    /// # Errors
    ///
    /// As [`nvs_core_session_regenerate`], less the cookie: no `Set-Cookie` is written, because the
    /// identifier the client holds now names nothing and a store that answers absent is already the
    /// whole of `rule:http-server/a-session-store-answers-four-operations`'s strict-id rule.
    fn nvs_core_session_destroy(ctx, _args: [0]) {
        let Some(open) = ctx.session() else {
            return Err(unstarted("destroy"));
        };
        let held = open.id.clone();

        // A session nobody wrote has no row to delete.
        if !held.is_empty() {
            let member = format!("{NAME}::destroy()");
            let store = opened(ctx, &member, ", which is where this session's record lives")?;
            forgot(&store, crate::cache::application(ctx), "destroy", &held)?;
        }
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
/// is the whole of `rule:http-server/a-session-is-loaded-once-and-written-whole`'s "writing only
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
/// **A stored session is replaced, and a session this request drew is created.** A replace changes
/// only a row that still exists. So when another request destroyed the session, or it expired,
/// nothing is written, this request's changes are dropped, and nothing is reported: that is what a
/// destroyed session means. A create sets [`nvs_runtime::Session::stored`], so a second end of the
/// same program replaces the row it made.
///
/// # Errors
///
/// The store's failure, as [`fetched`]'s, and a thrown `RuntimeError` where the store already has
/// a row under the identifier this request drew.
fn sent_to(ctx: &mut Ctx, store: &Store) -> Result<(), Fault> {
    let lifetime = ttl(ctx);
    let app = crate::cache::application(ctx);
    // Nothing is copied out of the record: the store calls borrow it, and the context is changed
    // only after the borrow has ended. A write-back that cloned the record would spend O(record)
    // on the request path to say exactly what the bytes already on the context say.
    let Some(open) = ctx.session().filter(|open| !open.id.is_empty()) else {
        return Ok(());
    };
    if open.stored {
        replaced(
            store,
            app,
            "the write-back",
            &open.id,
            &open.record,
            lifetime,
        )?;
        return Ok(());
    }
    if !created(
        store,
        app,
        "the write-back",
        &open.id,
        &open.record,
        lifetime,
    )? {
        return Err(Fault::thrown(format!(
            "{SENDER}: the store already has a session under the identifier this request drew"
        )));
    }
    if let Some(open) = ctx.session_mut() {
        open.stored = true;
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
    use std::collections::{HashMap, HashSet};
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    use nvs_config::session::{BACKENDS, Backend};
    use nvs_runtime::{Ctx, NvsArray, NvsStr, Value};

    use super::{
        DEFAULT_TTL, PREFIX, SEAL_DOMAIN, Store, backend, cookie, create, created, destroy,
        fetched, forgot, key_of, load, mint, opened, opened_from, record, regenerated, replaced,
        sealed_into, sealed_key, sent_to, sqlite, ttl, write_back,
    };
    use crate::cache::redis::Connection;
    use crate::cache::tests::memory_held;
    use crate::cache::{Dial, Target};

    /// The block that chooses the shared tier, which every case driving it through a member writes.
    const SHARED_BLOCK: &str = "[session]\nbackend = \"shared\"\n";

    /// The application of a context with no `[[app]]` block, which is every case's but one.
    const NO_APP: &[u8] = b"";

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
    /// Dropping it stops the accept loop and joins every thread it started, so a test ends with
    /// no thread of its own still running. It closes this thread's `crate::cache` connection
    /// first, which otherwise stays open until the thread exits. A test declares it before its
    /// own connections, so those close first too.
    struct Serving {
        address: SocketAddr,
        stop: Arc<AtomicBool>,
        accepting: Option<thread::JoinHandle<()>>,
        /// The keys whose expiry has passed. The store still has them, and answers each one as
        /// absent, as a real store answers an expired key.
        lapsed: Arc<Mutex<HashSet<Vec<u8>>>>,
    }

    impl Serving {
        fn start(listener: TcpListener, held: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>) -> Self {
            let address = listener.local_addr().expect("the port it took");
            let stop = Arc::new(AtomicBool::new(false));
            let stopping = Arc::clone(&stop);
            let lapsed = Arc::new(Mutex::new(HashSet::new()));
            let expired = Arc::clone(&lapsed);
            let accepting = thread::spawn(move || serving(&listener, &held, &expired, &stopping));
            Self {
                address,
                stop,
                accepting: Some(accepting),
                lapsed,
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
    /// `SET … PX` with `NX`, `XX` or neither, `GET` and `DEL` only, because those are the commands
    /// [`create`], [`super::replace`], [`load`] and [`destroy`] send. Anything else is a panic
    /// rather than a silent `+OK`, so a command added upstream fails here instead of passing
    /// untested. A key in `lapsed` is absent to every command, and `DEL` of it answers `0`.
    fn serving(
        listener: &TcpListener,
        held: &Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,
        lapsed: &Arc<Mutex<HashSet<Vec<u8>>>>,
        stop: &AtomicBool,
    ) {
        let mut connections = Vec::new();
        while let Ok((stream, _)) = listener.accept() {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let held = Arc::clone(held);
            let lapsed = Arc::clone(lapsed);
            let handler = thread::spawn(move || {
                let mut writing = stream.try_clone().expect("the write half");
                let mut reading = BufReader::new(stream);
                while let Some(parts) = command(&mut reading) {
                    let reply = match parts[0].as_slice() {
                        b"SET" => {
                            let (condition, px) = match parts[3].as_slice() {
                                b"PX" => (None, 3),
                                word => (Some(word.to_vec()), 4),
                            };
                            assert_eq!(
                                parts[px], b"PX",
                                "a record is always written with an expiry"
                            );
                            let mut store = held.lock().expect("the store");
                            if lapsed.lock().expect("the lapsed keys").remove(&parts[1]) {
                                store.remove(&parts[1]);
                            }
                            let present = store.contains_key(&parts[1]);
                            let writes = match condition.as_deref() {
                                None => true,
                                Some(b"NX") => !present,
                                Some(b"XX") => present,
                                Some(other) => panic!(
                                    "the session store sends no SET {:?}",
                                    String::from_utf8_lossy(other)
                                ),
                            };
                            if writes {
                                store.insert(parts[1].clone(), parts[2].clone());
                                b"+OK\r\n".to_vec()
                            } else {
                                b"$-1\r\n".to_vec()
                            }
                        }
                        b"GET" if lapsed.lock().expect("the lapsed keys").contains(&parts[1]) => {
                            b"$-1\r\n".to_vec()
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
                            let expired = lapsed.lock().expect("the lapsed keys").remove(&parts[1]);
                            let gone = held.lock().expect("the store").remove(&parts[1]);
                            format!(":{}\r\n", usize::from(gone.is_some() && !expired)).into_bytes()
                        }
                        other => panic!(
                            "the session store sends no {:?}",
                            String::from_utf8_lossy(other)
                        ),
                    };
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

    /// `rule:concurrency/the-memory-tier-cannot-hold-what-must-be-coherent`, as the roster and as the write.
    ///
    /// Three claims, because each alone passes something broken. The roster admits no memory tier —
    /// so no later reader can select one — and it is read from `nvs_config` rather than spelled
    /// here, so adding a variant there fails this. The word is refused by `Backend::of` rather than
    /// resolving to something. And a record actually written through [`create`] is **absent from the
    /// process's map** afterwards: a module that wrote into the memory tier on some branch would
    /// satisfy the first two and fail this one, which is the failure the section is about.
    #[test]
    fn a_session_is_never_backed_by_the_memory_cache_tier() {
        assert!(
            BACKENDS.iter().all(|(word, _)| *word != "memory"),
            "`rule:concurrency/the-memory-tier-cannot-hold-what-must-be-coherent` is enforced by the roster having no memory entry: {BACKENDS:?}"
        );
        assert_eq!(Backend::of("memory"), None);

        let (listener, address) = listening();
        let held = Arc::new(Mutex::new(HashMap::new()));
        let store = Arc::clone(&held);
        let _serving = Serving::start(listener, store);

        let mut open = Connection::new(
            Dial::configured(Target::Tcp(address), None, None),
            Duration::from_secs(5),
        );
        assert!(
            create(&mut open, NO_APP, ID, RECORD, DEFAULT_TTL)
                .expect("the record reaches the configured store")
        );

        assert_eq!(
            held.lock()
                .expect("the store")
                .get(&key_of(NO_APP, ID))
                .cloned(),
            Some(RECORD.to_vec()),
            "the record is in the shared store"
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
        let held: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
        let _serving = Serving::start(listener, held);

        let wrote = thread::spawn(move || {
            let mut open = Connection::new(
                Dial::configured(Target::Tcp(address), None, None),
                Duration::from_secs(5),
            );
            assert!(
                create(&mut open, NO_APP, ID, RECORD, DEFAULT_TTL).expect("the record is written")
            );
        });
        wrote.join().expect("the first core");

        let read = thread::spawn(move || {
            let mut open = Connection::new(
                Dial::configured(Target::Tcp(address), None, None),
                Duration::from_secs(5),
            );
            load(&mut open, NO_APP, ID).expect("the store answers")
        });
        let shared = read.join().expect("the second core");

        assert_eq!(
            shared,
            Some(RECORD.to_vec()),
            "the session the first core wrote is the session the second core reads"
        );
    }

    /// `rule:core-api/session-roster`'s read and write over one record, as
    /// `rule:http-server/a-session-is-loaded-once-and-written-whole` keeps it: what `set` put there,
    /// `get` reads back, and
    /// `remove` takes out again — with the record's own bytes asserted beside the answers.
    ///
    /// **Here rather than in a `.nvst` case**, because no conformance case can reach a session
    /// store and none of this needs one: the record is loaded once and every change until the end
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
            stored: true,
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
            "the end-of-request write-back is what the flag earns, and a `set` that left it clear would be a \
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

    /// `clear` empties the record and keeps the session: the identifier is the one the request
    /// had, the record is zero bytes and marked changed, and a clear of an empty record changes
    /// nothing, the flag included.
    ///
    /// The record starts as bytes no decode accepts, because `clear` is the one member that does
    /// not read the record, and a record this unit cannot decode is exactly what it recovers from.
    /// Before a session is open, and after it, the member throws.
    // covers: Core\Session::clear
    #[test]
    fn clear_empties_the_record_and_keeps_the_session_open() {
        let mut ctx = Ctx::buffered();
        assert!(
            nvs_runtime::call(super::nvs_core_session_clear, &mut ctx, &[]).is_err(),
            "a request that never started a session has no record to empty"
        );
        drop(ctx.take_pending());

        ctx.open_session(nvs_runtime::Session {
            id: ID.to_owned(),
            stored: true,
            record: b"no decode accepts this".to_vec(),
            dirty: false,
            write_back: super::send_at_end,
        });
        nvs_runtime::call(super::nvs_core_session_clear, &mut ctx, &[])
            .expect("a record this unit cannot decode is still clearable");

        let cleared = ctx.session().expect("the session is still open");
        assert_eq!(
            cleared.id, ID,
            "clearing keeps the identifier the client holds"
        );
        assert!(cleared.record.is_empty(), "the record is zero bytes");
        assert!(
            cleared.dirty,
            "an emptied record is a change the store must hear about"
        );

        ctx.session_mut().expect("the session is still open").dirty = false;
        nvs_runtime::call(super::nvs_core_session_clear, &mut ctx, &[])
            .expect("an empty record is clearable too");
        assert!(
            !ctx.session().expect("the session is still open").dirty,
            "clearing an empty record is not a change, so it earns no write"
        );

        ctx.close_session();
        assert!(
            nvs_runtime::call(super::nvs_core_session_clear, &mut ctx, &[]).is_err(),
            "a closed session throws again, as before `start`"
        );
    }

    /// `rule:http-server/a-session-is-loaded-once-and-written-whole`'s write-back, asserted **both
    /// ways**: the send a changed record earns, and the round
    /// trip a read-only request does not make.
    ///
    /// One direction alone passes something broken. A build that sent unconditionally would
    /// satisfy "a `set` reaches the store" while spending a write on every request that read its
    /// session — the cost that rule names when it says only a changed record is written — and a build
    /// that never sent would satisfy "a read-only request makes one round trip" perfectly.
    ///
    /// **The flag is earned through the real member path**, by the same [`write_back`] `set` calls,
    /// rather than written by hand: a `set` that stopped marking the record would pass a test that
    /// set `dirty` itself and lose every write in production.
    ///
    /// **The send is a test's own function pointer**, which is what the seam is for: whether the
    /// bytes reach a store is [`create`]'s question and three cases above already ask it, while what
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
            stored: true,
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
            "a request that only read its session makes one round trip, not two — the whole \
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

    /// The fourth operation of `rule:http-server/a-session-store-answers-four-operations`: after
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
        let held: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
        let _serving = Serving::start(listener, held);

        let answered = thread::spawn(move || {
            let mut open = Connection::new(
                Dial::configured(Target::Tcp(address), None, None),
                Duration::from_secs(5),
            );
            assert!(
                create(&mut open, NO_APP, ID, RECORD, DEFAULT_TTL).expect("the record is written")
            );
            let before = load(&mut open, NO_APP, ID).expect("the store answers");
            let first = destroy(&mut open, NO_APP, ID).expect("the record is forgotten");
            let after = load(&mut open, NO_APP, ID).expect("the store answers");
            let second = destroy(&mut open, NO_APP, ID)
                .expect("an id with no record under it is already forgotten");
            (before, after, first, second)
        });
        let (before, after, first, second) = answered.join().expect("the core");

        assert_eq!(before, Some(RECORD.to_vec()));
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
        Shared(Serving, Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>),
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

    /// `rule:http-server/a-session-is-loaded-once-and-written-whole`: a request loads a stored
    /// session and changes it, another request destroys the session, and then the first request
    /// ends. On every store the end writes nothing, so the session stays destroyed.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn a_destroyed_session_is_not_written_back_on_any_store() {
        for (word, backend) in BACKENDS {
            let mut ctx = Ctx::buffered();
            let (store, kept) = a_store(&mut ctx, "session-destroyed-stays", *backend);
            assert!(
                created(&store, NO_APP, "a case", ID, &[], DEFAULT_TTL).expect("the row is issued")
            );
            let loaded = fetched(&store, NO_APP, "a case", ID)
                .expect("the store answers")
                .expect("the row the case issued");
            CASE_STORE.with_borrow_mut(|slot| *slot = Some(store.clone()));
            ctx.open_session(nvs_runtime::Session {
                id: ID.to_owned(),
                record: loaded,
                dirty: false,
                stored: true,
                write_back: to_the_case_store,
            });
            let mut writing = record(&ctx, "set").expect("a started session has a record");
            writing.set(NvsStr::new(b"cart"), Value::int(17));
            write_back(&mut ctx, writing, "set").expect("the changed record encodes");

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
                created(&store, NO_APP, "a case", ID, RECORD, DEFAULT_TTL)
                    .expect("the row is issued")
            );
            ctx.open_session(nvs_runtime::Session {
                id: ID.to_owned(),
                record: RECORD.to_vec(),
                dirty: false,
                stored: true,
                write_back: super::send_at_end,
            });
            assert!(
                forgot(&store, NO_APP, "a case", ID).expect("another request destroys the session"),
                "{word}: the row was there to destroy"
            );

            regenerated(&mut ctx, &store).unwrap_or_else(|why| {
                panic!("{word}: a session that is gone does not throw: {why:?}")
            });
            assert_eq!(kept.rows(), 0, "{word}: no row moved to a new identifier");
            let left = ctx.session().expect("the request keeps a session");
            assert!(left.id.is_empty(), "{word}: the session has no identifier");
            assert!(left.record.is_empty(), "{word}: and an empty record");
            assert!(!left.stored && !left.dirty, "{word}: and nothing to write");
            assert!(
                ctx.take_headers().is_empty(),
                "{word}: and no cookie names the identifier that was removed"
            );
        }
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
                id: ID.to_owned(),
                record: RECORD.to_vec(),
                dirty: true,
                stored: false,
                write_back: to_the_case_store,
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
            assert!(
                !replaced(&store, shop_app, "a case", ID, b"taken", DEFAULT_TTL)
                    .expect("the store answers"),
                "{word}: and cannot write over the first application's row"
            );
            assert!(
                created(&store, shop_app, "a case", ID, b"its own", DEFAULT_TTL)
                    .expect("the store answers"),
                "{word}: its own session under that id is a row of its own"
            );
            assert_eq!(kept.rows(), 2, "{word}: so the store holds two rows");

            let mut again = Ctx::buffered();
            if let Some(config) = blog.config() {
                again.set_config(Arc::clone(config.snapshot()));
            }
            assert_eq!(
                fetched(&store, crate::cache::application(&again), "start", ID)
                    .expect("the store answers"),
                Some(RECORD.to_vec()),
                "{word}: the first application still loads its record"
            );
            CASE_STORE.with_borrow_mut(Option::take);
        }
    }

    /// `rule:http-server/a-session-store-answers-four-operations`: on every store, `replace`
    /// writes nothing for an absent id or an expired row, and `create` on an id that has a row
    /// fails and keeps that row's record.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn only_create_inserts_a_session_row() {
        for (word, backend) in BACKENDS {
            let mut ctx = Ctx::buffered();
            let (store, kept) = a_store(&mut ctx, "session-only-create", *backend);
            assert!(
                !replaced(&store, NO_APP, "a case", ID, RECORD, DEFAULT_TTL)
                    .expect("the store answers"),
                "{word}: a replace of an absent id writes nothing"
            );
            assert_eq!(kept.rows(), 0, "{word}: and the store holds no row");

            assert!(
                created(&store, NO_APP, "a case", ID, RECORD, DEFAULT_TTL)
                    .expect("the store answers"),
                "{word}: a create of an absent id inserts the row"
            );
            assert!(
                !created(&store, NO_APP, "a case", ID, b"another", DEFAULT_TTL)
                    .expect("the store answers"),
                "{word}: a create of a held id fails"
            );
            assert_eq!(
                fetched(&store, NO_APP, "a case", ID).expect("the store answers"),
                Some(RECORD.to_vec()),
                "{word}: and the held record stays"
            );

            kept.lapse(ID);
            assert!(
                !replaced(&store, NO_APP, "a case", ID, b"late", DEFAULT_TTL)
                    .expect("the store answers"),
                "{word}: a replace of an expired row writes nothing"
            );
            assert_eq!(
                fetched(&store, NO_APP, "a case", ID).expect("the store answers"),
                None,
                "{word}: and the row stays absent"
            );
        }
    }

    /// `Core\Session::destroy` as a program reaches it: the store is told first, and only then
    /// does the request forget.
    ///
    /// Both orders are asserted. With no store configured the member throws and the session is
    /// still open, holding the id and the record it had, so a sign-out that did not happen is
    /// never reported as one. With the store configured, over [`serving`], the entry is gone from
    /// the map both cores share, the request has no session, and a second `destroy` throws as one
    /// before `start` does.
    // covers: Core\Session::destroy
    #[test]
    fn destroy_forgets_the_record_before_it_closes_the_session() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(SHARED_BLOCK));
        assert!(
            nvs_runtime::call(super::nvs_core_session_destroy, &mut ctx, &[]).is_err(),
            "a request that never started a session has nothing to destroy"
        );
        drop(ctx.take_pending());

        ctx.open_session(nvs_runtime::Session {
            id: ID.to_owned(),
            stored: true,
            record: RECORD.to_vec(),
            dirty: false,
            write_back: super::send_at_end,
        });
        assert!(
            nvs_runtime::call(super::nvs_core_session_destroy, &mut ctx, &[]).is_err(),
            "no `[cache.shared] url`, so no store heard the sign-out"
        );
        drop(ctx.take_pending());
        let kept = ctx
            .session()
            .expect("a refused destroy leaves the session open");
        assert_eq!(kept.id, ID);
        assert_eq!(kept.record, RECORD);

        let (listener, address) = listening();
        let held: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
        held.lock()
            .expect("the store")
            .insert(key_of(NO_APP, ID), RECORD.to_vec());
        let store = Arc::clone(&held);
        let _serving = Serving::start(listener, store);
        ctx.set_config(crate::tests::granting(&format!(
            "{SHARED_BLOCK}[capabilities.cache]\nshared = true\n\
             [cache.shared]\nurl = \"redis://{address}\"\n"
        )));

        nvs_runtime::call(super::nvs_core_session_destroy, &mut ctx, &[])
            .expect("a configured store forgets the record");
        assert!(
            !held
                .lock()
                .expect("the store")
                .contains_key(&key_of(NO_APP, ID)),
            "the entry is gone from the store"
        );
        assert!(ctx.session().is_none(), "and the request has no session");

        assert!(
            nvs_runtime::call(super::nvs_core_session_destroy, &mut ctx, &[]).is_err(),
            "a destroyed session throws again, as before `start`"
        );
        drop(ctx.take_pending());
    }

    /// `Core\Session::regenerate` as a program reaches it: the record moves to a new identifier in
    /// the store and the old entry is forgotten, before the request's own session changes.
    ///
    /// With no store configured the member throws and the request keeps the id and record it had,
    /// so a login that did not harden is never reported as one. Over [`serving`] the store holds
    /// the record under the new id alone, and the request holds the same record, not dirty, because
    /// the move already wrote it.
    // covers: Core\Session::regenerate
    #[test]
    fn regenerate_moves_the_record_to_a_new_id_and_forgets_the_old_one() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(SHARED_BLOCK));
        assert!(
            nvs_runtime::call(super::nvs_core_session_regenerate, &mut ctx, &[]).is_err(),
            "a request that never started a session has nothing to move"
        );
        drop(ctx.take_pending());

        ctx.open_session(nvs_runtime::Session {
            id: ID.to_owned(),
            stored: true,
            record: RECORD.to_vec(),
            dirty: false,
            write_back: super::send_at_end,
        });
        assert!(
            nvs_runtime::call(super::nvs_core_session_regenerate, &mut ctx, &[]).is_err(),
            "no `[cache.shared] url`, so no store can take the record"
        );
        drop(ctx.take_pending());
        let kept = ctx
            .session()
            .expect("a refused regenerate leaves the session open");
        assert_eq!(kept.id, ID);
        assert_eq!(kept.record, RECORD);

        let (listener, address) = listening();
        let held: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
        held.lock()
            .expect("the store")
            .insert(key_of(NO_APP, ID), RECORD.to_vec());
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
        assert_eq!(moved.record, RECORD, "and the record is the one it had");
        assert!(
            !moved.dirty,
            "the move wrote the record, so nothing is left to write"
        );

        let entries = held.lock().expect("the store");
        assert!(
            !entries.contains_key(&key_of(NO_APP, ID)),
            "the old entry is gone from the store"
        );
        assert_eq!(
            entries.get(&key_of(NO_APP, &moved.id)).map(Vec::as_slice),
            Some(RECORD),
            "and the record is under the new id"
        );
    }

    /// `Core\Session::start` as a program reaches it: an identifier the store holds opens that
    /// record and writes no cookie, and one it does not hold opens a fresh, empty record with no
    /// identifier yet: nothing is written to the store and no `Set-Cookie` is sent.
    ///
    /// Both refusals come first, and neither leaves a session open: a context answering no
    /// request, and a request whose tree chose the shared tier and gave it no address. A second
    /// `start` on one request
    /// throws and keeps the record the first one opened. The held identifier arrives in the
    /// `nvsid` cookie and the forged one as the `$presented` argument, so both readings are
    /// driven.
    // covers: Core\Session::start
    #[test]
    fn start_opens_a_held_record_and_a_fresh_one_for_an_id_the_store_does_not_hold() {
        const HELD: &str = "PmA5tKz1QvR3sYbNcW8xLq";
        const FORGED: &str = "AAAAAAAAAAAAAAAAAAAAAA";
        let nothing = [Value::null()];

        let mut ctx = Ctx::buffered();
        assert!(
            nvs_runtime::call(super::nvs_core_session_start, &mut ctx, &nothing).is_err(),
            "a program answering no request has no session to open"
        );
        drop(ctx.take_pending());
        assert!(ctx.session().is_none());

        let mut inbound = nvs_runtime::Inbound::new("GET", "/", "");
        inbound.push_header("cookie", format!("nvsid={HELD}").as_bytes());
        ctx.set_inbound(inbound);
        ctx.set_config(crate::tests::granting(SHARED_BLOCK));
        assert!(
            nvs_runtime::call(super::nvs_core_session_start, &mut ctx, &nothing).is_err(),
            "the shared tier with no `[cache.shared] url`, so there is no store to open a record in"
        );
        drop(ctx.take_pending());
        assert!(ctx.session().is_none());

        let (listener, address) = listening();
        let held: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
        held.lock()
            .expect("the store")
            .insert(key_of(NO_APP, HELD), RECORD.to_vec());
        let store = Arc::clone(&held);
        let _serving = Serving::start(listener, store);
        let config = crate::tests::granting(&format!(
            "[capabilities.cache]\nshared = true\n[session]\nbackend = \"shared\"\n\
             [cache.shared]\nurl = \"redis://{address}\"\n"
        ));
        ctx.set_config(Arc::clone(&config));

        nvs_runtime::call(super::nvs_core_session_start, &mut ctx, &nothing)
            .expect("the cookie names a record the store holds");
        let opened = ctx.session().expect("the request has a session");
        assert_eq!(opened.id, HELD, "the presented identifier is kept");
        assert_eq!(opened.record, RECORD, "and its record is the stored one");
        assert!(!opened.dirty, "opening a record is not a change");
        assert!(
            ctx.take_headers().is_empty(),
            "the client already has this identifier, so no cookie is sent"
        );

        assert!(
            nvs_runtime::call(super::nvs_core_session_start, &mut ctx, &nothing).is_err(),
            "a second `start` on one request throws"
        );
        drop(ctx.take_pending());
        assert_eq!(
            ctx.session().expect("and the first session stays open").id,
            HELD
        );

        let mut fresh = Ctx::buffered();
        fresh.set_inbound(nvs_runtime::Inbound::new("GET", "/", ""));
        fresh.set_config(config);
        let forged = [Value::text(FORGED)];
        let answer = nvs_runtime::call(super::nvs_core_session_start, &mut fresh, &forged);
        forged.into_iter().for_each(dropped);
        answer.expect("an identifier the store does not hold opens a fresh session");
        let issued = fresh.session().expect("the request has a session");
        assert!(
            issued.id.is_empty(),
            "the forged identifier is not used, and none is drawn before the first write"
        );
        assert!(issued.record.is_empty(), "the fresh record is empty");
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

    /// Attack: 20,000 requests with no cookie each call `Core\Session::start` and only read.
    /// None of them writes a row or sends a cookie, so the store grows with sessions somebody
    /// wrote, never with requests served. The first write then sends one cookie, and the
    /// write-back stores the record under the identifier that cookie carries. Under Miri there
    /// are 20 requests, because each one is a full session start and 200 outlast nextest's
    /// five minutes.
    #[test]
    fn a_session_nobody_wrote_leaves_no_row_and_sends_no_cookie() {
        let (listener, address) = listening();
        let held: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
        let _serving = Serving::start(listener, Arc::clone(&held));
        let config = crate::tests::granting(&format!(
            "[capabilities.cache]\nshared = true\n[session]\nbackend = \"shared\"\n\
             [cache.shared]\nurl = \"redis://{address}\"\n"
        ));
        let nothing = [Value::null()];
        let request = || {
            let mut ctx = Ctx::buffered();
            ctx.set_inbound(nvs_runtime::Inbound::new("GET", "/", ""));
            ctx.set_config(Arc::clone(&config));
            nvs_runtime::call(super::nvs_core_session_start, &mut ctx, &nothing)
                .expect("a request with no cookie opens a session");
            ctx
        };

        let requests = if cfg!(miri) { 20 } else { 20_000 };
        for _ in 0..requests {
            let mut ctx = request();
            let read = [Value::text("user")];
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
        let write = [Value::text("user"), Value::text("ada")];
        let answer = nvs_runtime::call(super::nvs_core_session_set, &mut ctx, &write);
        write.into_iter().for_each(dropped);
        dropped(answer.expect("the first write is kept"));
        let headers = ctx.take_headers();
        assert_eq!(headers.len(), 1, "the first write sends one cookie");
        let id = ctx.session().expect("the session stays open").id.clone();
        assert!(
            headers[0].value.starts_with(&format!("nvsid={id};")),
            "{:?} carries the identifier",
            headers[0].value
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
    /// A record no decode accepts throws rather than answering `null`, so a session this unit
    /// cannot read is never mistaken for an empty one.
    // covers: Core\Session::get
    #[test]
    fn get_answers_a_held_key_and_null_for_the_rest_and_earns_no_write() {
        fn got(ctx: &mut Ctx, key: &[u8]) -> Result<Value, i32> {
            let key = Value::text(std::str::from_utf8(key).expect("the test writes UTF-8"));
            let answer = nvs_runtime::call(super::nvs_core_session_get, ctx, &[key]);
            dropped(key);
            answer
        }

        let mut ctx = Ctx::buffered();
        assert!(
            got(&mut ctx, b"cart").is_err(),
            "a request that never started a session has no record to read"
        );
        drop(ctx.take_pending());

        ctx.open_session(nvs_runtime::Session {
            id: ID.to_owned(),
            stored: true,
            record: Vec::new(),
            dirty: false,
            write_back: super::send_at_end,
        });
        let mut writing = record(&ctx, "set").expect("a started session has a record to write");
        writing.set(NvsStr::new(b"cart"), Value::int(17));
        writing.set(NvsStr::new(b"note"), Value::null());
        write_back(&mut ctx, writing, "set").expect("the record encodes");
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

        ctx.session_mut().expect("the session is open").record = b"no decode accepts this".to_vec();
        assert!(
            got(&mut ctx, b"cart").is_err(),
            "a record this unit cannot decode throws rather than reading as empty"
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
            let answer = nvs_runtime::call(super::nvs_core_session_remove, ctx, &[key]);
            dropped(key);
            answer
        }

        let mut ctx = Ctx::buffered();
        assert!(
            removed(&mut ctx, b"cart").is_err(),
            "a request that never started a session has no record to change"
        );
        drop(ctx.take_pending());

        ctx.open_session(nvs_runtime::Session {
            id: ID.to_owned(),
            stored: true,
            record: Vec::new(),
            dirty: false,
            write_back: super::send_at_end,
        });
        let mut writing = record(&ctx, "set").expect("a started session has a record to write");
        writing.set(NvsStr::new(b"cart"), Value::int(17));
        writing.set(NvsStr::new(b"user"), Value::int(4));
        write_back(&mut ctx, writing, "set").expect("the record encodes");
        ctx.session_mut().expect("the session is open").dirty = false;

        let before = ctx.session().expect("the session is open").record.clone();
        dropped(removed(&mut ctx, b"missing").expect("an absent key is not an error"));
        let session = ctx.session().expect("the session is open");
        assert!(!session.dirty, "removing an absent key is not a change");
        assert_eq!(
            session.record, before,
            "and leaves the record's bytes as they were"
        );

        dropped(removed(&mut ctx, b"cart").expect("a held key is removed"));
        assert!(
            ctx.session().expect("the session is open").dirty,
            "removing a held key is a change, so it earns a write"
        );
        let held = record(&ctx, "get").expect("the record decodes");
        assert!(!held.has_key(b"cart"), "the removed key is gone");
        assert!(held.has_key(b"user"), "and the other key stays");
        drop(held);
    }

    /// `Core\Session::set` through the member: the value is saved and earns a write, a second
    /// `set` replaces it, and the caller's own reference to a heap value survives the call.
    ///
    /// The string is released by this test after each call, so a member that took the argument's
    /// reference instead of retaining its own would free it twice here and panic.
    // covers: Core\Session::set
    #[test]
    fn set_saves_a_value_replaces_it_and_earns_a_write() {
        fn saved(ctx: &mut Ctx, key: &[u8], value: &[u8]) -> Result<Value, i32> {
            let key = Value::text(std::str::from_utf8(key).expect("the test writes UTF-8"));
            let value = Value::text(std::str::from_utf8(value).expect("the test writes UTF-8"));
            let answer = nvs_runtime::call(super::nvs_core_session_set, ctx, &[key, value]);
            dropped(key);
            dropped(value);
            answer
        }
        fn read(ctx: &Ctx, key: &[u8]) -> Option<String> {
            let held = record(ctx, "get").expect("the record decodes");
            let text = held
                .get(key)
                .and_then(|value| value.as_text().map(str::to_owned));
            drop(held);
            text
        }

        let mut ctx = Ctx::buffered();
        assert!(
            saved(&mut ctx, b"language", b"en").is_err(),
            "a request that never started a session has no record to write"
        );
        drop(ctx.take_pending());

        ctx.open_session(nvs_runtime::Session {
            id: ID.to_owned(),
            stored: true,
            record: Vec::new(),
            dirty: false,
            write_back: super::send_at_end,
        });
        dropped(saved(&mut ctx, b"language", b"en").expect("a started session saves"));
        assert!(
            ctx.session().expect("the session is open").dirty,
            "saving is a change, so it earns a write"
        );
        assert_eq!(read(&ctx, b"language").as_deref(), Some("en"));

        dropped(saved(&mut ctx, b"language", b"de").expect("a second value replaces the first"));
        assert_eq!(read(&ctx, b"language").as_deref(), Some("de"));
    }

    /// A context with a session open over an empty record, as `start` leaves one.
    fn started() -> Ctx {
        let mut ctx = Ctx::buffered();
        ctx.set_random_state(11);
        ctx.open_session(nvs_runtime::Session {
            id: ID.to_owned(),
            stored: true,
            record: Vec::new(),
            dirty: false,
            write_back: super::send_at_end,
        });
        ctx
    }

    /// Calls `setSecret(key, value, ring)` the way a program does, releasing what it built.
    fn secret_set(ctx: &mut Ctx, key: &[u8], value: &[u8], ring: &[&[u8]]) -> Result<Value, i32> {
        let args = [
            Value::text(std::str::from_utf8(key).expect("the test writes UTF-8")),
            Value::text(std::str::from_utf8(value).expect("the test writes UTF-8")),
            crate::keyring::tests::ring_of(ring),
        ];
        let answer = nvs_runtime::call(super::nvs_core_session_set_secret, ctx, &args);
        args.into_iter().for_each(dropped);
        answer
    }

    /// Calls `getSecret(key, ring)` and reads the answer as text, `None` for `null`.
    fn secret_got(ctx: &mut Ctx, key: &[u8], ring: &[&[u8]]) -> Result<Option<String>, i32> {
        let args = [
            Value::text(std::str::from_utf8(key).expect("the test writes UTF-8")),
            crate::keyring::tests::ring_of(ring),
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

    /// `Core\Session::setSecret` through the member: the value lands sealed and earns a write, the
    /// plain key stays empty so `get` cannot read it, and the stored bytes are not the plaintext.
    ///
    /// Both refusals are asserted before the write, because a member that checked the ring after
    /// sealing would leave a half-written record behind a `LogicError`.
    // covers: Core\Session::setSecret
    #[test]
    fn set_secret_seals_the_value_out_of_gets_reach_and_earns_a_write() {
        let mut unstarted = Ctx::buffered();
        assert!(
            secret_set(&mut unstarted, b"refresh", b"sk-live-7", &[NEW]).is_err(),
            "a request that never started a session has no record to write"
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
        let held = record(&ctx, "get").expect("the record decodes");
        assert!(
            !held.has_key(b"refresh"),
            "`get` finds nothing under the plain key"
        );
        let sealed = held
            .get(&sealed_key(b"refresh"))
            .and_then(|value| value.as_bytes().map(<[u8]>::to_vec))
            .expect("the sealed entry is bytes");
        drop(held);
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
    /// spelled to read a session record.
    #[test]
    fn a_record_is_keyed_under_its_own_prefix() {
        let key = key_of(b"/srv/blog", ID);
        assert!(key.starts_with(PREFIX.as_bytes()));
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
        assert_eq!(ttl(&ctx), DEFAULT_TTL);
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
            "[session]\nbackend = \"shared\"\nttl = \"30m\"\ncookie = \"sid\"\n",
        ));
        assert_eq!(
            backend(&ctx, "start").expect("a store the roster admits"),
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
            stored: true,
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
