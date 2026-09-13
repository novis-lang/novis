//! `Core\Http\Socket` — [ADR 0183](/docs/decisions/0183.md)'s outbound
//! WebSocket: a conversation the program opened through the door every outbound
//! call passes, and holds until its own task ends.
//!
//! # Decision: a row on `Core\Http\Client`, and no door of its own
//!
//! The opening handshake **is** an outbound call, so what opens a socket is
//! [`super::CLIENT`]'s `openSocket` row rather than a static on this class
//! (`rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call`).
//! The pin, the `net.connect` grant, the TLS policy and its grants, a client
//! identity and the operator's proxy apply by the row being on that class
//! instead of by being told to, and a second door would be the copy that comes
//! to be missing a check a year from now. The only thing this row asks that the
//! others do not is which schemes it speaks: [`super::Roster::Socket`] admits
//! `ws` and `wss` and refuses the two every other row takes, so what a URL is
//! *for* is written in the row that takes it.
//!
//! # Decision: the peer's frames are `Core\Socket\Message`, not a class here
//!
//! One RFC 6455 frame gets one shape in this language, and
//! [`crate::socket::MESSAGE`] is already it: `text` or `bytes` filled and
//! `tainted`, `topic` and `value` `null`, which is exactly what a client sees.
//! A second message class for the outbound direction would differ only in where
//! the octets came from, and the two would disagree the first time either was
//! touched.
//!
//! # What a socket holds
//!
//! Its URL, the subprotocol the handshake chose, how far through a scripted
//! peer's frames it has read, whether it has been closed, and the key its live
//! connection is filed under in the request's own table — all of them values
//! Novis can hold, which is [`crate::instance`]'s requirement of every `Core`
//! class. A socket a test answered holds `null` there, having no connection at
//! all: its frames are the scripted peer's and live in
//! [`nvs_runtime::AnswerTable`] beside the answers an outbound call is served
//! from, so two sockets opened to one URL each read the same script from their
//! own cursor.
//!
//! **The key is which conversation a member speaks to.** A socket holding one
//! reads and writes [`Open`]'s framed connection, and the request gives that
//! connection back when it ends; a socket holding `null` reads the scripted
//! peer and records what it sent where the test reads it back. Both halves
//! answer the same shape, so a program written against a table is the program
//! that runs against a host.
//!
//! **What it spends:** five slots per open socket, and against a real host what
//! [ADR 0183](/docs/decisions/0183.md) § 10 prices — one connection, a TLS
//! session for `wss`, and `tungstenite`'s own buffers — charged to the task
//! that opened it.

use super::*;

/// `Core\Http\Client::openSocket`, as its refusals spell it.
const MEMBER: &str = r"Core\Http\Client::openSocket";

/// The outbound socket, as [`CoreTy::Instance`] spells it. Under `Core\Http`
/// because it is what a row on `Core\Http\Client` answers with, as
/// `Core\Http\Response` and `Core\Http\Stream` are.
pub(crate) const SOCKET_NAME: &str = r"Core\Http\Socket";

/// The symbol [`super::CLIENT`]'s `openSocket` row is reached through.
pub(crate) const OPEN_SYMBOL: &str = "nvs_core_http_client_open_socket";

/// The symbols [`SOCKET`]'s five rows are reached through.
const RECEIVE_SYMBOL: &str = "nvs_core_http_socket_receive";
const SEND_SYMBOL: &str = "nvs_core_http_socket_send";
const SEND_BYTES_SYMBOL: &str = "nvs_core_http_socket_send_bytes";
const CLOSE_SYMBOL: &str = "nvs_core_http_socket_close";
const PROTOCOL_SYMBOL: &str = "nvs_core_http_socket_protocol";

/// [`SOCKET`]'s URL slot, by index — the layout its `slots` names. The URL the
/// socket was opened to, which is what finds the peer answering it.
const URL_AT: usize = 0;
/// The subprotocol the handshake chose, or `null` where it chose none.
const PROTOCOL_AT: usize = 1;
/// How many of the peer's frames this socket has taken.
const AT_AT: usize = 2;
/// Whether `close` has ended this socket.
const CLOSED_AT: usize = 3;
/// The key [`Open`] is filed under in the request's table, or `null` for a
/// socket a test answered, which holds no connection.
const HELD_AT: usize = 4;

