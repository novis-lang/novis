//! The value behind [`registry::CoreTy::Instance`]: what a native member
//! returns when its signature names a `Core`-owned object, and how another member
//! reads one back.
//!
//! # Decision: a `Core` instance is an ordinary Novis object
//!
//! Not a native handle, not a boxed `dyn Any`, not a tag of its own. A
//! `Core\Regex\Match` is exactly what `new Point(1, 2)` produces — one
//! [`nvs_runtime::NvsObj`] allocation, a [`ClassDesc`] pointer, and 16-byte
//! [`Value`] slots behind it — so **nothing below this line learns that `Core`
//! owns a class**: refcounting, the release sweep, strict identity, `Tag`/
//! `Untag` for a `?T` return and `nvs-codegen`'s argument slots all meet a
//! shape they already had.
//!
//! The cost is that every slot must be a value Novis can already hold, so a
//! member whose state is genuinely native (a compiled `regex::Regex`) has to
//! keep that state somewhere else — for `Core\Regex` that is the pattern text
//! plus [`crate::regex`]'s per-core compiled-pattern cache, which is a lookup
//! rather than a second representation. The rejected alternative was a
//! `resource` tag of its own, a handle into a per-request table: it buys
//! native state directly and costs a second heap shape, a second release path,
//! and a liveness rule every future `Core` class would have to restate — and
//! `nvs_runtime::Tag` carries no such row, which is where that refusal now
//! reads. `rule:security/closed-doors`'s
//! closed door on stream wrappers is the same instinct — an engine-owned
//! handle is a thing a program can hold and nothing can check.
//!
//! **What it spends:** one object allocation per instance, `FIELDS_OFFSET`
//! bytes plus 16 per declared slot, charged to the request that produced it and
//! released with it.
//!
//! # Decision: `new` on a `Core` class lowers to that class's own helper
//!
//! Most `Core` instances come from a member that produces one, and those need
//! nothing here. `Core\ObjectMap`, `Core\ObjectSet` and `Core\Heap` are the
//! carve-out — a program writes `new Core\ObjectSet<Tag>()` — and a `Core`
//! class still has no `constructor` member for `nvs_types` to resolve, because
//! its slots are this crate's layout rather than a surface a program fills in.
//!
//! So the roster is [`registry::CONSTRUCTORS`], one line per constructible
//! class, and `nvs-ir` lowers `new` on a name it holds to an ordinary
//! `InstKind::CoreCall` on that symbol instead of an `InstKind::New`
//! (`nvs_ir::lower::expr`'s `lower_new`). **Nothing below `nvs-ir` learns that
//! `Core` owns a class**, which is the promise this module's first decision
//! makes: codegen emits the same helper call it emits for `Core\Uuid::v4()`,
//! and the descriptor comes from the leaked table below rather than from the
//! program's own class list — which would not hold one. The rejected
//! alternative was a synthetic `constructor` row in [`registry::CLASSES`]: it
//! would make `Core\ObjectSet::constructor(…)` a spelling the checker
//! resolves, and every consumer that iterates a class's members would have to
//! learn to skip it.
//!
//! # Decision: a registered member the engine reaches by name gets a descriptor field
//!
//! Two members are reached on a `Core` instance by *name* rather than from a
//! call site that resolved them: `rule:classes/stringable`'s `toString`, when
//! an erased operand is echoed, and `rule:classes/comparable`'s `compareTo`,
//! when a `Core\Heap` or another collection orders two values it knows no
//! class for. Both are answered by a field on [`ClassDesc`] —
//! `ClassDesc::renderer` and `ClassDesc::comparer` — and not by a
//! [`DISPATCH_ROSTER`] row.
//!
//! **The fork is the calling convention, not the lookup.** A roster row is
//! called through `nvs_runtime::dispatch`'s `call_at`, which transfers a
//! reference per slot including the receiver, because that table otherwise
//! holds compiled Novis methods and those release their parameters. A
//! *registered* `Core` member is an `rule:errors/propagation` helper that
//! **borrows** its arguments, so listing one as a row would leak one reference
//! per operand per call — for an ordering, two per comparison, on every sift
//! of every heap. The roster's own members can be rows because they are
//! row-less symbols written for it ([`crate::cursor`]), which release argument
//! slot 0 themselves; a member a program can also call by name cannot be, and
//! a per-class transferring wrapper beside each one would be a second
//! implementation of every ordering in the library.
//!
//! **Derived from the registry, never written down here.** [`descriptors`]
//! asks `registry::render_member` and `registry::compare_member`, which are the
//! same rosters `registry::class_renders` and `registry::implements_comparable`
//! answer the *compiler* from. A hand-written table beside them would be free
//! to miss a class the checker had already let a program `echo` or order, which
//! is exactly the gap this closes.
//!
//! # Decision: the descriptors are one leaked table for the process
//!
//! A [`ClassDesc`]'s *address* is its identity, and it must outlive every
//! instance made from it. A compiled unit's own [`ClassTable`] cannot own these
//! — a `Core` class is not in any program's class list, and an instance can
//! outlive the unit that produced it in a hot-reload swap
//! (`rule:config/an-edit-reaches-the-next-request-without-a-restart`). So
//! this module builds one table, on first use, and **leaks** it.
//!
//! **One table for the whole process, not one per core**, because an identity
//! compiled code has to *write down* cannot be per-thread.
//! `rule:core-classes/html-template` folds a hole-free `` html`…` `` into a
//! `Core\Html\Markup` constant in the compiled unit's own data section, and the
//! class word of that constant is written once, while compiling, into bytes
//! every core then reads — so a second core's descriptor would be a second
//! identity that constant could not name. [`ClassTable`] is `Send` and `Sync`
//! already, for the reason `nvs_runtime::object` gives about a unit's own
//! table, and a [`OnceLock`] past its first call is a read rather than a lock.
//!
//! **What it spends:** one descriptor per `Core` instance class —
//! O(classes), bounded by [`registry::CLASSES`] and growing with neither
//! traffic nor cores, which is the property
//! [AGENTS.md](/AGENTS.md)'s memory rule actually asks for. Leaked
//! rather than dropped because a dangling descriptor is a use-after-free and
//! the bytes are bounded by a compile-time roster.
//!
//! # Decision: a `Core` class states its slot types, or it does not cross as bytes
//!
//! `Core\Serialize::decode` and the cache and session that read bytes check
//! each slot of an object they build against its stated type
//! (`rule:classes/serialize-is-a-closed-format`). A program's class has its
//! types from the checker. A `Core` class has them from [`SLOT_TYPES`], one
//! roster beside [`HOST_HANDLE_CLASSES`] whose row is one [`CoreTy`] per slot,
//! turned into the descriptor's tables by [`slot_type`]. A class with no row is
//! refused by the bytes carrier in both directions, and still crosses a live
//! boundary. A debug [`build`] checks each instance against its row, so every
//! test that makes one proves the row.
//!
//! **What it spends:** one tag, an optional class list, an optional check tree
//! and a spelling per slot of each typed class, built once with the
//! descriptors and leaked with them. Nothing per request. A decode pays one
//! check per slot of each `Core` object it builds.

use std::sync::OnceLock;

use nvs_runtime::sequence;
use nvs_runtime::{
    ClassDesc, ClassTable, Fault, NvsObj, ObjHeader, SlotCheck, SlotType, Tag, Value,
};

use crate::registry::{self, CoreClass, CoreTy};

/// `Core`-owned classes with instances that no program can name, and so with
/// no [`registry::CLASSES`] row: a runtime artifact rather than surface.
///
/// The singletons — the cursor the `Core` collections hand a `foreach`, whose own
/// module docs own why it has no row. They are named here beside the
/// registered classes because a descriptor is a descriptor: everything below
/// [`ClassDesc`] is the same for both. A row-less *family* is a slice of its
/// own instead, chained in [`descriptors`] where this one is.
const INTERNAL_CLASSES: &[&CoreClass] = &[&crate::cursor::CLASS];

/// Every `Core` class whose instance holds a **host handle** — the roster
/// behind [`ClassDesc::holds_host_handle`], which is what
/// `rule:security/isolate-values-cross-by-copy` refuses at a copy boundary and
/// `nvs_runtime::graph`'s walk reads off the descriptor.
///
/// **A class belongs here when one of its slots carries a key filed by a
/// `nvs_runtime::Ctx::hold_*` member**, because that key indexes the table of
/// the context that opened it and means something else entirely in another
/// one. `Core\Socket` is the one entry that carries no such slot and belongs
/// for the same reason twice over: its whole state is its isolate's, so the
/// value *is* the handle ([`crate::socket::CLASS`]).
///
/// It is a roster rather than a question asked of the slots because a key is a
/// `uint` like every other — `Core\Net\Message`'s port is one, and it crosses
/// — so the module that opens the resource is what knows, and this is where
/// the crate that holds all of them says so. A class added beside a new
/// `hold_*` call adds its line here.
const HOST_HANDLE_CLASSES: &[&CoreClass] = &[
    &crate::io::FILE,
    &crate::csv::ROWS,
    // The two JSON walks, whose first slot is the key their suspended reader
    // or writer is filed under.
    &crate::json::ELEMENTS,
    &crate::json::PIECES,
    // `Core\Markdown::toHtmlEach`'s walk, whose first slot is the key its
    // two passes are filed under.
    &crate::markdown::GROUPS,
    // `Core\Markdown::live`'s document, whose first slot is the key its text
    // and definitions are filed under.
    &crate::markdown::LIVE,
    &crate::process::HANDLE,
    &crate::script::HANDLE,
    &crate::socket::CLASS,
    &crate::http::socket::SOCKET,
    &crate::http::stream::STREAM,
    // The three walks over a stream's body, which take the reader's key out of
    // the stream's own slot and carry it from there — `crate::http::stream`'s
    // `consumed` is why the key moves rather than being shared.
    &crate::http::stream::EVENTS,
    &crate::http::stream::LINES,
    &crate::http::stream::CHUNKS,
    &crate::net::STREAM,
    &crate::net::LISTENER,
    &crate::net::DATAGRAM,
    &crate::db::CONNECTION,
    &crate::db::TRANSACTION,
    // A streamed result set holds the connection's key to pull the next row;
    // `Core\Db\Rows` holds the rows themselves and crosses.
    &crate::db::STREAM,
    &crate::ldap::CONNECTION,
    // A search holds the connection's key, and reads its next page through it.
    &crate::ldap::ENTRIES,
    // And a sync, which reads its next answer the same way.
    &crate::ldap::CHANGES_SET,
];

/// Every `Core` class whose instance frees its filed state when it dies — the
/// roster behind `nvs_runtime::ClassDesc`'s `free` hook.
///
/// **A class belongs here when its state is the object's alone and nothing
/// else can reach it once the object is gone.** A walk whose key moves to
/// another object, or a connection a later `Core\Db` call finds again, does
/// not belong. Every class here is a [`HOST_HANDLE_CLASSES`] entry too.
const FREES_STATE: &[(&CoreClass, nvs_runtime::FreeState)] =
    &[(&crate::markdown::LIVE, crate::markdown::free_live)];

