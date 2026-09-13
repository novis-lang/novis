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
//! Its URL, the subprotocol the handshake chose, how far through the peer's
//! frames it has read, and whether it has been closed — four slots, all of them
//! values Novis can hold, which is [`crate::instance`]'s requirement of every
//! `Core` class. The frames themselves are the scripted peer's and live in
//! [`nvs_runtime::AnswerTable`] beside the answers an outbound call is served
//! from, so two sockets opened to one URL each read the same script from their
//! own cursor.
//!
//! **What it spends:** four slots per open socket, and the cursor's worth of
//! nothing beyond them — a scripted peer's frames are the test's and are held
//! once however many sockets read them. What a socket against a real host
//! spends is [ADR 0183](/docs/decisions/0183.md) § 10's, and lands with the
//! handshake.

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
    slots: &["url", "protocol", "at", "closed"],
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
    errors: &[],
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
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The socket has been closed, so there is nobody left to send to.",
    }],
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
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The socket has been closed, so there is nobody left to send to.",
    }],
};

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
    /// **The subprotocol is judged against what was offered**, because a `101`
    /// choosing a name the request never sent is a peer answering a question
    /// nobody asked, and a program reading `protocol()` afterwards would act on
    /// it.
    fn nvs_core_http_client_open_socket(ctx, args: [SOCKET_ARITY]) {
        let url = super::given_url(args, MEMBER)?;
        super::judged_host(&url, MEMBER, Roster::Socket)?;
        judge_bound(args, SOCKET_DEADLINE, "deadline", MEMBER)?;
        judge_bound(args, SOCKET_CONNECT_TIMEOUT, "connectTimeout", MEMBER)?;
        judge_bound(args, SOCKET_IDLE, "idle", MEMBER)?;
        judge_bound(args, SOCKET_MAX_DURATION, "maxDuration", MEMBER)?;
        judge_bound(args, SOCKET_SEND_TIMEOUT, "sendTimeout", MEMBER)?;
        judge_bound(args, SOCKET_PING, "ping", MEMBER)?;

        let Some(peer) = ctx.faked_http().socket_for(&url) else {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{MEMBER}: this test answers outbound sockets from a table and no peer is \
                     registered for {url} — `Core\\Test::answerSocket` registers one, exactly or \
                     as a prefix ending in `*`"
                ),
            ));
        };
        let chosen = peer.protocol.clone();
        if let Some(name) = &chosen
            && !offered(args, name)
        {
            return Err(Fault::thrown(format!(
                "{MEMBER}: the peer chose the subprotocol `{name}`, which this call never offered"
            )));
        }

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
            ],
        ))
    }
}

/// Whether this call offered `name` as a subprotocol.
///
/// Anything in the array that is not text answers `false` rather than being
/// refused: the key's own type is `array<string>`, so `E0401` has already
/// refused a call that wrote anything else, and a check here would be a refusal
/// no program can reach.
fn offered(args: &[Value], name: &str) -> bool {
    let Some(array) = args[SOCKET_PROTOCOLS].array_ptr() else {
        return false;
    };
    let array = crate::arr::borrowed(array);
    let mut from = 0_usize;
    while let Some(slot) = array.next_slot(from) {
        from = slot + 1;
        let offer = array
            .value_at(slot)
            .expect("next_slot only names live entries");
        if offer.as_text() == Some(name) {
            return true;
        }
    }
    false
}

/// The peer's next frame, as a `Core\Socket\Message`, or `null` where the
/// conversation is over.
fn taken(ctx: &Ctx, url: &str, at: usize) -> Option<Value> {
    let frame = ctx.faked_http().socket_for(url)?.frames.get(at)?;
    Some(match frame {
        nvs_runtime::SocketFrame::Text(text) => crate::instance::build(
            &crate::socket::MESSAGE,
            [
                Value::null(),
                Value::str(NvsStr::new(text.as_bytes())),
                Value::null(),
                Value::null(),
            ],
        ),
        nvs_runtime::SocketFrame::Bytes(octets) => crate::instance::build(
            &crate::socket::MESSAGE,
            [
                Value::null(),
                Value::null(),
                Value::bytes(NvsStr::new(octets)),
                Value::null(),
            ],
        ),
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

nvs_runtime::nvs_helper! {
    /// `Core\Http\Socket::receive(): ?Core\Socket\Message` — the peer's next
    /// message, and `null` once the conversation is over.
    ///
    /// The cursor moves before the message is built, so a program that takes a
    /// message and then takes another gets the next one whatever it did with
    /// the first.
    fn nvs_core_http_socket_receive(ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &SOCKET, "receive")?;
        if crate::instance::slot(receiver, CLOSED_AT).as_bool() == Some(true) {
            return Ok(Value::null());
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
        let _ = writable(args[0], "send")?;
        let text = args[1].as_text().ok_or_else(|| {
            // Unreachable from source: the row's parameter is `CoreTy::Text`,
            // so `E0401` refuses anything else a phase earlier.
            Fault::fatal(format!(
                "{SOCKET_NAME}::send expected a `string` frame, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        ctx.faked_http_mut()
            .record_frame(nvs_runtime::SocketFrame::Text(text.to_owned()));
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Socket::sendBytes(bytes $frame): void` — one binary message
    /// to the peer.
    fn nvs_core_http_socket_send_bytes(ctx, args: [2]) {
        let _ = writable(args[0], "sendBytes")?;
        let octets = args[1].as_bytes().ok_or_else(|| {
            // Unreachable from source, for `send`'s reason one type over.
            Fault::fatal(format!(
                "{SOCKET_NAME}::sendBytes expected a `bytes` frame, got tag {}",
                args[1].tag_byte()
            ))
        })?;
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
    fn nvs_core_http_socket_close(_ctx, args: [3]) {
        let receiver = crate::instance::receiver(args[0], &SOCKET, "close")?;
        crate::instance::set_slot(receiver, CLOSED_AT, Value::bool(true));
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
