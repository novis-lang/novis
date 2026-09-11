//! `Core\Sse` — `rule:concurrency/two-doors-one-isolate`
//! 's event stream, as the signature a program writes.
//!
//! § 5 makes an SSE connection "the same model without `receive`": the isolate
//! is § 1's root one — its own arena, its own `[limits]` budget, its own grants
//! — and the operand is § 2's, under
//! `rule:security/isolate-shares-nothing`'s rule. So
//! everything behind the door is [`crate::socket`]'s, shared outright and
//! documented there once: why the member *prepares* an isolate the connection
//! starts, why all three refusals belong to the request, why it answers `void`,
//! and why `limits:`, `grants:` and `on:` are not parameters.
//!
//! # What is different is the door, and it is the whole module
//!
//! **The cell is [`nvs_runtime::SseSlot`] and not
//! [`nvs_runtime::UpgradeSlot`]**, and § 5 states why as a difference between
//! two hand-overs rather than as a preference. A WebSocket upgrade *takes the
//! socket*: the hand-over is the socket itself, it happens after the request's
//! own future has ended, and only a request the server framed an upgrade for
//! has one to give. An SSE connection takes nothing. Its response is an
//! ordinary `200 text/event-stream` that must still be **sent**, and what the
//! isolate writes into is that response's body while the connection future is
//! still running — so there is nothing for the server to frame in advance and
//! nothing to decide before the program has asked.
//!
//! Two consequences, and they are this module's whole surface area:
//!
//! - **Every request a server answers is offered the cell**, where its sibling
//!   is offered only to an upgradable one. That is § 1's fail-closed rule read
//!   against a different hand-over rather than a relaxation of it — what still
//!   has no cell, and so still throws, is everything that is not a served
//!   request: a command-line program, a `spawn script` child, a `#[Test]`
//!   method. The refusal names that, because "no server is answering this" is
//!   what a program has done wrong and "no upgrade was framed" is not.
//! - **The two cells cannot be confused**, because they are two types. A single
//!   slot carrying both would be a slot the connection has to ask the *kind* of
//!   before it could use it, which is a tag standing in for a distinction the
//!   types already make. `nvs_runtime::Inbound`'s module doc is the home of
//!   that reading from the carrier's side.
//!
//! A request that filled both cells is not refused here, and that is deliberate
//! rather than missing: one cell knows nothing of the other, and asking for a
//! socket and an event stream at once is a contradiction about the **response**
//! — refused where the response is written, which is the only place both
//! answers are in hand.
//!
//! # Door two, and the two members that are the whole of it
//!
//! [`nvs_core_sse_stream`] answers *this* request with an event stream that
//! ends when the request does — `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`
//! 's streaming response with `text/event-stream` over it and
//! [`nvs_runtime::sse`]'s framing behind it. It opens the body cell
//! `Core\Response::stream` opens, so a response has one body whichever member
//! wrote it, and it answers the same `Core\Sse` handle [`nvs_core_sse_current`]
//! does: a helper taking one works from either side, which is the reason
//! `rule:concurrency/two-doors-one-isolate` is one type and not two.
//!
//! [`nvs_runtime::sse::DECLARED_HEADERS`] goes out with the head, and none of
//! it is a choice — that constant is where each line is argued, and door one
//! writes the same list into a header map rather than declaring it.
//!
//! **A status declared on a path that opens an event stream is refused**, where
//! `Core\Response::stream` takes any status the program set. An event stream is
//! `200` by protocol, so a program that set a status and then opened one said
//! two things about one response, and the refusal is what stands in place of
//! picking a winner — the arrangement the both-cells-filled response already
//! has.
//!
//! [`nvs_core_sse_send`] is the one way onto the wire and is the same member in
//! both doors. A `string` payload goes out as it was written and anything else
//! is JSON-encoded by the encoder `Core\Json::encode` is, so `$data` accepts a
//! `tainted` value on the argument `Core\Response::json` already makes: the
//! framing belongs to the framer, and [`nvs_runtime::sse`] normalizes a payload
//! before it splits it, so nothing a program supplies can land outside a `data:`
//! line. `$event` and `$id` are the other side of that predicate — a client
//! dispatches on the name — so they are
//! `rule:security/unclassified-parameter-refuses-tainted` sinks, and the three
//! ways an event could not arrive are [`nvs_runtime::sse::Refused`]'s and are
//! thrown here unchanged.
//!
//! [`nvs_core_sse_retry`] is the other thing a program puts on the wire and it is
//! not an event: a `retry:` block carries the reconnection time and dispatches
//! nothing, which is what "do not come back for an hour" ahead of a planned drain
//! has to be. Both members reach the wire through [`onto_the_wire`], because
//! where an event stream's bytes go is a property of the context and never of
//! what was framed.
//!
//! # What is not here yet
//!
//! Door one's far side. `Core\Sse->receive()` is unregistered and so is
//! `Core\Sse\Message`, so a connection isolate can write its events and cannot
//! yet wait on a topic for the next one. The response half is landed:
//! `nvs_server::serve_connection` answers a request that filled the cell with a
//! `200 text/event-stream` and hands the isolate that body, which is what
//! [`nvs_core_sse_current`] answers inside door one. The method entry form
//! throws for [`crate::socket`]'s reason, unchanged: a `callable` carries no
//! parameter names and `rule:security/isolate-shares-nothing` binds `args:` by
//! name.

use nvs_runtime::sse::{DECLARED_HEADERS, Event, MEDIA_TYPE};
use nvs_runtime::{Ctx, Fault, Tag, ThrownClass, Upgrade, Value, copy_graph};