/// What one [`SLOT_TYPES`] row says about a `Core` class's slots.
enum Slots {
    /// The type of each slot, in slot order. `decode` checks each slot of an
    /// instance it builds against it, as it checks a program's properties.
    Typed(&'static [CoreTy]),
    /// Why the class is not written to bytes or read from them: a reader
    /// relies on more than each slot's type, such as one slot being an index
    /// into another. The sentence ends the refusal `decode` and `encode` give.
    NotReadBack(&'static str),
}

/// The reason of a class whose slots are the state of a walk, a stream or a
/// queue that is still running.
const RUNNING_STATE: &str =
    "It contains the state of something that is still running, and bytes cannot rebuild that.";

/// The reason of a class whose instance belongs to the request or the scope
/// that made it, such as a handle to its response stream or its route.
const THIS_REQUEST: &str =
    "It belongs to the request that made it, and another request cannot use it.";

/// The reason of a class whose slots hold a private key, a shared key or a
/// password.
const KEY_MATERIAL: &str = "It contains a key or a password, and those are not written to bytes.";

/// The reason of a class whose text is written out with no check, because
/// only a `Core` member makes one.
const TRUSTED_TEXT: &str =
    "Its text is used without a check, so only a `Core` function may make one.";

/// The reason of a class whose slots a `Core` member checked when it made the
/// instance, and whose readers do not check them again.
const CHECKED_ONCE: &str =
    "Its parts are checked when it is made, and bytes cannot show that they were.";

/// The reason of a class that finds the objects it holds by their identity.
const BY_IDENTITY: &str =
    "It finds its objects by their identity, and a copy contains new objects.";

/// Every `Core` class that says what its slots hold — the roster behind
/// [`ClassDesc::states_slot_types`], which `rule:classes/serialize-is-a-closed-format`'s
/// bytes carrier asks in both directions.
///
/// **A class with no row is not written to bytes or read from them.** That is
/// the safe default: a class added later is refused by `Core\Serialize`, the
/// cache and the session until its row says what each slot holds. A class with
/// no slots needs a row too, an empty one. A row's types are read from the code
/// that fills the slots, and a debug build checks them on every [`build`].
///
/// A row is [`Slots::Typed`] only when the types are everything a reader of the
/// slots relies on. Each `CoreTy` a row uses needs an arm in [`slot_type`], and
/// a row whose length is not the slot count stops the table's build.
const SLOT_TYPES: &[(&CoreClass, Slots)] = &[
    // `tls_info_of` fills all four from a handshake's session.
    (
        &crate::http::TLS_INFO,
        Slots::Typed(&[
            CoreTy::Str,
            CoreTy::Str,
            CoreTy::Bool,
            CoreTy::Array(&CoreTy::Bytes),
        ]),
    ),
    // `crate::uri`'s `built` fills all eight from one parsed reference: the
    // text, then each part, `null` where the text has none.
    (
        &crate::uri::CLASS,
        Slots::Typed(&[
            CoreTy::Str,
            CoreTy::Nullable(&CoreTy::Str),
            CoreTy::Nullable(&CoreTy::Str),
            CoreTy::Nullable(&CoreTy::Str),
            CoreTy::Nullable(&CoreTy::Int),
            CoreTy::Str,
            CoreTy::Nullable(&CoreTy::Str),
            CoreTy::Nullable(&CoreTy::Str),
        ]),
    ),
    // `sign` reads the sign slot alone, so a sign that disagrees with the
    // magnitude gives a wrong answer.
    (&crate::bigint::CLASS, Slots::NotReadBack(CHECKED_ONCE)),
    // `text` and `group` look keys up and throw where one is missing.
    (
        &crate::regex::MATCH,
        Slots::Typed(&[CoreTy::Array(&CoreTy::Nullable(&CoreTy::Str)), CoreTy::Int]),
    ),
    // The pattern is compiled again at each use, under the same limits, and a
    // flag that does not fit throws.
    (
        &crate::regex::PATTERN,
        Slots::Typed(&[CoreTy::Str, CoreTy::Int]),
    ),
    (
        &crate::io::LINES,
        Slots::Typed(&[CoreTy::Array(&CoreTy::Str)]),
    ),
    (&crate::io::STDIN_STREAM, Slots::NotReadBack(RUNNING_STATE)),
    (
        &crate::io::WALK,
        Slots::Typed(&[CoreTy::Array(&CoreTy::Str)]),
    ),
    (
        &crate::io::METADATA,
        Slots::Typed(&[
            CoreTy::Uint,
            CoreTy::Instance(r"Core\Time\Instant"),
            CoreTy::Bool,
            CoreTy::Bool,
        ]),
    ),
    (
        &crate::process::RESULT,
        Slots::Typed(&[CoreTy::Int, CoreTy::Bytes, CoreTy::Bytes]),
    ),
    // The time readers check each range and throw on a value out of it, and a
    // zone id is looked up again at each use.
    (
        &crate::time::INSTANT,
        Slots::Typed(&[CoreTy::Int, CoreTy::Int]),
    ),
    (
        &crate::time::DATETIME,
        Slots::Typed(&[CoreTy::Int, CoreTy::Int, CoreTy::Str]),
    ),
    (
        &crate::time::DATE,
        Slots::Typed(&[CoreTy::Int, CoreTy::Int, CoreTy::Int]),
    ),
    (
        &crate::time::TIME_OF_DAY,
        Slots::Typed(&[CoreTy::Int, CoreTy::Int, CoreTy::Int, CoreTy::Int]),
    ),
    (&crate::time::DURATION, Slots::Typed(&[CoreTy::Int])),
    (&crate::time::ZONE, Slots::Typed(&[CoreTy::Str])),
    (&crate::objmap::CLASS, Slots::NotReadBack(BY_IDENTITY)),
    (&crate::objset::CLASS, Slots::NotReadBack(BY_IDENTITY)),
    // Its entries are kept in heap order, and its comparator is a function.
    (&crate::heap::CLASS, Slots::NotReadBack(RUNNING_STATE)),
    // Every `int` is a state a seed could give.
    (&crate::random::SEEDED, Slots::Typed(&[CoreTy::Int])),
    (
        &crate::uuid::CLASS,
        Slots::Typed(&[CoreTy::Uint, CoreTy::Uint]),
    ),
    // Every pair of chunks and `open` is one a program reaches with `update`.
    (
        &crate::hash::STREAM,
        Slots::Typed(&[
            CoreTy::Enum(crate::hash::DIGEST_NAME),
            CoreTy::Array(&CoreTy::Union(&[CoreTy::Bytes, CoreTy::Str])),
            CoreTy::Bool,
        ]),
    ),
    // `row` is a key into this unit's route table, and `access` is the
    // access rule a gate reads.
    (&crate::router::MATCH, Slots::NotReadBack(THIS_REQUEST)),
    // `text` is a cache of `body`, and `body()` returns it unchecked.
    (&crate::test::RESPONSE, Slots::NotReadBack(CHECKED_ONCE)),
    (
        &crate::test::SENT_REQUEST,
        Slots::Typed(&[
            CoreTy::Enum(crate::router::METHOD_NAME),
            CoreTy::Str,
            CoreTy::Array(&CoreTy::Str),
            CoreTy::Bytes,
        ]),
    ),
    (
        &crate::test::hooks::OUTCOME,
        Slots::Typed(&[
            CoreTy::Str,
            CoreTy::Str,
            CoreTy::Int,
            CoreTy::Bool,
            CoreTy::Array(&CoreTy::Str),
        ]),
    ),
    (
        &crate::test::hooks::SUMMARY,
        Slots::Typed(&[
            CoreTy::Str,
            CoreTy::Int,
            CoreTy::Int,
            CoreTy::Int,
            CoreTy::Int,
            CoreTy::Array(&CoreTy::Str),
        ]),
    ),
    // Its waiting tasks are found by the object, and `capacity` must be one
    // or more.
    (&crate::channel::CLASS, Slots::NotReadBack(RUNNING_STATE)),
    (
        &crate::script::EXIT_REPORT,
        Slots::Typed(&[
            CoreTy::Enum(crate::script::EXIT_REASON_NAME),
            CoreTy::Int,
            CoreTy::Nullable(&CoreTy::Instance("Throwable")),
            CoreTy::Int,
        ]),
    ),
    // `echo` writes `text` to the terminal as it is.
    (&crate::cli::TEXT, Slots::NotReadBack(TRUSTED_TEXT)),
    // An unknown kind reads as an index, and each channel is masked.
    (
        &crate::cli::COLOR,
        Slots::Typed(&[CoreTy::Int, CoreTy::Uint]),
    ),
    (
        &crate::cli::STYLE,
        Slots::Typed(&[
            CoreTy::Nullable(&CoreTy::Instance(r"Core\Cli\Color")),
            CoreTy::Nullable(&CoreTy::Instance(r"Core\Cli\Color")),
            CoreTy::Int,
        ]),
    ),
    // `depth` is a key into the core's stack of live regions.
    (&crate::cli::LIVE, Slots::NotReadBack(THIS_REQUEST)),
    (&crate::cli::PROGRESS, Slots::NotReadBack(THIS_REQUEST)),
    (
        &crate::request::MOUNT,
        Slots::Typed(&[CoreTy::Str, CoreTy::Array(&CoreTy::Str)]),
    ),
    (
        &crate::request::BODY_STREAM,
        Slots::NotReadBack(RUNNING_STATE),
    ),
    (&crate::request::FILES, Slots::NotReadBack(RUNNING_STATE)),
    // `ordinal` is a key into this request's multipart walk.
    (&crate::request::PART, Slots::NotReadBack(THIS_REQUEST)),
    (
        &crate::request::PART_CONTENT,
        Slots::NotReadBack(RUNNING_STATE),
    ),
    (&crate::response::STREAM, Slots::NotReadBack(THIS_REQUEST)),
    (
        &crate::socket::MESSAGE,
        Slots::Typed(&[
            CoreTy::Nullable(&CoreTy::Str),
            CoreTy::Nullable(&CoreTy::Str),
            CoreTy::Nullable(&CoreTy::Bytes),
            CoreTy::Mixed,
        ]),
    ),
    (&crate::sse::CLASS, Slots::NotReadBack(THIS_REQUEST)),
    (
        &crate::sse::MESSAGE,
        Slots::Typed(&[CoreTy::Str, CoreTy::Mixed]),
    ),
    // The key is read again at each use, and a bad one throws.
    (
        &crate::crypto::PUBLIC_KEY,
        Slots::Typed(&[CoreTy::Bytes, CoreTy::Enum(crate::crypto::KEY_KIND_NAME)]),
    ),
    (&crate::crypto::KEY_PAIR, Slots::NotReadBack(KEY_MATERIAL)),
    // `of` checks the id's length, the count and the key, and the getters
    // return the slots unchecked.
    (
        &crate::passkey::CREDENTIAL,
        Slots::NotReadBack(CHECKED_ONCE),
    ),
    // `read` checks the endpoint's scheme, and the key lengths.
    (
        &crate::webpush::SUBSCRIPTION,
        Slots::NotReadBack(CHECKED_ONCE),
    ),
    (
        &crate::webpush::SENT,
        Slots::Typed(&[CoreTy::Int, CoreTy::Bool]),
    ),
    // The set is the keys a token is verified with, capped when it is read.
    (&crate::jwt::KEY_SET, Slots::NotReadBack(CHECKED_ONCE)),
    (&crate::jwe::KEY, Slots::NotReadBack(KEY_MATERIAL)),
    // HTML output writes the text with no escaping.
    (&crate::html::MARKUP, Slots::NotReadBack(TRUSTED_TEXT)),
    (&crate::markdown::UPDATE, Slots::NotReadBack(TRUSTED_TEXT)),
    // `source` checks each name and escapes each text again as it writes.
    (
        &crate::xml::NODE,
        Slots::Typed(&[
            CoreTy::Enum(crate::xml::KIND_NAME),
            CoreTy::Str,
            CoreTy::Str,
            CoreTy::Array(&CoreTy::Str),
            CoreTy::Array(&CoreTy::Instance(r"Core\Xml\Node")),
            CoreTy::Nullable(&CoreTy::Str),
        ]),
    ),
    // `cursor` slices the document, so a wrong one panics.
    (&crate::xml::READER, Slots::NotReadBack(RUNNING_STATE)),
    // `written` is the document so far, written out with no escaping.
    (&crate::xml::WRITER, Slots::NotReadBack(RUNNING_STATE)),
    (
        &crate::compress::COMPRESSOR,
        Slots::Typed(&[
            CoreTy::Enum(crate::compress::CODEC_NAME),
            CoreTy::Array(&CoreTy::Union(&[CoreTy::Bytes, CoreTy::Str])),
            CoreTy::Bool,
        ]),
    ),
    // Its two limits were bounded by the server's ceiling when it was made.
    (
        &crate::compress::DECOMPRESSOR,
        Slots::NotReadBack(CHECKED_ONCE),
    ),
    // Its addresses passed the network rules, and a request trusts them.
    (&crate::http::TARGET, Slots::NotReadBack(CHECKED_ONCE)),
    (&crate::http::IDENTITY, Slots::NotReadBack(KEY_MATERIAL)),
    (
        &crate::http::RESPONSE,
        Slots::Typed(&[
            CoreTy::Int,
            CoreTy::Bytes,
            CoreTy::Array(&CoreTy::Array(&CoreTy::Str)),
            CoreTy::Nullable(&CoreTy::Instance(r"Core\Http\TlsInfo")),
        ]),
    ),
    (
        &crate::http::stream::EVENT,
        Slots::Typed(&[
            CoreTy::Str,
            CoreTy::Nullable(&CoreTy::Str),
            CoreTy::Nullable(&CoreTy::Str),
        ]),
    ),
    // `path` picks the local file a request uploads.
    (&crate::http::PART, Slots::NotReadBack(CHECKED_ONCE)),
    (
        &crate::net::MESSAGE,
        Slots::Typed(&[CoreTy::Bytes, CoreTy::Str, CoreTy::Uint]),
    ),
    // `tier` says which store it reaches, and `shared()` checks the grant.
    (&crate::cache::STORE, Slots::NotReadBack(CHECKED_ONCE)),
    (
        &crate::cache::SECRET_ENTRY,
        Slots::NotReadBack(KEY_MATERIAL),
    ),
    (
        &crate::ratelimit::DECISION,
        Slots::Typed(&[
            CoreTy::Bool,
            CoreTy::Uint,
            CoreTy::Uint,
            CoreTy::Nullable(&CoreTy::Instance(r"Core\Time\Duration")),
        ]),
    ),
    // `constant` finds a class by `name`, and `forClass` describes only some.
    (
        &crate::reflect::CLASS_INFO,
        Slots::NotReadBack(CHECKED_ONCE),
    ),
    (
        &crate::reflect::METHOD_INFO,
        Slots::Typed(&[
            CoreTy::Str,
            CoreTy::Bool,
            CoreTy::Uint,
            CoreTy::Array(&CoreTy::Instance(r"Core\Reflect\ParameterInfo")),
        ]),
    ),
    (
        &crate::reflect::PROPERTY_INFO,
        Slots::Typed(&[CoreTy::Str, CoreTy::Bool, CoreTy::Nullable(&CoreTy::Str)]),
    ),
    (
        &crate::reflect::PARAMETER_INFO,
        Slots::Typed(&[CoreTy::Str, CoreTy::Nullable(&CoreTy::Str)]),
    ),
    (
        &crate::reflect::CONSTANT_INFO,
        Slots::Typed(&[CoreTy::Str, CoreTy::Bool, CoreTy::Bool]),
    ),
    // `field` throws where the two arrays do not line up.
    (
        &crate::reflect::ATTRIBUTE_INFO,
        Slots::Typed(&[
            CoreTy::Str,
            CoreTy::Str,
            CoreTy::Str,
            CoreTy::Array(&CoreTy::Str),
            CoreTy::Array(&CoreTy::Mixed),
        ]),
    ),
    // `valueOf` throws where the two arrays do not line up.
    (
        &crate::reflect::ENUM_INFO,
        Slots::Typed(&[
            CoreTy::Str,
            CoreTy::Array(&CoreTy::Str),
            CoreTy::Array(&CoreTy::Union(&[CoreTy::Int, CoreTy::Uint])),
            CoreTy::Bool,
        ]),
    ),
    // `nodes()` walks the children with no bound, and bytes can make them a
    // cycle.
    (&crate::ast::NODE, Slots::NotReadBack(CHECKED_ONCE)),
    // `class` is the class its rows are read into, which bytes cannot carry.
    (&crate::db::ROWS, Slots::NotReadBack(CHECKED_ONCE)),
    (
        &crate::db::ROW,
        Slots::Typed(&[CoreTy::Array(&CoreTy::Mixed)]),
    ),
    (
        &crate::db::WRITE,
        Slots::Typed(&[
            CoreTy::Uint,
            CoreTy::Nullable(&CoreTy::Uint),
            CoreTy::Nullable(&CoreTy::Uint),
        ]),
    ),
    (
        &crate::db::COLUMN,
        Slots::Typed(&[
            CoreTy::Str,
            CoreTy::Enum(r"Core\Db\Column::Type"),
            CoreTy::Bool,
        ]),
    ),
    // Each value is bound as a parameter, never written into the SQL.
    (
        &crate::db::IN_LIST,
        Slots::Typed(&[CoreTy::Array(&CoreTy::Mixed)]),
    ),
    // Each use reads the array again with `fromArray`'s checks.
    (
        &crate::db::SCHEMA,
        Slots::Typed(&[CoreTy::Array(&CoreTy::Mixed)]),
    ),
    (
        &crate::db::PLAN,
        Slots::Typed(&[CoreTy::Array(&CoreTy::Instance(r"Core\Db\Plan\Step"))]),
    ),
    (
        &crate::db::STEP,
        Slots::Typed(&[
            CoreTy::Enum(crate::db::GRADE_NAME),
            CoreTy::Str,
            CoreTy::Str,
            CoreTy::Bool,
        ]),
    ),
    (
        &crate::ldap::ENTRY,
        Slots::Typed(&[CoreTy::Str, CoreTy::Array(&CoreTy::Array(&CoreTy::Bytes))]),
    ),
    // A search writes the filter's bytes into the request as they are.
    (&crate::ldap::FILTER, Slots::NotReadBack(TRUSTED_TEXT)),
    // `modify` does not check the attribute name again, and an unknown kind
    // reads as a replace.
    (&crate::ldap::CHANGE, Slots::NotReadBack(CHECKED_ONCE)),
    // Each reader parses the text again and throws on a bad one.
    (&crate::ldap::DN, Slots::Typed(&[CoreTy::Str])),
    (&crate::ldap::SID, Slots::Typed(&[CoreTy::Bytes])),
    (
        &crate::ldap::ACCOUNT_FLAGS,
        Slots::Typed(&[CoreTy::Int, CoreTy::Nullable(&CoreTy::Int), CoreTy::Bool]),
    ),
    (&crate::ldap::GROUP_TYPE, Slots::Typed(&[CoreTy::Int])),
    // `parse` checks the token and the length that `toString` relies on.
    (&crate::queue::ID, Slots::NotReadBack(CHECKED_ONCE)),
    (
        &crate::queue::STATS,
        Slots::Typed(&[
            CoreTy::Uint,
            CoreTy::Uint,
            CoreTy::Uint,
            CoreTy::Uint,
            CoreTy::Uint,
            CoreTy::Uint,
        ]),
    ),
];

/// `ty` as a slot's stated type in run-time terms: its one tag, the classes an
/// object in it must be, the rest of the type as a [`SlotCheck`], and its
/// spelling. `None` for a type with no arm here. No arm means "any value", so a
/// row that names such a type stops the table's build rather than passing
/// every value.
fn slot_type(ty: &CoreTy) -> Option<SlotType> {
    let spelled = ty.spelled();
    let (tag, classes, check) = match ty {
        CoreTy::Instance(name) => (Some(Tag::Object), Some(vec![(*name).to_owned()]), None),
        CoreTy::Nullable(CoreTy::Instance(name)) => (None, Some(vec![(*name).to_owned()]), None),
        CoreTy::Nullable(inner) => (
            None,
            None,
            Some(SlotCheck::AnyOf(vec![
                SlotCheck::Tag(Tag::Null),
                slot_check(inner)?,
            ])),
        ),
        CoreTy::Array(inner) => (
            Some(Tag::Array),
            None,
            Some(SlotCheck::Array(Box::new(slot_check(inner)?))),
        ),
        CoreTy::Enum(_) | CoreTy::Mixed | CoreTy::Union(_) => (None, None, Some(slot_check(ty)?)),
        _ => (Some(scalar_tag(ty)?), None, None),
    };
    Some(SlotType {
        tag,
        classes,
        check,
        spelled,
    })
}

/// `ty` as one [`SlotCheck`] tree, for a type inside an array or a `?T`.
fn slot_check(ty: &CoreTy) -> Option<SlotCheck> {
    Some(match ty {
        CoreTy::Instance(name) => SlotCheck::Class((*name).to_owned()),
        CoreTy::Nullable(inner) => {
            SlotCheck::AnyOf(vec![SlotCheck::Tag(Tag::Null), slot_check(inner)?])
        }
        CoreTy::Array(inner) => SlotCheck::Array(Box::new(slot_check(inner)?)),
        // A slot stated as `mixed` admits every value, and its readers check
        // what they read.
        CoreTy::Mixed => SlotCheck::Any,
        CoreTy::Union(members) => {
            SlotCheck::AnyOf(members.iter().map(slot_check).collect::<Option<_>>()?)
        }
        // A `Core` enum in a slot is one of the enum's own case values. A member
        // stores either the case a program passed, or its `int`.
        CoreTy::Enum(name) => {
            let mut cases: Vec<u64> = registry::core_enum(name)?
                .cases
                .iter()
                .map(|(_, value)| value.cast_unsigned())
                .collect();
            cases.sort_unstable();
            cases.dedup();
            SlotCheck::AnyOf(vec![
                SlotCheck::Values(Tag::Int, cases.clone()),
                SlotCheck::Values(Tag::EnumInt, cases),
            ])
        }
        _ => SlotCheck::Tag(scalar_tag(ty)?),
    })
}

/// The one tag a value of scalar type `ty` has, or `None` where `ty` is not
/// one of these scalars.
fn scalar_tag(ty: &CoreTy) -> Option<Tag> {
    Some(match ty {
        CoreTy::Bool => Tag::Bool,
        CoreTy::Int => Tag::Int,
        CoreTy::Uint => Tag::Uint,
        CoreTy::Float => Tag::Float,
        CoreTy::Decimal => Tag::Decimal,
        CoreTy::Str => Tag::Str,
        CoreTy::Bytes => Tag::Bytes,
        _ => return None,
    })
}

/// Every member compiled code reaches on a `Core` instance **by name** — one
/// row per class, `(member, symbol)`.
///
/// `rule:iteration/two-interfaces`'s
/// iteration trio and nothing else so far. Those three declarations are
/// bodiless (`nvs_types::iter_lib`), so a `foreach` names no helper to call and
/// dispatches on the receiver's runtime class instead — this table is what a
/// `Core` receiver answers that lookup with, and [`crate::cursor`] owns the
/// decision and the one convention difference it carries: a member reached
/// this way is handed its receiver's reference rather than borrowing it.
///
/// Deliberately not a flag on [`registry::CoreMethod`]: a row here is *not* a
/// registered member — nothing resolves `$map->iterate()` in source, no
/// `.nvst` case can call one, and `nvs_stdlib::symbols` does not list it. The
/// registry is the surface a program reaches; this is the protocol the engine
/// reaches.
///
/// **`toString` and `compareTo` are not rows here and never become ones.** The
/// engine reaches both by name too, but through [`ClassDesc::renderer`] and
/// [`ClassDesc::comparer`] rather than the method table, because each is a
/// *registered* member and keeps the ordinary `Core` convention of borrowing
/// its receiver where a row here transfers one — see this module's
/// § *Decision: a registered member the engine reaches by name gets a
/// descriptor field*, and [`descriptors`], which derives both from the
/// registry instead.
const DISPATCH_ROSTER: &[(&str, &[(&str, &str)])] = &[
    (
        crate::objmap::NAME,
        &[(sequence::ITERATE, crate::objmap::ITERATE_SYMBOL)],
    ),
    (
        crate::objset::NAME,
        &[(sequence::ITERATE, crate::objset::ITERATE_SYMBOL)],
    ),
    (
        crate::heap::NAME,
        &[(sequence::ITERATE, crate::heap::ITERATE_SYMBOL)],
    ),
    (
        crate::channel::NAME,
        &[
            (sequence::ITERATE, crate::channel::ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::channel::ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::channel::CURRENT_SYMBOL),
        ],
    ),
    (
        crate::io::LINES_NAME,
        &[(sequence::ITERATE, crate::io::LINES_ITERATE_SYMBOL)],
    ),
    (
        crate::io::WALK_NAME,
        &[(sequence::ITERATE, crate::io::WALK_ITERATE_SYMBOL)],
    ),
    (
        crate::db::ROWS_NAME,
        &[(sequence::ITERATE, crate::db::ROWS_ITERATE_SYMBOL)],
    ),
    // Standard input as chunks, its own iterator for the body stream's reason
    // below: the next chunk has not been read when the walk is named.
    (
        crate::io::STDIN_STREAM_NAME,
        &[
            (sequence::ITERATE, crate::io::STDIN_STREAM_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::io::STDIN_STREAM_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::io::STDIN_STREAM_CURRENT_SYMBOL),
        ],
    ),
    // A class here that carries all three names rather than handing a
    // [`crate::cursor`] back, and for [`crate::channel`]'s reason: the next
    // element does not exist yet when the walk is named. `crate::request`'s
    // `BODY_STREAM` docs are the argument.
    (
        crate::request::BODY_STREAM_NAME,
        &[
            (
                sequence::ITERATE,
                crate::request::BODY_STREAM_ITERATE_SYMBOL,
            ),
            (
                sequence::ADVANCE,
                crate::request::BODY_STREAM_ADVANCE_SYMBOL,
            ),
            (
                sequence::CURRENT,
                crate::request::BODY_STREAM_CURRENT_SYMBOL,
            ),
        ],
    ),
    // And the third, for the third time: `rule:http-server/an-upload-is-received-only-through-files`'s walk over a multipart
    // body's file parts, whose next part has not arrived when the walk is named.
    // `crate::request`'s `FILES` docs are the argument.
    (
        crate::request::FILES_NAME,
        &[
            (sequence::ITERATE, crate::request::FILES_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::request::FILES_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::request::FILES_CURRENT_SYMBOL),
        ],
    ),
    // And the fourth: `rule:http-server/a-part-is-consumed-in-one-of-three-ways`'s walk over one part's bytes, which is the
    // body walk above narrowed to a position in the body. `crate::request`'s
    // `PART_CONTENT` docs are the argument.
    (
        crate::request::PART_CONTENT_NAME,
        &[
            (
                sequence::ITERATE,
                crate::request::PART_CONTENT_ITERATE_SYMBOL,
            ),
            (
                sequence::ADVANCE,
                crate::request::PART_CONTENT_ADVANCE_SYMBOL,
            ),
            (
                sequence::CURRENT,
                crate::request::PART_CONTENT_CURRENT_SYMBOL,
            ),
        ],
    ),
    // And the fifth: `rule:core-classes/db-statement-members`'s walk over a statement's rows, whose next row
    // does not exist until the walk asks the server for it. `crate::db::stream`
    // is the argument, and it is the same one the three above make.
    (
        crate::db::STREAM_NAME,
        &[
            (sequence::ITERATE, crate::db::STREAM_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::db::STREAM_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::db::STREAM_CURRENT_SYMBOL),
        ],
    ),
    // And `Core\Ldap`'s search, for the same reason: the next entry may be on
    // a page the server has not sent yet.
    (
        crate::ldap::ENTRIES_NAME,
        &[
            (sequence::ITERATE, crate::ldap::ENTRIES_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::ldap::ENTRIES_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::ldap::ENTRIES_CURRENT_SYMBOL),
        ],
    ),
    // And `rule:core-classes/ldap-search-pages-and-never-follows-a-referral`'s
    // sync, whose next answer is asked for the same way.
    (
        crate::ldap::CHANGES_NAME,
        &[
            (sequence::ITERATE, crate::ldap::CHANGES_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::ldap::CHANGES_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::ldap::CHANGES_CURRENT_SYMBOL),
        ],
    ),
    // And `Core\Csv`'s walk over a CSV file's records, which makes the same
    // argument over a descriptor: a record does not exist until the `advance`
    // that answers it has read that far. `crate::csv`'s `ROWS` docs are where
    // it is made, beside why `Core\IO\Lines` — which holds its lines — hands a
    // [`crate::cursor`] back instead.
    (
        crate::csv::ROWS_NAME,
        &[
            (sequence::ITERATE, crate::csv::ROWS_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::csv::ROWS_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::csv::ROWS_CURRENT_SYMBOL),
        ],
    ),
    // And `Core\Json`'s two walks, for `Core\Csv\Rows`' reason: an element is
    // parsed, and a piece written, by the `advance` that answers it.
    (
        crate::json::ELEMENTS_NAME,
        &[
            (sequence::ITERATE, crate::json::ELEMENTS_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::json::ELEMENTS_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::json::ELEMENTS_CURRENT_SYMBOL),
        ],
    ),
    (
        crate::json::PIECES_NAME,
        &[
            (sequence::ITERATE, crate::json::PIECES_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::json::PIECES_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::json::PIECES_CURRENT_SYMBOL),
        ],
    ),
    // `Core\Markdown::toHtmlEach`'s walk, for the same reason: a block group
    // is written by the `advance` that returns it.
    (
        crate::markdown::GROUPS_NAME,
        &[
            (sequence::ITERATE, crate::markdown::GROUPS_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::markdown::GROUPS_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::markdown::GROUPS_CURRENT_SYMBOL),
        ],
    ),
    // A streamed reply's three framings, which carry all three names for the
    // reason every walk above does: an event, a line and a chunk are framed off
    // the reply by the `advance` that answers them, so none of them exists when
    // the walk is named. `crate::http::stream`'s module doc is the argument, and
    // three classes over one implementation is [`registry::ITERABLES`]'s doing
    // rather than this table's.
    (
        crate::http::stream::EVENTS_NAME,
        &[
            (
                sequence::ITERATE,
                crate::http::stream::EVENTS_ITERATE_SYMBOL,
            ),
            (
                sequence::ADVANCE,
                crate::http::stream::EVENTS_ADVANCE_SYMBOL,
            ),
            (
                sequence::CURRENT,
                crate::http::stream::EVENTS_CURRENT_SYMBOL,
            ),
        ],
    ),
    (
        crate::http::stream::LINES_NAME,
        &[
            (sequence::ITERATE, crate::http::stream::LINES_ITERATE_SYMBOL),
            (sequence::ADVANCE, crate::http::stream::LINES_ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::http::stream::LINES_CURRENT_SYMBOL),
        ],
    ),
    (
        crate::http::stream::CHUNKS_NAME,
        &[
            (
                sequence::ITERATE,
                crate::http::stream::CHUNKS_ITERATE_SYMBOL,
            ),
            (
                sequence::ADVANCE,
                crate::http::stream::CHUNKS_ADVANCE_SYMBOL,
            ),
            (
                sequence::CURRENT,
                crate::http::stream::CHUNKS_CURRENT_SYMBOL,
            ),
        ],
    ),
    (
        crate::cursor::NAME,
        &[
            (sequence::ADVANCE, crate::cursor::ADVANCE_SYMBOL),
            (sequence::CURRENT, crate::cursor::CURRENT_SYMBOL),
        ],
    ),
];

/// `class`'s method table, as [`ClassTable::set_methods`] takes it — empty for
/// every class not on [`DISPATCH_ROSTER`], which is most of them.
///
/// Every row here is `native`, which is the whole of what
/// `nvs_runtime::MethodRow` can say about one: these addresses are `rule:errors/propagation`
/// helpers that **borrow** argument 0, and this crate holds no signature for
/// them, so the arity, the parameter tags and the parameter names a compiled
/// method's row carries are left empty rather than guessed. That field is what a caller with no
/// class in hand refuses on; the callers that reach these rows today —
/// `foreach` over a cursor, `crate::sequence` — name the member statically and
/// know the convention because they wrote it.
fn dispatch_table(class: &str) -> Vec<nvs_runtime::MethodRow> {
    DISPATCH_ROSTER
        .iter()
        .find(|(name, _)| *name == class)
        .map(|(_, members)| {
            members
                .iter()
                .map(|(member, symbol)| nvs_runtime::MethodRow {
                    name: (*member).to_owned(),
                    code: crate::address_of(symbol),
                    arity: 0,
                    param_tags: 0,
                    param_names: Vec::new(),
                    param_types: Vec::new(),
                    public: true,
                    protected: false,
                    native: true,
                    param_classes: Vec::new(),
                    param_checks: Vec::new(),
                    is_static: false,
                    by_reference: false,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The process's descriptors and their index by name.
struct Descriptors {
    table: ClassTable,
    /// Every descriptor's class name and id, sorted by name. Equal names keep
    /// the order they were defined in, so [`Descriptors::id_of`] finds the
    /// first, as [`ClassTable::id_of`] does.
    ///
    /// **What it spends:** one name and one id per `Core` descriptor, for the
    /// process. It buys every lookup by name a binary search, where
    /// [`ClassTable::id_of`] scans the whole table, and every construction of
    /// a `Core` instance makes one.
    by_name: Vec<(&'static str, nvs_runtime::ClassId)>,
    /// Each name's first id, so [`Descriptors::id_of`] is one hash and one
    /// compare of the name.
    ///
    /// A `Core` class is a `const` and has no static to keep its descriptor
    /// in, so [`descriptor`] finds one by the class's address in a thread's
    /// cache, and by name here on that cache's miss. A binary search of
    /// [`Self::by_name`] compares the name about eight times, and each name
    /// starts with `Core\`. The map is filled once from the binary's own names
    /// and never written again. A name a program looks up through
    /// [`core_class_desc`] is only read against it, so no input can grow a
    /// bucket, and the hash is `rustc_hash`'s, which is fast and not keyed.
    ///
    /// Four other ways to find a descriptor are declined. An index by the
    /// address of the name literal, or by the class's address built once,
    /// misses: a debug build copies each constant per codegen unit, even for
    /// `registry::CLASSES`' own entries. A compile-time `ClassId` changes the
    /// IR. A field on `CoreClass`, or class statics in place of consts, edits
    /// every class literal in the crate for one pointer.
    ///
    /// **What it spends:** one entry, a name and an id, per `Core` class, for
    /// the process.
    first: rustc_hash::FxHashMap<&'static str, nvs_runtime::ClassId>,
}

impl Descriptors {
    /// Every id defined under `name`, in definition order.
    fn ids_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = nvs_runtime::ClassId> + 'a {
        let start = self.by_name.partition_point(|(own, _)| *own < name);
        self.by_name[start..]
            .iter()
            .take_while(move |(own, _)| *own == name)
            .map(|(_, id)| *id)
    }

    /// The first id defined under `name`, or `None` if no descriptor has it.
    fn id_of(&self, name: &str) -> Option<nvs_runtime::ClassId> {
        self.first.get(name).copied()
    }

    /// `name`'s descriptor, or `None` if no descriptor has it.
    fn find(&self, name: &str) -> Option<*const ClassDesc> {
        Some(self.table.desc(self.id_of(name)?))
    }
}

/// The process's descriptors, built on first use and never dropped — see this
/// module's docs. A `OnceLock<&'static _>` rather than a `RwLock<ClassTable>`:
/// the table is written once and read from every construction on every core, so
/// what a reader needs is the address it was published at and nothing else.
static DESCRIPTORS: OnceLock<&'static Descriptors> = OnceLock::new();

/// The table, building and leaking it if this is the first call.
///
/// Each roster beside the registry is walked once, and each of its names is
/// found in the sorted index. Asking every class whether a roster names it
/// costs a string compare per class and roster entry, and under Miri that
/// was seconds for each test that builds a `Core` instance.
fn descriptors() -> &'static Descriptors {
    DESCRIPTORS.get_or_init(|| {
        // Built from the whole registry for every program alike, so the lookups below are no
        // program's use of a class.
        let _quiet = nvs_footprint::Quiet::new();
        shapes();
        let mut table = ClassTable::new();
        let mut by_name = Vec::new();
        // `Core\Ast\Node`'s id. It is defined with the registry's classes, so
        // before every production that names it as its parent.
        let mut node = None;
        for (class, registered, production) in registry::CLASSES
            .iter()
            .map(|class| (class, true, false))
            .chain(INTERNAL_CLASSES.iter().map(|class| (*class, false, false)))
            // `rule:core-classes/ast-is-inert`'s typed roster, which is the
            // other family of row-less classes and a slice rather than lines
            // above because it is one class per production of the grammar —
            // `crate::ast`'s own second decision owns why none of them is
            // registry surface.
            .chain(
                crate::ast::PRODUCTIONS
                    .iter()
                    .map(|class| (class, false, true)),
            )
        {
            // A **namespace** class is what is skipped here, and
            // [`has_instances`] is the one question that tells one apart.
            if !has_instances(class) {
                continue;
            }
            // A `Core` class has no parents, so `is` on one is true only for
            // itself. A production is the one exception: each is a
            // `Core\Ast\Node`, the type every member that returns a node
            // declares, so `is Core\Ast\Node` is true for each of them.
            let parents: Vec<nvs_runtime::ClassId> = if production {
                node.into_iter().collect()
            } else {
                Vec::new()
            };
            let id = table.define(class.name, class.slots, &parents);
            // Every descriptor this loop defines is a `Core` class, which only
            // a `Core` member makes (`nvs_runtime::ClassDesc::refusal`).
            table.set_core(id);
            if class.name == crate::ast::NODE_NAME {
                node = Some(id);
            }
            by_name.push((class.name, id));
            // The two members below are read off a registry row, so a class
            // with no row has neither.
            if !registered {
                continue;
            }
            // `rule:classes/stringable`'s one rendering member, which is *not* a
            // `DISPATCH_ROSTER` row: it has its own descriptor field because
            // it keeps the ordinary `Core` convention of borrowing its
            // receiver, and it is derived from the registry rather than
            // written down here so that it cannot name a member
            // `registry::class_renders` — the check the compiler makes at
            // `echo $uri` — did not see.
            if let Some(symbol) = registry::render_member(class) {
                table.set_render(id, crate::address_of(symbol));
            }
            // `rule:classes/comparable`'s one ordering member, on its own
            // descriptor field for the reason directly above and derived from
            // the registry for the same one: `registry::implements_comparable`
            // — the check the compiler makes at `$a < $b` — and this are one
            // question asked once, so a class the checker will let a program
            // order cannot be a class a `Core\Heap` finds no ordering for.
            if let Some(symbol) = registry::compare_member(class) {
                table.set_compare(id, crate::address_of(symbol));
            }
        }
        // Filled in definition order, so each name keeps the id defined first.
        let mut first = rustc_hash::FxHashMap::default();
        for (name, id) in &by_name {
            first.entry(*name).or_insert(*id);
        }
        // A stable sort, so equal names keep their definition order.
        by_name.sort_by_key(|(name, _)| *name);
        let mut found = Descriptors {
            table,
            by_name,
            first,
        };
        for (name, _) in DISPATCH_ROSTER {
            let ids: Vec<_> = found.ids_named(name).collect();
            for id in ids {
                found.table.set_methods(id, dispatch_table(name));
            }
        }
        // `rule:security/isolate-values-cross-by-copy`'s third refusal,
        // written at the one place that can know it: the walk at a copy
        // boundary sees a descriptor and nothing else, and [`HOST_HANDLE_CLASSES`]
        // is this crate's answer to which of them may not cross.
        for held in HOST_HANDLE_CLASSES {
            let ids: Vec<_> = found.ids_named(held.name).collect();
            for id in ids {
                found.table.set_host_handle(id);
            }
        }
        for (class, free) in FREES_STATE {
            let ids: Vec<_> = found.ids_named(class.name).collect();
            for id in ids {
                found.table.set_free(id, *free);
            }
        }
        // `rule:classes/serialize-is-a-closed-format`: what each slot holds,
        // for every descriptor of the name. A class with no row keeps neither
        // mark, so the bytes carrier refuses it.
        for (class, slots) in SLOT_TYPES {
            let ids: Vec<_> = found.ids_named(class.name).collect();
            for id in ids {
                match slots {
                    Slots::Typed(types) => {
                        let stated = types
                            .iter()
                            .map(|ty| {
                                slot_type(ty).unwrap_or_else(|| {
                                    panic!(
                                        "{}'s slot type `{}` has no run-time form",
                                        class.name,
                                        ty.spelled()
                                    )
                                })
                            })
                            .collect();
                        found.table.set_slot_types(id, stated);
                    }
                    Slots::NotReadBack(reason) => found.table.set_bytes_refusal(id, reason),
                }
            }
        }
        Box::leak(Box::new(found))
    })
}

/// Whether `class` has instances at all, which is what separates a `Core`
/// class with a descriptor from a **namespace** class — a name for static
/// members that no value is ever one of.
///
/// Either roster being non-empty is the answer, and declaring no slots is not
/// on its own what makes a namespace class: `Core\Socket` has instance members
/// and no slots, because a connection's whole state is its isolate's — the peer
/// and the topic queue are [`nvs_runtime::Ctx`] fields, so the receiver is a
/// handle and there is nothing for it to carry.
fn has_instances(class: &CoreClass) -> bool {
    !class.slots.is_empty() || !class.instance.is_empty()
}

/// [`has_instances`] asked by name, over the registry alone: whether `name` is
/// a `Core` class a value can be an instance of.
///
/// This is the whole of what `nvs_types` needs about a written `Core` name's
/// *identity* — derived from the registry the way every other pass derives it,
/// rather than a second roster that could disagree with [`descriptors`]'s skip
/// — so a name it admits as a downcast target is one a descriptor answers for.
/// A name this answers `false` for has no descriptor here and never will, which
/// is why `nvs-codegen` asks the same question before it emits a class test,
/// rather than leaving the backend a symbol to fail to resolve.
///
/// [`INTERNAL_CLASSES`] and `crate::ast::PRODUCTIONS` are deliberately not
/// consulted: they have descriptors but no registry row, so no source can name
/// one and no checker has a question about them.
#[must_use]
pub fn class_has_instances(name: &str) -> bool {
    registry::class(name).is_some_and(has_instances)
}

/// `class`'s descriptor, found by the class's address on this thread's
/// [`FOUND`] and by its name on a miss.
///
/// The key is the address of the `&'static CoreClass` the caller passed. A
/// `Core` class is a `const`, so one class can have several addresses: each
/// codegen unit may keep its own copy. Each copy is a key of its own, and every
/// key finds the same descriptor. That is why the address keys a cache filled
/// on a miss and never a table built once: no table can list every copy. A
/// `'static` borrow is what makes the key sound, because no other class is ever
/// at that address.
///
/// **What it costs:** one thread-local read and one compare when the entry is
/// there, which is every call after a thread's first for that copy of the
/// class. A miss pays [`Descriptors::find`]'s hash of the name as before.
///
/// # Panics
///
/// Panics naming the class if it declares no slots — a namespace class has no
/// instances, and asking for one is a build-time oversight in this crate.
fn descriptor(class: &'static CoreClass) -> *const ClassDesc {
    let key = std::ptr::from_ref(class).addr();
    // A class is 8-aligned, so the low three bits say nothing.
    let at = (key >> 3) & (CACHED - 1);
    FOUND
        .try_with(|found| {
            let entry = &found[at];
            let (cached, desc) = entry.get();
            if cached == key {
                return desc;
            }
            let desc = descriptor_by_name(class);
            entry.set((key, desc));
            desc
        })
        .unwrap_or_else(|_| descriptor_by_name(class))
}

/// `class`'s descriptor, found by its name: [`descriptor`]'s path on a miss.
fn descriptor_by_name(class: &CoreClass) -> *const ClassDesc {
    descriptors()
        .find(class.name)
        .unwrap_or_else(|| panic!("{} is not a `Core` class with instances", class.name))
}

/// How many entries each thread's [`FOUND`] has. A power of two.
const CACHED: usize = 64;

thread_local! {
    /// The descriptors this thread found last, as `(class address, descriptor)`,
    /// at the slot the address picks. Two classes that pick one slot take turns
    /// in it, and the loser pays a lookup by name. An empty slot's address is
    /// 0, which no class has.
    ///
    /// **What it spends:** 1 KiB per thread, for the thread's life, fixed and
    /// never allocated. It is filled from the process's descriptors, which no
    /// request owns, so a request that fills an entry is charged nothing.
    static FOUND: [std::cell::Cell<(usize, *const ClassDesc)>; CACHED] = const {
        [const { std::cell::Cell::new((0, std::ptr::null())) }; CACHED]
    };
}

/// Every `Core` descriptor a compiled unit may relocate against, as
/// `(class name, descriptor address)` — one pair per registered class with
/// instances ([`has_instances`]).
///
/// Three sites write one of these addresses into a unit. A hole-free
/// `` html`…` `` folds to a `Core\Html\Markup` constant in the unit's constant
/// pool (`rule:core-classes/html-template`), and a written duration to a
/// `Core\Time\Duration` one (`rule:types/duration`), and the class word of each
/// constant is one of them; and `$v is Core\Time\Date` tests against the
/// class's own address, which is the identity comparison [`is_instance`] makes
/// from this side. All are written once while compiling and read by every core
/// afterwards — the half of this module's § *Decision: the descriptors are one
/// leaked table for the process* that a per-core table would fail.
///
/// A namespace class and a row-less internal class are both absent, for
/// [`class_has_instances`]'s reasons: the first has no instances to test for,
/// and the second no name a program can write.
///
/// Separate from [`crate::symbols`] because that roster is *code*, under the
/// name a call site calls it by; a descriptor is data, under a mangled name
/// only the backend can spell (`nvs_codegen::class_desc_symbol`). The two
/// crates agree on the name there and on the address here.
#[must_use]
pub fn class_descriptors() -> Vec<(&'static str, *const ClassDesc)> {
    registry::CLASSES
        .iter()
        .filter(|class| has_instances(class))
        .map(|class| (class.name, descriptor(class)))
        .collect()
}

/// The `Core` half of [`nvs_runtime::Ctx::class_desc`]: a class name to the
/// descriptor this process leaked for it, or `None` for a name this crate does
/// not own.
///
/// This is the whole of what an embedder installs — `Ctx::set_core_classes`
/// takes exactly this signature — and it is a `fn` rather than a table handed
/// over because [`descriptors`] is already the process's, built on first use
/// and never dropped. So a context costs one word to arm and a boot costs
/// nothing at all.
///
/// A **namespace** class answers `None` here, for [`descriptors`]'s own reason:
/// it has no instances, so nothing encodes one and nothing can ask for it back.
#[must_use]
pub fn core_class_desc(name: &str) -> Option<*const ClassDesc> {
    // A lookup by name at run time, so it is the program's use of that class.
    nvs_footprint::class(name);
    descriptors().find(name)
}

/// Whether `value` is an instance of `class`, asked by descriptor address.
///
/// A descriptor's address **is** its identity — [`descriptors`] leaks one table
/// for the process and hands the same pointer back for the same class every
/// time — so this is a pointer comparison and never a name comparison, which
/// would be
/// both slower and true of a program's own class that spelled its name the
/// same way.
///
/// The one caller is a `mixed` parameter that has to tell one `Core` class from
/// everything else: [`crate::db`]'s bind reads `rule:core-classes/db-parameters`'s `Core\Db\InList`
/// out of a `array<mixed>` whose other elements are ordinary values. A member
/// whose parameter is *declared* as a class needs none of this — the checker
/// has already answered it.
pub(crate) fn is_instance(value: Value, class: &'static CoreClass) -> bool {
    let Some(object) = value.obj_ptr() else {
        return false;
    };
    #[expect(
        unsafe_code,
        reason = "the value argument owns a reference to a live allocation, so \
                  it is live for the length of this call"
    )]
    let found = unsafe { NvsObj::class_of(object) };
    std::ptr::eq(found, descriptor(class))
}

/// A fresh instance of `class`, its slots filled from `slots` in declaration
/// order, as the [`Value`] a helper returns.
///
/// Takes over each slot value's reference, exactly as
/// [`NvsObj::set_field`](nvs_runtime::NvsObj::set_field) does — so a caller
/// builds the values it means and hands them straight over.
///
/// # Panics
///
/// Panics if `slots` is not exactly as long as `class.slots`: the layout is
/// this crate's on both sides, so a mismatch is a paste error rather than
/// anything a program can cause.
pub(crate) fn build<const N: usize>(class: &'static CoreClass, slots: [Value; N]) -> Value {
    assert_eq!(
        N,
        class.slots.len(),
        "{} has {} slots, filled with {N} values",
        class.name,
        class.slots.len()
    );
    #[expect(
        unsafe_code,
        reason = "the descriptor is owned by this crate's leaked table, so it \
                  outlives every instance made from it — which is `NvsObj::new`'s \
                  whole safety obligation"
    )]
    let object = unsafe { NvsObj::new(descriptor(class)) };
    for (index, value) in slots.into_iter().enumerate() {
        object.set_field(index, value);
    }
    // A [`SLOT_TYPES`] row is proved by every test that builds the class: a
    // slot this crate filled with another type is a wrong row or a wrong
    // build. A release build pays nothing.
    #[cfg(debug_assertions)]
    if let Err(wrong) = object.check_fields() {
        panic!(
            "{} was built against its stated slot types: {wrong}",
            class.name
        );
    }
    Value::object(object)
}

/// The process's *shape* descriptors — see [`shape`]. A second table beside
/// [`DESCRIPTORS`] rather than more rows in it, because a shape is not a
/// [`CoreClass`]: it has no members, no name a program resolves, and no
/// registry row. Published once for the process for [`DESCRIPTORS`]'s reason,
/// a shape descriptor's address being an identity in exactly the same way.
static SHAPES: OnceLock<&'static ClassTable> = OnceLock::new();

/// [`SHAPES`], built and leaked on the first call.
///
/// [`descriptors`] builds it too, so the compile that relocates against the
/// class table has built this one before a request makes its first issue or
/// script result. The table is the process's, so it is built inside a
/// [`nvs_runtime::budget::Detached`] bracket: whichever thread builds it, no
/// request's balance is charged for it.
///
/// **What it spends:** one descriptor per [`SHAPE_ROSTER`] row, for the
/// process.
fn shapes() -> &'static ClassTable {
    SHAPES.get_or_init(|| {
        let _process = nvs_runtime::budget::Detached::begin();
        let mut table = ClassTable::new();
        for (shape, fields) in SHAPE_ROSTER {
            // No parents, and no methods: `rule:types/anonymous-object` makes a shape value an
            // anonymous *methodless* instance, so there is nothing to inherit
            // and nothing to dispatch.
            table.define(*shape, fields, &[]);
        }
        Box::leak(Box::new(table))
    })
}

/// Every `rule:types/object-top` shape a `Core` member builds a value of, as
/// `(descriptor name, fields in slot order)`.
///
/// **Slot order is the field name order, sorted** — `nvs_types::ty::Ty::Shape`
/// canonicalizes `{ y: …, x: … }` and `{ x: …, y: … }` to one interned type by
/// sorting, so the runtime layout has to be the same order or a written shape
/// type and a built value would disagree about which slot is which.
///
/// **The descriptor name is the shape's label**, `$shape{x,y}` over those sorted
/// fields, which is what `nvs_ir::lower::shape_class_label` names a program's
/// own anonymous object. `nvs_runtime::ClassDesc::is_shape` reads that label, so
/// a `Core`-built shape renders and encodes as one; the test below holds each
/// name to its fields.
const SHAPE_ROSTER: &[(&str, &[&str])] = &[
    (crate::issue::SHAPE, crate::issue::FIELDS),
    (crate::script::RESULT_SHAPE, crate::script::RESULT_FIELDS),
    (crate::script::FAILURE_SHAPE, crate::script::FAILURE_FIELDS),
];

/// `name`'s shape descriptor, built and leaked on first use.
///
/// # Panics
///
/// Panics naming the shape if it is not in [`SHAPE_ROSTER`].
fn shape_descriptor(name: &str) -> *const ClassDesc {
    let table = shapes();
    let id = table
        .id_of(name)
        .unwrap_or_else(|| panic!("{name} is not a `Core`-built shape"));
    table.desc(id)
}

/// A fresh `rule:types/object-top` shape value — `{ path: "…", message: "…" }` — its slots
/// filled from `slots` in [`SHAPE_ROSTER`]'s order.
///
/// The same anonymous methodless instance a Novis anonymous object (`{…}`) builds, so
/// nothing downstream learns that `Core` produced this one. Takes over each
/// slot value's reference, exactly as [`build`] does.
///
/// # Panics
///
/// Panics if `slots` is not exactly as long as the shape's field list, or if
/// `name` is not in [`SHAPE_ROSTER`].
pub(crate) fn shape<const N: usize>(name: &str, slots: [Value; N]) -> Value {
    let fields = SHAPE_ROSTER
        .iter()
        .find(|(shape, _)| *shape == name)
        .map_or(&[][..], |(_, fields)| fields);
    assert_eq!(
        N,
        fields.len(),
        "{name} has {} slots, filled with {N} values",
        fields.len()
    );
    #[expect(
        unsafe_code,
        reason = "the descriptor is owned by this crate's leaked table, so it \
                  outlives every instance made from it — which is `NvsObj::new`'s \
                  whole safety obligation"
    )]
    let object = unsafe { NvsObj::new(shape_descriptor(name)) };
    for (index, value) in slots.into_iter().enumerate() {
        object.set_field(index, value);
    }
    Value::object(object)
}

