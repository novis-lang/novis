//! `Core\Http\Stream` — [ADR 0180](/docs/decisions/0180.md) § 5's streamed
//! reply: a head read through members, and a body read one way and once.
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
//! the same one here. [`consumed`] is where it is drawn — it moves the octets
//! out of the receiver's slot, so a second reader finds nothing rather than
//! being told not to look, and the refusal names the member that took them.
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
//! The reply's octets, in the receiver's own slot: [`super::exchanged`] hands
//! the whole body over with the head, so a reader frames what is already in
//! hand and the transport's own ceiling is what bounds it
//! ([`super::transport`]). A line and one event's accumulated `data` are capped
//! on top of that, at [`super::transport::LINE_CEILING`] and
//! [`super::transport::EVENT_CEILING`] — these are bytes another host chose,
//! and "until memory runs out" would be that host deciding this process's
//! footprint.
//!
//! **What it spends:** the reply's octets, once, charged to the request and
//! released with the walk that took them — plus one framed element for as long
//! as the loop body holds it, and, inside `events()`, that event's accumulated
//! `data` under the cap above.

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
/// [`STREAM`]'s body slot, holding the octets until a reader takes them and
/// `null` from then on — [`consumed`] is the only writer.
const STREAM_BODY_AT: usize = 2;
/// The member that took the body, or `null` while none has: what the second
/// reader's refusal names.
const READ_BY_AT: usize = 3;

/// A reader's octets, which it borrows for the length of one `advance()`.
const READER_BODY_AT: usize = 0;
/// How far into those octets this walk has framed.
const READER_FROM_AT: usize = 1;
/// The element the last `advance()` framed, which `current()` answers, and
/// `null` before the first one and after the last.
const READER_CURRENT_AT: usize = 2;
/// [`EVENTS`]' fourth slot: the last event id the origin set, which the format
/// carries forward until the origin sets another — so an event that named no
/// `id` reports the one still in force rather than `null`.
const READER_LAST_ID_AT: usize = 3;

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

/// What `Core\Http\Client::stream` answers: the head, and a body no member has
/// framed yet.
pub(crate) const STREAM: CoreClass = CoreClass {
    name: STREAM_NAME,
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
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Uint],
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
    short: "The status code of the reply whose head has arrived, read before the body is.",
    params: &[],
    ret: "The three-digit code, as an `int`. A `4xx` or a `5xx` is an answer like any other and is \
          reported here rather than thrown.",
    errors: &[],
};

/// `Core\Http\Stream::header`'s reference card — `rule:core-api/reference-card`.
const HEADER_DOC: MethodDoc = MethodDoc {
    short: "One header field of the reply, by name, with a field the origin sent twice joined as \
            RFC 9110 § 5.3 makes the two lines equivalent.",
    params: &[ParamDoc {
        name: "name",
        desc: "The field name, matched without regard to case.",
        shape: &[],
    }],
    ret: "The field's value, `tainted` as every byte of a reply is, or `null` where the reply \
          carried no such field.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The name is `Set-Cookie`, which is not part of that equivalence — `headers` is the \
               reading for it.",
    }],
};

/// `Core\Http\Stream::headers`'s reference card — `rule:core-api/reference-card`.
const HEADERS_DOC: MethodDoc = MethodDoc {
    short: "Every line the reply carried under one field name, kept apart where `header` joins \
            them.",
    params: &[ParamDoc {
        name: "name",
        desc: "The field name, matched without regard to case.",
        shape: &[],
    }],
    ret: "The lines in arrival order, each `tainted`. A field that arrived once answers a list of \
          one and a field that never arrived an empty list.",
    errors: &[],
};

/// `Core\Http\Stream::events`'s reference card — `rule:core-api/reference-card`.
const EVENTS_DOC: MethodDoc = MethodDoc {
    short: "The body read as a walk over server-sent events, parsed as the WHATWG EventSource \
            format defines them — the reading for a `text/event-stream` reply.",
    params: &[],
    ret: "An `Iterable<Core\\Http\\Event>` a `foreach` walks once. An event is dispatched at the \
          blank line that ends it; a comment line is skipped, and `retry` is read and ignored, \
          because reconnecting is the program's decision.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "This stream's body has already been read. It is read one way and once, and the \
                   refusal names the member that read it.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "A line of the reply is longer than a streamed line may be, or one event's \
                   accumulated `data` is longer than one event may be. Both caps are constants: \
                   the bytes are the origin's choice and the footprint is this process's.",
        },
    ],
};