use crate::registry::{Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};
use crate::socket::{entry_program, release_crossed, retained};

/// `Core\Sse`'s fully-qualified name, written once, so the class's own row, the
/// [`CoreTy::Instance`] its two doors answer with and every message quoting it
/// cannot drift apart.
pub(crate) const NAME: &str = r"Core\Sse";

/// `Core\Sse`'s registry rows — `rule:concurrency/two-doors-one-isolate`'s two
/// doors, and the one member that writes onto whichever of them is open. See
/// [`crate::registry::CLASSES`].
///
/// **It is a namespace class and an instance class at once**, exactly as
/// [`crate::socket::CLASS`] is and for the same reason: `upgrade` and `stream`
/// are called on the class from the request, and `send` is called on the handle
/// either of them leaves behind. The instance carries no slots, because an
/// event stream's whole state is the body it writes into and that lives on
/// [`nvs_runtime::Ctx`] — so the handle is a handle, and two of them are not
/// identical and have nothing to tell apart.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "upgrade",
            names: &["entry", "args"],
            // The same two marks the sibling row carries, and for the same
            // reasons: [`CoreTy::Entry`] is `rule:concurrency/an-upgrade-is-spawn-shaped`'s operand rule — a path or a static
            // method written `Feed::run(...)`, never a `callable` in a variable
            // — and `args` takes no expected type because `rule:classes/graph-copy`'s walk decides
            // what may cross at run time. `crate::socket`'s row is where those
            // two are argued; § 5 gives this member "the identical clause", so a
            // difference between the two signatures would be one this ADR does
            // not license.
            params: &[CoreTy::Entry, CoreTy::Mixed],
            defaults: &[Const::Null],
            return_ty: CoreTy::Void,
            symbol: UPGRADE_SYMBOL,
            doc: Some(&UPGRADE_DOC),
        },
        CoreMethod {
            name: "stream",
            names: &[],
            // No parameters at all, where the sibling door takes two and
            // `Core\Response::stream` takes its media type: everything this one
            // could be told is already decided. The entry is this request's own
            // handler, the media type is the protocol's, and the status is the
            // thing the module doc says is refused rather than chosen.
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: STREAM_SYMBOL,
            doc: Some(&STREAM_DOC),
        },
        CoreMethod {
            name: "current",
            names: &[],
            // Nothing to take, for the sibling class's reason: the handle
            // carries no state, so there is nothing to name it by and the
            // context is the whole question. What separates the two members is
            // which question — `crate::socket`'s is the peer it was handed, and
            // this one has none to ask about, `rule:concurrency/two-doors-one-isolate`'s
            // event-stream hand-over taking nothing.
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: CURRENT_SYMBOL,
            doc: Some(&CURRENT_DOC),
        },
    ],
    instance: &[
        CoreMethod {
            name: "send",
            names: &["data", "event", "id"],
            // Three marks, and the split between them is the goal's § *Standing
            // decisions* rather than a reading taken here. `$data` is
            // [`CoreTy::Mixed`] and so carries no classification to refuse with,
            // which is what admits a `tainted` payload — sound because
            // [`nvs_runtime::sse`] normalizes before it splits, so a payload cannot
            // reach any line but a `data:` one. `$event` and `$id` are
            // [`Qual::Sink`]s on `rule:security/sink-predicate`'s own test: a client
            // dispatches on the name and echoes the id back in `Last-Event-ID`, so
            // an attacker-chosen one is an instruction and not a value.
            params: &[
                CoreTy::Mixed,
                CoreTy::Nullable(&CoreTy::Text(Qual::Sink)),
                CoreTy::Nullable(&CoreTy::Text(Qual::Sink)),
            ],
            defaults: &[Const::Null, Const::Null],
            return_ty: CoreTy::Void,
            symbol: SEND_SYMBOL,
            doc: Some(&SEND_DOC),
        },
        CoreMethod {
            name: "retry",
            names: &["after"],
            // A `Duration` and not a number of milliseconds, though milliseconds is
            // what goes on the wire: a parameter taking the number would put the
            // unit at every call site, where `Core\Time\Duration::hours(1)` says
            // which unit it is once and says it where it is written.
            params: &[CoreTy::Instance(crate::time::DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: RETRY_SYMBOL,
            doc: Some(&RETRY_DOC),
        },
    ],
    slots: &[],
    constants: &[],
};

/// The symbol [`CLASS`]'s `upgrade` row is reached through.
const UPGRADE_SYMBOL: &str = "nvs_core_sse_upgrade";

/// The symbol [`CLASS`]'s `stream` row is reached through.
const STREAM_SYMBOL: &str = "nvs_core_sse_stream";

/// The symbol [`CLASS`]'s `current` row is reached through.
const CURRENT_SYMBOL: &str = "nvs_core_sse_current";

/// The symbol [`CLASS`]'s `send` row is reached through.
const SEND_SYMBOL: &str = "nvs_core_sse_send";

/// The symbol [`CLASS`]'s `retry` row is reached through.
const RETRY_SYMBOL: &str = "nvs_core_sse_retry";

/// `Core\Sse::upgrade`'s reference card — `rule:core-api/reference-card`.
const UPGRADE_DOC: MethodDoc = MethodDoc {
    short: "Answers this request with an event stream running `$entry` as a root isolate — its \
            own arena, its own budget and its own grants, sharing nothing with the request that \
            opened it but the values `$args` copied in.",
    params: &[
        ParamDoc {
            name: "entry",
            desc: "What the stream runs: a file path, resolved and root-checked exactly as \
                   `spawn script`'s operand is, or a static method written `Feed::run(...)`. \
                   Never a closure — an isolate shares nothing but compiled code, so a capture \
                   would cross the boundary the isolate exists to be.",
            shape: &[],
        },
        ParamDoc {
            name: "args",
            desc: "The values the stream starts with, bound to the entry's parameters by name. \
                   They cross by the graph copy an isolate boundary already uses, so what arrives \
                   is a value and never a shared reference; a `secret` may not cross and a \
                   `tainted` value stays `tainted` on the other side.",
            shape: &[],
        },
    ],
    ret: "Nothing. Calling it opens the stream — this is not a response value a handler hands \
          back, because nothing interprets a handler's return.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "A request no server is answering, which is every command-line program and \
                   every `spawn script` child; an `$entry` path `script.spawn` does not grant or \
                   that does not compile; a second call on one request.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "An `$args` value with no meaning on the other side of an isolate boundary — a \
                   resource, or a `secret` the call site could not see through; and, for a static \
                   method entry, an `$args` map that omits a parameter the method declares or \
                   names one it does not.",
        },
    ],
};

