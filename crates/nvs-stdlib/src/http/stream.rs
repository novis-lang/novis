//! `Core\Http\Stream` — `rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`'s
//! streamed reply: a head read through members, and a body read one way and once.
//!
//! # Decision: the head is members and the body is a reader
//!
//! [`STREAM`] answers `status()`, `header()` and `headers()` exactly as
//! [`super::RESPONSE`] does — those are the same three readings over the same
//! two slots, and [`super::joined_field`] is the one implementation both doors
//! reach. What it does **not** have is that class's three body readers: a
//! streamed body is not a slot several members may read again, it is a walk,
//! and `events()`, `lines()`, `chunks()` and `saveTo()` are four framings of
//! the one walk rather than four readings of one value.
//!
//! So the body is **taken** by the first of the four that names it, and the
//! rest throw: `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`
//! draws that line for the inbound half and
//! `rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime` draws
//! the same one here. [`consumed`] is where it is drawn — it moves the key of
//! the reader out of the receiver's slot, so a second reader has nothing left
//! to reach the body with rather than being told not to look, and the refusal
//! names the member that took it.
//! Answering the second reader empty was the rejected alternative: an exhausted
//! walk and a body that was empty would then be one answer, which is the
//! ambiguity `rule:errors/ambiguous-input-refused` exists to refuse.
//!
//! # Decision: three reader classes, because three element types
//!
//! `CoreTy::Iterated` is parameter position only, so an `Iterable<T>` return is
//! a named class and `crate::registry::ITERABLES` is where its `T` is declared
//! — `Core\IO\Lines` is the precedent. An event, a line and a chunk are three
//! `T`s, so they are three classes: [`EVENTS`], [`LINES`] and [`CHUNKS`], one
//! layout and one implementation behind them ([`Framing`]), differing only in
//! what one `advance()` frames off the octets.
//!
//! Each **is its own iterator** rather than a [`crate::cursor`] over a
//! snapshot, which is `Core\Request\BodyStream`'s decision for
//! `Core\Request\BodyStream`'s reason: a framing decided per `advance()` is
//! what lets a walk hold one element rather than the list of them, and a
//! snapshot would have made every reader here a spelling of `text()` with an
//! extra allocation.
//!
//! # What a reader is walking over
//!
//! The body still on the socket. [`super::exchanged`] stops at the end of the
//! head and files [`super::transport::Incoming`] — the reply as a reader —
//! against the request, and what the receiver's slot holds is the key to it, so
//! a walk frames the next element out of what has arrived and waits for the
//! rest of it there. A reader is a Rust value and a slot holds a `Value`, which
//! is why the key rather than the reader: `Core\IO\File`'s design, argued at
//! [`nvs_runtime::Ctx::hold_open_reader`].
//!
//! What bounds the wait is the pair of bounds the call was made under, and
//! nothing in this module. What bounds the *memory* is here: a line and one
//! event's accumulated `data` are capped at [`super::transport::LINE_CEILING`]
//! and [`super::transport::EVENT_CEILING`] — these are bytes another host
//! chose, and "until memory runs out" would be that host deciding this
//! process's footprint.
//!
//! **What it spends:** one read's worth of body plus the element being framed,
//! charged to the request and released as each element is taken — plus one
//! framed element for as long as the loop body holds it, and, inside
//! `events()`, that event's accumulated `data` under the cap above. The
//! connection under the reader is given back at the end of the body, or with
//! the request where a program abandons the walk.
//!
//! An `id` line is copied once, into a string the walk keeps in force. Every
//! event whose block sets no `id` shares that string, so one long id costs its
//! length once and not once per event.

use super::*;

/// `Core\Http\Client::stream`, as its refusals spell it.
const STREAM_MEMBER: &str = r"Core\Http\Client::stream";

/// `Core\Http\Stream::saveTo`, as [`crate::io::stream_to_disk`] quotes it in
/// every refusal it raises on this member's behalf — the delegation is as
/// invisible in a message as `rule:core-classes/io-write-stream` makes it in
/// the language.
const SAVE_TO: &str = r"Core\Http\Stream::saveTo";

/// The streamed reply, as [`CoreTy::Instance`] spells it.
pub(crate) const STREAM_NAME: &str = r"Core\Http\Stream";
/// One event of an `events()` walk.
pub(crate) const EVENT_NAME: &str = r"Core\Http\Event";
/// `events()`' walk.
pub(crate) const EVENTS_NAME: &str = r"Core\Http\Events";
/// `lines()`' walk.
pub(crate) const LINES_NAME: &str = r"Core\Http\Lines";
/// `chunks()`' walk.
pub(crate) const CHUNKS_NAME: &str = r"Core\Http\Chunks";

/// [`STREAM`]'s status slot, by index — the layout its `slots` names.
const STATUS_AT: usize = 0;
/// [`STREAM`]'s header slot, holding [`super::header_map`]'s array.
const STREAM_HEADERS_AT: usize = 1;
/// [`STREAM`]'s body slot, holding the key of the reader the rest of the reply
/// is arriving on until a walk takes it and `null` from then on — [`consumed`]
/// is the only writer.
const STREAM_BODY_AT: usize = 2;
/// The member that took the body, or `null` while none has: what the second
/// reader's refusal names.
const READ_BY_AT: usize = 3;

/// The key of the reader this walk frames off, moved here out of the stream's
/// own slot, and `null` once the body has ended and the reader been dropped.
const READER_BODY_AT: usize = 0;
/// The element the last `advance()` framed, which `current()` answers, and
/// `null` before the first one and after the last.
const READER_CURRENT_AT: usize = 1;
/// [`EVENTS`]' third slot: the last event id the origin set, which the format
/// carries forward until the origin sets another — so an event that named no
/// `id` reports the one still in force rather than `null`.
const READER_LAST_ID_AT: usize = 2;

/// [`EVENT`]'s three slots, by index.
const EVENT_DATA_AT: usize = 0;
const EVENT_NAME_AT: usize = 1;
const EVENT_ID_AT: usize = 2;

/// The symbol behind `Iterable<T>::iterate()` for each walk, reached by name
/// through the class's method table rather than as a registered member — see
/// [`crate::instance`]'s dispatch roster.
pub(crate) const EVENTS_ITERATE_SYMBOL: &str = "nvs_core_http_events_iterate";
/// The symbol behind `Iterator<T>::advance()`, which is where the next element
/// is framed off the octets.
pub(crate) const EVENTS_ADVANCE_SYMBOL: &str = "nvs_core_http_events_advance";
/// The symbol behind `Iterator<T>::current()`.
pub(crate) const EVENTS_CURRENT_SYMBOL: &str = "nvs_core_http_events_current";
/// See [`EVENTS_ITERATE_SYMBOL`].
pub(crate) const LINES_ITERATE_SYMBOL: &str = "nvs_core_http_lines_iterate";
/// See [`EVENTS_ADVANCE_SYMBOL`].
pub(crate) const LINES_ADVANCE_SYMBOL: &str = "nvs_core_http_lines_advance";
/// See [`EVENTS_CURRENT_SYMBOL`].
pub(crate) const LINES_CURRENT_SYMBOL: &str = "nvs_core_http_lines_current";
/// See [`EVENTS_ITERATE_SYMBOL`].
pub(crate) const CHUNKS_ITERATE_SYMBOL: &str = "nvs_core_http_chunks_iterate";
/// See [`EVENTS_ADVANCE_SYMBOL`].
pub(crate) const CHUNKS_ADVANCE_SYMBOL: &str = "nvs_core_http_chunks_advance";
/// See [`EVENTS_CURRENT_SYMBOL`].
pub(crate) const CHUNKS_CURRENT_SYMBOL: &str = "nvs_core_http_chunks_current";

/// The address of one of this module's symbols — [`super::address`]'s tail.
pub(super) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_http_client_stream" => (nvs_core_http_client_stream as *const ()).cast(),
        "nvs_core_http_stream_status" => (nvs_core_http_stream_status as *const ()).cast(),
        "nvs_core_http_stream_header" => (nvs_core_http_stream_header as *const ()).cast(),
        "nvs_core_http_stream_headers" => (nvs_core_http_stream_headers as *const ()).cast(),
        "nvs_core_http_stream_events" => (nvs_core_http_stream_events as *const ()).cast(),
        "nvs_core_http_stream_lines" => (nvs_core_http_stream_lines as *const ()).cast(),
        "nvs_core_http_stream_chunks" => (nvs_core_http_stream_chunks as *const ()).cast(),
        "nvs_core_http_stream_save_to" => (nvs_core_http_stream_save_to as *const ()).cast(),
        "nvs_core_http_event_data" => (nvs_core_http_event_data as *const ()).cast(),
        "nvs_core_http_event_name" => (nvs_core_http_event_name as *const ()).cast(),
        "nvs_core_http_event_id" => (nvs_core_http_event_id as *const ()).cast(),
        EVENTS_ITERATE_SYMBOL => (nvs_core_http_events_iterate as *const ()).cast(),
        EVENTS_ADVANCE_SYMBOL => (nvs_core_http_events_advance as *const ()).cast(),
        EVENTS_CURRENT_SYMBOL => (nvs_core_http_events_current as *const ()).cast(),
        LINES_ITERATE_SYMBOL => (nvs_core_http_lines_iterate as *const ()).cast(),
        LINES_ADVANCE_SYMBOL => (nvs_core_http_lines_advance as *const ()).cast(),
        LINES_CURRENT_SYMBOL => (nvs_core_http_lines_current as *const ()).cast(),
        CHUNKS_ITERATE_SYMBOL => (nvs_core_http_chunks_iterate as *const ()).cast(),
        CHUNKS_ADVANCE_SYMBOL => (nvs_core_http_chunks_advance as *const ()).cast(),
        CHUNKS_CURRENT_SYMBOL => (nvs_core_http_chunks_current as *const ()).cast(),
        _ => return None,
    })
}