/// `Core\Http\Stream::lines`'s reference card — `rule:core-api/reference-card`.
const LINES_DOC: MethodDoc = MethodDoc {
    short: "The body read as a walk over its lines — the reading for a reply that arrives as text \
            a line at a time, such as JSON Lines.",
    params: &[],
    ret: "An `Iterable<tainted string>` a `foreach` walks once. A line ends at `\\n` and a trailing \
          `\\r` is stripped; the last line needs no terminator.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "This stream's body has already been read, and the refusal names the member \
                   that read it.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "A line of the reply is longer than a streamed line may be.",
        },
    ],
};

/// `Core\Http\Stream::chunks`'s reference card — `rule:core-api/reference-card`.
const CHUNKS_DOC: MethodDoc = MethodDoc {
    short: "The body read as a walk over its octets, framed by nothing — the reading for a reply \
            whose bytes carry their own structure.",
    params: &[],
    ret: "An `Iterable<tainted bytes>` a `foreach` walks once. A chunk boundary is the \
          transport's and carries no meaning, so there is no size to ask for: a reader that needs \
          fixed blocks is doing its own framing and buffers for it.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "This stream's body has already been read, and the refusal names the member that \
               read it.",
    }],
};

/// `Core\Http\Stream::saveTo`'s reference card — `rule:core-api/reference-card`.
const SAVE_TO_DOC: MethodDoc = MethodDoc {
    short: "Writes the body straight to `$path`, holding one chunk at a time. Needs the `fs.write` \
            capability for the path, and delegates to `Core\\IO::writeStream`, which is where every \
            stream in the language reaches disk.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "Where the body is to land. Nothing may already be there: replacing a file is \
                   `Core\\IO::writeStream`'s `overwrite`, and this member does not take one.",
            shape: &[],
        },
        ParamDoc {
            name: "max",
            desc: "The most bytes to accept. It is required rather than optional, because the \
                   length of a reply is the origin's choice and disk is what it would spend.",
            shape: &[],
        },
    ],
    ret: "Nothing. A failure part-way through removes the partial file before it throws, so no \
          later reader finds a truncated body the program believes it received whole.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "This stream's body has already been read, and the refusal names the member \
                   that read it.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.write` for this path, or the body ran \
                   past `max` bytes.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "Something is already at the path, or the operating system refused the create \
                   or a write.",
        },
    ],
};

