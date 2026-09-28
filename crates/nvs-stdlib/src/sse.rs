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
//! # Door one's far side, and the wait that is the whole of it
//!
//! [`nvs_core_sse_receive`] is [`crate::socket`]'s wait with its first source
//! taken away. An event stream is handed no peer, so a connection isolate waits
//! on the queue `rule:core-classes/topic`'s bus fills and on nothing else, and
//! each delivery arrives as a `Core\Sse\Message` — two slots, `topic` and
//! `value`, read back by the members beside them. A streaming response holds
//! the same handle and cannot wait; that member's own doc owns why that is a
//! refusal rather than a `null`.
//!
//! The response half is behind the door. `nvs_server::serve_connection` answers
//! a request that filled the cell with a `200 text/event-stream` and hands the
//! isolate that body, which is what [`nvs_core_sse_current`] answers inside
//! door one. A method entry — `Feed::run(...)` — opens the same isolate a path
//! entry does, because the parameter names `rule:security/isolate-shares-nothing`
//! binds `args:` by ride on the callable value: [`crate::socket`] § *The method
//! form carries its names on the value* is the home of that.

use nvs_runtime::sse::{DECLARED_HEADERS, Event, MEDIA_TYPE};
use nvs_runtime::{
    Ctx, Delivery, EventStreamDoor, Fault, NvsStr, Tag, ThrownClass, Upgrade, Value, copy_graph,
};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};
use crate::socket::{entry_program, release_crossed, retained};

/// `Core\Sse`'s fully-qualified name, written once, so the class's own row, the
/// [`CoreTy::Instance`] its two doors answer with and every message quoting it
/// cannot drift apart.
pub(crate) const NAME: &str = r"Core\Sse";

/// `Core\Sse`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "An event stream from your server to a client, as a browser's `EventSource` reads it. \
            A request calls `stream` to answer with events, or `upgrade` to start a script that \
            keeps sending after the request ends. Code that did not open the stream calls \
            `current` to get it, and `send` writes one event.",
};

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
    doc: Some(&CARD),
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
            name: "receive",
            names: &[],
            // Nothing to take and nothing to choose, where the sibling's row of
            // this name is the same: what a connection waits for is decided by
            // what it subscribed to, and a wait that took a topic would be a
            // second subscription written at the call site.
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(MESSAGE_NAME)),
            symbol: RECEIVE_SYMBOL,
            doc: Some(&RECEIVE_DOC),
        },
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

/// The symbol [`CLASS`]'s `receive` row is reached through.
const RECEIVE_SYMBOL: &str = "nvs_core_sse_receive";

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