/// `Core\Sse::stream`'s reference card — `rule:core-api/reference-card`.
const STREAM_DOC: MethodDoc = MethodDoc {
    short: "Answers this request with an event stream that ends when the request does — the head \
            goes out as soon as this is called, and every event written on the handle reaches the \
            client as it is written.",
    params: &[],
    ret: "The handle to write events on, which is the same `Core\\Sse` a connection-scoped stream \
          holds — so a helper taking one works from either door. A browser's `EventSource` \
          reconnects when this stream ends, because the request it belongs to has ended; the \
          readers this door is for are `fetch` and a progress UI that closes itself.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "A status was set on this request before the stream was opened — an event stream is \
               `200` by protocol; or this response already has a body being written over time, a \
               response having one body.",
    }],
};

/// `Core\Sse::current`'s reference card — `rule:core-api/reference-card`.
const CURRENT_DOC: MethodDoc = MethodDoc {
    short: "The event stream this program is writing, whichever door opened it — the one that \
            outlives the request that upgraded it, or the one this request opened for itself.",
    params: &[],
    ret: "The handle to write events on, which is the same `Core\\Sse` the member that opened the \
          stream answered with — so a helper taking one is written once and called from either \
          door.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "A program that is not writing an event stream, which is every command-line program, \
               every `spawn script` child and every request answering with an ordinary body — \
               including one streaming that body, a response body written over time not being an \
               event stream.",
    }],
};

/// `Core\Sse->send`'s reference card — `rule:core-api/reference-card`.
const SEND_DOC: MethodDoc = MethodDoc {
    short: "Writes one event to the client: the payload, and the optional name it dispatches on \
            and id it echoes back when it reconnects.",
    params: &[
        ParamDoc {
            name: "data",
            desc: "What the client's data buffer receives. A `string` goes out as it was written \
                   and anything else is JSON-encoded by the encoder `Core\\Json::encode` is. A \
                   `tainted` value is accepted: the payload is normalized and split into `data:` \
                   lines here, so it cannot reach any other field.",
            shape: &[],
        },
        ParamDoc {
            name: "event",
            desc: "The name the client dispatches this event on, or `null` for the default \
                   `message`. A sink — an attacker-chosen name is an instruction to the reader — \
                   so a `tainted` value is refused where it is written.",
            shape: &[],
        },
        ParamDoc {
            name: "id",
            desc: "The id the client echoes back in `Last-Event-ID` when it reconnects, or `null` \
                   for an event that sets none. A sink for the same reason, and there is no replay \
                   buffer behind it: resumption is the application's own event log, read off the \
                   header by the handler.",
            shape: &[],
        },
    ],
    ret: "Nothing, once the event is framed and handed to the body being written.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "An `$event` or `$id` carrying a line break or a NUL, either of which would end the \
               event early and change what the client acts on; or an empty `$data`, which a client \
               provably does not dispatch.",
    }],
};

/// `Core\Sse->retry`'s reference card — `rule:core-api/reference-card`.
const RETRY_DOC: MethodDoc = MethodDoc {
    short: "Tells the client how long to wait before it reconnects, as a block of its own that \
            dispatches no event.",
    params: &[ParamDoc {
        name: "after",
        desc: "How long the client waits before it comes back. It reaches the wire in \
               milliseconds, so a wait shorter than one arrives as `0` — the least the wire can \
               say rather than a refusal.",
        shape: &[],
    }],
    ret: "Nothing, once the block is handed to the body being written.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "A negative `$after`: a client waits this long before coming back, and there is no \
               wait shorter than none.",
    }],
};

/// `Core\Sse\Message`'s fully-qualified name, as [`CoreTy::Instance`] spells
/// it.
pub(crate) const MESSAGE_NAME: &str = r"Core\Sse\Message";

/// [`MESSAGE`]'s slots, in the order a delivery is read into them.
const MESSAGE_TOPIC: usize = 0;
const MESSAGE_VALUE: usize = 1;