/// The live conversation a real handshake left, as the request's own table
/// holds it.
///
/// A `Core` class's slots hold values Novis can hold, so what the socket
/// carries is a key and what the key names is this
/// ([`nvs_runtime::Ctx::hold_open_socket`] argues why a handle is a number).
/// The table is the request's, so the connection is closed when the task that
/// opened it ends whatever the program did with the socket — which is § 5's
/// lifetime with nothing here to remember it.
///
/// **The bounds are held beside the connection because every wait is one of
/// them.** The handshake left the stream bound by the deadline that covered the
/// opening call, and that instant says nothing about how long a conversation
/// may wait for its next frame — so each member arms the wait it is about to
/// take (`rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`).
/// The message cap is not here: it is the framing's, applied by the codec as it
/// reassembles.
pub(crate) struct Open {
    /// The framed conversation the handshake left.
    framed: transport::Upgraded,
    /// The longest silence this conversation may hold — `idle`.
    idle: Duration,
    /// When `maxDuration` runs out, measured from where the opening call began:
    /// a socket's whole life includes the handshake that opened it.
    until: Instant,
    /// How long one frame may wait to be written — the send wait.
    send: Duration,
}

impl std::fmt::Debug for Open {
    /// The table's own `Debug`, which nothing but a panic message reads. Written
    /// by hand because the framed connection under it is a trait object and has
    /// none.
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str("an open outbound WebSocket")
    }
}