/// Every `Core` descriptor's address, sorted, beside the registry class whose
/// instance members take an instance of it as their receiver — see
/// [`receivers`].
static RECEIVERS: OnceLock<Vec<(usize, &'static str)>> = OnceLock::new();

/// [`RECEIVERS`], built from [`descriptors`] on the first call.
///
/// A class's own descriptor receives its own members. The one exception is
/// `rule:core-classes/ast-is-inert`'s family: a node is an instance of its
/// production's class, and `Core\Ast\Node`'s members read every one of them,
/// so each production's descriptor receives `Core\Ast\Node`'s members.
///
/// **What it spends:** one address and one name per `Core` descriptor, for the
/// process, bounded by the same rosters [`descriptors`] is.
fn receivers() -> &'static [(usize, &'static str)] {
    RECEIVERS.get_or_init(|| {
        let found = descriptors();
        let mut rows: Vec<(usize, &'static str)> = registry::CLASSES
            .iter()
            .chain(INTERNAL_CLASSES.iter().copied())
            .filter(|class| has_instances(class))
            .map(|class| (class.name, class.name))
            .chain(
                crate::ast::PRODUCTIONS
                    .iter()
                    .map(|production| (production.name, crate::ast::NODE_NAME)),
            )
            .filter_map(|(own, receives)| Some((found.find(own)?.addr(), receives)))
            .collect();
        rows.sort_unstable_by_key(|(address, _)| *address);
        rows
    })
}

/// Whether `left` and `right` are objects of two different classes, at least
/// one of them a `Core` class.
///
/// `crate::ordering`'s object row asks it before it calls a `compareTo`. A
/// `Core` class's comparer reads its argument through [`receiver`], so it
/// cannot order an instance of another class, and the pair gets the ordinary
/// "no natural order" throw rather than [`receiver`]'s fatal error. Two
/// objects of a program's own classes are not asked about here: one may
/// extend the other, and their `compareTo` decides.
pub(crate) fn is_mixed_core_pair(left: Value, right: Value) -> bool {
    let (Some(left), Some(right)) = (left.obj_ptr(), right.obj_ptr()) else {
        return false;
    };
    #[expect(
        unsafe_code,
        reason = "both value arguments own a reference to a live allocation, so \
                  each is live for the length of this call"
    )]
    let (left, right) = unsafe { (NvsObj::class_of(left), NvsObj::class_of(right)) };
    if std::ptr::eq(left, right) {
        return false;
    }
    let rows = receivers();
    [left, right].iter().any(|found| {
        rows.binary_search_by_key(&found.addr(), |(address, _)| *address)
            .is_ok()
    })
}