/// `rule:concurrency/a-connection-is-a-loop`'s message for the door that has no
/// peer: what a publisher put on a topic this stream subscribed to.
///
/// # Two readers, and neither answers `null` to mean "the other kind"
///
/// [`crate::socket::MESSAGE`] carries four because two sources meet in one
/// wait — a frame the peer sent and a value the bus delivered — and `topic`
/// is what tells them apart. An event stream is handed no socket
/// (`rule:concurrency/two-doors-one-isolate`), so there is no second source
/// and every message is a delivery. That is why `topic` answers a `string`
/// rather than a `?string`: the discrimination the sibling's readers exist for
/// has nothing here to discriminate, and a nullable topic would be a question
/// with one answer.
///
/// # Its own class, not the sibling reused
///
/// A shared class would carry `text` and `bytes` that can only ever answer
/// `null` on this door, which is the silent wrong answer
/// `rule:errors/ambiguous-input-refused` refuses: a program would be free to
/// ask an event stream's message for a frame's payload and read the `null` as
/// "the peer sent nothing" rather than as "there is no peer". Two classes make
/// that a name error where it is written.
///
/// **`value` carries no qualifier of this row's.** It came from another isolate
/// on this side of the wire and crossed by `rule:classes/graph-copy`'s copy, so
/// it arrives with whatever qualifiers it already had, exactly as the sibling's
/// does.
pub(crate) const MESSAGE: CoreClass = CoreClass {
    name: MESSAGE_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "topic",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: MESSAGE_TOPIC_SYMBOL,
            doc: Some(&MESSAGE_TOPIC_DOC),
        },
        CoreMethod {
            name: "value",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Mixed,
            symbol: MESSAGE_VALUE_SYMBOL,
            doc: Some(&MESSAGE_VALUE_DOC),
        },
    ],
    slots: &["topic", "value"],
    constants: &[],
};

/// The symbols [`MESSAGE`]'s two readers are reached through.
const MESSAGE_TOPIC_SYMBOL: &str = "nvs_core_sse_message_topic";
const MESSAGE_VALUE_SYMBOL: &str = "nvs_core_sse_message_value";

/// `Core\Sse\Message::topic`'s reference card — `rule:core-api/reference-card`.
const MESSAGE_TOPIC_DOC: MethodDoc = MethodDoc {
    short: "The topic this value was published to, which is one of the names this stream \
            subscribed under.",
    params: &[],
    ret: "The topic's name. Never `null`: an event stream is handed no peer, so every message \
          it receives is a delivery. It is the name this program subscribed under and not \
          anything a client chose, so it is not `tainted`.",
    errors: &[],
};