/// One server-sent event, as `events()` frames it.
///
/// `name` rather than `event` for the field the format calls `event`, because
/// `Event::event()` reads as the whole event rather than as the field.
pub(crate) const EVENT: CoreClass = CoreClass {
    name: EVENT_NAME,
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

/// `Core\Http\Event::data`'s reference card — `rule:core-api/reference-card`.
const DATA_DOC: MethodDoc = MethodDoc {
    short: "This event's payload: every `data` line it carried, joined by a newline, with the \
            trailing one removed.",
    params: &[],
    ret: "The payload, `tainted` as every byte of a reply is. An event with no `data` is never \
          dispatched, so this is never empty for a reason a program has to tell apart.",
    errors: &[],
};

/// `Core\Http\Event::name`'s reference card — `rule:core-api/reference-card`.
const NAME_DOC: MethodDoc = MethodDoc {
    short: "The event type the origin named, which the format writes as an `event` line.",
    params: &[],
    ret: "The name, `tainted`, or `null` where the event carried no `event` line — which the \
          format calls a `message`.",
    errors: &[],
};

/// `Core\Http\Event::id`'s reference card — `rule:core-api/reference-card`.
const ID_DOC: MethodDoc = MethodDoc {
    short: "The last event id the origin set, which a program resuming a stream sends back as \
            `Last-Event-ID`.",
    params: &[],
    ret: "The id, `tainted`, or `null` where none was set. An id containing a NUL is ignored by \
          the format and answers `null` here.",
    errors: &[],
};

/// `events()`' walk — a class a return type can name, for
/// [`CoreTy::Iterated`]'s parameter-position-only reason.
pub(crate) const EVENTS: CoreClass = CoreClass {
    name: EVENTS_NAME,
    methods: &[],
    instance: &[],
    slots: &["body", "from", "current", "lastId"],
    constants: &[],
};

/// `lines()`' walk. See [`EVENTS`].
pub(crate) const LINES: CoreClass = CoreClass {
    name: LINES_NAME,
    methods: &[],
    instance: &[],
    slots: &["body", "from", "current"],
    constants: &[],
};

/// `chunks()`' walk. See [`EVENTS`].
pub(crate) const CHUNKS: CoreClass = CoreClass {
    name: CHUNKS_NAME,
    methods: &[],
    instance: &[],
    slots: &["body", "from", "current"],
    constants: &[],
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

/// The octets this stream is holding, moved out of its slot and owned by the
/// caller from here.
///
/// The move is what makes "read once" a property of the memory rather than only
/// of the refusal: the stream is left holding nothing, so a second reader has
/// nothing to find and an abandoned walk frees the body with itself.
///
/// # Errors
///
/// A `LogicError` naming the member that took the body, where one already has.
fn consumed(receiver: *mut nvs_runtime::ObjHeader, member: &str) -> Result<Value, Fault> {
    let held = crate::instance::slot(receiver, STREAM_BODY_AT);
    if octets_of(&held).is_none() {
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
    #[expect(
        unsafe_code,
        reason = "the slot's reference is the one being moved out, and the \
                  `set_slot` below releases exactly the reference this retain \
                  added back"
    )]
    unsafe {
        held.retain();
    }
    crate::instance::set_slot(receiver, STREAM_BODY_AT, Value::null());
    crate::instance::set_slot(
        receiver,
        READ_BY_AT,
        Value::str(NvsStr::new(member.as_bytes())),
    );
    Ok(held)
}

/// A walk over `body`, positioned at its first element.
fn walk(class: &CoreClass, body: Value) -> Value {
    crate::instance::build(class, [body, Value::uint(0), Value::null()])
}

/// An `events()` walk, which carries one slot the other two do not: the id the
/// format keeps in force between events.
fn event_walk(body: Value) -> Value {
    crate::instance::build(
        &EVENTS,
        [body, Value::uint(0), Value::null(), Value::null()],
    )
}

/// One step of a walk: the next element parked in the receiver's slot, and
/// whether there was one.
///
/// The element the previous step parked is released by
/// [`nvs_runtime::nvs_object_field_set`] unless the loop body is still holding
/// it, and the slot is cleared at the end of the walk rather than left holding
/// the last element — `Core\Request\BodyStream`'s reasoning, since keeping it
/// would hold one element for as long as the program held the walk.
///
/// # Errors
///
/// [`framed`]'s two caps.
fn step(value: Value, class: &CoreClass, framing: Framing) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(value, class, nvs_runtime::sequence::ADVANCE)?;
    let held = crate::instance::slot(receiver, READER_BODY_AT);
    // No case can reach this: the slot is filled by [`walk`] with octets
    // [`consumed`] has already read, and nothing else writes it.
    let octets = octets_of(&held).ok_or_else(|| {
        Fault::fatal(format!(
            "{}::{} expected octets in its `body` slot, got tag {}",
            class.name,
            nvs_runtime::sequence::ADVANCE,
            held.tag_byte()
        ))
    })?;
    let from = usize::try_from(
        crate::instance::slot(receiver, READER_FROM_AT)
            .as_uint()
            .unwrap_or(0),
    )
    .unwrap_or(usize::MAX);
    let framed = match framing {
        Framing::Events => {
            // Scoped, because the slot's own reference is released by the
            // `set_slot` below and nothing may still be reading the bytes it
            // lent to the parse by then.
            let parsed = {
                let carried = crate::instance::slot(receiver, READER_LAST_ID_AT);
                event_at(octets, from, carried.as_text().map(str::as_bytes))?
            };
            parsed.map(|(event, next, id)| {
                crate::instance::set_slot(
                    receiver,
                    READER_LAST_ID_AT,
                    optional_text(id.as_deref()),
                );
                (event, next)
            })
        }
        Framing::Lines => line_at(octets, from, false, "lines")?
            .map(|(line, next)| (Value::str(NvsStr::new(line)), next)),
        Framing::Chunks => (from < octets.len())
            .then(|| (Value::bytes(NvsStr::new(&octets[from..])), octets.len())),
    };
    match framed {
        None => {
            crate::instance::set_slot(receiver, READER_CURRENT_AT, Value::null());
            Ok(Value::bool(false))
        }
        Some((element, next)) => {
            crate::instance::set_slot(receiver, READER_FROM_AT, Value::uint(next as u64));
            crate::instance::set_slot(receiver, READER_CURRENT_AT, element);
            Ok(Value::bool(true))
        }
    }
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

/// One line of `octets` from `from`, and where the next one starts.
///
/// `ends_at_cr` is the whole difference between the two framings that read
/// lines: the EventSource format ends a line at `\r\n`, `\r` or `\n`, and
/// `lines()` ends one at `\n` and strips a trailing `\r`. A final line needs no
/// terminator — the body is over, and a fragment with nothing after it is what
/// the origin sent.
///
/// # Errors
///
/// A `RuntimeError` for a line over [`transport::LINE_CEILING`].
fn line_at<'a>(
    octets: &'a [u8],
    from: usize,
    ends_at_cr: bool,
    member: &str,
) -> Result<Option<(&'a [u8], usize)>, Fault> {
    if from >= octets.len() {
        return Ok(None);
    }
    let rest = &octets[from..];
    let (line, next) = match rest
        .iter()
        .position(|byte| *byte == b'\n' || (ends_at_cr && *byte == b'\r'))
    {
        None => (rest, octets.len()),
        Some(at) => {
            let crlf = rest[at] == b'\r' && rest.get(at + 1) == Some(&b'\n');
            (&rest[..at], from + at + if crlf { 2 } else { 1 })
        }
    };
    let line = match line.split_last() {
        Some((b'\r', head)) => head,
        _ => line,
    };
    if line.len() > transport::LINE_CEILING {
        return Err(Fault::thrown(format!(
            "{STREAM_NAME}::{member}(): a line of this reply is longer than the \
             {} bytes a streamed line may be — the length is the origin's choice and the cap is \
             this process's",
            transport::LINE_CEILING
        )));
    }
    Ok(Some((line, next)))
}