/// The receiver of an instance member, as a raw object pointer live for the
/// length of the call.
///
/// **It checks the receiver's class**, which is
/// `rule:core-api/core-classes-are-sealed`'s second half. The member reads its
/// slots by index against `class`'s layout, so an object of any other class
/// would have them read as something they are not: a string's pointer as an
/// integer, or a slot past the end of the object. The link pass already
/// refuses a program's class that extends or implements a `Core` class; this
/// check holds whatever path the checker did not see.
///
/// **What it costs:** [`descriptor`]'s cached read and one pointer compare, on
/// every call to a `Core` instance member. Only a receiver of another class,
/// which is a `Core\Ast\Node` member's production or a fault, pays a binary
/// search of [`receivers`] and one name comparison. No memory per call.
///
/// # Errors
///
/// A `Fault::fatal` if the slot does not hold an object, or holds an object
/// whose class is not `class`. Compiled code wrote the tag and `nvs_types`
/// checked the receiver's declared type, so neither is a program's mistake.
pub(crate) fn receiver(
    value: Value,
    class: &'static CoreClass,
    member: &str,
) -> Result<*mut ObjHeader, Fault> {
    let Some(object) = value.obj_ptr() else {
        return Err(Fault::fatal(format!(
            "{}::{member} expected {:?} for its receiver, got tag {}",
            class.name,
            Tag::Object,
            value.tag_byte()
        )));
    };
    #[expect(
        unsafe_code,
        reason = "the value argument owns a reference to a live allocation, so \
                  it is live for the length of this call"
    )]
    let found = unsafe { NvsObj::class_of(object) };
    if std::ptr::eq(found, descriptor(class)) {
        return Ok(object);
    }
    // A node is an instance of its production's class, and `Core\Ast\Node`'s
    // members receive it, so only the rows can say yes here.
    let rows = receivers();
    let receives = rows
        .binary_search_by_key(&found.addr(), |(address, _)| *address)
        .ok()
        .map(|at| rows[at].1);
    if receives == Some(class.name) {
        return Ok(object);
    }
    #[expect(
        unsafe_code,
        reason = "every object's class word points at a descriptor that outlives it"
    )]
    // SAFETY: the object is live for this call, so its descriptor is.
    let named = unsafe { (*found).name() };
    Err(Fault::fatal(format!(
        "{}::{member} expected a `{}` receiver, got an instance of `{named}`",
        class.name, class.name
    )))
}