/// `Core\Sse\Message::value`'s reference card — `rule:core-api/reference-card`.
const MESSAGE_VALUE_DOC: MethodDoc = MethodDoc {
    short: "What a publisher put on the topic, copied across the isolate boundary the way every \
            other value crosses one.",
    params: &[],
    ret: "The published value, with whatever qualifiers it already carried. It is a copy and \
          never a shared reference, so writing to it changes nothing the publisher can see.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::registry::CLASSES`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        UPGRADE_SYMBOL => (nvs_core_sse_upgrade as *const ()).cast(),
        STREAM_SYMBOL => (nvs_core_sse_stream as *const ()).cast(),
        CURRENT_SYMBOL => (nvs_core_sse_current as *const ()).cast(),
        SEND_SYMBOL => (nvs_core_sse_send as *const ()).cast(),
        RETRY_SYMBOL => (nvs_core_sse_retry as *const ()).cast(),
        MESSAGE_TOPIC_SYMBOL => (nvs_core_sse_message_topic as *const ()).cast(),
        MESSAGE_VALUE_SYMBOL => (nvs_core_sse_message_value as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Sse::upgrade(string $entry, mixed $args = null): void` — `rule:concurrency/two-doors-one-isolate`
    /// 's event stream, prepared here and started by the connection.
    ///
    /// The sibling body, with one line different, and that line is the module
    /// doc's subject: the cell asked for is the one every served request has.
    /// The three refusals are [`crate::socket`]'s — no cell, the entry, then
    /// the argument — in the order they cost the least, and each is a fact
    /// about a different thing.
    fn nvs_core_sse_upgrade(ctx, args: [2]) {
        // Cloned rather than borrowed, for the sibling's reason: `entry_program`
        // takes the context, and both halves of the cell are shared handles by
        // construction, so a clone is one refcount and no borrow held across a
        // call.
        let cell = ctx
            .inbound()
            .and_then(nvs_runtime::Inbound::sse_slot)
            .cloned()
            .ok_or_else(|| {
                Fault::thrown(
                    "`Core\\Sse::upgrade` needs a request a server is answering and this is not \
                     one: every request the server runs is offered an event stream, and a \
                     program run from the command line, a `spawn script` child and a `#[Test]` \
                     method are not requests",
                )
            })?;
        let program = entry_program(ctx, args[0], args[1], "Core\\Sse::upgrade")?;
        // No case can reach this, for the sibling's reason: it stands after the
        // cell, and a `.nvst` case runs a script no server is answering. The
        // crossing itself is `crate::socket`'s, and
        // `an_args_value_that_cannot_cross_is_refused_before_the_slot_is_filled`
        // there is the `#[test]` that asserts this arm of it.
        let crossed = copy_graph(retained(args[1])).map_err(|refused| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!("`Core\\Sse::upgrade`'s `args:` cannot cross into a connection: {refused}"),
            )
        })?;
        if let Err(returned) = cell.fill(Upgrade::new(program, crossed)) {
            let (_program, crossed) = returned.into_parts();
            release_crossed(crossed);
            // No case can reach this: a second stream needs a first, and a first
            // needs a cell no `.nvst` case is offered.
            // `a_second_event_stream_on_one_request_is_refused_and_the_first_still_stands`
            // is the `#[test]` that asserts it instead.
            return Err(Fault::thrown(
                "`Core\\Sse::upgrade` was called twice on one request, and a request opens at \
                 most one event stream",
            ));
        }
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Sse::stream(): Core\Sse` — door two, the stream that ends with its
    /// request.
    ///
    /// `Core\Response::stream`'s effects with one refusal in front of them and
    /// two headers beside them, every one of which the module doc argues: the
    /// status is refused rather than carried, `Cache-Control` and
    /// `X-Accel-Buffering` are declared before the head is taken, the media type
    /// is the protocol's rather than a parameter, the writing half of
    /// `nvs_runtime::stream` goes onto this context for [`nvs_core_sse_send`] to
    /// find, and the context is marked as writing an event stream, which is what
    /// [`nvs_core_sse_current`] answers.
    ///
    /// **Off a connection there is no cell, and then this member is inert**, on
    /// its sibling's reading: a CLI program, a `#[Test]` method and a `.nvst`
    /// case each open a stream that writes to their own output, so a case can
    /// assert the exact bytes of an event stream without being a request.
    fn nvs_core_sse_stream(ctx, _args: [0]) {
        if let Some(code) = ctx.take_status() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "Core\\Sse::stream(): this request set the status `{code}` and an event \
                     stream is `200` by protocol, so the two cannot both be answered"
                ),
            ));
        }
        // Declared rather than appended, and before the head is taken: these are
        // what make the stream arrive at all, so a program that set one of them
        // to something else set it for a response it no longer has. The list is
        // `nvs_runtime::sse`'s, which is where door one reads the same names
        // from.
        for (name, value) in DECLARED_HEADERS {
            ctx.declare_header(name, value);
        }
        // Cloned rather than borrowed, for `upgrade`'s reason: the cell is a
        // shared handle by construction, so a clone is one refcount and no
        // borrow of the carrier held across the write below.
        let cell = ctx
            .inbound()
            .and_then(nvs_runtime::Inbound::response_stream_slot)
            .cloned();
        if let Some(cell) = cell {
            // The declarations go with the head, because the head is on the wire
            // from here. The status is `None` and not `ctx.take_status()`: the
            // refusal above is what a program gets for having set one.
            let headers = ctx.take_headers();
            let emit = cell.open(MEDIA_TYPE, None, headers).ok_or_else(|| {
                // No case can reach this: a `.nvst` case runs a script no
                // connection is framing a response for, so it is offered no cell
                // and never gets here.
                // `an_event_stream_is_refused_where_a_response_body_is_already_being_written`
                // is the `#[test]` that asserts it instead.
                Fault::thrown_as(
                    ThrownClass::Logic,
                    "Core\\Sse::stream(): this request has already opened a response body \
                     stream, and a response has one body",
                )
            })?;
            ctx.set_body_stream(emit);
        }
        ctx.declare_content_type(MEDIA_TYPE);
        // After the refusal and outside the cell, on both counts deliberately: a
        // request that contradicted itself opened nothing, and a program off a
        // connection opened an event stream all the same — its events go to its
        // own output, and the handle it reaches for is the same one.
        ctx.mark_event_stream();
        Ok(crate::instance::build(&CLASS, []))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Sse::current(): Core\Sse` — the handle, for code that did not open
    /// the stream itself.
    ///
    /// [`crate::socket`]'s `current` one class over, with the question changed
    /// and nothing else changed: the handle carries nothing, so this allocates
    /// an object with no slots and the *context* is what [`nvs_core_sse_send`]
    /// reads. [`CLASS`]'s own doc owns why that is the right shape.
    ///
    /// **The question is neither the peer nor an open body**, and that is the
    /// whole of what this member had to settle. An event stream is handed no
    /// socket — `rule:concurrency/two-doors-one-isolate`'s hand-over takes
    /// nothing — so the sibling's question answers `false` inside the very
    /// isolate this must answer for. And a request streaming an ordinary body
    /// holds the writing half too, so *is a body open* answers `true` for a
    /// program this must refuse. What the context records instead is the door
    /// having been opened, which is the one thing both doors share and nothing
    /// else has.
    fn nvs_core_sse_current(ctx, _args: [0]) {
        if !ctx.has_event_stream() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                "`Core\\Sse::current` needs a program writing an event stream and this is not \
                 one: a stream is opened by `Core\\Sse::stream` for the request that answers with \
                 it and by `Core\\Sse::upgrade` for the isolate that outlives it, and a response \
                 writing an ordinary body over time is neither",
            ));
        }
        Ok(crate::instance::build(&CLASS, []))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Sse->send(mixed $data, ?string $event = null, ?string $id = null): void`
    /// — the one way onto the wire, in both doors.
    ///
    /// The member holds no policy of its own. What a payload may be is the
    /// module doc's, the framing and all three refusals are
    /// [`nvs_runtime::sse`]'s, and where the bytes go is whichever half of the
    /// context is holding a body — the cell a streaming response opened, or this
    /// program's own output where nothing opened one.
    ///
    /// The event is framed before anything is written, which is `json`'s order
    /// rather than `text`'s: an event that cannot be framed throws with no bytes
    /// on the wire, so a refusal is a refusal and not a half-written event a
    /// client would have to resynchronize after.
    fn nvs_core_sse_send(ctx, args: [4]) {
        crate::instance::receiver(args[0], &CLASS, "send")?;
        let encoded;
        let data = match args[1].as_str_bytes() {
            Some(written) => written,
            None => {
                // The encoder `Core\Json::encode` is, reached through the same
                // [`crate::json::Encodable`]: one serializer is what makes the
                // module doc's claim about a `tainted` payload true, since a
                // second one could frame a string differently.
                encoded = serde_json::to_string(&crate::json::Encodable::document(args[1]))
                    .map_err(|why| {
                        Fault::thrown_as(
                            ThrownClass::Logic,
                            format!("Core\\Sse::send(): the payload cannot be encoded: {why}"),
                        )
                    })?;
                encoded.as_bytes()
            }
        };
        let frame = Event {
            data,
            event: optional_field(&args[2], "event")?,
            id: optional_field(&args[3], "id")?,
            retry: None,
        }
        .frame()
        .map_err(|refused| {
            Fault::thrown_as(ThrownClass::Logic, format!("Core\\Sse::send(): {refused}"))
        })?;
        onto_the_wire(ctx, frame, "send")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Sse->retry(Core\Time\Duration $after): void` — the reconnection
    /// time, for the drain a program can see coming.
    ///
    /// A block of its own and not a field on the next event, which
    /// [`nvs_runtime::sse::reconnect_after`] is the home of. What it adds here
    /// is the one refusal: a negative wait cannot be spelled on the wire and
    /// reading it as "no wait" would be an answer to a question the program did
    /// not ask.
    ///
    /// The same member and the same bytes in both doors. A browser reconnects a
    /// request-scoped stream the moment it ends, so this is door two's one way
    /// of saying *when* — and it is the same sentence door one sends ahead of a
    /// drain it is about to take.
    fn nvs_core_sse_retry(ctx, args: [2]) {
        crate::instance::receiver(args[0], &CLASS, "retry")?;
        let nanos = crate::time::nanos_of(args, 1, "retry")?;
        let wait = u64::try_from(nanos)
            .map(std::time::Duration::from_nanos)
            .map_err(|_| {
                Fault::thrown_as(
                    ThrownClass::Logic,
                    "Core\\Sse::retry(): a reconnection time cannot be negative: a client waits \
                     this long before it comes back, and there is no wait shorter than none",
                )
            })?;
        onto_the_wire(ctx, nvs_runtime::sse::reconnect_after(wait), "retry")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Sse\Message::topic(): string` — the topic this delivery arrived
    /// on, and one of the names this stream subscribed under.
    fn nvs_core_sse_message_topic(_ctx, args: [1]) {
        crate::instance::read_slot(args, &MESSAGE, MESSAGE_TOPIC, "topic")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Sse\Message::value(): mixed` — the copy of what was published.
    fn nvs_core_sse_message_value(_ctx, args: [1]) {
        crate::instance::read_slot(args, &MESSAGE, MESSAGE_VALUE, "value")
    }
}

/// One framed block onto whichever half of the context is holding a body — the
/// cell a streaming response opened, or this program's own output where nothing
/// opened one.
///
/// One function rather than a tail on each member that writes, because where an
/// event stream's bytes go is a property of the context and never of what was
/// framed. Takes the block by value, so the chunk the connection drains is the
/// one the framing allocated rather than a copy of it.
///
/// # Errors
///
/// A `RuntimeError` where the connection draining the stream has gone, and a
/// [`Fault::fatal`] for a sink that could not take the bytes.
fn onto_the_wire(ctx: &mut Ctx, framed: Vec<u8>, member: &str) -> Result<Value, Fault> {
    if let Some(emit) = ctx.body_stream() {
        // No case can reach this: a stream that can close is one a connection is
        // draining, and a `.nvst` case is offered no cell.
        // `an_event_whose_reader_has_gone_is_refused_rather_than_parked` is the
        // `#[test]` that asserts it instead.
        return emit
            .send(framed)
            .map(|()| Value::null())
            .map_err(|closed| Fault::thrown(format!("Core\\Sse::{member}(): {closed}")));
    }
    // Unreachable from source, on `Core\Response::text`'s reasoning:
    // `OutputSink::Buffer` and `Sink` never fail, which `Ctx::write_output`'s own
    // `# Errors` states, and nothing in the language closes a descriptor the host
    // handed the process.
    ctx.write_output(&framed)
        .map_err(|error| Fault::fatal(format!("Core\\Sse::{member} could not write: {error}")))?;
    Ok(Value::null())
}