/// What one dispatched event is: the `Core\Http\Event` itself, where the parse
/// got to, and the id left in force after it.
type Dispatched = (Value, usize, Option<Vec<u8>>);

/// The next event of `octets` from `from`, parsed as the WHATWG EventSource
/// format defines one: the event, where the parse got to, and the id left in
/// force after it.
///
/// `carried` is that id on the way in. The format keeps it between events — an
/// `id` line sets it and nothing clears it, including the blank line that
/// dispatches — so an event that names no `id` reports the last one the origin
/// did, which is the value a program resuming the stream sends back as
/// `Last-Event-ID`. The `event` name has the opposite rule and is reset at every
/// dispatch, which is why only one of the two is threaded through here.
///
/// An event is dispatched at the **blank line** that ends its block and nowhere
/// else, so a block the body stopped in the middle of is not one: the origin
/// did not finish sending it, and a half-event handed to a program as a whole
/// one is the failure this format's terminator exists to prevent. A block whose
/// lines set no `data` dispatches nothing either, which is what makes an id-only
/// keep-alive block invisible to a walk.
///
/// `retry` is read and ignored — reconnecting is the program's decision, and a
/// server-chosen sleep inside a `Core` iterator is a wait with no bound the
/// caller wrote.
///
/// # Errors
///
/// [`line_at`]'s cap, and a `RuntimeError` where one event's accumulated `data`
/// passes [`transport::EVENT_CEILING`].
fn event_at(
    octets: &[u8],
    from: usize,
    carried: Option<&[u8]>,
) -> Result<Option<Dispatched>, Fault> {
    let mut at = from;
    let mut data: Vec<u8> = Vec::new();
    let mut written = false;
    let mut name: Option<Vec<u8>> = None;
    let mut id: Option<Vec<u8>> = carried.map(<[u8]>::to_vec);
    while let Some((line, next)) = line_at(octets, at, true, "events")? {
        at = next;
        if line.is_empty() {
            if written {
                return Ok(Some((
                    crate::instance::build(
                        &EVENT,
                        [
                            Value::str(NvsStr::new(&data)),
                            optional_text(name.as_deref()),
                            optional_text(id.as_deref()),
                        ],
                    ),
                    at,
                    id,
                )));
            }
            name = None;
            continue;
        }
        if line[0] == b':' {
            continue;
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
                if written {
                    data.push(b'\n');
                }
                data.extend_from_slice(value);
                written = true;
                if data.len() > transport::EVENT_CEILING {
                    return Err(Fault::thrown(format!(
                        "{STREAM_NAME}::events(): one event of this reply carries more than the \
                         {} bytes of `data` an event may accumulate — the length is the origin's \
                         choice and the cap is this process's",
                        transport::EVENT_CEILING
                    )));
                }
            }
            b"event" => name = Some(value.to_vec()),
            // A NUL in an id is the one value the format says to ignore, and
            // ignoring it leaves the previous one standing.
            b"id" if !value.contains(&0) => id = Some(value.to_vec()),
            _ => {}
        }
    }
    Ok(None)
}