/// `Core\Http\Stream`'s own card — `rule:core-api/reference-card`.
const STREAM_CARD: ClassDoc = ClassDoc {
    short: "A reply whose body is read while it arrives. `status()` and the header methods work \
            at any time. The body can be read once, with one of `events()`, `lines()`, `chunks()` or \
            `saveTo()`.",
};

/// What `Core\Http\Client::stream` answers: the head, and a body no member has
/// framed yet.
///
/// `rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`'s
/// answer, which is the reply that is read as it arrives: the same head
/// members over the same map, and four framings of a body that is taken by
/// the first of them to name it rather than read by all four.
// rank: 1210
pub(crate) const STREAM: CoreClass = CoreClass {
    name: STREAM_NAME,
    doc: Some(&STREAM_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "status",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_http_stream_status",
            doc: Some(&STATUS_DOC),
        },
        CoreMethod {
            name: "header",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::TaintedStr),
            symbol: "nvs_core_http_stream_header",
            doc: Some(&HEADER_DOC),
        },
        CoreMethod {
            name: "headers",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::TaintedStr),
            symbol: "nvs_core_http_stream_headers",
            doc: Some(&HEADERS_DOC),
        },
        CoreMethod {
            name: "events",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(EVENTS_NAME),
            symbol: "nvs_core_http_stream_events",
            doc: Some(&EVENTS_DOC),
        },
        CoreMethod {
            name: "lines",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(LINES_NAME),
            symbol: "nvs_core_http_stream_lines",
            doc: Some(&LINES_DOC),
        },
        CoreMethod {
            name: "chunks",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(CHUNKS_NAME),
            symbol: "nvs_core_http_stream_chunks",
            doc: Some(&CHUNKS_DOC),
        },
        CoreMethod {
            name: "saveTo",
            names: &["path", "max"],
            params: &[CoreTy::Path(Qual::Sink), CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_http_stream_save_to",
            doc: Some(&SAVE_TO_DOC),
        },
    ],
    slots: &["status", "headers", "body", "readBy"],
    constants: &[],
};

/// `Core\Http\Stream::status`'s reference card — `rule:core-api/reference-card`.
const STATUS_DOC: MethodDoc = MethodDoc {
    short: "Returns the status code of the reply. It works before the body is read.",
    params: &[],
    ret: "The three-digit status code, as an `int`. A `4xx` or `5xx` code is returned like any \
          other code. It does not throw an error.",
    errors: &[],
};

/// `Core\Http\Stream::header`'s reference card — `rule:core-api/reference-card`.
const HEADER_DOC: MethodDoc = MethodDoc {
    short: "Returns one header of the reply, by name. When the server sent the header more \
            than once, the values are joined into one, as RFC 9110 § 5.3 allows.",
    params: &[ParamDoc {
        name: "name",
        desc: "The header name. Upper and lower case do not matter.",
        shape: &[],
    }],
    ret: "The header value, or `null` when the reply has no such header. The value is \
          `tainted`, which means it came from outside your program, like every byte of a reply.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The name is `Set-Cookie`. Its values cannot be joined, so use `headers` to read \
               them.",
    }],
};

/// `Core\Http\Stream::headers`'s reference card — `rule:core-api/reference-card`.
const HEADERS_DOC: MethodDoc = MethodDoc {
    short: "Returns every value of one header of the reply, as separate items. `header` joins \
            the same values into one string.",
    params: &[ParamDoc {
        name: "name",
        desc: "The header name. Upper and lower case do not matter.",
        shape: &[],
    }],
    ret: "The values in the order they arrived, each `tainted`. A header that arrived once gives \
          a list of one value. A header that never arrived gives an empty list.",
    errors: &[],
};

/// `Core\Http\Stream::events`'s reference card — `rule:core-api/reference-card`.
const EVENTS_DOC: MethodDoc = MethodDoc {
    short: "Reads the body as server-sent events, in the WHATWG EventSource format. Use it for \
            a `text/event-stream` reply.",
    params: &[],
    ret: "An `Iterable<Core\\Http\\Event>` that a `foreach` can loop over once. An event is \
          complete at the blank line that ends it. A comment line is skipped. A `retry` line is \
          read and ignored, because your program decides when to reconnect.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The body of this stream has already been read. A body can be read only once, \
                   in one way. The error message names the method that read it.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "A line of the reply is longer than the line limit, or the `data` of one \
                   event is longer than the event limit. Both limits are fixed, so a server \
                   cannot make your program use unlimited memory.",
        },
    ],
};

/// `Core\Http\Stream::lines`'s reference card — `rule:core-api/reference-card`.
const LINES_DOC: MethodDoc = MethodDoc {
    short: "Reads the body one line at a time. Use it for a reply that sends text line by line, \
            such as JSON Lines.",
    params: &[],
    ret: "An `Iterable<tainted string>` that a `foreach` can loop over once. A line ends at `\\n`, \
          and a `\\r` at the end of a line is removed. The last line does not need a `\\n`.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The body of this stream has already been read. The error message names the \
                   method that read it.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "A line of the reply is longer than the line limit.",
        },
    ],
};

/// `Core\Http\Stream::chunks`'s reference card — `rule:core-api/reference-card`.
const CHUNKS_DOC: MethodDoc = MethodDoc {
    short: "Reads the body as raw bytes, in pieces. Use it for a reply whose bytes have a \
            format of their own.",
    params: &[],
    ret: "An `Iterable<tainted bytes>` that a `foreach` can loop over once. The size of each \
          piece depends on the network and has no meaning, so you cannot choose a size. If you \
          need blocks of a fixed size, collect the bytes yourself.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The body of this stream has already been read. The error message names the method \
               that read it.",
    }],
};

/// `Core\Http\Stream::saveTo`'s reference card — `rule:core-api/reference-card`.
const SAVE_TO_DOC: MethodDoc = MethodDoc {
    short: "Writes the body directly to the file at `$path`. Only one piece of the body is in \
            memory at a time. Needs the `fs.write` capability for the path. It uses \
            `Core\\IO::writeStream` to write the file.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The file to write. Nothing may exist at the path yet. This method cannot \
                   replace a file. The `overwrite` option of `Core\\IO::writeStream` can.",
            shape: &[],
        },
        ParamDoc {
            name: "max",
            desc: "The largest number of bytes to accept. It is required, because the server \
                   chooses the length of the reply, and the bytes use disk space.",
            shape: &[],
        },
    ],
    ret: "Nothing. When writing fails part of the way through, the partial file is deleted \
          before the error is thrown. So nobody later finds an incomplete file.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The body of this stream has already been read. The error message names the \
                   method that read it.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not allow `fs.write` for this path, or the body is \
                   longer than `max` bytes.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "Something is already at the path, or the operating system could not create \
                   or write the file.",
        },
    ],
};

/// One server-sent event, as `events()` frames it.
///
/// `name` rather than `event` for the field the format calls `event`, because
/// `Event::event()` reads as the whole event rather than as the field.
///
/// One server-sent event, which is what the first of those four frames.
// rank: 1220
pub(crate) const EVENT: CoreClass = CoreClass {
    name: EVENT_NAME,
    doc: Some(&EVENT_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "data",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_http_event_data",
            doc: Some(&DATA_DOC),
        },
        CoreMethod {
            name: "name",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::TaintedStr),
            symbol: "nvs_core_http_event_name",
            doc: Some(&NAME_DOC),
        },
        CoreMethod {
            name: "id",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::TaintedStr),
            symbol: "nvs_core_http_event_id",
            doc: Some(&ID_DOC),
        },
    ],
    slots: &["data", "name", "id"],
    constants: &[],
};

/// `Core\Http\Event`'s own card — `rule:core-api/reference-card`.
const EVENT_CARD: ClassDoc = ClassDoc {
    short: "One server-sent event: its data, its name and its id. A `foreach` over \
            `Core\\Http\\Stream::events()` gives one for each event the server finished sending.",
};

/// `Core\Http\Event::data`'s reference card — `rule:core-api/reference-card`.
const DATA_DOC: MethodDoc = MethodDoc {
    short: "Returns the data of this event. Every `data` line of the event is joined with a \
            newline, and the last newline is removed.",
    params: &[],
    ret: "The data, `tainted` like every byte of a reply. An event with no `data` line is not \
          given to the loop at all.",
    errors: &[],
};

/// `Core\Http\Event::name`'s reference card — `rule:core-api/reference-card`.
const NAME_DOC: MethodDoc = MethodDoc {
    short: "Returns the type of this event. The server sends it as an `event` line.",
    params: &[],
    ret: "The name, `tainted`. It is `null` when the event has no `event` line. The format calls \
          such an event a `message`.",
    errors: &[],
};

/// `Core\Http\Event::id`'s reference card — `rule:core-api/reference-card`.
const ID_DOC: MethodDoc = MethodDoc {
    short: "Returns the last event id the server set. A program that resumes a stream sends it \
            back in the `Last-Event-ID` header.",
    params: &[],
    ret: "The id, `tainted`, or `null` when no id was set. The format ignores an id that \
          contains a NUL byte, so the result is `null`.",
    errors: &[],
};

/// `events()`' walk — a class a return type can name, for
/// [`CoreTy::Iterated`]'s parameter-position-only reason.
///
/// The three walks those framings answer with. Three classes rather than one
/// because `CoreTy::Iterated` is parameter position only, so an
/// `Iterable<T>` return is a named class and [`ITERABLES`](crate::registry::ITERABLES) is where its `T`
/// is declared — an event, a line and a chunk are three `T`s over one
/// implementation.
// rank: 1230
pub(crate) const EVENTS: CoreClass = CoreClass {
    name: EVENTS_NAME,
    doc: Some(&EVENTS_CARD),
    methods: &[],
    instance: &[],
    slots: &["body", "current", "lastId"],
    constants: &[],
};