/// One optional field of an event as the bytes it was written as, or [`None`]
/// where the call site said nothing.
fn optional_field<'a>(value: &'a Value, field: &str) -> Result<Option<&'a [u8]>, Fault> {
    if matches!(value.tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    value.as_str_bytes().map(Some).ok_or_else(|| {
        // Unreachable from source: the row's parameter is a
        // `CoreTy::Nullable(CoreTy::Text)`, so `E0401` refuses anything that is
        // neither a `string` nor `null` before this runs — and refuses a
        // `tainted` one besides, both of these being sinks.
        Fault::fatal(format!(
            "Core\\Sse::send expected a `string` or `null` for `{field}`, got tag {}",
            value.tag_byte()
        ))
    })
}

#[cfg(test)]
mod tests {
    use nvs_runtime::script::{Installed, Program, Resolver, install};
    use nvs_runtime::{Ctx, Inbound, NvsArray, NvsStr, SseSlot, Value};

    use super::nvs_core_sse_upgrade;
    use crate::socket::release_crossed;

    /// A resolver answering with the length of the path it was asked for —
    /// `crate::socket`'s test fixture, and its doc is the home of why a length
    /// is what proves a program's identity on this side of the seam.
    #[derive(Debug)]
    struct Fixed;

    impl Resolver for Fixed {
        fn resolve(&self, path: &str) -> Result<Program, String> {
            let len = i64::try_from(path.len()).unwrap_or(-1);
            Ok(Box::new(move |ctx, args| {
                ctx.set_isolate_argument(args);
                Value::int(len)
            }))
        }
    }