/// The `?string` slot behind `name()` and `id()`.
fn optional_text(value: Option<&[u8]>) -> Value {
    value.map_or_else(Value::null, |text| Value::str(NvsStr::new(text)))
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
    /// arguments.
    fn nvs_core_http_client_stream(ctx, args: [REQUEST_ARITY + 1]) {
        let verb = crate::router::method_verb(&args[0], STREAM_MEMBER)?.to_ascii_uppercase();
        let (status, body, headers) = exchanged(ctx, &args[1..], "stream", &verb)?;
        Ok(crate::instance::build(
            &STREAM,
            [
                Value::int(status),
                headers,
                Value::bytes(NvsStr::new(&body)),
                Value::null(),
            ],
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
    fn nvs_core_http_events_advance(_ctx, args: [1]) {
        let stepped = step(args[0], &EVENTS, Framing::Events);
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
    fn nvs_core_http_lines_advance(_ctx, args: [1]) {
        let stepped = step(args[0], &LINES, Framing::Lines);
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
    fn nvs_core_http_chunks_advance(_ctx, args: [1]) {
        let stepped = step(args[0], &CHUNKS, Framing::Chunks);
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
    use super::{CHUNKS, EVENTS, LINES, STREAM, event_at, line_at};

    /// The walks' slot layouts, asserted together: one implementation reads all
    /// three, so a class whose slots drifted would frame the wrong field of the
    /// wrong object and still run. `events()` carries one more than the other
    /// two, which is the id the format keeps in force between events.
    #[test]
    fn every_walk_has_the_layout_its_stepper_reads() {
        for class in [&LINES, &CHUNKS] {
            assert_eq!(
                class.slots,
                ["body", "from", "current"],
                "{} does not carry the layout `step` reads",
                class.name
            );
        }
        assert_eq!(EVENTS.slots, ["body", "from", "current", "lastId"]);
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
            let (_, next, id) = event_at(body.as_bytes(), 0, None)
                .expect("no cap is reached")
                .expect("the block ends with a blank line");
            assert_eq!(next, body.len(), "{body:?} was not read to its end");
            assert_eq!(id.as_deref(), Some(&b"7"[..]));
        }
    }

    /// A lone `\r` ends a line for the event format and does not for `lines()`,
    /// which is the one difference between the two framings — asserted here
    /// rather than inferred from either side alone.
    #[test]
    fn a_lone_carriage_return_ends_a_line_for_one_framing_only() {
        let body = b"one\rtwo\n";
        let (line, _) = line_at(body, 0, true, "events")
            .expect("no cap is reached")
            .expect("there is a line");
        assert_eq!(line, b"one");
        let (line, _) = line_at(body, 0, false, "lines")
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
        let (_, _, id) = event_at(body, 0, None)
            .expect("no cap is reached")
            .expect("the second block carries data");
        assert_eq!(id.as_deref(), Some(&b"7"[..]));
    }

    /// A body that stops inside a block is a block the origin did not finish,
    /// so nothing is dispatched from it — where the same bytes followed by the
    /// blank line are one event.
    #[test]
    fn a_block_the_body_stopped_inside_is_not_an_event() {
        assert!(
            event_at(b"data: half", 0, None)
                .expect("no cap is reached")
                .is_none()
        );
        assert!(
            event_at(b"data: half\n\n", 0, None)
                .expect("no cap is reached")
                .is_some()
        );
    }
}
