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
use tungstenite::protocol::CloseFrame;
use tungstenite::protocol::frame::coding::CloseCode;

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
    /// How long a silence runs before this end asks whether the peer is still
    /// there, or `None` where the call asked for no ping — which is the shipped
    /// answer, because a ping is traffic the peer did not ask for.
    ping: Option<Duration>,
    /// Whether a `receive` is already waiting on this conversation — the mark
    /// [`Reading`] sets and clears, shared so that the guard can clear it
    /// without holding the socket.
    reading: std::rc::Rc<std::cell::Cell<bool>>,
}

/// The mark a `receive` holds while it is waiting on a conversation.
///
/// Two `receive`s waiting at once on one socket is a `LogicError`
/// (`rule:http-server/an-outbound-socket-belongs-to-the-task-that-opened-it`),
/// and nothing but a mark can answer that question: the wait is a park, so the
/// second call runs *while* the first is still inside its own and finds a
/// socket that looks idle. Clearing on drop is what keeps the refusal narrow —
/// a first wait that ended in a `TimeoutError` leaves a socket a program may
/// sensibly receive on again, and every way out of that wait passes through
/// here.
struct Reading(std::rc::Rc<std::cell::Cell<bool>>);

impl Drop for Reading {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl Open {
    /// Marks this conversation as being read, or `None` where a `receive` is
    /// already waiting on it.
    fn reading(&self) -> Option<Reading> {
        if self.reading.replace(true) {
            return None;
        }
        Some(Reading(std::rc::Rc::clone(&self.reading)))
    }
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

impl Drop for Open {
    /// The task that opened this socket has ended, so the conversation ends with
    /// it: `1001`, which is the code for an end that is going away rather than
    /// one with anything to say about the conversation
    /// (`rule:http-server/an-outbound-socket-belongs-to-the-task-that-opened-it`).
    ///
    /// **Here rather than at the request's teardown** because this is the one
    /// place that runs however the socket was let go — the table dropping it
    /// with the task, a `close` taking it out, or a handshake whose socket never
    /// reached the table at all. A socket the program closed itself is already
    /// closed and `tungstenite` writes no second frame, so the code a program
    /// chose is never overwritten by this one.
    ///
    /// The write is bounded by the send wait and its failure is dropped: a peer
    /// that has already gone must not hold up a teardown or fail it.
    fn drop(&mut self) {
        self.framed
            .socket
            .get_mut()
            .bound_by(Some(Instant::now() + self.send));
        drop(self.framed.socket.close(Some(CloseFrame {
            code: CloseCode::Away,
            reason: "the task that opened this socket has ended".into(),
        })));
        drop(self.framed.socket.flush());
    }
}

/// `rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call`'s
/// conversation: what a program does with a socket once it has one.
///
/// # Two send members, because the payload kinds are two
///
/// `send` and `sendBytes` are [`crate::socket::CLASS`]'s shapes and are here
/// for its reason: RFC 6455 frames text and binary separately, so the member a
/// call picks is the opcode it means. Both parameters carry
/// [`Qual::Neutral`] — a frame is not an instruction on this side of the wire
/// — and a `string|bytes` parameter would answer that same mark off its arms
/// ([`CoreTy::classification`]), so the loop that forwards what the peer just
/// sent compiles either way.
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
    doc: Some(&SOCKET_CARD),
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

/// `Core\Http\Socket`'s own card — `rule:core-api/reference-card`.
const SOCKET_CARD: ClassDoc = ClassDoc {
    short: "A WebSocket connection your program opened to another server: it sends and receives \
            messages until one side closes it. `Core\\Http\\Client::openSocket()` returns one.",
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
        ErrorDoc {
            error: "RuntimeError",
            desc: "The peer sent a message past `maxMessage`. The socket is closed with `1009` \
                   before this is thrown, because a message nobody will read is memory the peer \
                   chose to make this task hold.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "Another `receive()` is already waiting on this socket. One message has one \
                   recipient, so a socket is read by the one task that holds it.",
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
    short: "Sends one binary message to the peer. `send` sends a text message.",
    params: &[ParamDoc {
        name: "frame",
        desc: "The bytes to send, as one binary message. A `tainted` value is accepted, as it \
               is for `send`.",
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
        let offered = offers(args)?;
        // A header the handshake would refuse is refused before anything
        // connects, and a test sees the same error.
        for (name, value) in super::headers_of(args, SOCKET_HEADERS, MEMBER)? {
            transport::judged_field(&name, &value, MEMBER)?;
        }

        let (chosen, held) = if ctx.faked_http().is_armed() {
            (
                transport::settled(scripted(ctx, &url)?, &offered, MEMBER)?,
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
    let upgraded = transport::upgrade(&call, &offers(args)?, framing)?;
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
        // The one key with an off position, and the reason it is read with no
        // directive and no default behind it: a program that sets it is
        // choosing to have `idle` end a *dead* peer rather than a quiet one,
        // and that is a request rather than a bound.
        ping: super::wait_of(args, SOCKET_PING, "ping")?,
        reading: std::rc::Rc::new(std::cell::Cell::new(false)),
    };
    Ok((protocol, Value::uint(ctx.hold_open_socket(Box::new(open)))))
}

/// The subprotocols this call offered, in the order the array wrote them.
///
/// Anything in it that is not text is skipped rather than refused: the key's
/// own type is `array<string>`, so `E0401` has already refused a call that
/// wrote anything else, and a check here would be a refusal no program can
/// reach.
///
/// # Errors
///
/// A `RuntimeError` naming an offer that is not an RFC 6455 § 4.1 subprotocol
/// name — one non-empty RFC 7230 token. The names travel joined by `, ` in one
/// header, so a comma or a space inside one is a list the peer reads
/// differently from the one the program wrote, and a control byte would end
/// the line. The refusal is here rather than at the header so a socket a test
/// answered refuses the same offer a live one does.
fn offers(args: &[Value]) -> Result<Vec<String>, Fault> {
    let Some(array) = args[SOCKET_PROTOCOLS].array_ptr() else {
        return Ok(Vec::new());
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
            let token =
                |byte: u8| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte);
            if name.is_empty() || !name.bytes().all(token) {
                return Err(Fault::thrown(format!(
                    "{MEMBER}: {name:?} is not a subprotocol name — a name is one word of letters, \
                     digits and the characters !#$%&'*+-.^_`|~, with no space, comma or control \
                     character"
                )));
            }
            names.push(name.to_owned());
        }
    }
    Ok(names)
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
/// is. Both are the peer *speaking*, so the silence `idle` bounds starts again
/// from each of them: a peer whose only traffic is pings is alive, and what
/// ends it is `maxDuration`. The peer's close is the end, and `null` is how
/// that reads.
///
/// **A ping this end sends is the one bound that is a question**, and it is off
/// unless the call asked for one. Every wait takes whichever of the three
/// instants is nearest, and a wait that ended at the ping's asks the peer
/// whether it is there rather than ending the conversation — so a silence the
/// peer answers is not a silence, and one it does not is over at `idle` all the
/// same.
///
/// # Errors
///
/// A `TimeoutError` for a silence past `idle` or a conversation past its
/// `maxDuration`, and whatever else the connection failed with.
fn heard(open: &mut Open) -> Result<Value, Fault> {
    let Some(_reading) = open.reading() else {
        return Err(crowded("receive"));
    };
    // The silence this wait bounds is the peer's, so it is measured from the
    // last thing the peer said and not from each round of the loop.
    let mut spoke = Instant::now();
    let mut prod = open.ping.map(|after| spoke + after);
    loop {
        let now = Instant::now();
        if now >= open.until {
            return Err(outlived("receive"));
        }
        let (idle, until) = (open.idle, open.until);
        let quiet_ends = spoke + idle;
        if now >= quiet_ends {
            return Err(silent("receive", idle));
        }
        // Every bound on the one wait, which is [`transport`]'s streamed reader
        // one protocol up: whichever is nearest is what the socket parks under,
        // and which of them fired is read back off the clock.
        let bound = until.min(quiet_ends).min(prod.unwrap_or(quiet_ends));
        open.framed.socket.get_mut().bound_by(Some(bound));
        let read = open.framed.socket.read();
        match read {
            Ok(tungstenite::Message::Text(text)) => return Ok(text_message(text.as_str())),
            Ok(tungstenite::Message::Binary(octets)) => return Ok(bytes_message(&octets)),
            Ok(tungstenite::Message::Ping(_) | tungstenite::Message::Pong(_)) => {
                spoke = Instant::now();
                prod = open.ping.map(|after| spoke + after);
            }
            // `Frame` is a raw frame the codec only produces for a caller that
            // asked for one, which this is not.
            Ok(tungstenite::Message::Close(_) | tungstenite::Message::Frame(_)) => {
                return Ok(Value::null());
            }
            Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                return Ok(Value::null());
            }
            Err(tungstenite::Error::Capacity(why)) => return Err(oversized(open, &why, "receive")),
            Err(why) => {
                let asking = open
                    .ping
                    .filter(|_| timed_out(&why) && prod.is_some_and(|at| Instant::now() >= at));
                let Some(after) = asking else {
                    // A connection that ends after *this* end has closed is the
                    // conversation being over and not a failure: a peer is free
                    // to answer a close by hanging up, and a program that has
                    // already been told why the socket closed would otherwise
                    // hear about it twice, the second time as an `IOError`.
                    if !open.framed.socket.can_write() {
                        return Ok(Value::null());
                    }
                    return Err(failed(why, idle, until, "receive"));
                };
                written(
                    open,
                    tungstenite::Message::Ping(Vec::new().into()),
                    "receive",
                )?;
                prod = Some(Instant::now() + after);
            }
        }
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
    if timed_out(&why) {
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

/// A message past `maxMessage`, which ends the conversation with `1009`.
///
/// **The close goes out before the throw rather than leaving the socket open.**
/// The codec stopped mid-reassembly, so what is still arriving belongs to a
/// message nobody will read, and every octet of it is memory a peer chose to
/// make this task hold; `1009` is the code that says which bound was crossed
/// (`rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`).
/// The write is bounded by the send wait and its failure is dropped: the
/// conversation is over either way, and what the program hears about is the
/// message that was refused.
fn oversized(open: &mut Open, why: &tungstenite::error::CapacityError, member: &str) -> Fault {
    let cap = open.framed.socket.get_config().max_message_size;
    open.framed
        .socket
        .get_mut()
        .bound_by(Some(Instant::now() + open.send));
    drop(open.framed.socket.close(Some(CloseFrame {
        code: CloseCode::Size,
        reason: "a message past this socket's `maxMessage`".into(),
    })));
    drop(open.framed.socket.flush());
    Fault::thrown_as(
        ThrownClass::Runtime,
        format!(
            "{SOCKET_NAME}::{member}(): the peer sent a message past this socket's `maxMessage` \
             of {} octets — {why} — so the conversation is closed with `1009`",
            cap.unwrap_or(usize::MAX)
        ),
    )
}

/// Whether a wait on the connection ended because the bound it was armed with
/// ran out, which is the one failure that is a clock rather than a connection.
///
/// An expired bound arrives as an ordinary timed-out read — `NvsTcp` has the
/// `Read` and `Write` return to report it through and no other channel — so
/// this is where the two readings of `TimedOut` are told apart, and every caller
/// asks the same way.
fn timed_out(why: &tungstenite::Error) -> bool {
    matches!(why, tungstenite::Error::Io(error) if error.kind() == std::io::ErrorKind::TimedOut)
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

/// The close frame a `close` sends, as the program wrote it.
///
/// An omitted code is `1000` rather than a frame carrying no code at all: a
/// peer reading a bare close cannot tell a program that ended the conversation
/// from a connection that fell over, and "a normal ending" is what an omitted
/// code means. An omitted reason is the empty one — a close carrying its code
/// and nothing else.
///
/// A code no `u16` holds goes out as `u16::MAX`, which is not a close code
/// either: a number that is not one is never quietly turned into a different
/// valid one, so the peer refuses what the program actually wrote.
fn ending(code: &Value, reason: &Value) -> CloseFrame {
    CloseFrame {
        code: code.as_uint().map_or(CloseCode::Normal, |written| {
            u16::try_from(written).unwrap_or(u16::MAX).into()
        }),
        reason: reason.as_text().unwrap_or_default().into(),
    }
}

/// The peer's own close, waited for under the send wait — the other half of
/// [ADR 0183](/docs/decisions/0183.md) § 5's close handshake, and the reason
/// `close` is a member that takes a wait at all.
///
/// Whatever the peer says before its close is read and dropped: this end has
/// ended the conversation, and a program that ended it is not owed the frames
/// that were already on their way. **Every exit is silent** — the peer's close,
/// a peer that hung up instead of answering, or the wait running out — for
/// [`CLOSE_DOC`]'s reason: a close that threw would put a `catch` around every
/// normal ending, and the connection is gone either way.
///
/// The bound is the send wait, and `maxDuration` still caps it: a socket whose
/// life has already run out waits no longer for a farewell than for anything
/// else.
fn farewell(open: &mut Open) {
    let bound = open.until.min(Instant::now() + open.send);
    open.framed.socket.get_mut().bound_by(Some(bound));
    // A read answers what is still arriving and then fails — on the peer's
    // close, on its hanging up, or on the bound — so the first failure is the
    // conversation being over however it ended.
    while open.framed.socket.read().is_ok() {}
}

/// A second `receive` on a socket one is already waiting on.
///
/// A `LogicError` because it is a program bug and not a condition: one message
/// has one recipient, and the alternative is a fan-out policy invented for
/// something nobody meant to write
/// (`rule:http-server/an-outbound-socket-belongs-to-the-task-that-opened-it`).
fn crowded(member: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "{SOCKET_NAME}::{member}(): another {member}() is already waiting on this socket, and \
             one message has one recipient — a socket is read by the one task that holds it"
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
    /// end of the task: the close frame goes out carrying [`ending`]'s code and
    /// reason, [`farewell`] waits for the peer's own close under the send wait,
    /// and the request's table drops its entry, which is the connection
    /// released. None of it is an error a program hears about — a peer that
    /// never answers is closed anyway, and the connection is gone either way.
    fn nvs_core_http_socket_close(ctx, args: [3]) {
        let receiver = crate::instance::receiver(args[0], &SOCKET, "close")?;
        crate::instance::set_slot(receiver, CLOSED_AT, Value::bool(true));
        if let Some(key) = crate::instance::slot(receiver, HELD_AT).as_uint()
            && let Some(mut held) = ctx.take_open_socket(key)
            && let Some(open) = held.as_any_mut().downcast_mut::<Open>()
        {
            drop(open.framed.socket.close(Some(ending(&args[1], &args[2]))));
            drop(open.framed.socket.flush());
            farewell(open);
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

#[cfg(test)]
mod tests {
    use super::{Open, ending, farewell, heard};
    use crate::http::transport::{self, tests::Say, tests::call, tests::talking_origin};
    use nvs_runtime::{Ctx, Fault, OutputSink, Tag, ThrownClass, Value};
    use std::net::SocketAddr;
    use std::time::{Duration, Instant};

    /// A live socket to `at`, under the bounds a case names.
    ///
    /// The handshake is the real one, over the loopback peer that is about to
    /// talk, because every case here asserts a wait *taken on a connection* —
    /// which is the one thing a hand-built [`Open`] could not carry. The send
    /// wait is generous throughout: nothing here is about a peer that stopped
    /// reading.
    fn opened(at: SocketAddr, idle: Duration, life: Duration, cap: usize) -> Open {
        let mut opening = call(at, super::MEMBER);
        opening.url = format!("ws://{at}/chat");
        let framing = tungstenite::protocol::WebSocketConfig::default().max_message_size(Some(cap));
        // The lifetime runs from before the handshake, as the row's does: a
        // socket's whole life includes the call that opened it.
        let began = Instant::now();
        let framed =
            transport::upgrade(&opening, &[], framing).expect("a `101` from the loopback peer");
        Open {
            framed,
            idle,
            until: began + life,
            send: Duration::from_secs(5),
            // Off, as it ships: the one case about it turns it on itself.
            ping: None,
            reading: std::rc::Rc::new(std::cell::Cell::new(false)),
        }
    }

    /// `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`:
    /// `idle` bounds the silence, so a peer that says nothing for longer than it
    /// ends the wait.
    ///
    /// The peer is alive and answering the protocol for the whole of it, which
    /// is what makes this a case about the bound: a connection that had gone
    /// away is a failure any wait would have noticed. What says the bound fired
    /// rather than the peer's own clock is the time it took.
    // covers: Core\Http\Socket::receive
    #[test]
    fn socket_idle_ends_a_silent_peer() {
        let (at, served) = talking_origin(None, vec![Say::Quiet(Duration::from_secs(3))]);
        let mut open = opened(
            at,
            Duration::from_millis(200),
            Duration::from_secs(30),
            1 << 20,
        );

        let began = Instant::now();
        let refused = heard(&mut open).expect_err("a silence longer than `idle`");
        let took = began.elapsed();

        let Fault::Thrown(class, why) = refused else {
            panic!("a bound that ran out is a throw")
        };
        assert!(
            matches!(class, ThrownClass::Timeout),
            "a silence past `idle` is a `TimeoutError`: {why}"
        );
        assert!(
            why.contains("sent nothing"),
            "the refusal names the silence rather than the lifetime: {why}"
        );
        assert!(
            took < Duration::from_secs(3),
            "the wait ended at `idle` and not when the peer gave up: {took:?}"
        );

        drop(open);
        served.join().expect("the origin thread");
    }

    /// `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`:
    /// a program that sets `ping` is choosing to have `idle` end a *dead* peer
    /// rather than a quiet one, so a live peer with nothing to say stays open
    /// past a silence that would otherwise have ended it.
    ///
    /// The peer says nothing until it has answered enough pings to outlast
    /// the `idle`, and then speaks. What keeps the socket open is its pong,
    /// which is why the peer holds its silence by reading rather than by
    /// sleeping: a peer that had stopped answering the protocol is the case
    /// this one is written to be distinguishable from.
    ///
    /// The client sends its next ping `ping` after the last pong, so the pings
    /// are never closer together than that, and the silence is longer than
    /// `idle` however slowly the machine runs. Each pong has to come back
    /// within `idle` less `ping`, and the gap between the two is the room left
    /// for a loaded machine.
    #[test]
    fn socket_ping_keeps_a_quiet_live_peer_open() {
        let idle = Duration::from_secs(1);
        let ping = Duration::from_millis(50);
        let pings =
            usize::try_from(idle.as_millis() / ping.as_millis()).expect("a small count") + 2;
        let (at, served) = talking_origin(None, vec![Say::Answering(pings), Say::Text("late")]);
        let mut open = opened(at, idle, Duration::from_secs(30), 1 << 20);
        open.ping = Some(ping);

        let began = Instant::now();
        let message = heard(&mut open).expect("the message a quiet peer sent when it had one");
        let took = began.elapsed();

        assert_eq!(
            message.tag(),
            Some(Tag::Object),
            "what a live peer eventually said is a message and not the end of the conversation"
        );
        assert!(
            took > idle,
            "the wait outlived an `idle` a pong is what carried it past: {took:?}"
        );

        drop(open);
        served.join().expect("the origin thread");
    }

    /// `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`:
    /// a message past `maxMessage` closes the socket with `1009`.
    ///
    /// Both halves are asserted, because either alone reads as done: the program
    /// hears a refusal naming the cap, and the *peer* is told which bound ended
    /// the conversation. A socket that threw and left the connection open would
    /// pass the first half while still holding everything the peer was sending.
    #[test]
    fn a_message_past_max_message_closes_the_socket_with_1009() {
        let (at, served) = talking_origin(None, vec![Say::Bytes(4096)]);
        let mut open = opened(at, Duration::from_secs(5), Duration::from_secs(30), 1024);

        let refused = heard(&mut open).expect_err("a message past `maxMessage`");
        let Fault::Thrown(class, why) = refused else {
            panic!("a refused message is a throw")
        };
        assert!(
            matches!(class, ThrownClass::Runtime),
            "a peer that sent too much is not this end's timing or its connection: {why}"
        );
        assert!(
            why.contains("`maxMessage`") && why.contains("`1009`"),
            "the refusal names the bound that was crossed and the code the peer was sent: {why}"
        );
        assert_eq!(
            heard(&mut open).expect("a receive after the close").tag(),
            Some(Tag::Null),
            "the conversation is over, so a later receive answers `null` rather than waiting"
        );

        drop(open);
        let ended = served.join().expect("the origin thread");
        assert_eq!(
            ended.closed.map(|(code, _)| code),
            Some(1009),
            "the peer is told which bound ended it, which is what the code is for"
        );
    }

    /// `rule:http-server/an-outbound-socket-belongs-to-the-task-that-opened-it`:
    /// a socket the program never closed is closed with `1001` when the task
    /// that opened it ends.
    ///
    /// The task ending is the request's table being given back, so that is what
    /// this drops — not the socket itself. A socket closed only by its own
    /// value being dropped would pass a case that dropped it directly and still
    /// leave a conversation open for as long as the table held one.
    #[test]
    fn a_socket_is_closed_with_1001_when_its_task_ends() {
        let (at, served) = talking_origin(None, Vec::new());
        let mut ctx = Ctx::new(OutputSink::Sink);
        let key = ctx.hold_open_socket(Box::new(opened(
            at,
            Duration::from_secs(5),
            Duration::from_secs(30),
            1 << 20,
        )));
        assert!(
            ctx.open_socket_mut(key).is_some(),
            "the request holds the connection for as long as the task runs"
        );

        drop(ctx);

        let ended = served.join().expect("the origin thread");
        let (code, reason) = ended
            .closed
            .expect("a close frame rather than a dropped connection");
        assert_eq!(
            code, 1001,
            "a task that ended is an end going away, which is what `1001` says"
        );
        assert!(
            reason.contains("task"),
            "and the reason says so rather than leaving the peer the code alone: {reason}"
        );
    }

    /// `rule:http-server/an-outbound-socket-belongs-to-the-task-that-opened-it`:
    /// two `receive`s waiting at once on one socket is a `LogicError`, because
    /// one message has one recipient.
    ///
    /// The mark is what the second call sees of the first, and it is all it
    /// could see: the wait is a park, so a socket one task is already inside
    /// looks idle to the next. What the case asserts beside the refusal is that
    /// the mark is *cleared* — a socket that refused a second wait for ever
    /// would refuse the program's next ordinary `receive` too.
    // covers: Core\Http\Socket::receive
    #[test]
    fn two_receives_waiting_on_one_socket_is_a_logic_error() {
        let (at, served) = talking_origin(
            None,
            vec![Say::Quiet(Duration::from_millis(200)), Say::Text("mine")],
        );
        let mut open = opened(at, Duration::from_secs(5), Duration::from_secs(30), 1 << 20);

        let waiting = open
            .reading()
            .expect("what a receive already parked here holds");
        let refused = heard(&mut open).expect_err("a second receive while one is waiting");
        let Fault::Thrown(class, why) = refused else {
            panic!("a program bug is a throw")
        };
        assert!(
            matches!(class, ThrownClass::Logic),
            "receiving twice at once is a bug in the program and not a condition: {why}"
        );
        assert!(
            why.contains("already waiting"),
            "the refusal says what the other call is doing: {why}"
        );

        drop(waiting);
        assert_eq!(
            heard(&mut open)
                .expect("a receive once the first has returned")
                .tag(),
            Some(Tag::Object),
            "the socket is read again once nothing is waiting on it"
        );

        drop(open);
        served.join().expect("the origin thread");
    }

    /// ADR 0183 § 7's close codes, as the frame a program's own `close` sends:
    /// an omitted code is a normal ending, and a number that is no close code
    /// is not turned into one that is.
    // covers: Core\Http\Socket::close
    #[test]
    fn a_close_carries_the_programs_code_and_reason() {
        let quiet = ending(&Value::null(), &Value::null());
        assert_eq!(
            u16::from(quiet.code),
            1000,
            "a `close()` that named no code ended the conversation normally"
        );
        assert_eq!(
            quiet.reason.as_str(),
            "",
            "and carried its code with nothing beside it"
        );

        let named = ending(&Value::uint(4001), &Value::null());
        assert_eq!(
            u16::from(named.code),
            4001,
            "the code the program wrote is the code on the wire"
        );

        let nonsense = ending(&Value::uint(70_000), &Value::null());
        assert_eq!(
            u16::from(nonsense.code),
            u16::MAX,
            "a number no close code holds goes out as one the peer refuses, not as its low half"
        );
    }

    /// `protocol` answers the name the handshake settled, before and after a
    /// `close`, and `null` for a socket whose handshake settled none: the name
    /// is what the two ends agreed, not a property of the connection being
    /// open.
    // covers: Core\Http\Socket::protocol
    // covers: Core\Http\Socket::close
    #[test]
    fn protocol_reads_the_settled_name_whether_or_not_the_socket_is_closed() {
        let text = |value: &str| Value::str(nvs_runtime::NvsStr::new(value.as_bytes()));
        let socket = |protocol: Value| {
            crate::instance::build(
                &super::SOCKET,
                [
                    text("wss://gateway.example.com/v1"),
                    protocol,
                    Value::int(0),
                    Value::bool(false),
                    Value::null(),
                ],
            )
        };
        let mut ctx = Ctx::buffered();

        let settled = socket(text("chat.v2"));
        let before = nvs_runtime::call(super::nvs_core_http_socket_protocol, &mut ctx, &[settled])
            .expect("`protocol` reads a slot and throws nothing");
        assert_eq!(before.as_text(), Some("chat.v2"), "the name the peer chose");

        nvs_runtime::call(
            super::nvs_core_http_socket_close,
            &mut ctx,
            &[settled, Value::null(), Value::null()],
        )
        .expect("a scripted socket closes without an error");
        let after = nvs_runtime::call(super::nvs_core_http_socket_protocol, &mut ctx, &[settled])
            .expect("a closed socket still answers `protocol`");
        assert_eq!(
            after.as_text(),
            Some("chat.v2"),
            "closing ends the conversation and leaves the agreed name readable"
        );
        let heard = nvs_runtime::call(super::nvs_core_http_socket_receive, &mut ctx, &[settled])
            .expect("a closed socket answers `receive` without an error");
        assert_eq!(
            heard.tag(),
            Some(Tag::Null),
            "a closed socket has nothing left to read"
        );

        let bare = socket(Value::null());
        let none = nvs_runtime::call(super::nvs_core_http_socket_protocol, &mut ctx, &[bare])
            .expect("`protocol` reads a slot and throws nothing");
        assert_eq!(
            none.tag(),
            Some(Tag::Null),
            "a handshake that settled nothing answers `null`"
        );
    }

    /// [ADR 0183](/docs/decisions/0183.md) § 5: a `close` sends its frame and
    /// then waits for the peer's own, rather than writing one and walking away.
    ///
    /// What says the handshake happened is the peer's side of it: an end that
    /// hung up on its own frame would leave the origin's read failing on a
    /// reset rather than recording the code it was sent. The peer is still
    /// talking when the close goes out, so this is also the frame in flight
    /// being read and dropped rather than turning into a failure.
    // covers: Core\Http\Socket::close
    #[test]
    fn a_close_waits_for_the_peers_own_close() {
        let (at, served) = talking_origin(None, vec![Say::Text("hello")]);
        let mut open = opened(at, Duration::from_secs(5), Duration::from_secs(5), 1 << 20);

        let began = Instant::now();
        drop(
            open.framed
                .socket
                .close(Some(ending(&Value::uint(4001), &Value::null()))),
        );
        drop(open.framed.socket.flush());
        farewell(&mut open);
        let waited = began.elapsed();

        let heard = served.join().expect("the peer's own thread");
        assert_eq!(
            heard.closed,
            Some((4001, String::new())),
            "the peer read the close this end sent, so the conversation ended by the handshake"
        );
        assert!(
            waited < Duration::from_secs(5),
            "a peer that answers is not waited on for the whole send wait: {waited:?}"
        );
    }

    /// Both send members write one whole message of their own kind, in the
    /// order they were called, and neither writes anything once the socket's
    /// `maxDuration` has passed.
    ///
    /// The peer is what reads the wire here, so a text sent as binary, a
    /// message split in two or two messages swapped all fail on what arrived
    /// rather than on what this end believes it wrote. The text is larger than
    /// one read so a message written in pieces would show.
    // covers: Core\Http\Socket::send
    // covers: Core\Http\Socket::sendBytes
    #[test]
    fn a_send_writes_one_whole_message_of_its_kind_and_nothing_past_the_lifetime() {
        let (at, served) = talking_origin(None, Vec::new());
        let mut open = opened(at, Duration::from_secs(5), Duration::from_secs(30), 1 << 20);
        let long = "w".repeat(200_000);

        super::written(
            &mut open,
            tungstenite::Message::Text("hello".into()),
            "send",
        )
        .expect("a text message to a peer that is reading");
        super::written(
            &mut open,
            tungstenite::Message::Binary(vec![0, 255, 7].into()),
            "sendBytes",
        )
        .expect("a binary message to a peer that is reading");
        super::written(
            &mut open,
            tungstenite::Message::Text(long.clone().into()),
            "send",
        )
        .expect("a long text message to a peer that is reading");

        open.until = Instant::now();
        let refused = super::written(&mut open, tungstenite::Message::Text("late".into()), "send")
            .expect_err("a socket past its lifetime");
        let Fault::Thrown(class, why) = refused else {
            panic!("a bound that ran out is a throw")
        };
        assert!(
            matches!(class, ThrownClass::Timeout),
            "a send past `maxDuration` is a `TimeoutError`: {why}"
        );
        assert!(
            why.contains("`maxDuration`"),
            "the refusal names the lifetime: {why}"
        );

        drop(
            open.framed
                .socket
                .close(Some(ending(&Value::null(), &Value::null()))),
        );
        drop(open.framed.socket.flush());
        farewell(&mut open);

        let heard = served.join().expect("the peer's own thread");
        assert_eq!(
            heard.said,
            vec![
                tungstenite::Message::Text("hello".into()),
                tungstenite::Message::Binary(vec![0, 255, 7].into()),
                tungstenite::Message::Text(long.into()),
            ],
            "the peer read three whole messages in the order they were sent, and not the late one"
        );
        assert_eq!(heard.closed, Some((1000, String::new())));
    }

    /// `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`:
    /// `maxDuration` bounds the whole conversation, so a peer that never stops
    /// talking is ended by the lifetime.
    ///
    /// The two bounds are told apart by the peer rather than by the message
    /// alone: it speaks every few milliseconds under an `idle` of seconds, so a
    /// socket that only ever checked the silence would read this conversation as
    /// endless. The count of what arrived first is what says it was talking.
    ///
    /// The lifetime is a second rather than a fraction of one because it runs
    /// from before the handshake, which is a real connection made while every
    /// other test binary is running: a bound tight enough to expire inside that
    /// upgrade would end a conversation the peer never got to start, and the
    /// script talks for long enough that the lifetime is still what ends it.
    #[test]
    fn socket_max_duration_ends_an_endless_conversation() {
        let script = (0..300)
            .flat_map(|_| [Say::Text("tick"), Say::Quiet(Duration::from_millis(20))])
            .collect();
        let (at, served) = talking_origin(None, script);
        let mut open = opened(at, Duration::from_secs(5), Duration::from_secs(1), 1 << 20);

        let mut arrived = 0;
        let refused = loop {
            match heard(&mut open) {
                Ok(message) if message.tag() == Some(Tag::Null) => {
                    panic!("the peer was still talking, so nothing here is the end of it")
                }
                Ok(_) => arrived += 1,
                Err(why) => break why,
            }
        };

        let Fault::Thrown(class, why) = refused else {
            panic!("a bound that ran out is a throw")
        };
        assert!(
            matches!(class, ThrownClass::Timeout),
            "a conversation past `maxDuration` is a `TimeoutError`: {why}"
        );
        assert!(
            why.contains("`maxDuration`"),
            "the refusal names the lifetime rather than the silence: {why}"
        );
        assert!(
            arrived > 0,
            "the peer was talking, so what ended this is the lifetime and not a silence"
        );

        drop(open);
        served.join().expect("the origin thread");
    }
}