/// Overwrites slot `index` of `receiver`, **releasing** what it held and
/// taking over `value`'s reference — the write half of [`slot`].
///
/// The one thing that mutates a built instance. The classes whose state changes
/// after they are built — the collections, and the streams and cursors that
/// advance — write through it, and every other class here is built once and read.
pub(crate) fn set_slot(receiver: *mut ObjHeader, index: usize, value: Value) {
    #[expect(
        unsafe_code,
        reason = "the receiver argument owns a reference to a live allocation, so \
                  it is live for the length of the call, the index is one of this \
                  crate's own slot constants, and the slot held a well-formed \
                  `Value` this object owned"
    )]
    unsafe {
        nvs_runtime::nvs_object_field_set(receiver, index, value);
    }
}

/// Takes slot `index`'s reference out of `receiver`, leaving `null` in the
/// slot — the first half of growing what a slot holds in place, which
/// [`set_slot`] finishes by writing the grown value back.
///
/// The caller then owns the reference the slot owned, so a string or an array
/// it held alone is solely owned by the caller, and an append into it writes
/// in place rather than copying. Where something else also holds that value,
/// the append's own copy-on-write still separates the two.
pub(crate) fn take_slot(receiver: *mut ObjHeader, index: usize) -> Value {
    let held = slot(receiver, index);
    #[expect(
        unsafe_code,
        reason = "the slot holds a live reference for the length of the call, which \
                  is `Value::retain`'s whole obligation"
    )]
    unsafe {
        held.retain();
    }
    set_slot(receiver, index, Value::null());
    held
}