/// `Core\Http\Events`' own card — `rule:core-api/reference-card`.
const EVENTS_CARD: ClassDoc = ClassDoc {
    short: "The server-sent events of a streamed reply's body, in the order they arrived. A \
            `foreach` over `Core\\Http\\Stream::events()` gives each one as a `Core\\Http\\Event`.",
};

/// `lines()`' walk. See [`EVENTS`].
// rank: 1240
pub(crate) const LINES: CoreClass = CoreClass {
    name: LINES_NAME,
    doc: Some(&LINES_CARD),
    methods: &[],
    instance: &[],
    slots: &["body", "current"],
    constants: &[],
};

/// `Core\Http\Lines`' own card — `rule:core-api/reference-card`.
const LINES_CARD: ClassDoc = ClassDoc {
    short: "The lines of a streamed reply's body, in the order they arrived. A `foreach` over \
            `Core\\Http\\Stream::lines()` gives each line as a `tainted string`.",
};

/// `chunks()`' walk. See [`EVENTS`].
// rank: 1250
pub(crate) const CHUNKS: CoreClass = CoreClass {
    name: CHUNKS_NAME,
    doc: Some(&CHUNKS_CARD),
    methods: &[],
    instance: &[],
    slots: &["body", "current"],
    constants: &[],
};

/// `Core\Http\Chunks`' own card — `rule:core-api/reference-card`.
const CHUNKS_CARD: ClassDoc = ClassDoc {
    short: "The pieces of a streamed reply's body, in the order they arrived. A `foreach` over \
            `Core\\Http\\Stream::chunks()` gives each piece as `tainted bytes`.",
};

/// What one `advance()` frames off the octets, which is the whole of what the
/// three walks differ by.
#[derive(Clone, Copy)]
enum Framing {
    /// One server-sent event, ended by a blank line.
    Events,
    /// One line, ended at `\n`.
    Lines,
    /// Whatever is left, in one piece.
    Chunks,
}

/// The key of the reader this stream's body is arriving on, moved out of its
/// slot and carried by the caller from here.
///
/// The move is what makes "read once" a property of the memory rather than only
/// of the refusal: the stream is left holding no key, so a second reader has
/// nothing to reach the body with, and the one walk that took it is the only
/// thing in the request that can name that reader.
///
/// # Errors
///
/// A `LogicError` naming the member that took the body, where one already has.
fn consumed(receiver: *mut nvs_runtime::ObjHeader, member: &str) -> Result<Value, Fault> {
    let held = crate::instance::slot(receiver, STREAM_BODY_AT);
    if held.as_uint().is_none() {
        let taken = crate::instance::slot(receiver, READ_BY_AT);
        let taken = taken.as_text().unwrap_or("a reader");
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{STREAM_NAME}::{member}(): this stream's body was read by `{taken}()` — a \
                 streamed body is read one way and once, and a second reader is refused rather \
                 than answered empty, which would be the same answer as a body that was empty"
            ),
        ));
    }
    // No retain: a key is a `uint` and holds no reference, which is the whole
    // of what moving it costs compared with moving the octets it replaced.
    crate::instance::set_slot(receiver, STREAM_BODY_AT, Value::null());
    crate::instance::set_slot(receiver, READ_BY_AT, Value::text(member));
    Ok(held)
}

/// A walk over the reader `body` keys, positioned before its first element.
///
/// There is no cursor beside the key: the reader drops each element as the walk
/// takes it ([`transport::Incoming::consume`]), so where the next one starts is
/// where what the reader still holds starts.
fn walk(class: &CoreClass, body: Value) -> Value {
    crate::instance::build(class, [body, Value::null()])
}

/// An `events()` walk, which carries one slot the other two do not: the id the
/// format keeps in force between events.
fn event_walk(body: Value) -> Value {
    crate::instance::build(&EVENTS, [body, Value::null(), Value::null()])
}

/// One step of a walk: the next element framed off the reader and parked in the
/// receiver's slot, and whether there was one.
///
/// **The framing waits where it has to.** An element the reader cannot complete
/// out of what it is holding is not an element yet, so this pulls and frames
/// again, and what ends an origin that never completes one is the pair of
/// bounds the call was made under
/// (`rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`)
/// rather than anything here. A body that has ended is framed once more, since
/// the last line of a reply needs no terminator.
///
/// **The reader is taken out of the request's table at the end of the body**,
/// which closes the connection the rest of it would have arrived on there
/// rather than at the end of the request. The key goes with it, so an
/// `advance()` after `false` is answered `false` again instead of reaching a
/// slot some later stream has filled.
///
/// The element the previous step parked is released by
/// [`nvs_runtime::nvs_object_field_set`] unless the loop body is still holding
/// it, and the slot is cleared at the end of the walk rather than left holding
/// the last element — `Core\Request\BodyStream`'s reasoning, since keeping it
/// would hold one element for as long as the program held the walk.
///
/// # Errors
///
/// [`line_from`]'s and [`EventScan::next`]'s caps, and
/// [`transport::Incoming::pull`]'s two bounds and its `IOError`.
fn step(ctx: &mut Ctx, value: Value, class: &CoreClass, framing: Framing) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(value, class, nvs_runtime::sequence::ADVANCE)?;
    let Some(key) = crate::instance::slot(receiver, READER_BODY_AT).as_uint() else {
        // The body ended, the reader went with it and the key was cleared:
        // this is the state a finished walk is left in, not a slot to refuse.
        return Ok(Value::bool(false));
    };
    // Borrowed from the slot. Nothing writes the slot until a dispatch, and a
    // dispatch writes it only with an id its own block set, after which the
    // scan never reads the borrow again.
    let carried = match framing {
        Framing::Events => crate::instance::slot(receiver, READER_LAST_ID_AT),
        Framing::Lines | Framing::Chunks => Value::null(),
    };
    // Both scans resume where the previous pull left them. A pull only appends,
    // and the event scan consumes only lines it has finished with
    // (`EventScan::settle`), so what they have read is still the front of what
    // the reader holds.
    let mut scan = EventScan::new(carried);
    let mut searched = 0;
    let reader = reader_at(ctx, key, class)?;
    let element = loop {
        let ended = reader.ended();
        let held = reader.held();
        let framed = match framing {
            Framing::Events => scan.next(held, ended)?.map(|(event, used, id)| {
                if let Some(id) = id {
                    crate::instance::set_slot(receiver, READER_LAST_ID_AT, id);
                }
                (event, used)
            }),
            Framing::Lines => line_from(held, &mut searched, ended, false, "lines")?
                .map(|(line, used)| text_of(line, "lines").map(|line| (line, used)))
                .transpose()?,
            // A walk over chunks frames nothing, so whatever has arrived is an
            // element and the read that delivered it is the piece a program
            // sees.
            Framing::Chunks => {
                (!held.is_empty()).then(|| (Value::bytes(NvsStr::new(held)), held.len()))
            }
        };
        match framed {
            Some((element, used)) => {
                reader.consume(used);
                break Some(element);
            }
            None if ended => break None,
            None => {
                if matches!(framing, Framing::Events) {
                    let (used, id) = scan.settle()?;
                    if let Some(id) = id {
                        crate::instance::set_slot(receiver, READER_LAST_ID_AT, id);
                    }
                    reader.consume(used);
                }
                reader.pull()?;
            }
        }
    };
    match element {
        None => {
            drop(ctx.take_open_reader(key));
            crate::instance::set_slot(receiver, READER_BODY_AT, Value::null());
            crate::instance::set_slot(receiver, READER_CURRENT_AT, Value::null());
            Ok(Value::bool(false))
        }
        Some(element) => {
            crate::instance::set_slot(receiver, READER_CURRENT_AT, element);
            Ok(Value::bool(true))
        }
    }
}

/// The reader `key` names, as the type this module framed it as.
///
/// # Errors
///
/// A [`Fault::fatal`] where the request holds no such reader. No case can reach
/// it: a key is filed by [`super::filed`], moved by [`consumed`] and cleared in
/// the same breath as the reader is taken out, and all three are this module's.
fn reader_at<'a>(
    ctx: &'a mut Ctx,
    key: u64,
    class: &CoreClass,
) -> Result<&'a mut transport::Incoming, Fault> {
    ctx.open_reader_mut(key)
        .and_then(|reader| reader.as_any_mut().downcast_mut::<transport::Incoming>())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{}::{} found no reply reader under the key in its `body` slot",
                class.name,
                nvs_runtime::sequence::ADVANCE
            ))
        })
}

/// The element `current()` answers, with a reference of its own: the slot keeps
/// its own until the next step, so a loop body that keeps an element keeps a
/// value nothing can invalidate.
fn current(value: Value, class: &CoreClass) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(value, class, nvs_runtime::sequence::CURRENT)?;
    let held = crate::instance::slot(receiver, READER_CURRENT_AT);
    #[expect(
        unsafe_code,
        reason = "the slot keeps its reference until the next step, so the value \
                  handed back needs one of its own"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