/// `Core\Sse->receive`'s reference card — `rule:core-api/reference-card`.
const RECEIVE_DOC: MethodDoc = MethodDoc {
    short: "Waits for the next value published to a topic this stream subscribed to, and answers \
            it as one message.",
    params: &[],
    ret: "The next message, or `null` once this stream is over — which is what ends the \
          `while (var $msg = $sse->receive())` loop a stream's script is written as. A message \
          carries the published value and the name of the topic it arrived on. `null` is also \
          what a stream that fell too far behind its topics is answered: its queue overflowed, \
          so this stream is closed rather than a publisher being made to wait for it.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "This program is a streaming response rather than the isolate a stream outlives \
               its request on — it ends with its own events, so nothing published could reach \
               it, and there is no wait to be had.",
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

/// `Core\Sse\Message`'s class card — `rule:core-api/reference-card`.
const MESSAGE_CARD: ClassDoc = ClassDoc {
    short: "One value that arrived on an event stream's script, returned by `Core\\Sse->receive`. \
            `topic` returns the name of the topic it was published to, and `value` returns what \
            was published.",
};

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
    doc: Some(&MESSAGE_CARD),
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
        RECEIVE_SYMBOL => (nvs_core_sse_receive as *const ()).cast(),
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
        // own output, and the handle it reaches for is the same one. The door
        // named here is the one this member *is*, and it lands only where no
        // door is marked yet: a connection isolate was marked as one before its
        // first statement, and opening a stream on its own output does not make
        // it a response that ends.
        ctx.mark_event_stream(EventStreamDoor::Response);
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
                encoded = crate::json::document(ctx, args[1], |why| {
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

/// How long the wait parks for before it looks at what the other cores
/// published again.
///
/// A publish from **this** core wakes the wait where it stands —
/// [`nvs_runtime::Inbox`]'s own wake, fired by the fan-out that filled the
/// queue — so this bound is the cross-core half alone: a publish from another
/// core lands in a queue this core drains ([`crate::topic`]'s
/// `deliver_from_other_cores`) rather than in a push it is told about, and a
/// stream with nothing happening on its own core would otherwise never look.
///
/// **What it spends:** one wakeup per idle stream per tick, and nothing on a
/// stream whose topics are busy on its own core. It is the one number here that
/// trades priority 3 against itself — a shorter tick buys cross-core latency
/// with wakeups — and the trade goes away rather than being tuned the day the
/// bus wakes the core it hands a value to.
const CROSS_CORE_TICK: std::time::Duration = std::time::Duration::from_millis(50);

/// One [`MESSAGE`] built out of a value the bus delivered.
///
/// The topic is read before the value is taken, because taking it consumes the
/// delivery — [`Delivery`] hands its one owned reference on rather than copying
/// it, and the slot is where that reference comes to rest. Two slots and not the
/// sibling's four: [`MESSAGE`]'s own doc owns why this door has one source.
fn message_of_delivery(delivery: Delivery) -> Value {
    let topic = Value::str(NvsStr::new(delivery.topic().as_bytes()));
    let value = delivery.into_value();
    crate::instance::build(&MESSAGE, [topic, value])
}

nvs_runtime::nvs_helper! {
    /// `Core\Sse->receive(): ?Core\Sse\Message` — the wait on the door that
    /// outlives its request, over topics and over nothing else.
    ///
    /// [`crate::socket`]'s wait with its first source taken away. An event
    /// stream is handed no peer (`rule:concurrency/two-doors-one-isolate`), so
    /// there is no frame to read and no second kind of message; what is left is
    /// the queue `rule:core-classes/topic`'s bus fills, which is why this is the
    /// sibling's loop minus the socket read. `rule:concurrency/a-connection-is-a-loop`'s
    /// `while (var $msg = $sse->receive())` is the shape a stream is written as
    /// either way.
    ///
    /// **A streaming response is refused rather than answered `null`.** Both
    /// doors hold this handle and only one of them can wait: a request
    /// answering with its own events ends with them, so a `null` there would
    /// read as "the stream is over" and end a loop for a reason that was never
    /// true.
    ///
    /// The park is where the core goes back to its neighbours, and it is
    /// bounded — [`CROSS_CORE_TICK`] owns why a wait that is woken still has a
    /// deadline. A wake is a hint, so the queue is looked at again on every
    /// turn rather than the resume being taken for an answer.
    fn nvs_core_sse_receive(ctx, args: [1]) {
        crate::instance::receiver(args[0], &CLASS, "receive")?;
        if !ctx.has_event_stream_connection() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                "`Core\\Sse::receive` needs a connection and this program is a streaming \
                 response: a request answering with its own events ends when they do, so a \
                 publisher has nothing to reach it on — only a script `Core\\Sse::upgrade` \
                 opened can wait",
            ));
        }
        loop {
            crate::topic::deliver_from_other_cores(ctx);
            // `rule:core-classes/topic`'s bound, taken by the subscriber
            // itself: the publisher refused the value rather than waiting, and
            // this is where that refusal becomes the close it means. The stream
            // ends instead of carrying a close code — an event stream has no
            // frame to put one in, so the body ending *is* what the client
            // sees, and the next `EventSource` reconnect is an ordinary request
            // the application answers as it likes.
            if ctx.inbox_overflowed() {
                if let Some(emit) = ctx.body_stream() {
                    emit.finish();
                }
                return Ok(Value::null());
            }
            if let Some(delivery) = ctx.take_delivery() {
                return Ok(message_of_delivery(delivery));
            }
            // The stream is over once nothing will read it: the client went
            // away, a send met its bound, or the body was ended. A queued
            // delivery is still answered first, as a socket's `receive` still
            // answers the frames its peer sent before closing. The close wakes
            // no one, so a parked stream sees it at the end of its tick.
            if ctx.body_stream().is_some_and(|emit| emit.is_closed()) {
                return Ok(Value::null());
            }
            // In hand before the wait is committed to, which is
            // `nvs_runtime::host::Host::waker`'s ordering rule. Nothing can
            // publish between the look above and the park below — a publisher
            // on this core runs only where this task yields, and this one does
            // not yield until `park` — so the registration cannot be late.
            let Some(waker) = nvs_runtime::host::with_current(|host| host.waker()).flatten() else {
                // No case can reach this: a script is never handed the door mark
                // a connection isolate carries, so a `.nvst` case is refused
                // above and never reaches the wait.
                // `a_wait_with_no_scheduler_under_it_is_a_fatal_rather_than_a_stalled_core`
                // is the `#[test]` that asserts it instead.
                return Err(Fault::fatal(
                    "`Core\\Sse::receive` would wait and there is no scheduler on this thread \
                     to wait on",
                ));
            };
            ctx.inbox().wake_on(waker);
            let until = std::time::Instant::now() + CROSS_CORE_TICK;
            if let Some(nvs_runtime::host::Woken::Cancelled) =
                nvs_runtime::host::with_current(|host| host.park(Some(until)))
            {
                return Err(ctx.cancel());
            }
        }
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

    /// The cell a connection offers, at both of its bounds: a send timeout no
    /// case below can reach, and room for every event one of them writes.
    /// `nvs_server::bounds` is where either number is the claim.
    fn offered_cell() -> nvs_runtime::stream::BodySlot {
        nvs_runtime::stream::BodySlot::new(std::time::Duration::from_secs(30), 1 << 20)
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
    // covers: Core\Sse::stream, Core\Sse::send
    #[test]
    fn an_event_stream_declares_its_head_and_its_events_reach_the_connections_half() {
        let slot = offered_cell();
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

    /// A reconnection time reaching the connection as a `retry:` block of its
    /// own, in whole milliseconds, and a negative one refused with nothing
    /// written — so the connection never frames half a block it cannot take
    /// back.
    // covers: Core\Sse::retry
    #[test]
    fn a_reconnection_time_reaches_the_connection_and_a_negative_one_writes_nothing() {
        let slot = offered_cell();
        let mut ctx = framing(&slot);

        let handle = nvs_runtime::call(super::nvs_core_sse_stream, &mut ctx, &[])
            .expect("an offered cell takes the stream");
        let backwards = crate::time::duration_of(-1);
        nvs_runtime::call(super::nvs_core_sse_retry, &mut ctx, &[handle, backwards])
            .expect_err("a wait shorter than none is refused");
        let message = ctx
            .pending()
            .expect("a throw carries the message it was raised with")
            .into_owned();
        assert!(
            message.contains("cannot be negative"),
            "the refusal does not say what was wrong: {message}"
        );
        drop(ctx.take_pending());
        let later = crate::time::duration_of(3_000_000_000);
        nvs_runtime::call(super::nvs_core_sse_retry, &mut ctx, &[handle, later])
            .expect("a reconnection time goes into an empty cell without parking");

        let mut head = slot
            .take(std::task::Waker::noop())
            .expect("the member filled the cell");
        let nvs_runtime::stream::Drained::Chunk(framed) =
            head.drain.next_chunk(std::task::Waker::noop())
        else {
            panic!("the reconnection time never reached the connection");
        };
        assert_eq!(
            framed, b"retry: 3000\n\n",
            "the refused call wrote something, or the block is not whole milliseconds"
        );

        dropped(later);
        dropped(backwards);
        dropped(handle);
    }

    /// The cell's refusal reaching a program: a response has one body, so an
    /// event stream over a response already writing one is refused and the
    /// first body is what the connection still frames. Unreachable from a
    /// `.nvst` case — the cell it needs is offered to no script.
    #[test]
    fn an_event_stream_is_refused_where_a_response_body_is_already_being_written() {
        let slot = offered_cell();
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
    // covers: Core\Sse::current
    #[test]
    fn current_outside_an_event_stream_is_refused_by_name() {
        let slot = offered_cell();
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

    /// A context shaped like the isolate `Core\Sse::upgrade` opens: the writing
    /// half of the response its connection is already framing, and the door
    /// marked as the one the stream outlives its request on.
    ///
    /// No cell and no peer, which is the whole of what door one is — the cell
    /// belongs to the request that opened the stream, and that request has
    /// ended by the time this isolate runs.
    fn upgraded(slot: &nvs_runtime::stream::BodySlot) -> Ctx {
        let mut ctx = Ctx::buffered();
        ctx.set_body_stream(
            slot.open(super::MEDIA_TYPE, None, Vec::new())
                .expect("an empty cell opens"),
        );
        ctx.mark_event_stream(nvs_runtime::EventStreamDoor::Connection);
        ctx
    }

    /// The whole of door one's far side in one case: a stream subscribes, a
    /// publisher on this core puts a value on that topic, and the wait answers
    /// it as a message carrying the topic it arrived on and a copy of what was
    /// published.
    ///
    /// It goes through `Core\Topic`'s own members rather than queueing on the
    /// context directly, because what is claimed is that the *bus* reaches a
    /// door it was written before: § 4's table holds a handle onto the queue,
    /// and a stream that subscribed is an entry in it like any connection.
    // covers: Core\Sse\Message::topic, Core\Sse\Message::value
    #[test]
    fn a_published_value_reaches_a_subscribed_event_stream_as_a_message() {
        let slot = offered_cell();
        let mut stream = upgraded(&slot);
        let handle = nvs_runtime::call(super::nvs_core_sse_current, &mut stream, &[])
            .expect("an event stream's isolate answers `current()`");

        let topic = Value::str(NvsStr::new(b"room:feed"));
        nvs_runtime::call(
            crate::topic::nvs_core_topic_subscribe,
            &mut stream,
            &[topic],
        )
        .expect("an event stream's isolate is a connection and may subscribe");

        let mut publisher = Ctx::buffered();
        let published = Value::str(NvsStr::new(b"the board changed"));
        let reached = nvs_runtime::call(
            crate::topic::nvs_core_topic_publish,
            &mut publisher,
            &[topic, published],
        )
        .expect("a publish answers how many it reached");
        assert_eq!(
            reached.as_uint(),
            Some(1),
            "the subscribed stream was not counted a subscriber"
        );

        let message = nvs_runtime::call(super::nvs_core_sse_receive, &mut stream, &[handle])
            .expect("a queued delivery is answered without waiting for anything");
        let name = nvs_runtime::call(super::nvs_core_sse_message_topic, &mut stream, &[message])
            .expect("a message names its topic");
        assert_eq!(name.as_text(), Some("room:feed"));
        let carried = nvs_runtime::call(super::nvs_core_sse_message_value, &mut stream, &[message])
            .expect("a message carries the value");
        assert_eq!(carried.as_text(), Some("the board changed"));

        for value in [topic, published, message, name, carried, handle] {
            dropped(value);
        }
    }

    /// Two subscriptions on one stream, each message naming the topic it
    /// arrived on in the order the values were published, and a value that is
    /// not a `string` arriving as the value it was rather than as text.
    // covers: Core\Sse\Message::topic, Core\Sse\Message::value
    #[test]
    fn each_message_names_its_own_topic_and_carries_a_value_that_is_not_text() {
        let slot = offered_cell();
        let mut stream = upgraded(&slot);
        let handle = nvs_runtime::call(super::nvs_core_sse_current, &mut stream, &[])
            .expect("an event stream's isolate answers `current()`");

        let prices = Value::str(NvsStr::new(b"prices"));
        let orders = Value::str(NvsStr::new(b"orders"));
        for topic in [prices, orders] {
            nvs_runtime::call(
                crate::topic::nvs_core_topic_subscribe,
                &mut stream,
                &[topic],
            )
            .expect("an event stream's isolate may subscribe");
        }

        let mut publisher = Ctx::buffered();
        let price = Value::int(1842);
        let order = Value::str(NvsStr::new(b"order 1042"));
        for (topic, value) in [(orders, order), (prices, price)] {
            nvs_runtime::call(
                crate::topic::nvs_core_topic_publish,
                &mut publisher,
                &[topic, value],
            )
            .expect("a publish answers how many it reached");
        }

        let mut received = Vec::new();
        for (want_topic, want_value) in [("orders", "order 1042"), ("prices", "")] {
            let message = nvs_runtime::call(super::nvs_core_sse_receive, &mut stream, &[handle])
                .expect("a queued delivery is answered without waiting for anything");
            let name =
                nvs_runtime::call(super::nvs_core_sse_message_topic, &mut stream, &[message])
                    .expect("a message names its topic");
            assert_eq!(
                name.as_text(),
                Some(want_topic),
                "a message named the wrong topic"
            );
            let carried =
                nvs_runtime::call(super::nvs_core_sse_message_value, &mut stream, &[message])
                    .expect("a message carries the value");
            if want_value.is_empty() {
                assert_eq!(
                    carried.as_int(),
                    Some(1842),
                    "an `int` did not arrive as one"
                );
            } else {
                assert_eq!(carried.as_text(), Some(want_value));
            }
            received.extend([message, name, carried]);
        }

        for value in received
            .into_iter()
            .chain([prices, orders, price, order, handle])
        {
            dropped(value);
        }
    }

    /// `rule:core-classes/topic`'s bound as the subscriber performs it: a stream
    /// that fell behind its topics is answered `null` and **its body ends**, so
    /// the publisher fanning out was never made to wait for it.
    ///
    /// The close is the whole of the claim that is this door's own. A socket
    /// carries a close code; an event stream has no frame to put one in, so the
    /// response body ending is what the client sees, and the case asserts it on
    /// the connection's half rather than on the context that performed it.
    #[test]
    fn an_overflowing_subscriber_queue_answers_null_and_closes_that_stream() {
        let slot = offered_cell();
        let mut stream = upgraded(&slot);
        let handle = nvs_runtime::call(super::nvs_core_sse_current, &mut stream, &[])
            .expect("an event stream's isolate answers `current()`");
        let mut head = slot
            .take(std::task::Waker::noop())
            .expect("the isolate was handed the body of a response");

        // One past the bound, which is what raises the overflow: the queue is
        // full at `INBOX_CAP`, and the refused value comes back to the
        // publisher rather than being dropped here.
        for n in 0..=nvs_runtime::INBOX_CAP {
            let queued = stream.deliver(nvs_runtime::Delivery::new(
                "room:flood",
                Value::int(i64::try_from(n).expect("a bound this small fits an `int`")),
            ));
            if let Some(refused) = queued {
                dropped(refused.into_value());
            }
        }

        let answered = nvs_runtime::call(super::nvs_core_sse_receive, &mut stream, &[handle])
            .expect("an overflowed queue is answered rather than thrown on");
        assert!(
            answered.tag() == Some(nvs_runtime::Tag::Null),
            "a stream that fell behind was handed a message instead of the end"
        );
        assert!(
            matches!(
                head.drain.next_chunk(std::task::Waker::noop()),
                nvs_runtime::stream::Drained::Ended
            ),
            "the overflow answered `null` and left the response body open"
        );

        dropped(handle);
    }

    /// The end of the stream is what ends a stream's `while` loop: once the
    /// client has gone, a value published before it went is still answered,
    /// and the wait after that is `null` rather than a wait for a value no
    /// client would read.
    ///
    /// The published value goes through `crate::topic::publish_text`, which is
    /// the publish `nvs run --events` makes, so this case pins that seam too.
    // covers: Core\Sse::receive
    #[test]
    fn a_stream_whose_client_has_gone_answers_what_was_queued_and_then_null() {
        let slot = offered_cell();
        let mut stream = upgraded(&slot);
        let handle = nvs_runtime::call(super::nvs_core_sse_current, &mut stream, &[])
            .expect("an event stream's isolate answers `current()`");
        let topic = Value::str(NvsStr::new(b"room:feed"));
        nvs_runtime::call(
            crate::topic::nvs_core_topic_subscribe,
            &mut stream,
            &[topic],
        )
        .expect("an event stream's isolate is a connection and may subscribe");
        assert_eq!(crate::topic::publish_text("room:feed", "last word"), 1);

        // The connection's half going away is the client disconnecting.
        drop(
            slot.take(std::task::Waker::noop())
                .expect("the isolate was handed the body of a response"),
        );

        let message = nvs_runtime::call(super::nvs_core_sse_receive, &mut stream, &[handle])
            .expect("a queued delivery is answered after the client has gone");
        let carried = nvs_runtime::call(super::nvs_core_sse_message_value, &mut stream, &[message])
            .expect("a message carries the value");
        assert_eq!(carried.as_text(), Some("last word"));
        let over = nvs_runtime::call(super::nvs_core_sse_receive, &mut stream, &[handle])
            .expect("the end of a stream is answered rather than thrown on");
        assert!(
            over.tag() == Some(nvs_runtime::Tag::Null),
            "a stream whose client has gone went on waiting for a value"
        );

        for value in [topic, message, carried, handle] {
            dropped(value);
        }
    }

    /// A wait with no scheduler under it is a fatal error rather than a core
    /// standing still — `rule:http-server/a-core-is-never-blocked-on-a-syscall`
    /// read at the one place this member could have broken it.
    ///
    /// The member learns there is no task *before* it commits to waiting, which
    /// is why `Host::waker` is a question of its own: with nothing to wake, a
    /// park could only block the thread every other request on this core is
    /// running on. Unreachable from a `.nvst` case, which is never handed the
    /// door mark that gets this far.
    #[test]
    fn a_wait_with_no_scheduler_under_it_is_a_fatal_rather_than_a_stalled_core() {
        let slot = offered_cell();
        let mut stream = upgraded(&slot);
        let handle = nvs_runtime::call(super::nvs_core_sse_current, &mut stream, &[])
            .expect("an event stream's isolate answers `current()`");

        let status = nvs_runtime::call(super::nvs_core_sse_receive, &mut stream, &[handle])
            .expect_err("an empty queue with no scheduler beneath it has nowhere to wait");
        assert_eq!(
            status,
            nvs_runtime::FATAL,
            "a wait that cannot be entered was reported as something a `catch` sees"
        );
        drop(stream.take_pending());

        dropped(handle);
    }

    /// The refusal door two gets, named after the member it called: a request
    /// answering with its own events holds this handle and cannot wait on it.
    ///
    /// `null` would have been the silently wrong answer — a loop written
    /// `while (var $msg = $sse->receive())` would end at once and read as a
    /// stream that was over — so the refusal is what tells a program it opened
    /// the door that does not wait.
    #[test]
    fn receive_in_a_streaming_response_is_refused_by_name() {
        let slot = offered_cell();
        let mut ctx = framing(&slot);
        let handle = nvs_runtime::call(super::nvs_core_sse_stream, &mut ctx, &[])
            .expect("an offered cell takes the stream");

        let status = nvs_runtime::call(super::nvs_core_sse_receive, &mut ctx, &[handle])
            .expect_err("a streaming response has nothing to wait for");
        assert_eq!(
            status,
            nvs_runtime::THROWN,
            "a door that cannot wait is a throw a program catches, not a fatal"
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
            message.contains(r"Core\Sse::receive"),
            "the refusal does not name the member that was called: {message}"
        );
        drop(ctx.take_pending());

        dropped(handle);
    }

    /// An event whose reader has gone is refused rather than parked — the
    /// connection dropping its half closes the stream at once, so a program
    /// learns it on the next event instead of at the send timeout.
    ///
    /// Unreachable from a `.nvst` case for the reason above: a stream that can
    /// close is one a connection is draining.
    #[test]
    fn an_event_whose_reader_has_gone_is_refused_rather_than_parked() {
        let slot = offered_cell();
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