/// Appends `suffix`'s bytes to the string in slot `index` of `receiver`.
///
/// The bytes are written in place when the slot was the string's one owner
/// and it has room, and [`nvs_runtime::nvs_str_append`]'s doubling makes the
/// room. A loop of these is therefore linear in the bytes appended. Both
/// strings must be in a live slot, and they may be the same allocation.
pub(crate) fn append_to_text_slot(receiver: *mut ObjHeader, index: usize, suffix: Value) {
    let held = take_slot(receiver, index);
    let (Some(target), Some(suffix)) = (held.str_ptr(), suffix.str_ptr()) else {
        unreachable!("a carrier's text slot holds a string, which its caller checked");
    };
    #[expect(
        unsafe_code,
        reason = "`target`'s reference was taken out of the slot and is handed to \
                  `nvs_str_append`, which yields the one that replaces it; `suffix` \
                  is owned by a slot that is live for the length of the call"
    )]
    let grown =
        unsafe { nvs_runtime::NvsStr::from_raw(nvs_runtime::nvs_str_append(target, suffix)) };
    #[expect(clippy::disallowed_methods, reason = "two `string`s joined")]
    set_slot(receiver, index, Value::str(grown));
}

/// Appends `suffix` to the string in slot `index` of `receiver` —
/// [`append_to_text_slot`] for Rust text a member has in hand, so the suffix
/// is never a string of its own.
///
/// An append that fits the string's room allocates nothing, and the room
/// doubles, so a loop of these is linear in the bytes appended.
pub(crate) fn append_str_to_text_slot(receiver: *mut ObjHeader, index: usize, suffix: &str) {
    let held = take_slot(receiver, index);
    let Some(target) = held.str_ptr() else {
        unreachable!("a text slot holds a string, which its caller checked");
    };
    #[expect(
        unsafe_code,
        reason = "`target`'s reference was taken out of the slot, so this handle \
                  owns it, and the append's result goes back into that slot"
    )]
    let owned = unsafe { nvs_runtime::NvsStr::from_raw(target) };
    #[expect(clippy::disallowed_methods, reason = "a `string` followed by a `&str`")]
    set_slot(receiver, index, Value::str(owned.append(suffix.as_bytes())));
}

/// Whether `receiver` has exactly one owner — the one test that lets a
/// carrier's composition write into its left operand.
///
/// An argument of a `CoreCall` is borrowed, so the one owner is whatever holds
/// the value the caller passed. Only `nvs_ir`'s lowering of `$x = $x + …` calls
/// a member that asks this, and there that owner is `$x`, which the statement
/// re-points at the answer anyway.
pub(crate) fn sole_owner(receiver: *mut ObjHeader) -> bool {
    #[expect(
        unsafe_code,
        reason = "the receiver argument owns a reference to a live allocation, so \
                  it is live for the length of the call"
    )]
    unsafe {
        NvsObj::refcount_of(receiver) == 1
    }
}

/// Slot `index` of `receiver`, **borrowed** — the caller takes no reference,
/// exactly as `nvs_ir::InstKind::FieldGet` does not.
///
/// A member handing the result back to Novis code owes it a
/// [`Value::retain`](nvs_runtime::Value::retain) first; one reading it in
/// passing owes nothing.
pub(crate) fn slot(receiver: *mut ObjHeader, index: usize) -> Value {
    #[expect(
        unsafe_code,
        reason = "the receiver argument owns a reference to a live allocation, so \
                  it is live for the length of the call, and the index is one of \
                  this crate's own slot constants"
    )]
    unsafe {
        nvs_runtime::nvs_object_field_get(receiver, index)
    }
}