/// One line off the front of `octets`, and how many of them it took — or
/// `None` where there is not a whole line there yet.
///
/// `ends_at_cr` is the whole difference between the two framings that read
/// lines: the EventSource format ends a line at `\r\n`, `\r` or `\n`, and
/// `lines()` ends one at `\n` and strips a trailing `\r`. A final line needs no
/// terminator, which is what `ended` decides: the body being over is what makes
/// a fragment the last line rather than the start of one still arriving.
///
/// `searched` is how many octets at the front an earlier call already found no
/// line end in. The search starts there, and a call that frames no line leaves
/// it where the next search has to start, so a long line that arrives in many
/// reads is searched once. A call that frames a line sets it back to zero,
/// because the next call is handed what follows that line.
///
/// # Errors
///
/// [`capped`]'s, on the framed line and on a fragment alike.
fn line_from<'a>(
    octets: &'a [u8],
    searched: &mut usize,
    ended: bool,
    ends_at_cr: bool,
    member: &str,
) -> Result<Option<(&'a [u8], usize)>, Fault> {
    let from = (*searched).min(octets.len());
    let (line, used) = match octets[from..]
        .iter()
        .position(|byte| *byte == b'\n' || (ends_at_cr && *byte == b'\r'))
        .map(|at| from + at)
    {
        // Nothing that ends a line is here. A body that is over ends its last
        // line by being over; one that is not is holding a fragment, which is
        // a line only once the octet that ends it arrives — and the cap is
        // what an origin that never sends one runs into.
        None => {
            if !ended {
                capped(octets.len(), member)?;
                *searched = octets.len();
                return Ok(None);
            }
            if octets.is_empty() {
                return Ok(None);
            }
            (octets, octets.len())
        }
        // A `\r` at the very end of what has arrived may be the first half of
        // a CRLF whose second half is still on the wire, and a framing that
        // ends a line at either would read one line ending as two. The next
        // search starts at that `\r` again.
        Some(at) if ends_at_cr && octets[at] == b'\r' && at + 1 == octets.len() && !ended => {
            *searched = at;
            return Ok(None);
        }
        Some(at) => {
            let crlf = octets[at] == b'\r' && octets.get(at + 1) == Some(&b'\n');
            (&octets[..at], at + if crlf { 2 } else { 1 })
        }
    };
    let line = match line.split_last() {
        Some((b'\r', head)) => head,
        _ => line,
    };
    capped(line.len(), member)?;
    *searched = 0;
    Ok(Some((line, used)))
}

/// Refuses a line past [`transport::LINE_CEILING`].
///
/// Asked of a framed line and of a fragment still waiting for its terminator
/// alike: an origin that sends no line ending would otherwise be an origin this
/// process buffers without a bound, which is the same failure the cap on a
/// framed line prevents one read later.
///
/// # Errors
///
/// A `RuntimeError` naming the cap and the member the program called.
fn capped(length: usize, member: &str) -> Result<(), Fault> {
    if length > transport::LINE_CEILING {
        return Err(Fault::thrown(format!(
            "{STREAM_NAME}::{member}(): a line of this reply is longer than the \
             {} bytes a streamed line may be — the length is the origin's choice and the cap is \
             this process's",
            transport::LINE_CEILING
        )));
    }
    Ok(())
}

/// What one dispatched event is: the `Core\Http\Event` itself, how many octets
/// the parse took, and the id its block set, with a reference for the walk's
/// slot. `None` where the block set no id, so the one in force stays.
type Dispatched = (Value, usize, Option<Value>);

/// The id in force while one block is scanned.
enum Id {
    /// The walk's own `lastId`, a `string` or `null`, borrowed from its slot.
    /// An event whose block sets no id shares this string, so an id is copied
    /// once, when its line is read, and not once per event after it.
    Carried(Value),
    /// What the last `id` line of this block set.
    Set(Vec<u8>),
}

/// A parse of the next event off the front of the reader, as the WHATWG
/// EventSource format defines one: the event, the octets it took, and the id
/// left in force after it.
///
/// The id [`EventScan::new`] is given is that id on the way in. The format keeps it between events — an
/// `id` line sets it and nothing clears it, including the blank line that
/// dispatches — so an event that names no `id` reports the last one the origin
/// did, which is the value a program resuming the stream sends back as
/// `Last-Event-ID`. The `event` name has the opposite rule and is reset at every
/// dispatch, which is why only one of the two is threaded through here.
///
/// An event is dispatched at the **blank line** that ends its block and nowhere
/// else, so a block that is still arriving is not one and neither is the block
/// a body stopped in the middle of: a half-event handed to a program as a whole
/// one is the failure this format's terminator exists to prevent. A block whose
/// lines set no `data` dispatches nothing either, which is what makes an id-only
/// keep-alive block invisible to a walk.
///
/// The parse is a scan that one step keeps across its pulls: each call to
/// [`EventScan::next`] resumes at the first line the previous call could not
/// finish, with the fields it had already read. One event that arrives in many
/// reads is therefore read once, and costs time linear in its size. The scan
/// lives only inside one step, so nothing but the id is threaded through slots.
/// What one event may hold is bounded by [`transport::EVENT_CEILING`]: an
/// origin sending `data` and never a blank line runs into it rather than into
/// this walk's memory.
///
/// `retry` is read and ignored — reconnecting is the program's decision, and a
/// server-chosen sleep inside a `Core` iterator is a wait with no bound the
/// caller wrote.
struct EventScan {
    /// The octets of the block already read, which every line before it took.
    at: usize,
    /// Where the block still being scanned starts: just past the last blank
    /// line that ended a block with nothing to dispatch.
    block: usize,
    /// [`line_from`]'s resume point inside the line that starts at `at`.
    searched: usize,
    data: Vec<u8>,
    written: bool,
    name: Option<Vec<u8>>,
    id: Id,
    /// Whether an `id` line was read in the block still being scanned.
    open_id: bool,
}

/// The most octets one block may take before it dispatches, comments and
/// field names included. A blank line that dispatches nothing starts the count
/// again.
///
/// [`transport::EVENT_CEILING`] bounds the `data` and this bounds the rest: a
/// comment inside an open block cannot be consumed before the block ends, so an
/// origin that sends comments and never a blank line runs into this cap. Twice
/// the `data` cap leaves room for one `data: ` prefix per short line.
const BLOCK_CEILING: usize = 2 * transport::EVENT_CEILING;

impl EventScan {
    /// A scan positioned at the front of a block, with `carried` in force: a
    /// `string` or `null` that stays live while the scan runs.
    fn new(carried: Value) -> Self {
        Self {
            at: 0,
            block: 0,
            searched: 0,
            data: Vec::new(),
            written: false,
            name: None,
            id: Id::Carried(carried),
            open_id: false,
        }
    }

    /// The octets every line read so far took, where those lines leave nothing
    /// a later event needs, and the id a finished block set, for the walk's slot.
    ///
    /// The scan starts again at the front, so the walk consumes what it
    /// returns. A stream of comments and blocks with no `data` is then held one
    /// line at a time. A block that is still open returns `0`, because the
    /// lines it read are part of the event it may still dispatch.
    ///
    /// An id that a block with no `data` set still goes to `lastId` here, as the
    /// WHATWG event-stream format says. No `.nvs` program sends a stream line
    /// by line, so the attack is the Rust test
    /// `lines_that_dispatch_nothing_are_consumed_as_they_are_read`.
    ///
    /// # Errors
    ///
    /// A `RuntimeError` where the id a finished block set is not text, as at a
    /// dispatch.
    fn settle(&mut self) -> Result<(usize, Option<Value>), Fault> {
        if self.written || self.name.is_some() || self.open_id {
            return Ok((0, None));
        }
        let used = std::mem::take(&mut self.at);
        self.block = 0;
        let Id::Set(octets) = &self.id else {
            return Ok((used, None));
        };
        #[expect(
            clippy::disallowed_methods,
            reason = "`checked` passes these bytes first"
        )]
        let id = Value::str(NvsStr::new(checked(octets, "events")?.as_bytes()));
        // The slot takes the reference the string was built with, and the scan
        // borrows it from there, as it borrows the id it started with.
        self.id = Id::Carried(id);
        Ok((used, Some(id)))
    }

    /// The next event of `octets`, which must start with every octet an
    /// earlier call on this scan was handed.
    ///
    /// # Errors
    ///
    /// [`line_from`]'s cap, and a `RuntimeError` where one event's accumulated
    /// `data` passes [`transport::EVENT_CEILING`].
    fn next(&mut self, octets: &[u8], ended: bool) -> Result<Option<Dispatched>, Fault> {
        while let Some((line, next)) = line_from(
            &octets[self.at..],
            &mut self.searched,
            ended,
            true,
            "events",
        )? {
            self.at += next;
            if self.at - self.block > BLOCK_CEILING {
                return Err(Fault::thrown(format!(
                    "{STREAM_NAME}::events(): one event of this reply is longer than the {BLOCK_CEILING} \
                     bytes an event may take before its blank line"
                )));
            }
            if line.is_empty() {
                self.open_id = false;
                if self.written {
                    // All three are checked before any is allocated, so a
                    // refusal leaves nothing to release.
                    let data = checked(&self.data, "events")?;
                    let name = match &self.name {
                        Some(name) => Some(checked(name, "events")?),
                        None => None,
                    };
                    if let Id::Set(id) = &self.id {
                        checked(id, "events")?;
                    }
                    let (id, set) =
                        match std::mem::replace(&mut self.id, Id::Carried(Value::null())) {
                            Id::Carried(id) => (id, None),
                            Id::Set(octets) => {
                                #[expect(
                                    clippy::disallowed_methods,
                                    reason = "`checked` passed these bytes above"
                                )]
                                let id = Value::str(NvsStr::new(&octets));
                                (id, Some(id))
                            }
                        };
                    // A reference for the event. A carried id is borrowed, so
                    // this is its only one. A new id was built with one, which
                    // goes to the walk's slot, so both name the one string.
                    #[expect(
                        unsafe_code,
                        reason = "`id` is either the walk's slot value, live while \
                                  the scan runs, or the string built above"
                    )]
                    unsafe {
                        id.retain();
                    }
                    let event = crate::instance::build(
                        &EVENT,
                        [
                            Value::text(data),
                            name.map_or_else(Value::null, Value::text),
                            id,
                        ],
                    );
                    return Ok(Some((event, self.at, set)));
                }
                self.name = None;
                self.block = self.at;
                continue;
            }
            self.line(line)?;
        }
        Ok(None)
    }

    /// One line of a block that is not blank: a comment, or a field.
    ///
    /// # Errors
    ///
    /// A `RuntimeError` where the `data` passes [`transport::EVENT_CEILING`].
    fn line(&mut self, line: &[u8]) -> Result<(), Fault> {
        if line[0] == b':' {
            return Ok(());
        }
        let (field, value) = match line.iter().position(|byte| *byte == b':') {
            None => (line, &b""[..]),
            Some(colon) => {
                let value = &line[colon + 1..];
                (&line[..colon], value.strip_prefix(b" ").unwrap_or(value))
            }
        };
        match field {
            b"data" => {
                if self.written {
                    self.data.push(b'\n');
                }
                self.data.extend_from_slice(value);
                self.written = true;
                if self.data.len() > transport::EVENT_CEILING {
                    return Err(Fault::thrown(format!(
                        "{STREAM_NAME}::events(): one event of this reply carries more than the \
                         {} bytes of `data` an event may accumulate — the length is the origin's \
                         choice and the cap is this process's",
                        transport::EVENT_CEILING
                    )));
                }
            }
            b"event" => self.name = Some(value.to_vec()),
            // A NUL in an id is the one value the format says to ignore, and
            // ignoring it leaves the previous one standing.
            b"id" if !value.contains(&0) => {
                self.id = Id::Set(value.to_vec());
                self.open_id = true;
            }
            _ => {}
        }
        Ok(())
    }
}