    // `install` takes a `&'static dyn Resolver`, so a `static` is the only way
    // to reach it directly.
    static FIXED: Fixed = Fixed;

    fn resolving() -> Installed {
        install(&FIXED)
    }

    /// A context granting `script.spawn` for everything, because `rule:security/capability-check-at-the-door`'s
    /// door is inside `resolve` and a bare context grants nothing — every case
    /// below is about the cell rather than about the grant.
    fn granting() -> Ctx {
        let mut snapshot = nvs_config::Snapshot::default();
        snapshot.config.capabilities = Some(nvs_config::tree::Capabilities {
            script: Some(nvs_config::tree::CapScript {
                spawn: Some(nvs_config::tree::Setting::Bool(true)),
            }),
            ..nvs_config::tree::Capabilities::default()
        });
        let mut ctx = Ctx::buffered();
        ctx.set_config(std::sync::Arc::new(snapshot));
        ctx
    }

    /// A carrier for an ordinary request a server is answering — no upgrade
    /// offered, which is the difference § 5 turns on.
    fn served(ctx: &mut Ctx, cell: &SseSlot) {
        let mut inbound = Inbound::new("GET", "/events", "");
        inbound.offer_sse(cell.clone());
        ctx.set_inbound(inbound);
    }

    /// `{room: 7}`, as the one refcounted argument a crossing can be observed
    /// on: an `int` would cross as itself and prove nothing.
    fn a_room() -> Value {
        let mut map = NvsArray::new();
        map.set(NvsStr::new(b"room"), Value::int(7));
        Value::array(map)
    }

    /// § 5's hand-over, asserted as the sibling's is: after the call the cell
    /// holds the resolver's program for the path that was *written*, and the
    /// argument beside it is a **copy** rather than the request's own graph.
    ///
    /// What this adds over the sibling's assertion is the carrier it runs on —
    /// a plain `GET` no upgrade was framed for, which is the request § 5 says
    /// is offered a cell and § 1 says is not offered a slot.
    #[test]
    fn an_event_stream_leaves_the_resolvers_program_and_a_copy_of_its_argument_in_the_cell() {
        let _resolver = resolving();
        let mut ctx = granting();
        let cell = SseSlot::new();
        served(&mut ctx, &cell);

        let path = Value::str(NvsStr::new(b"streams/feed.nvs"));
        let mine = a_room();
        nvs_runtime::call(nvs_core_sse_upgrade, &mut ctx, &[path, mine])
            .expect("an offered cell takes the stream");

        let (program, crossed) = cell
            .take()
            .expect("the member filled the cell the server left")
            .into_parts();
        assert_ne!(
            crossed.array_ptr(),
            mine.array_ptr(),
            "the connection was handed the request's own array rather than a copy of it"
        );

        // Run it the way the connection does — on a context of its own, with
        // the crossed argument transferred into it — so the program is proved
        // to be the resolver's answer for the written path and the crossed
        // reference lands in an ownership root that will release it.
        let mut connection = Ctx::buffered();
        assert_eq!(
            program(&mut connection, crossed).as_int(),
            Some(16),
            "the cell holds a program for some other path"
        );
        release_crossed(mine);
    }

    /// `nvs_runtime::SseSlot::fill`'s refusal reaching a program: the second
    /// call is told so and the first stream is what the connection still
    /// starts. Unreachable from a `.nvst` case — a second stream needs a first,
    /// and a first needs a cell nothing offers a command-line script.
    #[test]
    fn a_second_event_stream_on_one_request_is_refused_and_the_first_still_stands() {
        let _resolver = resolving();
        let mut ctx = granting();
        let cell = SseSlot::new();
        served(&mut ctx, &cell);

        let first = Value::str(NvsStr::new(b"streams/feed.nvs"));
        nvs_runtime::call(nvs_core_sse_upgrade, &mut ctx, &[first, Value::null()])
            .expect("an offered cell takes the first stream");

        let second = Value::str(NvsStr::new(b"streams/other-feed.nvs"));
        nvs_runtime::call(nvs_core_sse_upgrade, &mut ctx, &[second, Value::null()])
            .expect_err("one request opens at most one event stream");

        // The first one is intact rather than displaced, which is the half a
        // refusal that overwrote would still pass without.
        let (program, crossed) = cell.take().expect("the first fill stands").into_parts();
        let mut connection = Ctx::buffered();
        assert_eq!(
            program(&mut connection, crossed).as_int(),
            Some(16),
            "the second call displaced the first stream's program"
        );
    }

    /// A context carrying the body cell a connection offers, bounded generously
    /// enough that no case below can meet the send timeout — `crate::response`'s
    /// own fixture, one door over, and every claim here is about the seam rather
    /// than about the clock.
    fn framing(slot: &nvs_runtime::stream::BodySlot) -> Ctx {
        let mut ctx = Ctx::buffered();
        let mut inbound = Inbound::new("GET", "/events", "");
        inbound.offer_response_stream(slot.clone());
        ctx.set_inbound(inbound);
        ctx
    }