/// Slot `index` of the receiver in `args[0]`, **retained** for the caller —
/// the whole of a reader on a `Core` class that is built once and never
/// written to.
///
/// One function rather than one per class, because a reader is the same three
/// steps wherever it is written: check the receiver's tag, borrow the slot,
/// and take the reference the value is handed back with.
/// `Core\Socket\Message`'s four readers and `Core\Sse\Message`'s two are the
/// callers, and neither module spells the retain itself.
///
/// # Errors
///
/// The [`receiver`] fault for a receiver that is not an object of `class`,
/// which compiled code cannot produce.
pub(crate) fn read_slot(
    args: &[Value],
    class: &'static CoreClass,
    index: usize,
    member: &str,
) -> Result<Value, Fault> {
    let held = slot(receiver(args[0], class, member)?, index);
    #[expect(
        unsafe_code,
        reason = "the slot is owned by the receiver, which the argument slot holds a \
                  reference to for the length of the call, so the copy handed back to \
                  Novis code needs a reference of its own"
    )]
    // SAFETY: the receiver is live for the length of this call, so its slot is.
    unsafe {
        held.retain();
    }
    Ok(held)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every registered class with instances gets a descriptor of the right
    /// width, and asking twice answers with the same address — which is what
    /// makes a descriptor's address an identity rather than a per-call
    /// accident.
    ///
    /// The skip is [`descriptors`]'s own, so a slotless class with instance
    /// members — `Core\Socket` — is covered here rather than silently passed
    /// over.
    #[test]
    fn one_descriptor_per_instance_class() {
        for class in registry::CLASSES {
            if !has_instances(class) {
                continue;
            }
            let first = descriptor(class);
            assert!(std::ptr::eq(first, descriptor(class)));
            #[expect(
                unsafe_code,
                reason = "the leaked table owns the descriptor for the whole \
                          process, so this borrow is sound for any lifetime"
            )]
            let desc = unsafe { &*first };
            assert_eq!(desc.name(), class.name);
            assert_eq!(desc.field_count(), class.slots.len());
        }
    }

    /// The address path finds what the name finds, for every class with a
    /// descriptor, on the miss that fills the entry and on the hit after it.
    /// A second copy of each class at another address is a key of its own and
    /// finds the same descriptor. There are more classes than entries, so two
    /// classes share a slot and take turns in it, and each turn still finds
    /// the right descriptor.
    #[test]
    fn every_core_class_with_instances_finds_its_descriptor_by_address() {
        let classes: Vec<&'static CoreClass> = registry::CLASSES
            .iter()
            .chain(INTERNAL_CLASSES.iter().copied())
            .chain(crate::ast::PRODUCTIONS.iter())
            .filter(|class| has_instances(class))
            .collect();
        assert!(classes.len() > CACHED, "the walk must cover a shared slot");
        // A static is its own allocation, so each is a second address of its class.
        static DATE: CoreClass = crate::time::DATE;
        static MARKUP: CoreClass = crate::html::MARKUP;
        for _ in 0..2 {
            for class in classes.iter().copied().chain([&DATE, &MARKUP]) {
                let named = descriptors().find(class.name).expect("a descriptor");
                assert!(std::ptr::eq(descriptor(class), named), "{}", class.name);
                assert!(std::ptr::eq(descriptor(class), named), "{}", class.name);
            }
        }
        assert!(!std::ptr::eq(&DATE, &crate::time::DATE));
    }

    /// The published roster and the question the checker asks are the same
    /// skip: a name `nvs_types` admits as a downcast target has an address
    /// `nvs-codegen` can relocate against, and a namespace class is in neither —
    /// which is what keeps a refusal at the checker from becoming an unresolved
    /// symbol at the backend.
    #[test]
    fn the_published_roster_is_what_the_checker_admits() {
        let published: Vec<&str> = class_descriptors()
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        for class in registry::CLASSES {
            assert_eq!(
                published.contains(&class.name),
                class_has_instances(class.name),
                "{} is published and admitted differently",
                class.name
            );
        }
        assert!(published.contains(&crate::html::MARKUP_NAME));
        assert!(!class_has_instances(r"Core\Str"));
        assert!(!class_has_instances(r"Core\Nothing\Registered"));
    }

    /// `rule:security/isolate-values-cross-by-copy`'s refusal is read off the
    /// descriptor, so every [`HOST_HANDLE_CLASSES`] entry has to reach one
    /// carrying the bit: a roster line naming a class this crate no longer
    /// registers would otherwise leave that handle crossing and say nothing.
    #[test]
    fn every_class_on_the_handle_roster_carries_the_bit() {
        for class in HOST_HANDLE_CLASSES {
            #[expect(
                unsafe_code,
                reason = "the leaked table owns the descriptor for the whole \
                          process, so this borrow is sound for any lifetime"
            )]
            let desc = unsafe { &*descriptor(class) };
            assert!(
                desc.holds_host_handle(),
                "{} is on the handle roster and its descriptor does not say so",
                class.name
            );
        }

        // And it is the roster's bit, not every `Core` class's: a value that is
        // nothing but its own state crosses a boundary like any other.
        #[expect(
            unsafe_code,
            reason = "the leaked table owns the descriptor for the whole \
                      process, so this borrow is sound for any lifetime"
        )]
        let date = unsafe { &*descriptor(&crate::time::DATE) };
        assert!(!date.holds_host_handle());
    }

    /// A second core answers with the *same* descriptor address, for a class
    /// and for a shape alike — this module's § *Decision: the descriptors are
    /// one leaked table for the process*.
    ///
    /// This is the half a per-thread table would fail while still passing the
    /// test above: a compiled unit writes `Core\Html\Markup`'s address into a
    /// data section once, while compiling, and every core reads those same
    /// bytes (`rule:core-classes/html-template`). An address is carried across
    /// as a `usize` because a raw pointer is not `Send`, which is the whole
    /// property under test stated in the type system.
    /// Each `Core`-built shape is named by the label of its own sorted fields,
    /// the label `nvs_ir::lower::shape_class_label` mints for a program's
    /// anonymous object, so `ClassDesc::is_shape` is true of it. The crates
    /// cannot see each other, so the format is spelled here, and
    /// `tests/conformance/isolate/an-await-result-renders-and-encodes-as-a-shape.nvst`
    /// holds the two spellings together.
    #[test]
    fn every_core_shape_is_named_by_its_sorted_field_label() {
        for (name, fields) in SHAPE_ROSTER {
            assert!(fields.is_sorted(), "{name}'s fields are not sorted");
            assert_eq!(*name, format!("$shape{{{}}}", fields.join(",")));
            shape_descriptor(name);
            let table = SHAPES.get().expect("the table was just built");
            let desc = table.desc_of(name).expect("the roster row is defined");
            assert!(desc.is_shape(), "{name} is not a shape");
        }
    }

    /// The hash map and the sorted index agree: for every name, the map's id
    /// is the first one the index has, so a class found by either is the same
    /// descriptor.
    #[test]
    fn every_core_class_finds_its_descriptor_in_one_hash_lookup() {
        let found = descriptors();
        for (name, _) in &found.by_name {
            assert_eq!(
                found.first.get(name).copied(),
                found.ids_named(name).next(),
                "{name}'s first id differs between the map and the index"
            );
        }
        assert_eq!(
            found.first.len(),
            found
                .by_name
                .iter()
                .map(|(name, _)| name)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        );
        assert!(found.id_of("Core\\NoSuchClass").is_none());
        assert!(std::ptr::eq(
            descriptor(&crate::html::MARKUP),
            found.table.desc(
                found
                    .ids_named(crate::html::MARKUP.name)
                    .next()
                    .expect("it has instances")
            )
        ));
    }

    #[test]
    fn every_core_sees_one_descriptor_address() {
        let here = (
            descriptor(&crate::html::MARKUP).addr(),
            shape_descriptor(crate::issue::SHAPE).addr(),
        );
        let there = std::thread::spawn(|| {
            (
                descriptor(&crate::html::MARKUP).addr(),
                shape_descriptor(crate::issue::SHAPE).addr(),
            )
        })
        .join()
        .expect("the probing thread does not panic");
        assert_eq!(here, there);
    }

    /// The same pairing for `rule:classes/stringable`'s rendering: a class
    /// [`registry::class_renders`] answers `true` for is one the checker lets
    /// `echo $x` compile against, so a `mixed` holding one has to render at
    /// run time rather than throw. The two ways it can are the two rows of
    /// that check, and this asserts each class takes exactly one of them —
    /// a carrier through `rule:security/capture-answers-the-carrier`'s slot, everything else through the
    /// descriptor's own renderer.
    #[test]
    fn every_rendering_class_carries_a_renderer_or_is_a_carrier() {
        for class in registry::CLASSES {
            if !registry::class_renders(class.name) {
                assert!(
                    class.slots.is_empty() || renderer_of(class).is_none(),
                    "{} renders at run time but not where it is written",
                    class.name
                );
                continue;
            }
            if nvs_runtime::is_carrier(class.name) {
                continue;
            }
            assert!(
                !class.slots.is_empty(),
                "{} declares a `toString` but has no instances to render",
                class.name
            );
            assert!(
                renderer_of(class).is_some(),
                "{} renders where it is written but not at run time",
                class.name
            );
        }
    }

    /// `class`'s descriptor's native renderer — the test-side spelling of the
    /// read `nvs_runtime::stringify`'s dispatch makes.
    fn renderer_of(class: &'static CoreClass) -> Option<*const u8> {
        #[expect(
            unsafe_code,
            reason = "the leaked table owns the descriptor for the whole \
                      process, so this borrow is sound for any lifetime"
        )]
        unsafe { &*descriptor(class) }.renderer()
    }

    /// The same pairing for `rule:classes/comparable`'s ordering, and the
    /// gap it closes: a class the checker lets `$a < $b` compile against is
    /// one a `Core\Heap` holding it with no comparator has to order at run
    /// time, and the only thing that reaches the member there is the
    /// descriptor. Asserted in both directions, and against the class's own
    /// registered symbol rather than merely against "some address", so a
    /// derivation that found *a* member would still fail.
    #[cfg_attr(
        miri,
        ignore = "it compares function addresses, and Miri gives one function several"
    )]
    #[test]
    fn every_core_class_declaring_compare_to_has_a_dispatch_row() {
        let mut comparable = 0;
        for class in registry::CLASSES {
            // [`descriptors`]'s own skip: a namespace class has no descriptor
            // to ask, and asking for one panics.
            if class.slots.is_empty() && class.instance.is_empty() {
                assert!(
                    registry::compare_member(class).is_none(),
                    "{} declares a comparison but has no instances to order",
                    class.name
                );
                continue;
            }
            let Some(symbol) = registry::compare_member(class) else {
                assert!(
                    comparer_of(class).is_none(),
                    "{} orders at run time but declares no `{}`",
                    class.name,
                    nvs_runtime::COMPARE_TO
                );
                continue;
            };
            comparable += 1;
            assert!(
                registry::implements_comparable(class.name),
                "{} carries a comparison the interface check does not see",
                class.name
            );
            assert_eq!(
                comparer_of(class),
                Some(crate::address_of(symbol)),
                "{} orders where it is written but not at run time",
                class.name
            );
        }
        assert!(
            comparable > 0,
            "no `Core` class declares `{}`, so this asserted nothing",
            nvs_runtime::COMPARE_TO
        );
    }

    /// `class`'s descriptor's native comparison — the test-side spelling of
    /// the read `nvs_runtime::dispatch::call_compare_to` makes.
    fn comparer_of(class: &'static CoreClass) -> Option<*const u8> {
        #[expect(
            unsafe_code,
            reason = "the leaked table owns the descriptor for the whole \
                      process, so this borrow is sound for any lifetime"
        )]
        unsafe { &*descriptor(class) }.comparer()
    }

    /// The two rosters this module's docs pair up: a class the checker will
    /// let a `foreach` compile over has to answer the protocol at run time, or
    /// the program type-checks and faults.
    #[test]
    fn every_iterable_class_answers_the_iteration_protocol() {
        for (class, _) in registry::ITERABLES {
            assert!(
                dispatch_table(class)
                    .iter()
                    .any(|row| row.name == sequence::ITERATE),
                "{class} is `Iterable` to the checker and answers no `{}` at run time",
                sequence::ITERATE
            );
        }
        let cursor = dispatch_table(crate::cursor::NAME);
        for member in [sequence::ADVANCE, sequence::CURRENT] {
            assert!(
                cursor.iter().any(|row| row.name == member),
                "the cursor every `iterate()` hands back owes `{member}`"
            );
        }
    }

    /// A dispatch roster row reaches the descriptor, which is what a
    /// `CallVirtual` reads — the half `dispatch_table` alone does not prove.
    #[test]
    fn a_dispatch_row_lands_on_the_descriptor() {
        let desc = descriptor(&crate::cursor::CLASS);
        #[expect(
            unsafe_code,
            reason = "the leaked table owns the descriptor for the whole \
                      process, so this borrow is sound for any lifetime"
        )]
        let desc = unsafe { &*desc };
        assert!(desc.method(sequence::ADVANCE).is_some());
        assert!(desc.method(sequence::CURRENT).is_some());
        assert!(desc.method("nothing").is_none());
    }

    /// `rule:core-api/core-classes-are-sealed`'s runtime half, with the link
    /// pass out of the way: `Core\Test\Response::status` is called directly,
    /// the way compiled code calls it, on objects the checker would never let
    /// reach it. A program's class with a string in slot 0 and an instance of
    /// another `Core` class are both refused, before any slot is read. The
    /// member's own class still reads, and so does an `Core\Ast` production
    /// through `Core\Ast\Node`'s members.
    #[test]
    fn a_receiver_of_another_class_is_refused_before_a_slot_is_read() {
        use crate::test::{RESPONSE, nvs_core_test_response_status};

        let table: &'static mut ClassTable = Box::leak(Box::new(ClassTable::new()));
        let forged_id = table.define(r"Forged", &["note"], &[]);
        #[expect(
            unsafe_code,
            reason = "the table is leaked, so the descriptor outlives every \
                      instance made from it, which is `NvsObj::new`'s obligation"
        )]
        let forged = unsafe { NvsObj::new(table.desc(forged_id)) };
        forged.set_field(0, Value::text("x"));
        let forged = Value::object(forged);
        let other = build(&crate::bigint::CLASS, [Value::null(), Value::int(7)]);
        let genuine = build(
            &RESPONSE,
            [
                Value::int(201),
                Value::text(""),
                Value::array(nvs_runtime::NvsArray::new()),
                Value::null(),
            ],
        );

        let mut ctx = nvs_runtime::Ctx::buffered();
        for wrong in [forged, other] {
            assert!(
                nvs_runtime::call(nvs_core_test_response_status, &mut ctx, &[wrong]).is_err(),
                "a receiver of another class is refused"
            );
            let err = receiver(wrong, &RESPONSE, "status").expect_err("refused");
            assert!(
                format!("{err:?}").contains("Response::status expected"),
                "{err:?}"
            );
        }
        let status = nvs_runtime::call(nvs_core_test_response_status, &mut ctx, &[genuine])
            .expect("the member's own class is its receiver");
        assert_eq!(status.as_int(), Some(201));

        let node = build(
            &crate::ast::PRODUCTIONS[0],
            [
                Value::null(),
                Value::array(nvs_runtime::NvsArray::new()),
                Value::int(1),
                Value::int(1),
                Value::int(0),
            ],
        );
        assert!(receiver(node, &crate::ast::NODE, "kind").is_ok());
        assert!(receiver(genuine, &crate::ast::NODE, "kind").is_err());

        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference each `build` and \
                      `Value::object` produced, and releases each once"
        )]
        unsafe {
            for value in [forged, other, genuine, node] {
                value.release();
            }
        }
    }

    /// A built instance holds exactly what it was given, and one reference —
    /// the property every member returning one depends on, since the compiled
    /// caller releases that one reference and nothing else does.
    #[test]
    fn a_built_instance_owns_one_reference_and_its_slots() {
        let value = build(&crate::bigint::CLASS, [Value::null(), Value::int(7)]);
        let ptr = value.obj_ptr().expect("a built instance is an object");
        assert_eq!(slot(ptr, 1).as_int(), Some(7));
        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference `build` produced, and \
                      releasing it is what the compiled caller would do"
        )]
        unsafe {
            value.release();
        }
    }

    /// Bytes that name `desc`'s class with its own slot names, each `null`.
    /// They are written by the real encoder, from a twin class with the same
    /// name and slots and none of the marks, because `encode` writes nothing
    /// for a class it refuses.
    fn payload_naming(desc: &ClassDesc) -> Vec<u8> {
        payload_holding(
            desc,
            (0..desc.field_count()).map(|_| Value::null()).collect(),
        )
    }

    /// [`payload_naming`], with `values` in the slots in slot order. Takes
    /// over each value's reference.
    fn payload_holding(desc: &ClassDesc, values: Vec<Value>) -> Vec<u8> {
        let fields: Vec<&str> = (0..desc.field_count())
            .map(|index| desc.field_name(index).unwrap_or_default())
            .collect();
        assert_eq!(
            values.len(),
            fields.len(),
            "one value per slot of {}",
            desc.name()
        );
        let mut twins = ClassTable::new();
        let id = twins.define(desc.name(), &fields, &[]);
        let twins: &'static ClassTable = Box::leak(Box::new(twins));
        #[expect(unsafe_code, reason = "the leaked table outlives the object")]
        let object = unsafe { nvs_runtime::NvsObj::new(twins.desc(id)) };
        for (index, value) in values.into_iter().enumerate() {
            object.set_field(index, value);
        }
        nvs_runtime::encode(Value::object(object)).expect("the twin has no marks, so it encodes")
    }

    /// The process's descriptor for `class`, borrowed.
    fn desc_of(class: &'static CoreClass) -> &'static ClassDesc {
        #[expect(
            unsafe_code,
            reason = "the leaked table owns the descriptor for the whole process"
        )]
        unsafe {
            &*descriptor(class)
        }
    }

    /// Releases each value this frame owns one reference to.
    fn release_all(values: impl IntoIterator<Item = Value>) {
        #[expect(unsafe_code, reason = "the caller owns one reference to each")]
        unsafe {
            for value in values {
                value.release();
            }
        }
    }

    /// The four slots of a `Core\Http\TlsInfo` as a handshake fills them.
    fn tls_slots() -> Vec<Value> {
        let mut chain = nvs_runtime::NvsArray::new();
        chain.append(Value::bytes(nvs_runtime::NvsStr::new(b"\x30\x82")));
        vec![
            Value::text("TLSv1.3"),
            Value::text("TLS_AES_128_GCM_SHA256"),
            Value::bool(true),
            Value::array(chain),
        ]
    }

    #[test]
    fn decode_checks_every_slot_of_a_core_class() {
        let tls = &crate::http::TLS_INFO;
        let desc = desc_of(tls);
        assert!(
            desc.states_slot_types(),
            "{} states its slot types",
            tls.name
        );
        let resolve = nvs_runtime::Receiving::over(None, Some(core_class_desc));
        // One value of another kind for each slot, in slot order. The chain's
        // is an array of the wrong element type.
        let wrong = || {
            let mut texts = nvs_runtime::NvsArray::new();
            texts.append(Value::text("not bytes"));
            [
                Value::int(13),
                Value::bool(false),
                Value::text("yes"),
                Value::array(texts),
            ]
        };
        for (index, slot_name) in tls.slots.iter().enumerate() {
            let mut values = tls_slots();
            let mut others = Vec::from(wrong());
            let replaced = std::mem::replace(&mut values[index], others.remove(index));
            release_all(others);
            release_all([replaced]);
            let bytes = payload_holding(desc, values);
            let Err(why) = nvs_runtime::decode(&bytes, &resolve) else {
                panic!("{} decoded with a wrong `{slot_name}`", tls.name);
            };
            let why = why.to_string();
            assert!(
                why.contains(tls.name) && why.contains(&format!("`{slot_name}`")),
                "the refusal of a wrong `{slot_name}` names the class and the slot: {why}"
            );
        }
        let bytes = payload_holding(desc, tls_slots());
        let decoded = nvs_runtime::decode(&bytes, &resolve).expect("the stated kinds decode");
        let version = read_slot(&[decoded], tls, 0, "version").expect("a `TlsInfo` receiver");
        assert_eq!(version.as_text(), Some("TLSv1.3"));
        release_all([version, decoded]);
    }

    /// A `Core\Uri` that `Core\Uri::parse` made goes through the bytes carrier
    /// and back, and each member returns on the copy what it returned on the
    /// first one.
    #[test]
    fn a_core_value_a_member_made_decodes_back() {
        use nvs_runtime::{Ctx, OutputSink, call};
        let mut ctx = Ctx::new(OutputSink::Sink);
        let text = Value::text("https://user@example.com:8443/a/b?x=1#top");
        let made =
            call(crate::uri::nvs_core_uri_parse, &mut ctx, &[text]).expect("a URI reference");
        release_all([text]);
        // `encode` consumes one reference, so it gets one of its own.
        #[expect(unsafe_code, reason = "this frame owns `made`")]
        unsafe {
            made.retain();
        }
        let bytes = nvs_runtime::encode(made).expect("a `Core\\Uri` states its slot types");
        let resolve = nvs_runtime::Receiving::over(None, Some(core_class_desc));
        let back = nvs_runtime::decode(&bytes, &resolve).expect("the bytes `encode` wrote decode");
        assert!(
            is_instance(back, &crate::uri::CLASS),
            "the copy is a `Core\\Uri`"
        );
        let members: [(&str, nvs_runtime::NvsFn); 6] = [
            ("scheme", crate::uri::nvs_core_uri_scheme),
            ("host", crate::uri::nvs_core_uri_host),
            ("port", crate::uri::nvs_core_uri_port),
            ("path", crate::uri::nvs_core_uri_path),
            ("query", crate::uri::nvs_core_uri_query),
            ("fragment", crate::uri::nvs_core_uri_fragment),
        ];
        for (member, helper) in members {
            let first = call(helper, &mut ctx, &[made]).expect("a `Uri` member");
            let copy = call(helper, &mut ctx, &[back]).expect("a `Uri` member");
            assert!(
                first.as_text() == copy.as_text() && first.as_int() == copy.as_int(),
                "`{member}` returns what it returned on the first one"
            );
            assert!(
                first.as_text().is_some() || first.as_int().is_some(),
                "`{member}` has a value"
            );
            release_all([first, copy]);
        }
        release_all([made, back]);
    }

    #[test]
    fn a_core_class_that_states_no_slot_types_does_not_decode() {
        // Its row gives a reason, which ends the sentence.
        let class = &crate::bigint::CLASS;
        let desc = desc_of(class);
        assert!(
            !desc.states_slot_types() && !desc.holds_host_handle(),
            "{} is a `Core` class with no row that may cross a live boundary",
            class.name
        );
        let sentence = format!(
            "`{}` cannot be written to bytes or read from bytes",
            class.name
        );
        let resolve = nvs_runtime::Receiving::over(None, Some(core_class_desc));
        let Err(decoded) = nvs_runtime::decode(&payload_naming(desc), &resolve) else {
            panic!("{} decoded with no stated slot types", class.name);
        };
        assert!(decoded.to_string().contains(&sentence), "{decoded}");
        let built = build(class, [Value::null(), Value::int(7)]);
        // `encode` consumes one reference, so it gets one of its own.
        #[expect(unsafe_code, reason = "this frame owns `built`")]
        unsafe {
            built.retain();
        }
        let Err(encoded) = nvs_runtime::encode(built) else {
            panic!("{} encoded with no stated slot types", class.name);
        };
        assert_eq!(
            encoded.to_string(),
            decoded.to_string(),
            "both directions say one sentence"
        );
        let into = nvs_runtime::Receiving::over(None, Some(core_class_desc));
        let copied = nvs_runtime::copy_graph_to(built, &into).expect("a live copy still crosses");
        assert!(is_instance(copied, class), "the copy is a {}", class.name);
        release_all([copied]);
    }

    /// Whether `name`'s [`SLOT_TYPES`] row states slot `index` as `mixed`.
    fn stated_mixed(name: &str, index: usize) -> bool {
        SLOT_TYPES.iter().any(|(class, slots)| {
            class.name == name
                && matches!(slots, Slots::Typed(types) if matches!(types.get(index), Some(CoreTy::Mixed)))
        })
    }

    /// Walks every descriptor the process defined, the ones with no registry
    /// row included, and holds no list of its own.
    #[test]
    fn every_core_descriptor_states_its_slots_or_is_refused() {
        let table = descriptors();
        let resolve = nvs_runtime::Receiving::over(None, Some(core_class_desc));
        let mut neither = Vec::new();
        for (name, id) in &table.by_name {
            #[expect(
                unsafe_code,
                reason = "the leaked table owns the descriptor for the whole process"
            )]
            let desc = unsafe { &*table.table.desc(*id) };
            if !desc.states_slot_types() {
                if nvs_runtime::decode(&payload_naming(desc), &resolve).is_ok() {
                    neither.push(format!("{name} states no slot types and decoded"));
                }
                continue;
            }
            for index in 0..desc.field_count() {
                let slot_name = desc.field_name(index).unwrap_or_default();
                // A value of another kind than the slot states: the first of
                // these that the stated type does not admit.
                let candidates = [Value::float(0.5), Value::bool(true)];
                let Some(wrong) = candidates
                    .into_iter()
                    .find(|value| desc.check_slot(index, slot_name, *value).is_err())
                else {
                    // A slot its row states as `mixed` admits every value.
                    if stated_mixed(name, index) {
                        continue;
                    }
                    neither.push(format!("{name}::{slot_name} admits a float and a bool"));
                    continue;
                };
                let values = (0..desc.field_count())
                    .map(|at| if at == index { wrong } else { Value::null() })
                    .collect();
                if nvs_runtime::decode(&payload_holding(desc, values), &resolve).is_ok() {
                    neither.push(format!(
                        "{name}::{slot_name} decoded a value of another kind"
                    ));
                }
            }
        }
        assert!(neither.is_empty(), "{}", neither.join("\n"));
    }

    /// Walks the registry's classes with instances, not a list written here, so
    /// a class added later fails here until its row says what it holds.
    #[test]
    fn every_core_class_with_instances_says_what_its_slots_hold() {
        let held = |class: &CoreClass| HOST_HANDLE_CLASSES.iter().any(|h| h.name == class.name);
        let row = |class: &CoreClass| SLOT_TYPES.iter().find(|(c, _)| c.name == class.name);
        let mut missing = Vec::new();
        let mut checked = 0;
        for class in registry::CLASSES
            .iter()
            .filter(|class| has_instances(class))
        {
            checked += 1;
            match (held(class), row(class)) {
                (true, Some(_)) => {
                    missing.push(format!("{} holds a host handle and has a row", class.name));
                }
                (false, None) => {
                    missing.push(format!(
                        "{} has no row: [{}]",
                        class.name,
                        class.slots.join(", ")
                    ));
                }
                _ => {}
            }
        }
        for (class, slots) in SLOT_TYPES {
            assert!(
                has_instances(class),
                "{} has a row and no instances",
                class.name
            );
            if let Slots::Typed(types) = slots {
                assert_eq!(
                    types.len(),
                    class.slots.len(),
                    "{}'s row has one type per slot",
                    class.name
                );
            }
        }
        assert!(checked > 0, "the registry has classes with instances");
        assert!(missing.is_empty(), "{}", missing.join("\n"));
    }

    /// Walks every descriptor the process defined, not a list written here, so
    /// a `Core` class marked later is covered on the day it is marked.
    #[test]
    fn every_core_class_that_holds_a_host_handle_is_refused_by_decode() {
        let table = descriptors();
        let mut refused = 0;
        for (name, id) in &table.by_name {
            #[expect(
                unsafe_code,
                reason = "the leaked table owns the descriptor for the whole \
                          process, so this borrow is sound for any lifetime"
            )]
            let desc = unsafe { &*table.table.desc(*id) };
            if !desc.holds_host_handle() {
                continue;
            }
            refused += 1;
            let bytes = payload_naming(desc);
            let resolve = nvs_runtime::Receiving::over(None, Some(core_class_desc));
            let Err(why) = nvs_runtime::decode(&bytes, &resolve) else {
                panic!("{name} holds a host handle and decoded");
            };
            let why = why.to_string();
            assert!(
                why.contains(name) && why.contains("holds a handle onto something"),
                "{name} was not refused with the host-handle sentence: {why}"
            );
        }
        assert!(refused > 0, "no `Core` descriptor holds a host handle");
    }

    /// `construct` makes no `Core` class. The walk is over the process's
    /// descriptors, so a class added later is covered without a line here.
    #[test]
    fn every_core_descriptor_is_refused_by_construct() {
        let table = descriptors();
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        assert!(
            !table.by_name.is_empty(),
            "the process defines no `Core` class"
        );
        for (name, id) in &table.by_name {
            let desc = table.table.desc(*id);
            #[expect(
                unsafe_code,
                reason = "the leaked table owns the descriptor for the whole process"
            )]
            let built = unsafe { nvs_runtime::construct_erased_from(&mut ctx, desc, &[], None) };
            match built {
                Err(nvs_runtime::Fault::Thrown(nvs_runtime::ThrownClass::Logic, said)) => {
                    assert!(said.contains(name), "{name}: {said}");
                }
                other => panic!("{name} was not refused by construct: {other:?}"),
            }
        }
    }
}