/// One framed piece of the reply as a `string`, or the error saying it is not
/// text.
fn text_of(octets: &[u8], member: &str) -> Result<Value, Fault> {
    Ok(Value::text(checked(octets, member)?))
}

/// The text of `octets` when they may become a `string`. They are whatever the origin
/// sent, and a `string` is well-formed UTF-8 by `NvsStr`'s own invariant, so
/// every framed piece is checked before it becomes one — the way
/// `Core\Http\Response::text` checks a whole body. `chunks()` reads the same
/// reply as `bytes`, which asks nothing of it.
fn checked<'a>(octets: &'a [u8], member: &str) -> Result<&'a str, Fault> {
    match std::str::from_utf8(octets) {
        Ok(text) => Ok(text),
        Err(err) => Err(Fault::thrown(format!(
            "{STREAM_NAME}::{member}(): a piece of this reply is not valid UTF-8, so it is not a \
             `string` — byte {} of it is where it stops being text, and `chunks()` reads the \
             reply as `bytes`",
            err.valid_up_to()
        ))),
    }
}

/// One of [`EVENT`]'s three slots, with a reference of its own.
fn event_slot(value: Value, at: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(value, &EVENT, member)?;
    let held = crate::instance::slot(receiver, at);
    #[expect(
        unsafe_code,
        reason = "the event owns its slot's reference for as long as the program \
                  holds the event, so the value handed back needs one of its own"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::stream(Core\Http\Method $method, string|Core\Http\Target $url, Core\Http\Options): Core\Http\Stream`
    /// — `rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`.
    ///
    /// The verb is a parameter rather than the member's own name because a
    /// streamed reply is usually a `POST` — a chat completion, a query that
    /// answers rows — and a `stream` that could only `GET` would be the member
    /// nobody uses. It arrives at slot 0 and every option sits one slot further
    /// on, which is [`nvs_core_http_client_request`]'s layout and is why the
    /// rest of the row is that member's reading under a slice of its own
    /// arguments. Its bag is two keys wider than that member's — the bounds
    /// [`super::STREAM_OPTIONS`] adds — and those two are judged here, because
    /// they are the only part of the call the shared reading does not know
    /// about.
    fn nvs_core_http_client_stream(ctx, args: [STREAM_ARITY + 1]) {
        let verb = crate::router::method_verb(&args[0], STREAM_MEMBER)?.to_ascii_uppercase();
        let bag = &args[1..];
        judge_bound(bag, IDLE, "idle", STREAM_MEMBER)?;
        judge_bound(bag, MAX_DURATION, "maxDuration", STREAM_MEMBER)?;
        // The session the reply arrived over is dropped here rather than kept:
        // `rule:http-server/a-reply-reports-its-tls-session` puts the report on
        // `Core\Http\Response` alone, and a second surface for one measurement
        // is the copy that disagrees.
        let (status, body, headers, _) = exchanged(ctx, bag, "stream", &verb, true)?;
        Ok(crate::instance::build(
            &STREAM,
            [Value::int(status), headers, body, Value::null()],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Stream::status(): int` — the head, read before the body is.
    ///
    /// # Errors
    ///
    /// A [`Fault::fatal`] naming the member if the receiver is not a
    /// `Core\Http\Stream` or its `status` slot holds no `int`. Both are
    /// unreachable from source — `E0401` refuses a receiver of another type
    /// before any of this runs, and the slot is written by this crate alone.
    fn nvs_core_http_stream_status(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &STREAM, "status")?;
        let status = crate::instance::slot(receiver, STATUS_AT);
        status.as_int().map(Value::int).ok_or_else(|| {
            Fault::fatal(format!(
                "{STREAM_NAME}::status expected an `int` in its `status` slot, got tag {}",
                status.tag_byte()
            ))
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Stream::header(string $name): ?tainted string` — the reply's
    /// own, joined as [`nvs_core_http_response_header`] joins one, because they
    /// are one reading of one map and [`joined_field`] is where it lives.
    ///
    /// # Errors
    ///
    /// That member's `LogicError` for `Set-Cookie`, and its two fatals.
    fn nvs_core_http_stream_header(_ctx, args: [2]) {
        let receiver = crate::instance::receiver(args[0], &STREAM, "header")?;
        let name = field_name(&args[1], STREAM_NAME, "header")?;
        joined_field(receiver, STREAM_HEADERS_AT, name, STREAM_NAME)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Stream::headers(string $name): array<tainted string>` —
    /// [`nvs_core_http_stream_header`]'s other half, keeping apart what that one
    /// joins.
    ///
    /// # Errors
    ///
    /// That member's two fatals, and no throw.
    fn nvs_core_http_stream_headers(_ctx, args: [2]) {
        let receiver = crate::instance::receiver(args[0], &STREAM, "headers")?;
        let name = field_name(&args[1], STREAM_NAME, "headers")?;
        Ok(listed_field(receiver, STREAM_HEADERS_AT, name))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Stream::events(): Iterable<Core\Http\Event>` — the body read
    /// as the WHATWG EventSource format frames it.
    ///
    /// # Errors
    ///
    /// [`consumed`]'s `LogicError` where the body has already been read.
    fn nvs_core_http_stream_events(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &STREAM, "events")?;
        Ok(event_walk(consumed(receiver, "events")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Stream::lines(): Iterable<tainted string>` — the body read a
    /// line at a time.
    ///
    /// # Errors
    ///
    /// [`consumed`]'s `LogicError` where the body has already been read.
    fn nvs_core_http_stream_lines(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &STREAM, "lines")?;
        Ok(walk(&LINES, consumed(receiver, "lines")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Stream::chunks(): Iterable<tainted bytes>` — the body read as
    /// it is framed by nothing.
    ///
    /// # Errors
    ///
    /// [`consumed`]'s `LogicError` where the body has already been read.
    fn nvs_core_http_stream_chunks(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &STREAM, "chunks")?;
        Ok(walk(&CHUNKS, consumed(receiver, "chunks")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Stream::saveTo(string $path, uint $max): void` —
    /// `rule:core-classes/io-write-stream`, with a streamed reply on the other
    /// end of it.
    ///
    /// **A delegation, not an implementation.** That rule makes
    /// `Core\IO::writeStream` the one place a stream reaches disk, so what is
    /// here is the walk `chunks()` already answers with, handed to
    /// [`crate::io::stream_to_disk`]. Its two rules — a destination that must
    /// not already exist, and a failure that removes the partial file — are
    /// therefore this member's without being restated.
    ///
    /// **`max` is required**, where `Core\Request\Part::saveTo`'s is optional:
    /// an upload is already bounded by `[limits] upload_total`, and the length
    /// of a reply another host chose is bounded by nothing this process wrote.
    ///
    /// # Errors
    ///
    /// [`consumed`]'s `LogicError`, the door's `RuntimeError` where `fs.write`
    /// does not cover the path, a `RuntimeError` past `max`, and an `IOError`
    /// for a destination that exists or an operating system that refused. A
    /// [`Fault::fatal`] for an argument of the wrong tag, unreachable from
    /// source because the row's own types are what `E0401` refuses first.
    fn nvs_core_http_stream_save_to(ctx, args: [3]) {
        let receiver = crate::instance::receiver(args[0], &STREAM, "saveTo")?;
        let path = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{SAVE_TO} expected a `string` path, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        let max = args[2].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "{SAVE_TO} expected a `uint` max, got tag {}",
                args[2].tag_byte()
            ))
        })?;
        let path = std::path::Path::new(path);
        let walked = walk(&CHUNKS, consumed(receiver, "saveTo")?);
        let wrote = crate::io::stream_to_disk(ctx, path, walked, max, false, SAVE_TO);
        #[expect(
            unsafe_code,
            reason = "the walk was built in this frame and `stream_to_disk` \
                      borrows its source, so this frame owns the one reference \
                      to it on either arm"
        )]
        unsafe {
            walked.release();
        }
        wrote?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Event::data(): tainted string` — every `data` line of this
    /// event, joined by a newline.
    fn nvs_core_http_event_data(_ctx, args: [1]) {
        event_slot(args[0], EVENT_DATA_AT, "data")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Event::name(): ?tainted string` — the `event` line's value.
    fn nvs_core_http_event_name(_ctx, args: [1]) {
        event_slot(args[0], EVENT_NAME_AT, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Event::id(): ?tainted string` — the `id` line's value.
    fn nvs_core_http_event_id(_ctx, args: [1]) {
        event_slot(args[0], EVENT_ID_AT, "id")
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<Core\Http\Event>::iterate()` — the walk itself, because an
    /// event does not exist until the octets ahead of it are framed.
    ///
    /// [`crate::request`]'s body stream shape rather than [`crate::cursor`]'s:
    /// the receiver's transferred reference is handed straight back out, so
    /// nothing is allocated and the cursor *is* the walk.
    fn nvs_core_http_events_iterate(_ctx, args: [1]) {
        crate::instance::receiver(args[0], &EVENTS, nvs_runtime::sequence::ITERATE)?;
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<Core\Http\Event>::advance(): bool` — the next event framed, or
    /// `false` at the end of the body.
    fn nvs_core_http_events_advance(ctx, args: [1]) {
        let stepped = step(ctx, args[0], &EVENTS, Framing::Events);
        crate::cursor::consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<Core\Http\Event>::current(): Core\Http\Event`.
    fn nvs_core_http_events_current(_ctx, args: [1]) {
        let framed = current(args[0], &EVENTS);
        crate::cursor::consume(args[0]);
        framed
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<tainted string>::iterate()` — see
    /// [`nvs_core_http_events_iterate`].
    fn nvs_core_http_lines_iterate(_ctx, args: [1]) {
        crate::instance::receiver(args[0], &LINES, nvs_runtime::sequence::ITERATE)?;
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<tainted string>::advance(): bool` — the next line framed.
    fn nvs_core_http_lines_advance(ctx, args: [1]) {
        let stepped = step(ctx, args[0], &LINES, Framing::Lines);
        crate::cursor::consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<tainted string>::current(): tainted string`.
    fn nvs_core_http_lines_current(_ctx, args: [1]) {
        let framed = current(args[0], &LINES);
        crate::cursor::consume(args[0]);
        framed
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<tainted bytes>::iterate()` — see
    /// [`nvs_core_http_events_iterate`].
    fn nvs_core_http_chunks_iterate(_ctx, args: [1]) {
        crate::instance::receiver(args[0], &CHUNKS, nvs_runtime::sequence::ITERATE)?;
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<tainted bytes>::advance(): bool` — the octets the transport
    /// has, in one piece.
    fn nvs_core_http_chunks_advance(ctx, args: [1]) {
        let stepped = step(ctx, args[0], &CHUNKS, Framing::Chunks);
        crate::cursor::consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<tainted bytes>::current(): tainted bytes`.
    fn nvs_core_http_chunks_current(_ctx, args: [1]) {
        let framed = current(args[0], &CHUNKS);
        crate::cursor::consume(args[0]);
        framed
    }
}

#[cfg(test)]
mod tests {
    use super::{CHUNKS, Dispatched, EVENTS, EventScan, Fault, LINES, STREAM, line_from};
    use nvs_runtime::{Ctx, Tag, Value};

    /// One line off the front of `octets`, by a search that starts at its front.
    fn line_at<'a>(
        octets: &'a [u8],
        ended: bool,
        ends_at_cr: bool,
        member: &str,
    ) -> Result<Option<(&'a [u8], usize)>, Fault> {
        line_from(octets, &mut 0, ended, ends_at_cr, member)
    }

    /// The next event off the front of `octets`, by a scan of its own, with
    /// `carried` in force: a `string` or `null`.
    fn event_at(octets: &[u8], ended: bool, carried: Value) -> Result<Option<Dispatched>, Fault> {
        EventScan::new(carried).next(octets, ended)
    }

    /// The text of the id a block set, or `None` where it set none.
    fn set_id(id: Option<Value>) -> Option<String> {
        id.and_then(|id| id.as_text().map(str::to_owned))
    }

    /// An event that arrives one octet at a time is parsed by one scan that
    /// resumes after each octet, and the result is the event the whole block
    /// gives in one read: the same data, name, id and octets taken. A scan
    /// that lost a field or a line between two reads would still dispatch an
    /// event, which is why every field is compared.
    #[test]
    fn an_event_read_in_pieces_is_the_event_read_whole() {
        let body = b": hi\r\nevent: tick\r\ndata: one\rdata: two\nid: 7\r\n\r\ndata: next\n\n";
        let mut ctx = Ctx::buffered();
        let mut fields = |event: Value| -> Vec<Option<String>> {
            [
                super::nvs_core_http_event_data,
                super::nvs_core_http_event_name,
                super::nvs_core_http_event_id,
            ]
            .into_iter()
            .map(|reader| {
                nvs_runtime::call(reader, &mut ctx, &[event])
                    .expect("a reader reads a slot")
                    .as_text()
                    .map(str::to_owned)
            })
            .collect()
        };
        let (whole, used, id) = event_at(body, false, Value::null())
            .expect("no cap is reached")
            .expect("the block ends with a blank line");
        let mut scan = EventScan::new(Value::null());
        let mut pieces = None;
        for end in 1..=body.len() {
            if let Some(found) = scan.next(&body[..end], false).expect("no cap is reached") {
                assert_eq!(
                    end, used,
                    "the event is dispatched once its blank line arrives"
                );
                pieces = Some(found);
                break;
            }
        }
        let (piece, piece_used, piece_id) = pieces.expect("the pieces finish one event");
        assert_eq!((piece_used, set_id(piece_id)), (used, set_id(id)));
        let whole = fields(whole);
        assert_eq!(fields(piece), whole);
        assert_eq!(
            whole,
            [
                Some("one\ntwo".to_owned()),
                Some("tick".to_owned()),
                Some("7".to_owned())
            ]
        );
    }

    /// A line that arrives in pieces is found once its end arrives, and a `\r`
    /// at the end of one piece still waits to see whether a `\n` follows it.
    #[test]
    fn a_line_search_resumes_where_the_last_one_stopped() {
        let body = b"one two\r\nthree";
        let mut searched = 0;
        for end in 1..=8 {
            assert_eq!(
                line_from(&body[..end], &mut searched, false, true, "events").expect("short"),
                None,
                "no whole line is in the first {end} octets"
            );
        }
        assert_eq!(searched, 7, "the next search starts at the `\\r` again");
        let (line, used) = line_from(&body[..9], &mut searched, false, true, "events")
            .expect("short")
            .expect("the CRLF has arrived");
        assert_eq!((line, used, searched), (&b"one two"[..], 9, 0));
    }

    /// The walks' slot layouts, asserted together: one implementation reads all
    /// three, so a class whose slots drifted would frame the wrong field of the
    /// wrong object and still run. `events()` carries one more than the other
    /// two, which is the id the format keeps in force between events.
    #[test]
    fn every_walk_has_the_layout_its_stepper_reads() {
        for class in [&LINES, &CHUNKS] {
            assert_eq!(
                class.slots,
                ["body", "current"],
                "{} does not carry the layout `step` reads",
                class.name
            );
        }
        assert_eq!(EVENTS.slots, ["body", "current", "lastId"]);
        assert_eq!(STREAM.slots, ["status", "headers", "body", "readBy"]);
    }

    /// An event is dispatched at the blank line that ends its block, whatever
    /// ended each line inside it — `\r\n`, `\r` and `\n` are one terminator to
    /// this format, and a parser that learned only two of them reads a whole
    /// block as one unterminated line.
    #[test]
    fn every_line_ending_ends_a_line_of_one_event() {
        for body in [
            "event: tick\ndata: one\nid: 7\n\n",
            "event: tick\r\ndata: one\r\nid: 7\r\n\r\n",
            "event: tick\rdata: one\rid: 7\r\r",
        ] {
            let (_, next, id) = event_at(body.as_bytes(), true, Value::null())
                .expect("no cap is reached")
                .expect("the block ends with a blank line");
            assert_eq!(next, body.len(), "{body:?} was not read to its end");
            assert_eq!(set_id(id).as_deref(), Some("7"));
        }
    }

    /// A lone `\r` ends a line for the event format and does not for `lines()`,
    /// which is the one difference between the two framings — asserted here
    /// rather than inferred from either side alone.
    #[test]
    fn a_lone_carriage_return_ends_a_line_for_one_framing_only() {
        let body = b"one\rtwo\n";
        let (line, _) = line_at(body, true, true, "events")
            .expect("no cap is reached")
            .expect("there is a line");
        assert_eq!(line, b"one");
        let (line, _) = line_at(body, true, false, "lines")
            .expect("no cap is reached")
            .expect("there is a line");
        assert_eq!(line, b"one\rtwo");
    }

    /// A block with no `data` dispatches nothing, and the id it set stays in
    /// force — a keep-alive that moves the resume point without being an event
    /// is what the format's separate buffers are for.
    #[test]
    fn a_block_carrying_no_data_dispatches_nothing_and_keeps_its_id() {
        let body = b": a comment\nid: 7\nretry: 500\n\ndata: one\n\n";
        let (_, _, id) = event_at(body, true, Value::null())
            .expect("no cap is reached")
            .expect("the second block carries data");
        assert_eq!(set_id(id).as_deref(), Some("7"));
    }

    /// Comments and blocks with no `data` are consumed as they are read, so a
    /// stream that sends only those holds one line at a time. An id such a
    /// block set goes to the walk's slot then, and a block still open keeps its
    /// lines until it dispatches.
    #[test]
    fn lines_that_dispatch_nothing_are_consumed_as_they_are_read() {
        let mut scan = EventScan::new(Value::null());
        let comments = b": ping\n: ping\n\n";
        assert!(scan.next(comments, false).expect("short").is_none());
        let (used, id) = scan.settle().expect("no id to check");
        assert_eq!((used, id.is_none()), (comments.len(), true));

        let mut scan = EventScan::new(Value::null());
        assert!(scan.next(b"id: 7\n\n", false).expect("short").is_none());
        let (used, id) = scan.settle().expect("an id that is text");
        assert_eq!((used, set_id(id).as_deref()), (7, Some("7")));
        assert!(scan.next(b"data: x\n", false).expect("short").is_none());
        assert_eq!(
            scan.settle().expect("open").0,
            0,
            "an open block keeps its lines"
        );
        let (_, used, set) = scan
            .next(b"data: x\n\n", false)
            .expect("short")
            .expect("the block ends");
        assert_eq!(
            (used, set.is_none()),
            (9, true),
            "the id was already in force"
        );

        let mut scan = EventScan::new(Value::null());
        let mut open = b"data: x\n".to_vec();
        open.extend(b": c\n".repeat(super::BLOCK_CEILING / 4 + 1));
        let refused = scan
            .next(&open, false)
            .expect_err("an open block has a cap");
        assert!(
            format!("{refused:?}").contains("before its blank line"),
            "{refused:?}"
        );
    }

    /// The block cap counts from the last blank line, so a reply that arrives
    /// whole and holds only blank lines past the cap ends with no event.
    #[test]
    fn blank_lines_past_the_block_cap_end_with_no_event() {
        let blank = b"\r".repeat(super::BLOCK_CEILING + 1);
        assert!(
            EventScan::new(Value::null())
                .next(&blank, true)
                .expect("each blank line ends a block")
                .is_none()
        );
    }

    /// A body that stops inside a block is a block the origin did not finish,
    /// so nothing is dispatched from it — where the same bytes followed by the
    /// blank line are one event.
    #[test]
    fn a_block_the_body_stopped_inside_is_not_an_event() {
        assert!(
            event_at(b"data: half", true, Value::null())
                .expect("no cap is reached")
                .is_none()
        );
        assert!(
            event_at(b"data: half\n\n", true, Value::null())
                .expect("no cap is reached")
                .is_some()
        );
    }

    /// A fragment with nothing after it yet is not a line, and the same octets
    /// once the body is over are one — the distinction a walk over a reader has
    /// to make and a walk over a finished body never had to.
    #[test]
    fn a_fragment_is_a_line_only_once_the_body_is_over() {
        assert!(
            line_at(b"one", false, false, "lines")
                .expect("no cap is reached")
                .is_none()
        );
        let (line, used) = line_at(b"one", true, false, "lines")
            .expect("no cap is reached")
            .expect("a body that is over ends its last line");
        assert_eq!((line, used), (&b"one"[..], 3));
    }

    /// A `\r` at the end of what has arrived is half of a line ending whose
    /// other half may still be on the wire, so the event framing waits for it.
    /// A reader that did not would end the line at the `\r` and then read the
    /// `\n` as an empty line, which is a dispatch the origin never sent.
    #[test]
    fn a_carriage_return_at_the_end_of_a_read_waits_for_its_newline() {
        assert!(
            line_at(b"one\r", false, true, "events")
                .expect("no cap is reached")
                .is_none()
        );
        let (line, used) = line_at(b"one\r\n", false, true, "events")
            .expect("no cap is reached")
            .expect("the whole line ending is here");
        assert_eq!((line, used), (&b"one"[..], 5));
    }

    /// A block still arriving dispatches nothing, exactly as one the body
    /// stopped inside does. The octets stay where they are, so the next read
    /// frames the same block again with more of it — which is what makes the
    /// parse restart at the front of the block rather than carry `data`
    /// forward.
    #[test]
    fn a_block_that_is_still_arriving_dispatches_nothing() {
        assert!(
            event_at(b"data: one\n", false, Value::null())
                .expect("no cap is reached")
                .is_none()
        );
        let (_, used, _) = event_at(b"data: one\n\n", false, Value::null())
            .expect("no cap is reached")
            .expect("the blank line dispatched it");
        assert_eq!(used, 11, "the block is taken to the end of its blank line");
    }

    /// The three readers answer the fields of the event they were framed from,
    /// each its own: `data` joins its lines with a newline, `name` is reset at
    /// every dispatch while `id` stays in force, and an id carrying a NUL leaves
    /// the previous one standing. A reader wired to a neighbouring slot would
    /// still answer a string, which is why every field differs from the others.
    // covers: Core\Http\Event::data
    // covers: Core\Http\Event::name
    // covers: Core\Http\Event::id
    #[test]
    fn each_reader_answers_its_own_field_of_the_framed_event() {
        let body = b"event: tick\nid: 7\ndata: one\ndata: two\n\ndata: three\nid: x\0y\n\n";
        let (first, used, id) = event_at(body, true, Value::null())
            .expect("no cap is reached")
            .expect("the first block ends with a blank line");
        let carried = id.expect("the first block sets an id");
        let (second, _, _) = event_at(&body[used..], true, carried)
            .expect("no cap is reached")
            .expect("the second block ends with a blank line");
        let mut ctx = Ctx::buffered();
        let mut read = |event: Value, member: &str| {
            let reader = match member {
                "data" => super::nvs_core_http_event_data,
                "name" => super::nvs_core_http_event_name,
                _ => super::nvs_core_http_event_id,
            };
            nvs_runtime::call(reader, &mut ctx, &[event]).expect("a reader reads a slot")
        };

        assert_eq!(read(first, "data").as_text(), Some("one\ntwo"));
        assert_eq!(read(first, "name").as_text(), Some("tick"));
        assert_eq!(read(first, "id").as_text(), Some("7"));

        assert_eq!(read(second, "data").as_text(), Some("three"));
        assert_eq!(
            read(second, "name").tag(),
            Some(Tag::Null),
            "the event name does not outlive its own event"
        );
        assert_eq!(
            read(second, "id").as_text(),
            Some("7"),
            "the id stays in force, and one with a NUL in it is ignored"
        );
    }

    /// An event whose block sets no id holds the walk's own id string, not a
    /// copy of it, and leaves the walk's slot alone. A copy per event would make
    /// one long id cost its whole length again on every short event after it.
    #[test]
    fn an_event_with_no_id_of_its_own_shares_the_one_in_force() {
        let carried = Value::text(&"7".repeat(4096));
        let (event, _, set) = event_at(b"data: one\n\n", true, carried)
            .expect("no cap is reached")
            .expect("the block ends with a blank line");
        assert!(
            set.is_none(),
            "a block with no id leaves the walk's id alone"
        );
        let receiver = crate::instance::receiver(event, &super::EVENT, "id").expect("an event");
        let held = crate::instance::slot(receiver, super::EVENT_ID_AT);
        assert_eq!(
            held.str_ptr(),
            carried.str_ptr(),
            "the event shares the string"
        );
    }

    /// `chunks()` takes the body: the reader's key moves out of the stream into
    /// a walk positioned before its first chunk, the stream records `chunks` as
    /// the reader that took it, and a second `chunks()` throws naming it. A
    /// member that left the key behind would hand both walks the same reader,
    /// and one that answered the second call with an empty walk would read like
    /// a body that was empty.
    // covers: Core\Http\Stream::chunks
    #[test]
    fn chunks_takes_the_body_and_a_second_call_is_refused() {
        let stream = crate::instance::build(
            &STREAM,
            [
                Value::int(200),
                Value::null(),
                Value::uint(41),
                Value::null(),
            ],
        );
        let mut ctx = Ctx::buffered();
        let walk = nvs_runtime::call(super::nvs_core_http_stream_chunks, &mut ctx, &[stream])
            .expect("nothing has read the body yet");
        let walked = walk.obj_ptr().expect("a walk is an object");
        assert_eq!(crate::instance::slot(walked, 0).as_uint(), Some(41));
        assert_eq!(crate::instance::slot(walked, 1).tag(), Some(Tag::Null));

        let taken = stream.obj_ptr().expect("a stream is an object");
        assert_eq!(crate::instance::slot(taken, 2).tag(), Some(Tag::Null));
        assert_eq!(crate::instance::slot(taken, 3).as_text(), Some("chunks"));

        let again = nvs_runtime::call(super::nvs_core_http_stream_chunks, &mut ctx, &[stream]);
        assert!(again.is_err(), "a second reader walked the body");
        let message = ctx.take_pending().expect("the refusal says why");
        assert!(
            message.contains(
                "Core\\Http\\Stream::chunks(): this stream's body was read by `chunks()`"
            ),
            "{message}"
        );
    }

    /// `events()` takes the body into a walk that carries no id yet, and the
    /// next reader throws naming `events` whichever reader it is. The walk
    /// starts with an empty `lastId` slot: one that started with a stale id
    /// would hand it to the first event that sets none.
    // covers: Core\Http\Stream::events
    #[test]
    fn events_takes_the_body_with_no_id_in_force_and_lines_is_refused() {
        let stream = crate::instance::build(
            &STREAM,
            [
                Value::int(200),
                Value::null(),
                Value::uint(7),
                Value::null(),
            ],
        );
        let mut ctx = Ctx::buffered();
        let walk = nvs_runtime::call(super::nvs_core_http_stream_events, &mut ctx, &[stream])
            .expect("nothing has read the body yet");
        let walked = walk.obj_ptr().expect("a walk is an object");
        assert_eq!(crate::instance::slot(walked, 0).as_uint(), Some(7));
        assert_eq!(crate::instance::slot(walked, 1).tag(), Some(Tag::Null));
        assert_eq!(crate::instance::slot(walked, 2).tag(), Some(Tag::Null));

        let taken = stream.obj_ptr().expect("a stream is an object");
        assert_eq!(crate::instance::slot(taken, 3).as_text(), Some("events"));

        let late = nvs_runtime::call(super::nvs_core_http_stream_lines, &mut ctx, &[stream]);
        assert!(late.is_err(), "`lines()` walked a body `events()` took");
        let message = ctx.take_pending().expect("the refusal says why");
        assert!(
            message
                .contains("Core\\Http\\Stream::lines(): this stream's body was read by `events()`"),
            "{message}"
        );
    }

    /// `header()` reads the head, which a body reader does not take: it joins a
    /// field that arrived twice with `, ` before and after `chunks()` took the
    /// body, matches the name in any case, answers `null` for a field that never
    /// arrived, and throws for `Set-Cookie`, naming `headers` as the reader.
    // covers: Core\Http\Stream::header
    #[test]
    fn header_reads_the_head_after_the_body_is_taken_and_refuses_set_cookie() {
        let lines = [
            ("x-trace".to_owned(), "one".to_owned()),
            ("set-cookie".to_owned(), "a=1".to_owned()),
            ("x-trace".to_owned(), "two".to_owned()),
        ];
        let stream = crate::instance::build(
            &STREAM,
            [
                Value::int(200),
                crate::http::header_map(&lines),
                Value::uint(3),
                Value::null(),
            ],
        );
        let mut ctx = Ctx::buffered();
        let mut header = |name: &str| {
            let name = Value::text(name);
            nvs_runtime::call(
                super::nvs_core_http_stream_header,
                &mut ctx,
                &[stream, name],
            )
        };
        assert_eq!(
            header("X-Trace").expect("a field").as_text(),
            Some("one, two")
        );
        assert_eq!(header("x-absent").expect("no field").tag(), Some(Tag::Null));
        assert!(header("Set-Cookie").is_err(), "two cookies were joined");

        nvs_runtime::call(super::nvs_core_http_stream_chunks, &mut ctx, &[stream])
            .expect("nothing has read the body yet");
        let mut header = |name: &str| {
            let name = Value::text(name);
            nvs_runtime::call(
                super::nvs_core_http_stream_header,
                &mut ctx,
                &[stream, name],
            )
        };
        assert_eq!(
            header("x-trace")
                .expect("the head outlives the body")
                .as_text(),
            Some("one, two")
        );
        assert!(header("set-cookie").is_err());
        let message = ctx.take_pending().expect("the refusal says why");
        assert!(message.contains("headers(\"set-cookie\")"), "{message}");
    }

    /// `lines()` takes the body into a walk with nothing framed yet, and the
    /// next reader throws naming `lines`. The framing it steps with ends a line
    /// at `\n` only: a `\r` before it is stripped, a lone `\r` stays inside the
    /// line, an empty line is a line, and the last one needs no terminator.
    // covers: Core\Http\Stream::lines
    #[test]
    fn lines_takes_the_body_and_frames_at_a_newline_alone() {
        let stream = crate::instance::build(
            &STREAM,
            [
                Value::int(200),
                Value::null(),
                Value::uint(12),
                Value::null(),
            ],
        );
        let mut ctx = Ctx::buffered();
        let walk = nvs_runtime::call(super::nvs_core_http_stream_lines, &mut ctx, &[stream])
            .expect("nothing has read the body yet");
        let walked = walk.obj_ptr().expect("a walk is an object");
        assert_eq!(crate::instance::slot(walked, 0).as_uint(), Some(12));
        assert_eq!(crate::instance::slot(walked, 1).tag(), Some(Tag::Null));

        let taken = stream.obj_ptr().expect("a stream is an object");
        assert_eq!(crate::instance::slot(taken, 3).as_text(), Some("lines"));

        let late = nvs_runtime::call(super::nvs_core_http_stream_chunks, &mut ctx, &[stream]);
        assert!(late.is_err(), "`chunks()` walked a body `lines()` took");
        let message = ctx.take_pending().expect("the refusal says why");
        assert!(
            message
                .contains("Core\\Http\\Stream::chunks(): this stream's body was read by `lines()`"),
            "{message}"
        );

        let mut body: &[u8] = b"one\r\ntwo\rthree\n\nlast";
        let mut framed = Vec::new();
        while let Some((line, used)) = line_at(body, true, false, "lines").expect("under the cap") {
            framed.push(String::from_utf8(line.to_vec()).expect("text"));
            body = &body[used..];
        }
        assert_eq!(framed, ["one", "two\rthree", "", "last"]);
    }

    /// `status()` answers the code in the head as it is, a `5xx` included, and
    /// the same code again after a reader took the body: the head is not part
    /// of what a body reader consumes.
    // covers: Core\Http\Stream::status
    #[test]
    fn status_answers_the_head_code_before_and_after_the_body_is_taken() {
        let stream = crate::instance::build(
            &STREAM,
            [
                Value::int(503),
                Value::null(),
                Value::uint(5),
                Value::null(),
            ],
        );
        let mut ctx = Ctx::buffered();
        let status = nvs_runtime::call(super::nvs_core_http_stream_status, &mut ctx, &[stream])
            .expect("a head carries a code");
        assert_eq!(status.as_int(), Some(503));

        nvs_runtime::call(super::nvs_core_http_stream_lines, &mut ctx, &[stream])
            .expect("nothing has read the body yet");
        let status = nvs_runtime::call(super::nvs_core_http_stream_status, &mut ctx, &[stream])
            .expect("the head outlives the body");
        assert_eq!(status.as_int(), Some(503));
    }

    /// `headers()` keeps apart the lines `header()` joins, in arrival order,
    /// matches the name in any case, answers an empty list for a field that
    /// never arrived, and reads `Set-Cookie` without throwing — this is the
    /// reader that field is refused to `header()` for.
    // covers: Core\Http\Stream::headers
    #[test]
    fn headers_keeps_each_line_apart_and_reads_set_cookie() {
        let lines = [
            ("set-cookie".to_owned(), "a=1".to_owned()),
            ("x-trace".to_owned(), "one".to_owned()),
            ("set-cookie".to_owned(), "b=2".to_owned()),
        ];
        let stream = crate::instance::build(
            &STREAM,
            [
                Value::int(200),
                crate::http::header_map(&lines),
                Value::uint(9),
                Value::null(),
            ],
        );
        let mut ctx = Ctx::buffered();
        let mut headers = |name: &str| {
            let name = Value::text(name);
            let list = nvs_runtime::call(
                super::nvs_core_http_stream_headers,
                &mut ctx,
                &[stream, name],
            )
            .expect("`headers` does not throw");
            let array = crate::arr::borrowed(list.array_ptr().expect("the answer is an array"));
            let read: Vec<String> = (0..array.count())
                .map(|at| {
                    let line = array
                        .get_index(i64::try_from(at).expect("an index"))
                        .expect("a line");
                    line.as_text().expect("each line is text").to_owned()
                })
                .collect();
            #[expect(
                unsafe_code,
                reason = "this frame owns the list `headers` answered with, and it is not \
                          its caller's"
            )]
            unsafe {
                list.release();
            }
            read
        };
        assert_eq!(headers("Set-Cookie"), ["a=1", "b=2"]);
        assert_eq!(headers("X-TRACE"), ["one"]);
        assert!(headers("etag").is_empty());
    }

    /// `saveTo()` is refused twice before a byte reaches disk: by a stream whose
    /// body another reader already took, naming that reader, and by a context
    /// that grants no `fs.write`, naming `saveTo`. The second refusal comes after
    /// the body is taken, so the stream records `saveTo` as its reader and no
    /// file exists at the path. A member that created the file before asking
    /// the door would leave an empty one behind for a later reader to trust.
    // covers: Core\Http\Stream::saveTo
    #[test]
    fn save_to_refuses_a_taken_body_and_a_path_it_may_not_write() {
        let dir = nvs_repo::scratch("http-stream-save-to");
        let path = dir.join("refused.bin");
        let target = Value::text(&path.to_string_lossy());
        let mut ctx = Ctx::buffered();

        let taken = crate::instance::build(
            &STREAM,
            [
                Value::int(200),
                Value::null(),
                Value::null(),
                Value::text("lines"),
            ],
        );
        let again = nvs_runtime::call(
            super::nvs_core_http_stream_save_to,
            &mut ctx,
            &[taken, target, Value::uint(1024)],
        );
        assert!(again.is_err(), "a second reader saved the body");
        let message = ctx.take_pending().expect("the refusal says why");
        assert!(
            message
                .contains("Core\\Http\\Stream::saveTo(): this stream's body was read by `lines()`"),
            "{message}"
        );

        let fresh = crate::instance::build(
            &STREAM,
            [
                Value::int(200),
                Value::null(),
                Value::uint(41),
                Value::null(),
            ],
        );
        let denied = nvs_runtime::call(
            super::nvs_core_http_stream_save_to,
            &mut ctx,
            &[fresh, target, Value::uint(1024)],
        );
        assert!(denied.is_err(), "a context with no `fs.write` wrote a file");
        let message = ctx.take_pending().expect("the refusal says why");
        assert!(message.contains("saveTo"), "{message}");
        assert!(message.contains("fs.write"), "{message}");
        assert!(!path.exists(), "a refused save left a file behind");

        let stream = fresh.obj_ptr().expect("a stream is an object");
        assert_eq!(crate::instance::slot(stream, 2).tag(), Some(Tag::Null));
        assert_eq!(crate::instance::slot(stream, 3).as_text(), Some("saveTo"));
    }
}