impl nvs_runtime::HeldSocket for Open {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// `rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call`'s
/// conversation: what a program does with a socket once it has one.
///
/// # Two send members, because two payload kinds and one qualifier question
///
/// `send` and `sendBytes` are [`crate::socket::CLASS`]'s shapes and are here
/// for its reason, which is the qualifier rather than symmetry:
/// [`CoreTy::classification`] answers `None` for a [`CoreTy::Union`], and a
/// text-like parameter carrying no classification refuses a `tainted`
/// argument — so a parameter spelled `string|bytes` would have made `send` a
/// sink by accident, and the loop that forwards what the peer just sent would
/// not compile. Two classified parameters say what a union cannot.
///
/// # `receive` answers `null` once, and that is the peer's close
///
/// A socket that has run out of the peer's messages answers `null` rather than
/// waiting for one that is not coming, which is the same reading a server-side
/// connection's `receive` has. `close` makes every later `receive` answer
/// `null` too, because a program that closed the socket asking for another
/// message is asking a question with one honest answer.
pub(crate) const SOCKET: CoreClass = CoreClass {
    name: SOCKET_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "receive",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(crate::socket::MESSAGE_NAME)),
            symbol: RECEIVE_SYMBOL,
            doc: Some(&RECEIVE_DOC),
        },
        CoreMethod {
            name: "send",
            // [`Qual::Neutral`] because a frame is not an instruction on this
            // side of the wire — `rule:security/sink-predicate`'s test — and
            // because `send` answers `void`, so there is no result for the
            // argument's qualifier to reach. A program may forward what it
            // received without laundering it.
            names: &["frame"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: SEND_SYMBOL,
            doc: Some(&SEND_DOC),
        },
        CoreMethod {
            name: "sendBytes",
            names: &["frame"],
            params: &[CoreTy::Blob(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: SEND_BYTES_SYMBOL,
            doc: Some(&SEND_BYTES_DOC),
        },
        CoreMethod {
            name: "close",
            names: &["code", "reason"],
            // The reason is [`Qual::Neutral`] for `send`'s reason: it is a line
            // the peer may log and never an instruction on this side of the
            // wire, so a program may close with text it was handed.
            params: &[
                CoreTy::Nullable(&CoreTy::Uint),
                CoreTy::Nullable(&CoreTy::Text(Qual::Neutral)),
            ],
            defaults: &[Const::Null, Const::Null],
            return_ty: CoreTy::Void,
            symbol: CLOSE_SYMBOL,
            doc: Some(&CLOSE_DOC),
        },
        CoreMethod {
            name: "protocol",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: PROTOCOL_SYMBOL,
            doc: Some(&PROTOCOL_DOC),
        },
    ],
    slots: &["url", "protocol", "at", "closed", "held"],
    constants: &[],
};

/// `Core\Http\Socket::receive`'s reference card — `rule:core-api/reference-card`.
const RECEIVE_DOC: MethodDoc = MethodDoc {
    short: "Waits for the peer's next message and answers it, or answers `null` once the peer \
            has closed.",
    params: &[],
    ret: "A `Core\\Socket\\Message` whose `text()` or `bytes()` carries the payload — `tainted`, \
          because it came off a wire — and whose `topic()` and `value()` are `null`, since those \
          are what a delivery from another isolate fills. `null` means the conversation is over: \
          the peer closed, or this end did.",
    errors: &[
        ErrorDoc {
            error: "TimeoutError",
            desc: "The peer sent nothing for longer than `idle`, or the socket's `maxDuration` ran \
                   out while this call was waiting. A peer whose only traffic is pings is not \
                   silent and is bounded by `maxDuration` instead.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed while this call was waiting for a message.",
        },
    ],
};

/// `Core\Http\Socket::send`'s reference card — `rule:core-api/reference-card`.
const SEND_DOC: MethodDoc = MethodDoc {
    short: "Sends one text message to the peer.",
    params: &[ParamDoc {
        name: "frame",
        desc: "The text to send, as one RFC 6455 text message however many frames it takes on \
               the wire. A `tainted` value is accepted: what a program sends over a socket it \
               opened is not an instruction on this side of the wire.",
        shape: &[],
    }],
    ret: "Nothing.",
    errors: SEND_ERRORS,
};

/// `Core\Http\Socket::sendBytes`'s reference card — `rule:core-api/reference-card`.
const SEND_BYTES_DOC: MethodDoc = MethodDoc {
    short: "Sends one binary message to the peer — the other of RFC 6455's two payload kinds, \
            and a member of its own rather than an argument that could be either.",
    params: &[ParamDoc {
        name: "frame",
        desc: "The octets to send, as one binary message.",
        shape: &[],
    }],
    ret: "Nothing.",
    errors: SEND_ERRORS,
};

/// What either send member throws, written once because the two differ in their
/// payload kind and in nothing else — a second list is the copy that comes to
/// disagree.
const SEND_ERRORS: &[ErrorDoc] = &[
    ErrorDoc {
        error: "LogicError",
        desc: "The socket has been closed, so there is nobody left to send to.",
    },
    ErrorDoc {
        error: "TimeoutError",
        desc: "The frame was still waiting to be written when the send wait ran out — a peer that \
               has stopped reading — or the socket's `maxDuration` ran out first.",
    },
    ErrorDoc {
        error: "IOError",
        desc: "The connection failed while the frame was going out.",
    },
];

/// `Core\Http\Socket::close`'s reference card — `rule:core-api/reference-card`.
const CLOSE_DOC: MethodDoc = MethodDoc {
    short: "Ends the conversation: sends the close frame, waits for the peer's under the send \
            wait, and lets the connection go.",
    params: &[
        ParamDoc {
            name: "code",
            desc: "The close code to send. Left out, it is `1000` — a normal ending.",
            shape: &[],
        },
        ParamDoc {
            name: "reason",
            desc: "The text to send beside the code, for a peer that logs it. Left out, none is \
                   sent.",
            shape: &[],
        },
    ],
    ret: "Nothing. A peer that never answers its own close is closed anyway and that is not an \
          error — the connection is gone either way, and a throw would put a `catch` around every \
          normal ending. Closing a socket that is already closed does nothing.",
    errors: &[],
};

/// `Core\Http\Socket::protocol`'s reference card — `rule:core-api/reference-card`.
const PROTOCOL_DOC: MethodDoc = MethodDoc {
    short: "The subprotocol the opening handshake settled on, out of the ones `protocols` \
            offered.",
    params: &[],
    ret: "The name the peer chose, or `null` where the call offered none or the peer chose none. \
          A peer that chooses a name which was never offered does not get this far: the handshake \
          is refused instead.",
    errors: &[],
};

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::openSocket(string|Core\Http\Target $url, Core\Http\Options): Core\Http\Socket`
    /// — `rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call`.
    ///
    /// **Everything decided from the arguments is decided here**, and in the
    /// order a request decides it: the scheme roster, the demand for a host,
    /// and every bound the bag carries. A socket answered from a test's table
    /// keeps each of those refusals, so a wrong URL fails the same way scripted
    /// and real — which is [`super::faked`]'s rule one door over.
    ///
    /// **The connect is the rule and the table is the exception**, which is the
    /// same branch every request row takes one door over: a context no test has
    /// armed opens a connection through [`super::transport`], and one that has
    /// been armed reaches no network at all.
    ///
    /// **The subprotocol is judged against what was offered**, and both arms
    /// pass through [`transport::settled`] to do it — the live one inside the
    /// handshake, the scripted one here — so a peer a test wrote cannot say
    /// what a real one would have been refused for.
    fn nvs_core_http_client_open_socket(ctx, args: [SOCKET_ARITY]) {
        let url = super::given_url(args, MEMBER)?;
        super::judged_host(&url, MEMBER, Roster::Socket)?;
        judge_bound(args, SOCKET_DEADLINE, "deadline", MEMBER)?;
        judge_bound(args, SOCKET_CONNECT_TIMEOUT, "connectTimeout", MEMBER)?;
        judge_bound(args, SOCKET_IDLE, "idle", MEMBER)?;
        judge_bound(args, SOCKET_MAX_DURATION, "maxDuration", MEMBER)?;
        judge_bound(args, SOCKET_SEND_TIMEOUT, "sendTimeout", MEMBER)?;
        judge_bound(args, SOCKET_PING, "ping", MEMBER)?;

        let (chosen, held) = if ctx.faked_http().is_armed() {
            (
                transport::settled(scripted(ctx, &url)?, &offers(args), MEMBER)?,
                Value::null(),
            )
        } else {
            connected(ctx, args)?
        };

        let protocol = match &chosen {
            Some(name) => Value::str(NvsStr::new(name.as_bytes())),
            None => Value::null(),
        };
        Ok(crate::instance::build(
            &SOCKET,
            [
                Value::str(NvsStr::new(url.as_bytes())),
                protocol,
                Value::int(0),
                Value::bool(false),
                held,
            ],
        ))
    }
}

/// The subprotocol the peer scripted for `url` chose, for a context a test has
/// armed.
///
/// # Errors
///
/// A `LogicError` naming the URL no row answers. It names a test because only a
/// test can reach it: nothing arms the table but `Core\Test`, and the branch
/// above is what keeps this refusal off a program's path
/// (`rule:testing/an-outbound-socket-is-answered-by-a-scripted-peer`).
fn scripted(ctx: &Ctx, url: &str) -> Result<Option<String>, Fault> {
    let Some(peer) = ctx.faked_http().socket_for(url) else {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{MEMBER}: this test answers outbound sockets from a table and no peer is \
                 registered for {url} — `Core\\Test::answerSocket` registers one, exactly or \
                 as a prefix ending in `*`"
            ),
        ));
    };
    Ok(peer.protocol.clone())
}

/// The handshake against the host the URL names: the subprotocol it settled on,
/// and the key the live connection is filed under.
///
/// **The call is assembled here and made in [`super::transport`]**, which is
/// the split every request row already takes: the questions that need a `Ctx` —
/// the pin, the grants, the deployment's bounds, the operator's proxy — are
/// this crate's, and what crosses into that module is their answers. The
/// exchange half of a [`transport::Call`] is what an upgrade is: a `GET` with
/// no body, one attempt, and no hop to follow.
///
/// # Errors
///
/// [`super::approved`]'s refusals, [`super::judge_trust`]'s, and whatever
/// [`transport::upgrade`] raised — which includes the `3xx` a socket cannot
/// follow.
fn connected(ctx: &mut Ctx, args: &[Value]) -> Result<(Option<String>, Value), Fault> {
    // The resolve time the event reports is this call's own, and a call handed
    // an already-pinned `Core\Http\Target` spent none of it, exactly as at a
    // request row.
    let pinned_already = matches!(args[0].tag(), Some(Tag::Object));
    let began = Instant::now();
    let (url, addresses) = super::approved(ctx, args, MEMBER, super::SOCKET_BAG)?;
    let resolve = if pinned_already {
        Duration::ZERO
    } else {
        began.elapsed()
    };
    super::judge_trust(ctx, args, &url, MEMBER, super::SOCKET_BAG)?;

    let call = transport::Call {
        member: MEMBER,
        verb: "GET",
        url,
        addresses,
        deadline: Instant::now()
            + super::bound_of(
                ctx,
                args,
                SOCKET_DEADLINE,
                "deadline",
                "http.client.deadline",
                super::DEFAULT_DEADLINE,
            )?,
        connect_timeout: super::bound_of(
            ctx,
            args,
            SOCKET_CONNECT_TIMEOUT,
            "connectTimeout",
            "http.client.connect_timeout",
            super::DEFAULT_CONNECT_TIMEOUT,
        )?,
        idle: super::bound_of(
            ctx,
            args,
            SOCKET_IDLE,
            "idle",
            "http.client.idle",
            super::DEFAULT_IDLE,
        )?,
        max_duration: super::bound_of(
            ctx,
            args,
            SOCKET_MAX_DURATION,
            "maxDuration",
            "http.client.max_duration",
            super::DEFAULT_MAX_DURATION,
        )?,
        headers: super::headers_of(args, SOCKET_HEADERS, MEMBER)?,
        // The exchange half, as an upgrade spells it: no hop, one attempt, and
        // therefore no backoff and no key for a retry to carry.
        redirects: 0,
        attempts: 1,
        backoff: Duration::ZERO,
        idempotency_key: None,
        body: None,
        // Two ceilings nothing on this path reads: a socket is never pooled
        // (ADR 0183 § 4) and nothing here arrives under a content coding. They
        // are the call's fields, so they are answered rather than left out.
        pool: super::pool_of(ctx),
        compress: crate::compress::Bound::ceiling(ctx),
        identity: super::identity_option(args, MEMBER, super::SOCKET_BAG)?,
        policy: super::policy_of(args, MEMBER, super::SOCKET_BAG)?,
        traceparent: super::traceparent_of(ctx),
        span: std::cell::RefCell::new(super::span::HttpSpan::opened("GET", resolve)),
        proxy: super::proxy_of(ctx),
    };

    // The framing carries § 7's message cap rather than `tungstenite`'s own
    // 64 MiB: how much of the opening task's memory a peer may make this
    // process hold is the deployment's decision and then the program's, and a
    // library default is neither. Every step of it is finite, so nothing here
    // is the unbounded spelling
    // `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` refuses.
    // The send wait this call judged is not here: it bounds a write rather than
    // the frames, so it belongs to the members that write.
    let cap = super::cap_of(
        ctx,
        args,
        SOCKET_MAX_MESSAGE,
        "maxMessage",
        "http.client.socket.max_message",
        super::DEFAULT_MAX_MESSAGE,
    )?;
    let framing = tungstenite::protocol::WebSocketConfig::default()
        .max_message_size(Some(usize::try_from(cap).unwrap_or(usize::MAX)));
    let upgraded = transport::upgrade(&call, &offers(args), framing)?;
    super::traced(ctx, &call);
    let protocol = upgraded.protocol.clone();
    let open = Open {
        framed: upgraded,
        // The two bounds the handshake already read, now covering what it
        // opened: one call's `idle` and `maxDuration` are the conversation's,
        // which is why the bag carries one of each rather than a pair per side.
        idle: call.idle,
        until: began + call.max_duration,
        send: super::bound_of(
            ctx,
            args,
            SOCKET_SEND_TIMEOUT,
            "sendTimeout",
            "http.client.socket.send_timeout",
            super::DEFAULT_SEND_TIMEOUT,
        )?,
    };
    Ok((protocol, Value::uint(ctx.hold_open_socket(Box::new(open)))))
}

/// The subprotocols this call offered, in the order the array wrote them.
///
/// Anything in it that is not text is skipped rather than refused: the key's
/// own type is `array<string>`, so `E0401` has already refused a call that
/// wrote anything else, and a check here would be a refusal no program can
/// reach.
fn offers(args: &[Value]) -> Vec<String> {
    let Some(array) = args[SOCKET_PROTOCOLS].array_ptr() else {
        return Vec::new();
    };
    let array = crate::arr::borrowed(array);
    let mut names = Vec::new();
    let mut from = 0_usize;
    while let Some(slot) = array.next_slot(from) {
        from = slot + 1;
        let offer = array
            .value_at(slot)
            .expect("next_slot only names live entries");
        if let Some(name) = offer.as_text() {
            names.push(name.to_owned());
        }
    }
    names
}

/// One text message, as the shape both halves of this module answer in.
///
/// `topic` and `value` are `null` because they are what a delivery from another
/// isolate fills, and a frame off a wire is not one.
fn text_message(text: &str) -> Value {
    crate::instance::build(
        &crate::socket::MESSAGE,
        [
            Value::null(),
            Value::str(NvsStr::new(text.as_bytes())),
            Value::null(),
            Value::null(),
        ],
    )
}

/// One binary message — [`text_message`] for RFC 6455's other payload kind.
fn bytes_message(octets: &[u8]) -> Value {
    crate::instance::build(
        &crate::socket::MESSAGE,
        [
            Value::null(),
            Value::null(),
            Value::bytes(NvsStr::new(octets)),
            Value::null(),
        ],
    )
}

/// The scripted peer's next frame, as a `Core\Socket\Message`, or `None` where
/// the script is over.
fn taken(ctx: &Ctx, url: &str, at: usize) -> Option<Value> {
    let frame = ctx.faked_http().socket_for(url)?.frames.get(at)?;
    Some(match frame {
        nvs_runtime::SocketFrame::Text(text) => text_message(text),
        nvs_runtime::SocketFrame::Bytes(octets) => bytes_message(octets),
    })
}

/// The receiver of a member that writes to the peer, once it is known there is
/// still a peer to write to.
///
/// # Errors
///
/// A `LogicError` naming the member, for a socket `close` has ended. A
/// [`Fault::fatal`] for a receiver of another class, which `E0401` refuses a
/// phase earlier.
fn writable(value: Value, member: &str) -> Result<*mut nvs_runtime::ObjHeader, Fault> {
    let receiver = crate::instance::receiver(value, &SOCKET, member)?;
    if crate::instance::slot(receiver, CLOSED_AT).as_bool() == Some(true) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{SOCKET_NAME}::{member}(): this socket was closed, so there is nobody left to \
                 send to"
            ),
        ));
    }
    Ok(receiver)
}

/// The live conversation filed under `key`, or `None` where the request's table
/// no longer holds one.
///
/// A key that names nothing is a connection the task gave back, which is the
/// conversation being over rather than a fault: the only writers of that table
/// are the row that opened this socket and the task that ends it.
fn open_at(ctx: &mut Ctx, key: u64) -> Option<&mut Open> {
    ctx.open_socket_mut(key)?
        .as_any_mut()
        .downcast_mut::<Open>()
}

/// The peer's next message over the live conversation, or `null` where the
/// conversation is over.
///
/// **A control frame is answered rather than handed to the program.** A ping is
/// the protocol asking whether this end is alive and the codec queues the pong
/// itself, a pong is the answer to a ping this end sent, and neither is a
/// message anyone wrote — so the loop goes back round and waits for one that
/// is. The peer's close is the end, and `null` is how that reads.
///
/// # Errors
///
/// A `TimeoutError` for a silence past `idle` or a conversation past its
/// `maxDuration`, and whatever else the connection failed with.
fn heard(open: &mut Open) -> Result<Value, Fault> {
    loop {
        let now = Instant::now();
        if now >= open.until {
            return Err(outlived("receive"));
        }
        // Both bounds on the one wait, which is [`transport`]'s streamed reader
        // one protocol up: whichever is nearer is what the socket parks under,
        // and which of them fired is read back off the clock.
        let (idle, until) = (open.idle, open.until);
        open.framed
            .socket
            .get_mut()
            .bound_by(Some(until.min(now + idle)));
        return match open.framed.socket.read() {
            Ok(tungstenite::Message::Text(text)) => Ok(text_message(text.as_str())),
            Ok(tungstenite::Message::Binary(octets)) => Ok(bytes_message(&octets)),
            Ok(tungstenite::Message::Ping(_) | tungstenite::Message::Pong(_)) => continue,
            // `Frame` is a raw frame the codec only produces for a caller that
            // asked for one, which this is not.
            Ok(tungstenite::Message::Close(_) | tungstenite::Message::Frame(_)) => {
                Ok(Value::null())
            }
            Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                Ok(Value::null())
            }
            Err(why) => Err(failed(why, idle, until, "receive")),
        };
    }
}

/// One message out over the live conversation, under the send wait.
///
/// # Errors
///
/// A `TimeoutError` for a frame that waited longer than the send wait to be
/// written, or a socket already past its `maxDuration`, and whatever else the
/// connection failed with. A peer that has stopped reading is the one failure a
/// program must hear about: a frame that went out and a frame that did not are
/// otherwise the same call.
fn written(open: &mut Open, message: tungstenite::Message, member: &str) -> Result<(), Fault> {
    let now = Instant::now();
    if now >= open.until {
        return Err(outlived(member));
    }
    let (idle, until) = (open.idle, open.until);
    open.framed
        .socket
        .get_mut()
        .bound_by(Some(until.min(now + open.send)));
    match open.framed.socket.send(message) {
        Ok(()) => Ok(()),
        Err(why) => Err(failed(why, idle, until, member)),
    }
}

/// A live conversation that failed, as the class the failure belongs to.
///
/// An expired wait arrives as an ordinary timed-out read, so which bound fired
/// is read off the clock the same way [`transport`]'s streamed body reads it.
fn failed(why: tungstenite::Error, idle: Duration, until: Instant, member: &str) -> Fault {
    if let tungstenite::Error::Io(error) = &why
        && error.kind() == std::io::ErrorKind::TimedOut
    {
        return if Instant::now() >= until {
            outlived(member)
        } else {
            silent(member, idle)
        };
    }
    Fault::thrown_as(
        if matches!(why, tungstenite::Error::Io(_)) {
            ThrownClass::Io
        } else {
            ThrownClass::Runtime
        },
        format!("{SOCKET_NAME}::{member}(): the conversation failed — {why}"),
    )
}

/// A conversation still going past its `maxDuration`.
fn outlived(member: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Timeout,
        format!(
            "{SOCKET_NAME}::{member}(): this socket's `maxDuration` ran out, which bounds the \
             whole conversation however much of it was left"
        ),
    )
}

/// A peer that said nothing for longer than `idle`.
fn silent(member: &str, idle: Duration) -> Fault {
    Fault::thrown_as(
        ThrownClass::Timeout,
        format!(
            "{SOCKET_NAME}::{member}(): the peer sent nothing for {idle:?}, which is as long a \
             silence as this socket's `idle` allows"
        ),
    )
}

/// A socket whose connection the task it belonged to has already given back.
fn gone(member: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "{SOCKET_NAME}::{member}(): the connection this socket held has been given back, so \
             there is nobody left to send to"
        ),
    )
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Socket::receive(): ?Core\Socket\Message` — the peer's next
    /// message, and `null` once the conversation is over.
    ///
    /// A socket holding a connection waits on it; one holding `null` reads the
    /// scripted peer, where the cursor moves before the message is built, so a
    /// program that takes a message and then takes another gets the next one
    /// whatever it did with the first.
    fn nvs_core_http_socket_receive(ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &SOCKET, "receive")?;
        if crate::instance::slot(receiver, CLOSED_AT).as_bool() == Some(true) {
            return Ok(Value::null());
        }
        if let Some(key) = crate::instance::slot(receiver, HELD_AT).as_uint() {
            let Some(open) = open_at(ctx, key) else {
                return Ok(Value::null());
            };
            return heard(open);
        }
        let url = crate::instance::slot(receiver, URL_AT);
        let url = url.as_text().ok_or_else(|| {
            // Unreachable from source: this crate is the only writer of the
            // slot, and the row that builds a socket writes the URL it opened.
            Fault::fatal(format!(
                "{SOCKET_NAME}::receive expected a `string` in its `url` slot, got tag {}",
                url.tag_byte()
            ))
        })?;
        let at = crate::instance::slot(receiver, AT_AT).as_int().unwrap_or_default();
        let Some(message) = taken(ctx, url, usize::try_from(at).unwrap_or_default()) else {
            return Ok(Value::null());
        };
        crate::instance::set_slot(receiver, AT_AT, Value::int(at + 1));
        Ok(message)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Socket::send(string $frame): void` — one text message to the
    /// peer.
    fn nvs_core_http_socket_send(ctx, args: [2]) {
        let receiver = writable(args[0], "send")?;
        let text = args[1].as_text().ok_or_else(|| {
            // Unreachable from source: the row's parameter is `CoreTy::Text`,
            // so `E0401` refuses anything else a phase earlier.
            Fault::fatal(format!(
                "{SOCKET_NAME}::send expected a `string` frame, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        if let Some(key) = crate::instance::slot(receiver, HELD_AT).as_uint() {
            let open = open_at(ctx, key).ok_or_else(|| gone("send"))?;
            written(open, tungstenite::Message::Text(text.into()), "send")?;
            return Ok(Value::null());
        }
        ctx.faked_http_mut()
            .record_frame(nvs_runtime::SocketFrame::Text(text.to_owned()));
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Socket::sendBytes(bytes $frame): void` — one binary message
    /// to the peer.
    fn nvs_core_http_socket_send_bytes(ctx, args: [2]) {
        let receiver = writable(args[0], "sendBytes")?;
        let octets = args[1].as_bytes().ok_or_else(|| {
            // Unreachable from source, for `send`'s reason one type over.
            Fault::fatal(format!(
                "{SOCKET_NAME}::sendBytes expected a `bytes` frame, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        if let Some(key) = crate::instance::slot(receiver, HELD_AT).as_uint() {
            let open = open_at(ctx, key).ok_or_else(|| gone("sendBytes"))?;
            written(
                open,
                tungstenite::Message::Binary(octets.to_vec().into()),
                "sendBytes",
            )?;
            return Ok(Value::null());
        }
        ctx.faked_http_mut()
            .record_frame(nvs_runtime::SocketFrame::Bytes(octets.to_vec()));
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Socket::close(?uint $code, ?string $reason): void` — the end
    /// of the conversation.
    ///
    /// Closing a socket that is already closed does nothing, rather than
    /// throwing: a program ending a conversation it has already ended has made
    /// no mistake a refusal could tell it about.
    ///
    /// A socket that holds a connection gives it back here rather than at the
    /// end of the task: the close frame goes out and the request's table drops
    /// its entry, which is the connection released. Neither write is an error a
    /// program hears about — a peer that never answers its own close is closed
    /// anyway, and the connection is gone either way.
    fn nvs_core_http_socket_close(ctx, args: [3]) {
        let receiver = crate::instance::receiver(args[0], &SOCKET, "close")?;
        crate::instance::set_slot(receiver, CLOSED_AT, Value::bool(true));
        if let Some(key) = crate::instance::slot(receiver, HELD_AT).as_uint()
            && let Some(mut held) = ctx.take_open_socket(key)
            && let Some(open) = held.as_any_mut().downcast_mut::<Open>()
        {
            drop(open.framed.socket.close(None));
            drop(open.framed.socket.flush());
        }
        crate::instance::set_slot(receiver, HELD_AT, Value::null());
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Socket::protocol(): ?string` — the subprotocol the handshake
    /// settled on.
    fn nvs_core_http_socket_protocol(_ctx, args: [1]) {
        crate::instance::read_slot(args, &SOCKET, PROTOCOL_AT, "protocol")
    }
}

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        OPEN_SYMBOL => (nvs_core_http_client_open_socket as *const ()).cast(),
        RECEIVE_SYMBOL => (nvs_core_http_socket_receive as *const ()).cast(),
        SEND_SYMBOL => (nvs_core_http_socket_send as *const ()).cast(),
        SEND_BYTES_SYMBOL => (nvs_core_http_socket_send_bytes as *const ()).cast(),
        CLOSE_SYMBOL => (nvs_core_http_socket_close as *const ()).cast(),
        PROTOCOL_SYMBOL => (nvs_core_http_socket_protocol as *const ()).cast(),
        _ => return None,
    })
}