    /// Drops a reference this module built and the borrowing member did not
    /// take, exactly as `crate::response`'s tests own theirs.
    fn dropped(value: Value) {
        #[expect(
            unsafe_code,
            reason = "this test owns the reference it built, and a `Core` member \
                      borrows its arguments rather than consuming them"
        )]
        unsafe {
            value.release();
        }
    }

    /// Door two opening the cell: the head the connection is handed declares
    /// `text/event-stream` and carries the two headers without which the stream
    /// does not arrive, and the event the program sent reaches the connection's
    /// half rather than the request's own output.
    ///
    /// One claim and not three, because a member that declared the media type
    /// and wrote its events somewhere else would pass any of them alone.
    #[test]
    fn an_event_stream_declares_its_head_and_its_events_reach_the_connections_half() {
        let slot = nvs_runtime::stream::BodySlot::new(std::time::Duration::from_secs(30));
        let mut ctx = framing(&slot);

        let handle = nvs_runtime::call(super::nvs_core_sse_stream, &mut ctx, &[])
            .expect("an offered cell takes the stream");
        let data = Value::str(NvsStr::new(b"ready"));
        nvs_runtime::call(
            super::nvs_core_sse_send,
            &mut ctx,
            &[handle, data, Value::null(), Value::null()],
        )
        .expect("the first event goes into an empty cell without parking");

        let mut head = slot
            .take(std::task::Waker::noop())
            .expect("the member filled the cell");
        assert_eq!(&*head.content_type, "text/event-stream");
        let names: Vec<&str> = head.headers.iter().map(|header| &*header.name).collect();
        assert!(
            names.contains(&"Cache-Control") && names.contains(&"X-Accel-Buffering"),
            "the head went out without the headers an event stream needs: {names:?}"
        );
        let nvs_runtime::stream::Drained::Chunk(framed) =
            head.drain.next_chunk(std::task::Waker::noop())
        else {
            panic!("the event the program sent never reached the connection");
        };
        assert_eq!(framed, b"data: ready\n\n");
        assert_eq!(
            ctx.take_buffered_output().unwrap_or_default(),
            b"",
            "a framed event went to the request's own output as well"
        );

        dropped(data);
        dropped(handle);
    }

    /// The cell's refusal reaching a program: a response has one body, so an
    /// event stream over a response already writing one is refused and the
    /// first body is what the connection still frames. Unreachable from a
    /// `.nvst` case — the cell it needs is offered to no script.
    #[test]
    fn an_event_stream_is_refused_where_a_response_body_is_already_being_written() {
        let slot = nvs_runtime::stream::BodySlot::new(std::time::Duration::from_secs(30));
        let mut ctx = framing(&slot);

        // The cell opened directly, which is what `Core\Response::stream` does
        // with it: what this asserts is the refusal reaching a program, and
        // reaching through the sibling member to fill the cell would make it a
        // claim about that member as well.
        let _writing = slot
            .open("text/csv", None, Vec::new())
            .expect("an empty cell opens");
        nvs_runtime::call(super::nvs_core_sse_stream, &mut ctx, &[])
            .expect_err("one response opens at most one body stream");

        // Intact rather than displaced, which is the half a refusal that
        // overwrote would still pass without.
        let head = slot
            .take(std::task::Waker::noop())
            .expect("the first open stands");
        assert_eq!(&*head.content_type, "text/csv");
    }

    /// The refusal a program that is not writing an event stream gets, named
    /// after the member it called.
    ///
    /// Asserted here as well as in a `.nvst` case because of the *carrier*: the
    /// program a stream-open question would wrongly admit is a request
    /// streaming a body of its own, and a case reaches neither cell.
    #[test]
    fn current_outside_an_event_stream_is_refused_by_name() {
        let slot = nvs_runtime::stream::BodySlot::new(std::time::Duration::from_secs(30));
        let mut ctx = framing(&slot);

        let status = nvs_runtime::call(super::nvs_core_sse_current, &mut ctx, &[])
            .expect_err("a request that opened no event stream has no handle to answer");
        assert_eq!(
            status,
            nvs_runtime::THROWN,
            "an absent event stream is a throw a program catches, not a fatal"
        );
        assert_eq!(
            ctx.pending_class().as_deref(),
            Some("LogicError"),
            "the refusal is not the class the signature promises"
        );
        let message = ctx
            .pending()
            .expect("a throw carries the message it was raised with")
            .into_owned();
        assert!(
            message.contains(r"Core\Sse::current"),
            "the refusal does not name the member that was called: {message}"
        );
        drop(ctx.take_pending());

        // The cell opened directly, which is what `Core\Response::stream` does
        // with it: a response body written over time is the program that holds
        // a writing half and is still not an event stream.
        let _writing = slot
            .open("text/csv", None, Vec::new())
            .expect("an empty cell opens");
        nvs_runtime::call(super::nvs_core_sse_current, &mut ctx, &[])
            .expect_err("a response body stream is not an event stream");
        drop(ctx.take_pending());
    }

    /// An event whose reader has gone is refused rather than parked — the
    /// connection dropping its half closes the stream at once, so a program
    /// learns it on the next event instead of at the send timeout.
    ///
    /// Unreachable from a `.nvst` case for the reason above: a stream that can
    /// close is one a connection is draining.
    #[test]
    fn an_event_whose_reader_has_gone_is_refused_rather_than_parked() {
        let slot = nvs_runtime::stream::BodySlot::new(std::time::Duration::from_secs(30));
        let mut ctx = framing(&slot);

        let handle = nvs_runtime::call(super::nvs_core_sse_stream, &mut ctx, &[])
            .expect("an offered cell takes the stream");
        drop(
            slot.take(std::task::Waker::noop())
                .expect("the member filled the cell"),
        );

        let data = Value::str(NvsStr::new(b"too late"));
        nvs_runtime::call(
            super::nvs_core_sse_send,
            &mut ctx,
            &[handle, data, Value::null(), Value::null()],
        )
        .expect_err("a send whose reader has gone is told so rather than parked");

        dropped(data);
        dropped(handle);
    }
}
