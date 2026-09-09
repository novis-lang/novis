//! `Core\Net` — `rule:core-classes/net-one-api-three-transports`'s socket
//! surface, over the runtime's own reactor and nothing else.
//!
//! Spec § 16's one class replacing `socket_*`, `stream_socket_*` and
//! `fsockopen`. What is here is the TCP half: an outbound connection, a
//! listening socket, and the accept on the listener that is a member rather
//! than a sixth way in. UDP and the two Unix-domain entry points are the same
//! rule's remaining three and are not on disk.
//!
//! # Decision: the surface is members, not spellings of one member
//!
//! A host and a socket path are separate members taking separately-typed
//! arguments, so `rule:security/a-path-is-not-a-url`'s refusal holds by
//! construction: nothing here reads the *content* of its argument to decide
//! which transport it opens. That is why [`CLASS`] will end at five entry
//! points where PHP has two, and it is the reason a `unix:` prefix handed to
//! [`nvs_core_net_connect`] is a hostname that will not resolve rather than a
//! second door.
//!
//! # Decision: every wait names its own bound
//!
//! Every member here that can wait takes a `Core\Time\Duration`, and there is
//! no spelling that omits one. A socket's peer is not the program: a read with
//! no bound is a request another party decides the length of, which is exactly
//! the shape `rule:http-server/no-spelling-for-an-unbounded-wait` refuses for
//! the outbound HTTP client. That rule reaches its default through
//! `[http.client]`; this class has no configuration block, so the narrow answer
//! — the argument is required — is what the standing form of this goal asks
//! for over inventing one. A caller that wants the same bound everywhere writes
//! it once into a constant.
//!
//! The bound covers **one call**, not the socket: it is filed on the stream
//! before the syscall and lifted after it, so two reads are two budgets, and a
//! `connect` that spent four of its five seconds does not shorten the read that
//! follows. `nvs_host::net`'s own `connect_timeout` makes the same split for
//! the same reason.
//!
//! # Decision: a socket is a key into the request's own table
//!
//! A `Core` instance's slots hold Novis values, so an `NvsTcp` cannot go in one
//! ([`crate::instance`]'s first decision is the home of why). The slot holds
//! the key `nvs_runtime::Ctx::hold_open_socket` filed it under, exactly as
//! `Core\IO\File`'s does for a descriptor — and here that is not merely the
//! available representation but the lifetime the rule asks for: a socket closes
//! with the request that opened it, because a connection outliving one would be
//! cross-request state (`rule:security/no-cross-request-state`).
//!
//! **What it spends:** one reactor registration per open socket, plus one
//! pointer pair in the request's table, released with the request's arena.
//! Nothing per process, and nothing that grows with sockets served.
//!
//! # Which grant each door asks
//!
//! `rule:security/net-listen-is-a-separate-grant-from-net-connect`: reaching
//! outward is `net.connect`, asked at `Scope::Host` and carrying the denied
//! ranges, which is `nvs_runtime::capability::pin_host` in one call — the host
//! is refused before it is resolved, so an ungranted program cannot use this
//! door as a resolver. Binding is `net.listen`, asked of the endpoint and
//! carrying no address policy. Neither widens the other, and a member here asks
//! exactly one of them.
//!
//! `Core\Net::listen` takes a **literal** address and resolves no name, which
//! is what `Scope::Endpoint` being a `SocketAddr` already requires: the grant is
//! matched exactly against an endpoint, and a name that resolved to two
//! addresses would be a bind the operator could not have named.

use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use nvs_config::capability::{Cap, Scope};
use nvs_host::{NvsListener, NvsTcp};
use nvs_runtime::{Ctx, Fault, HeldSocket, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// The class's own name, for the rosters in [`crate::registry`] that key on it.
pub(crate) const NAME: &str = r"Core\Net";

/// What [`nvs_core_net_connect`] and [`nvs_core_net_listener_accept`] answer.
pub(crate) const STREAM_NAME: &str = r"Core\Net\Stream";

/// What [`nvs_core_net_listen`] answers.
pub(crate) const LISTENER_NAME: &str = r"Core\Net\Listener";

/// The one slot both handle classes hold: the key their socket is filed under
/// in the request (`nvs_runtime::Ctx::hold_open_socket`).
const SOCKET_SLOT: usize = 0;

/// The largest buffer one [`nvs_core_net_stream_read`] allocates, whatever
/// `$max` says.
///
/// A socket read answers what has **arrived**, not what was asked for, so a
/// caller naming a megabyte would charge a megabyte to the request's memory
/// limit to receive a packet's worth. A short read is already this member's
/// ordinary answer, so the ceiling is invisible to a correct caller and is the
/// difference between a loop over `$max = 16 MiB` costing a request 16 MiB and
/// costing it this. `rule:programs/memory-priority`'s footprint is last, but a
/// number a program picks is not a footprint the runtime chose.
const READ_CEILING: u64 = 256 * 1024;

/// `Core\Time\Duration` as this module's parameter type, since every waiting
/// member takes one — see this module's second decision.
const WITHIN: CoreTy = CoreTy::Instance(crate::time::DURATION_NAME);

/// Spec § 16's `Core\Net`, TCP-shaped so far —
/// `rule:core-classes/net-one-api-three-transports`'s first two entry points of
/// five, with the datagram and Unix-domain three still to land.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "connect",
            names: &["host", "port", "within"],
            // A sink, on `rule:security/outbound-url-is-a-sink`'s reading: an
            // outbound address is where a tainted value must not arrive, and
            // the host is the whole of what decides who this program talks to.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Uint, WITHIN],
            defaults: &[],
            return_ty: CoreTy::Instance(STREAM_NAME),
            symbol: "nvs_core_net_connect",
            doc: Some(&CONNECT_DOC),
        },
        CoreMethod {
            name: "listen",
            names: &["address", "port"],
            // A sink for `connect`'s reason with the direction reversed: the
            // address decides which network this program is reachable from, and
            // a tainted one is how a program is made to expose a surface its
            // operator never chose.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Instance(LISTENER_NAME),
            symbol: "nvs_core_net_listen",
            doc: Some(&LISTEN_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Net::connect`'s reference card — `rule:core-api/reference-card`.
const CONNECT_DOC: MethodDoc = MethodDoc {
    short: "Opens a TCP connection to `$host` on `$port`, parking on the runtime's reactor while \
            the handshake is in flight. Needs `net.connect` for the host, and the address it \
            resolves to must not be one the address policy denies.",
    params: &[
        ParamDoc {
            name: "host",
            desc: "A hostname or an address literal. It is checked against the grant before it is \
                   resolved, and the one address it resolves to is what the connection is made to.",
            shape: &[],
        },
        ParamDoc {
            name: "port",
            desc: "The port to connect to, 1 to 65535.",
            shape: &[],
        },
        ParamDoc {
            name: "within",
            desc: "How long the handshake may take. The bound is lifted once the connection is up, \
                   so a later read takes whatever bound its own caller names.",
            shape: &[],
        },
    ],
    ret: "An open `Core\\Net\\Stream`, closed with this request if the program does not close it \
          first.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `net.connect` for this host, the host resolves \
                   to no address, the address it resolves to is in a denied range, or `$port` is \
                   not a port.",
        },
        ErrorDoc {
            error: "TimeoutError",
            desc: "The handshake was still in flight when `$within` ran out.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed — refused, unreachable, or reset while it was being \
                   established.",
        },
    ],
};

/// `Core\Net::listen`'s reference card — `rule:core-api/reference-card`.
const LISTEN_DOC: MethodDoc = MethodDoc {
    short: "Binds a listening TCP socket to `$address` on `$port`. Needs `net.listen` for that \
            exact endpoint; the address policy `net.connect` carries does not apply, because its \
            terms invert under a bind.",
    params: &[
        ParamDoc {
            name: "address",
            desc: "An address literal — `127.0.0.1`, `::1`, `0.0.0.0`. Not a hostname: the grant \
                   is matched against one endpoint, and a name resolving to two could not be the \
                   one an operator named.",
            shape: &[],
        },
        ParamDoc {
            name: "port",
            desc: "The port to bind, or `0` to let the operating system pick one — which \
                   `Core\\Net\\Listener::port` then reports.",
            shape: &[],
        },
    ],
    ret: "A bound `Core\\Net\\Listener`, closed with this request if the program does not close it \
          first.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `net.listen` for this endpoint, `$address` is \
                   not an address literal, or `$port` is not a port.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The operating system refused the bind — the port is taken, or the address is \
                   not one of this host's.",
        },
    ],
};

/// `rule:core-classes/net-one-api-three-transports`'s connected transport: a
/// `Read` and a `Write` shaped like every other stream in the language.
///
/// Its slot holds a key into the request's own table of open sockets, which
/// this module's third decision argues for; `Core\IO\File` is the same shape
/// over a descriptor and landed first.
pub(crate) const STREAM: CoreClass = CoreClass {
    name: STREAM_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "read",
            names: &["max", "within"],
            params: &[CoreTy::Uint, WITHIN],
            defaults: &[],
            // `rule:security/tainted-qualifier`: octets off a socket are as
            // untrusted as a request body's, and `Core\Request::bodyStream`
            // writes the same type over the same question.
            return_ty: CoreTy::TaintedBytes,
            symbol: "nvs_core_net_stream_read",
            doc: Some(&STREAM_READ_DOC),
        },
        CoreMethod {
            name: "write",
            names: &["payload", "within"],
            params: &[CoreTy::Blob(Qual::Neutral), WITHIN],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_net_stream_write",
            doc: Some(&STREAM_WRITE_DOC),
        },
        CoreMethod {
            name: "close",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_net_stream_close",
            doc: Some(&STREAM_CLOSE_DOC),
        },
    ],
    slots: &["socket"],
    constants: &[],
};

/// `Core\Net\Stream::read`'s reference card — `rule:core-api/reference-card`.
const STREAM_READ_DOC: MethodDoc = MethodDoc {
    short: "Reads whatever has arrived, up to `$max` octets, waiting no longer than `$within`. A \
            short answer is ordinary and not an error: a socket reports what arrived, never what \
            was asked for.",
    params: &[
        ParamDoc {
            name: "max",
            desc: "The most octets to answer with. The buffer is capped below this where `$max` is \
                   larger than one read can usefully be.",
            shape: &[],
        },
        ParamDoc {
            name: "within",
            desc: "How long to wait for the first octet. It bounds this call alone.",
            shape: &[],
        },
    ],
    ret: "The octets that arrived, as `tainted bytes`, or an empty `bytes` once the peer has \
          closed its half — which is how the end of a stream is spelled.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This handle is closed.",
        },
        ErrorDoc {
            error: "TimeoutError",
            desc: "Nothing arrived within `$within`.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed — reset by the peer, or dropped by the network.",
        },
    ],
};

/// `Core\Net\Stream::write`'s reference card — `rule:core-api/reference-card`.
const STREAM_WRITE_DOC: MethodDoc = MethodDoc {
    short: "Writes `$payload` to the peer, waiting no longer than `$within`, and answers how many \
            octets the socket took. A write that took only some of them is ordinary: the caller \
            sends the rest.",
    params: &[
        ParamDoc {
            name: "payload",
            desc: "The octets to send.",
            shape: &[],
        },
        ParamDoc {
            name: "within",
            desc: "How long to wait for the socket to take octets. It bounds this call alone.",
            shape: &[],
        },
    ],
    ret: "How many octets of `$payload` were written, which is between `0` and its length.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This handle is closed.",
        },
        ErrorDoc {
            error: "TimeoutError",
            desc: "The socket took nothing within `$within`.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed — reset by the peer, or its reading half closed.",
        },
    ],
};

/// `Core\Net\Stream::close`'s reference card — `rule:core-api/reference-card`.
const STREAM_CLOSE_DOC: MethodDoc = MethodDoc {
    short: "Closes this connection and gives its reactor registration back. A request that forgets \
            closes every socket it opened when it ends.",
    params: &[],
    ret: "Nothing.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "This handle is already closed.",
    }],
};

/// The accepting half of `rule:core-classes/net-one-api-three-transports` —
/// where accepting is a member on the listener rather than a sixth entry point
/// on `Core\Net`.
pub(crate) const LISTENER: CoreClass = CoreClass {
    name: LISTENER_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "accept",
            names: &["within"],
            params: &[WITHIN],
            defaults: &[],
            return_ty: CoreTy::Instance(STREAM_NAME),
            symbol: "nvs_core_net_listener_accept",
            doc: Some(&LISTENER_ACCEPT_DOC),
        },
        CoreMethod {
            name: "port",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_net_listener_port",
            doc: Some(&LISTENER_PORT_DOC),
        },
        CoreMethod {
            name: "close",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_net_listener_close",
            doc: Some(&LISTENER_CLOSE_DOC),
        },
    ],
    slots: &["socket"],
    constants: &[],
};

/// `Core\Net\Listener::accept`'s reference card — `rule:core-api/reference-card`.
const LISTENER_ACCEPT_DOC: MethodDoc = MethodDoc {
    short: "Takes the next connection off this listener, waiting no longer than `$within`, and \
            answers it as a stream. It asks no capability: the bind was granted when \
            `Core\\Net::listen` opened this socket.",
    params: &[ParamDoc {
        name: "within",
        desc: "How long to wait for a connection. It bounds this call alone.",
        shape: &[],
    }],
    ret: "The accepted connection, closed with this request if the program does not close it \
          first.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This handle is closed.",
        },
        ErrorDoc {
            error: "TimeoutError",
            desc: "No connection arrived within `$within`.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The operating system failed the accept.",
        },
    ],
};

/// `Core\Net\Listener::port`'s reference card — `rule:core-api/reference-card`.
const LISTENER_PORT_DOC: MethodDoc = MethodDoc {
    short: "The port this listener is bound to, which is how a program that asked for `0` learns \
            the one the operating system picked.",
    params: &[],
    ret: "The bound port, 1 to 65535.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This handle is closed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The operating system would not answer for this socket.",
        },
    ],
};

/// `Core\Net\Listener::close`'s reference card — `rule:core-api/reference-card`.
const LISTENER_CLOSE_DOC: MethodDoc = MethodDoc {
    short: "Stops listening and gives the port back. Connections already accepted are unaffected: \
            each is its own socket.",
    params: &[],
    ret: "Nothing.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "This handle is already closed.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_net_connect" => (nvs_core_net_connect as *const ()).cast(),
        "nvs_core_net_listen" => (nvs_core_net_listen as *const ()).cast(),
        "nvs_core_net_stream_read" => (nvs_core_net_stream_read as *const ()).cast(),
        "nvs_core_net_stream_write" => (nvs_core_net_stream_write as *const ()).cast(),
        "nvs_core_net_stream_close" => (nvs_core_net_stream_close as *const ()).cast(),
        "nvs_core_net_listener_accept" => (nvs_core_net_listener_accept as *const ()).cast(),
        "nvs_core_net_listener_port" => (nvs_core_net_listener_port as *const ()).cast(),
        "nvs_core_net_listener_close" => (nvs_core_net_listener_close as *const ()).cast(),
        _ => return None,
    })
}

/// A connected socket, in the shape the request's table holds.
///
/// A newtype rather than an impl on [`NvsTcp`] itself: neither that type nor
/// [`HeldSocket`] is this crate's, so the impl has to be written on something
/// local. What it buys back is that the two handle classes are two types here,
/// so a listener's key read as a stream's fails the downcast instead of
/// answering.
#[derive(Debug)]
struct Connected(NvsTcp);

impl HeldSocket for Connected {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A bound listening socket, in the shape the request's table holds —
/// [`Connected`]'s twin, and the home of why it is a newtype.
#[derive(Debug)]
struct Bound(NvsListener);

impl HeldSocket for Bound {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// One `string` argument, or the engine fault a wrongly-tagged one is: the
/// signature is checked at compile time, so a bad tag here is a lowering bug
/// and not something a program can provoke.
fn text<'a>(value: &'a Value, member: &str, position: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected {:?} for its {position}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

/// The port in argument slot `at`, refused where it is not one.
///
/// A `uint` covers far more than a port does, and the operating system's own
/// answer to an out-of-range one is a cast rather than a complaint — so this is
/// asked here, where the number the program wrote can be named.
///
/// `zero` is what `0` means for this caller: a bind may ask the operating
/// system to pick, and a connect may not.
///
/// # Errors
///
/// A [`Fault::fatal`] for a wrongly-tagged argument, which is a lowering bug
/// rather than anything a program can write, and a catchable `RuntimeError`
/// naming the number for one outside the range.
fn port_of(args: &[Value], at: usize, member: &str, zero: bool) -> Result<u16, Fault> {
    let asked = args[at].as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected {:?} for its port, got tag {}",
            Tag::Uint,
            args[at].tag_byte()
        ))
    })?;
    let port = u16::try_from(asked).unwrap_or(0);
    if u64::from(port) != asked || (port == 0 && !zero) {
        return Err(Fault::thrown(format!(
            "{NAME}::{member}: {asked} is not a port"
        )));
    }
    Ok(port)
}

/// The `Core\Time\Duration` in argument slot `at`, as a bound this call can
/// file on a socket.
///
/// A negative or absurd `Duration` is a program error rather than an unbounded
/// wait: the whole of this module's second decision is that there is no
/// spelling for waiting forever, and reading a negative bound as "no bound"
/// would be one.
///
/// # Errors
///
/// [`crate::time::nanos_of`]'s fatal for a wrongly-tagged argument, and a
/// catchable `RuntimeError` for a bound that is not a positive length of time.
fn within(args: &[Value], at: usize, member: &str) -> Result<Duration, Fault> {
    let nanos = crate::time::nanos_of(args, at, member)?;
    u64::try_from(nanos)
        .ok()
        .filter(|nanos| *nanos > 0)
        .map(Duration::from_nanos)
        .ok_or_else(|| {
            Fault::thrown(format!(
                "{NAME}::{member}: a bound must be a positive length of time"
            ))
        })
}

/// The key a handle of `class` holds, for the members that read their receiver.
///
/// # Errors
///
/// A [`Fault::fatal`] if the receiver is not one of that class's instances or
/// its slot holds the wrong tag: the slot is written where the handle is built
/// and nowhere else, so either is this crate disagreeing with itself.
fn key_of(value: Value, class: &CoreClass, member: &str) -> Result<u64, Fault> {
    let receiver = crate::instance::receiver(value, class, member)?;
    crate::instance::slot(receiver, SOCKET_SLOT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{}::{member} expected {:?} in its `{}` slot",
                class.name,
                Tag::Uint,
                class.slots[SOCKET_SLOT]
            ))
        })
}

/// The catchable `RuntimeError` every member of both handle classes raises on
/// a handle that has already been closed.
///
/// A `RuntimeError` and not an `IOError` for `Core\IO\File`'s reason: nothing
/// about the socket went wrong, and the program asked a question of something
/// it had already given up (`rule:errors/on-uncaught-throw`'s split).
fn already_closed(class: &CoreClass, member: &str) -> Fault {
    Fault::thrown(format!("{}::{member}: this handle is closed", class.name))
}

/// The connected socket `key` names, borrowed for one call.
///
/// # Errors
///
/// [`already_closed`] for a handle a `close` has consumed, and a
/// [`Fault::fatal`] for a key that names a listener — the two classes are
/// distinct types in the request's one table, so that is a paste error here
/// rather than anything a program can write.
fn stream_of<'a>(ctx: &'a mut Ctx, key: u64, member: &str) -> Result<&'a mut NvsTcp, Fault> {
    let held = ctx
        .open_socket_mut(key)
        .ok_or_else(|| already_closed(&STREAM, member))?;
    held.as_any_mut()
        .downcast_mut::<Connected>()
        .map(|connected| &mut connected.0)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{STREAM_NAME}::{member} found a listener at its key"
            ))
        })
}

/// The listening socket `key` names, borrowed for one call — [`stream_of`]'s
/// twin, with its errors.
fn listener_of<'a>(ctx: &'a mut Ctx, key: u64, member: &str) -> Result<&'a mut NvsListener, Fault> {
    let held = ctx
        .open_socket_mut(key)
        .ok_or_else(|| already_closed(&LISTENER, member))?;
    held.as_any_mut()
        .downcast_mut::<Bound>()
        .map(|bound| &mut bound.0)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{LISTENER_NAME}::{member} found a connection at its key"
            ))
        })
}

/// One socket failure, as the thing a program catches.
///
/// A bound that ran out is a `TimeoutError` and everything else an `IOError`,
/// which is `rule:core-api/failure-throws` over the two questions a caller
/// actually distinguishes: whether to wait longer, or to give up on the peer.
/// `nvs_host::net` reports an expired deadline as `TimedOut` from every one of
/// its four waiting functions, so this is the one place that reading is made.
fn failed(member: &str, err: &std::io::Error) -> Fault {
    if err.kind() == std::io::ErrorKind::TimedOut {
        return Fault::thrown_as(
            ThrownClass::Timeout,
            format!("{member} ran out of time waiting on the socket"),
        );
    }
    Fault::thrown_as(ThrownClass::Io, format!("{member} failed: {err}"))
}

/// Files `socket` against the request and builds the handle that names it.
///
/// The socket is filed *before* the object exists, so a failure between the two
/// closes it by dropping it rather than stranding it in a table nothing points
/// at — which is [`crate::io`]'s `open` making the same ordering choice.
fn handle(ctx: &mut Ctx, class: &CoreClass, socket: Box<dyn HeldSocket>) -> Value {
    let key = ctx.hold_open_socket(socket);
    crate::instance::build(class, [Value::uint(key)])
}

nvs_runtime::nvs_helper! {
    /// `Core\Net::connect(string $host, uint $port, Core\Time\Duration $within): Core\Net\Stream`
    /// — replacing `fsockopen` and `stream_socket_client`'s `tcp://` half.
    ///
    /// `nvs_runtime::capability::pin_host` is the whole door: it asks
    /// `net.connect` about the host, resolves it, and asks the denied-range
    /// table about the one address that came back — in that order, so a program
    /// with no grant cannot use this member as a resolver. The connection is
    /// then made to **that address** and not to the name again, which is the
    /// pinning `rule:http-server/allow-url-pins-the-address` describes for the
    /// HTTP client: re-resolving here would reopen the rebinding gap the pin
    /// closes.
    ///
    /// There is no persistent spelling. `pfsockopen`'s row in the migration
    /// table is `member` for this class with its persistent half dropped, and
    /// this module's third decision is where that lands: the socket is filed
    /// against the request and closed with it.
    fn nvs_core_net_connect(ctx, args: [3]) {
        const MEMBER: &str = r"Core\Net::connect";

        let host = text(&args[0], MEMBER, "host")?;
        let port = port_of(args, 1, "connect", false)?;
        let bound = within(args, 2, "connect")?;
        let address = nvs_runtime::capability::pin_host(ctx, host, MEMBER)?;
        let socket = NvsTcp::connect_timeout(SocketAddr::new(address, port), bound)
            .map_err(|err| failed(MEMBER, &err))?;
        Ok(handle(ctx, &STREAM, Box::new(Connected(socket))))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net::listen(string $address, uint $port): Core\Net\Listener` —
    /// replacing `stream_socket_server`.
    ///
    /// `net.listen` and **not** `net.connect`, asked of the endpoint as a
    /// parsed `SocketAddr` — `rule:security/net-listen-is-a-separate-grant-from-net-connect`,
    /// where the two grants are separate powers and neither widens the other.
    /// The denied-range table is deliberately absent: its terms invert under a
    /// bind, so importing it would refuse `127.0.0.1` and permit `0.0.0.0`,
    /// which is the wrong way round for both.
    ///
    /// The address is parsed rather than resolved, so this member does no DNS
    /// at all. `Scope::Endpoint` is a `SocketAddr` for exactly that reason: a
    /// grant is matched against one endpoint, and a name resolving to several
    /// could not be the one an operator wrote down.
    ///
    /// **This member takes no bound**, and is the one door here that cannot
    /// wait: a bind either succeeds or is refused by the operating system on
    /// the spot. Waiting for a connection is `Core\Net\Listener::accept`, which
    /// takes its own.
    fn nvs_core_net_listen(ctx, args: [2]) {
        const MEMBER: &str = r"Core\Net::listen";

        let written = text(&args[0], MEMBER, "address")?;
        let port = port_of(args, 1, "listen", true)?;
        let address: IpAddr = written.parse().map_err(|_| {
            Fault::thrown(format!(
                "{MEMBER}: `{written}` is not an address literal — a bind names an endpoint, and \
                 a hostname is not one"
            ))
        })?;
        let endpoint = SocketAddr::new(address, port);
        nvs_runtime::capability::require(ctx, Cap::NetListen, Scope::Endpoint(endpoint), MEMBER)?;
        let socket = NvsListener::bind(endpoint).map_err(|err| failed(MEMBER, &err))?;
        Ok(handle(ctx, &LISTENER, Box::new(Bound(socket))))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net\Stream::read(uint $max, Core\Time\Duration $within): tainted bytes` —
    /// replacing `fread` on a socket and `socket_recv`.
    ///
    /// **One read, not a fill.** What comes back is what had arrived, which is
    /// the only answer a socket can give without deciding for the caller how
    /// long to keep waiting — `$max` is a ceiling and a short answer is the
    /// ordinary case. The end of the stream is an empty `bytes` and not `null`,
    /// because a peer that closed its writing half is a state a caller loops
    /// until, not an absence: `rule:core-api/shape-rules` R5's `null` is for a
    /// value that is not there, and zero octets is a value.
    ///
    /// The bound is filed on the stream and lifted again here, whichever way
    /// the read ended, so this call's budget cannot become the next one's — the
    /// same split `nvs_host::net`'s own `connect_timeout` makes.
    ///
    /// **What it spends:** one buffer of at most [`READ_CEILING`] octets for
    /// the length of the call, plus the answer, both charged to the request
    /// that asked.
    fn nvs_core_net_stream_read(ctx, args: [3]) {
        const MEMBER: &str = r"Core\Net\Stream::read";

        let key = key_of(args[0], &STREAM, "read")?;
        let max = args[1].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "{MEMBER} expected {:?} for its max, got tag {}",
                Tag::Uint,
                args[1].tag_byte()
            ))
        })?;
        let bound = within(args, 2, "read")?;
        let wanted = usize::try_from(max.min(READ_CEILING)).unwrap_or(0);
        let stream = stream_of(ctx, key, "read")?;
        let mut buffer = vec![0u8; wanted];
        stream.set_deadline(Some(Instant::now() + bound));
        let outcome = stream.read(&mut buffer);
        stream.set_deadline(None);
        let read = outcome.map_err(|err| failed(MEMBER, &err))?;
        Ok(Value::bytes(NvsStr::new(&buffer[..read])))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net\Stream::write(bytes $payload, Core\Time\Duration $within): uint` —
    /// replacing `fwrite` on a socket and `socket_send`.
    ///
    /// **One write, not a `write_all`.** It answers how many octets the socket
    /// took, for [`nvs_core_net_stream_read`]'s reason read the other way: a
    /// member that looped until the whole payload was gone would be waiting for
    /// as long as the peer chose to make it, and `$within` would then bound
    /// something other than what it says. A caller with a full payload to send
    /// loops on the count, under a bound it renews and can give up on.
    fn nvs_core_net_stream_write(ctx, args: [3]) {
        const MEMBER: &str = r"Core\Net\Stream::write";

        let key = key_of(args[0], &STREAM, "write")?;
        let payload = args[1].as_bytes().ok_or_else(|| {
            Fault::fatal(format!(
                "{MEMBER} expected {:?} for its payload, got tag {}",
                Tag::Bytes,
                args[1].tag_byte()
            ))
        })?;
        let bound = within(args, 2, "write")?;
        let stream = stream_of(ctx, key, "write")?;
        stream.set_deadline(Some(Instant::now() + bound));
        let outcome = stream.write(payload);
        stream.set_deadline(None);
        let written = outcome.map_err(|err| failed(MEMBER, &err))?;
        Ok(Value::uint(written as u64))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net\Stream::close(): void` — replacing `fclose` on a socket.
    ///
    /// Taking the socket out of the request's table *is* the close, and it is
    /// what gives the reactor registration back: nothing else holds one, so
    /// dropping it here is the syscall. A second `close` finds the slot empty
    /// and throws rather than succeeding quietly — a key is never reused, so
    /// this can only be the same handle twice.
    fn nvs_core_net_stream_close(ctx, args: [1]) {
        let key = key_of(args[0], &STREAM, "close")?;
        let socket = ctx
            .take_open_socket(key)
            .ok_or_else(|| already_closed(&STREAM, "close"))?;
        drop(socket);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net\Listener::accept(Core\Time\Duration $within): Core\Net\Stream`
    /// — replacing `stream_socket_accept`.
    ///
    /// A member on the listener rather than a sixth entry point on `Core\Net`,
    /// which `rule:core-classes/net-one-api-three-transports` decides: what a
    /// program is doing here is taking the next connection off a socket it
    /// already holds, and that is not another way to open one.
    ///
    /// **It asks no capability.** The bind was granted when
    /// [`nvs_core_net_listen`] opened this socket, and an accepted connection
    /// reaches nothing the program chose — `net.connect`'s question is which
    /// host this program may reach, and the answer here was decided by whoever
    /// dialled in. Asking `net.connect` of a peer address would make a program
    /// that may listen unable to serve anyone outside its outbound grant, which
    /// is a rule nobody wrote and the opposite of what the two separate grants
    /// mean.
    fn nvs_core_net_listener_accept(ctx, args: [2]) {
        const MEMBER: &str = r"Core\Net\Listener::accept";

        let key = key_of(args[0], &LISTENER, "accept")?;
        let bound = within(args, 1, "accept")?;
        let listener = listener_of(ctx, key, "accept")?;
        listener.set_deadline(Some(Instant::now() + bound));
        let outcome = listener.accept();
        listener.set_deadline(None);
        let (socket, _peer) = outcome.map_err(|err| failed(MEMBER, &err))?;
        Ok(handle(ctx, &STREAM, Box::new(Connected(socket))))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net\Listener::port(): uint` — replacing `stream_socket_get_name`'s
    /// local half.
    ///
    /// The member that makes a `0` port usable: a program binds ephemerally and
    /// reads back the port the operating system picked, which is the only way
    /// it can tell anyone where to reach it. The address is deliberately not
    /// answered beside it — the program wrote that itself, and a member
    /// answering what its own caller passed in is the shape
    /// `rule:core-api/shape-rules` R5 refuses.
    fn nvs_core_net_listener_port(ctx, args: [1]) {
        const MEMBER: &str = r"Core\Net\Listener::port";

        let key = key_of(args[0], &LISTENER, "port")?;
        let listener = listener_of(ctx, key, "port")?;
        let bound = listener.local_addr().map_err(|err| failed(MEMBER, &err))?;
        Ok(Value::uint(u64::from(bound.port())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net\Listener::close(): void` — replacing `fclose` on a server
    /// socket.
    ///
    /// [`nvs_core_net_stream_close`]'s body and its reasoning, over the other
    /// of the two handle classes. Connections this listener already accepted
    /// are untouched: each was filed under a key of its own the moment it was
    /// accepted, and closing the door does not close what came through it.
    fn nvs_core_net_listener_close(ctx, args: [1]) {
        let key = key_of(args[0], &LISTENER, "close")?;
        let socket = ctx
            .take_open_socket(key)
            .ok_or_else(|| already_closed(&LISTENER, "close"))?;
        drop(socket);
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CLASS, LISTENER, LISTENER_NAME, NAME, STREAM, STREAM_NAME, nvs_core_net_connect,
        nvs_core_net_listen,
    };
    use nvs_runtime::{Ctx, NvsStr, Value};

    /// One second, as the bound every waiting member takes. Short because no
    /// case here waits it out: what is being asserted is which door refuses,
    /// and a refusal arrives before anything is waited on.
    fn a_second() -> Value {
        crate::time::duration_of(1_000_000_000)
    }

    /// Gives back the references a case built for its arguments.
    fn released(values: [Value; 3]) {
        #[expect(unsafe_code, reason = "each value owns the reference it releases")]
        unsafe {
            for value in values {
                value.release();
            }
        }
    }

    /// `rule:security/net-address-policy` and
    /// `rule:security/the-policy-lives-in-the-capability`, asserted as
    /// **agreement** rather than as a value: `Core\Net::connect` holds no copy
    /// of the denied-range table, so what it refuses and what
    /// `nvs_runtime::capability::pin_host` refuses are one sentence. A member
    /// that had grown a table of its own would read plausibly on its own line
    /// and disagree only here.
    ///
    /// The deployment grants the host by name and excepts no address, which is
    /// the configuration that separates the two questions: the grant says yes
    /// and the policy still says no.
    #[test]
    fn a_tcp_connect_walks_the_denied_range_table_through_pin_host() {
        const MEMBER: &str = r"Core\Net::connect";
        const HOST: &str = "127.0.0.1";

        let mut denied = Ctx::buffered();
        denied.set_config(crate::tests::granting(
            "[capabilities.net]\nconnect = [\"127.0.0.1\"]\n",
        ));
        let args = [
            Value::str(NvsStr::new(HOST.as_bytes())),
            Value::uint(9),
            a_second(),
        ];
        nvs_runtime::call(nvs_core_net_connect, &mut denied, &args)
            .expect_err("loopback is the first range the address policy denies");
        let by_member = denied
            .pending()
            .expect("a refusal is a throw and carries its message")
            .into_owned();
        released(args);

        let by_door = nvs_runtime::capability::pin_host(&denied, HOST, MEMBER)
            .expect_err("and the door is where that refusal is written");
        let nvs_runtime::Fault::Thrown(class, by_door) = by_door else {
            panic!("a capability refusal is catchable — `rule:security/denial-is-a-runtime-error`");
        };
        assert_eq!(class, nvs_runtime::ThrownClass::Runtime);
        assert_eq!(
            by_member, by_door,
            r"the sentence a program reads is the door's own, not one `Core\Net` composed"
        );
        assert!(
            by_member.contains("net.internal"),
            "and it names the key an operator would have to write: {by_member}"
        );

        // The grant is asked *first*, so a deployment that granted nothing gets
        // a `net.connect` refusal without the name ever being resolved — which
        // is what stops this member being a resolver for hosts it may not
        // reach.
        let mut ungranted = Ctx::buffered();
        let args = [
            Value::str(NvsStr::new(b"example.invalid")),
            Value::uint(80),
            a_second(),
        ];
        nvs_runtime::call(nvs_core_net_connect, &mut ungranted, &args)
            .expect_err("a context with no configuration grants nothing");
        let refused = ungranted.pending().expect("a message").into_owned();
        released(args);
        assert!(
            refused.contains("net.connect") && refused.contains("example.invalid"),
            "the refusal names the grant and what it was wanted for: {refused}"
        );
    }

    /// `rule:security/net-listen-is-a-separate-grant-from-net-connect`: which
    /// capability an opening asks is decided by what the program is doing, so a
    /// bind asks `net.listen` and the widest possible `net.connect` buys it
    /// nothing.
    ///
    /// Both directions, because either alone is met by a member that asks the
    /// wrong grant consistently: the outbound grant does not open a port, and
    /// the inbound grant alone does.
    #[test]
    fn a_tcp_listener_asks_net_listen_and_not_net_connect() {
        let mut outbound = Ctx::buffered();
        outbound.set_config(crate::tests::granting(
            "[capabilities.net]\nconnect = true\ninternal = [\"127.0.0.1\"]\n",
        ));
        let args = [
            Value::str(NvsStr::new(b"127.0.0.1")),
            Value::uint(0),
            Value::null(),
        ];
        nvs_runtime::call(nvs_core_net_listen, &mut outbound, &args[..2])
            .expect_err("reaching a host is not permission to bind one");
        let refused = outbound.pending().expect("a message").into_owned();
        released(args);
        assert!(
            refused.contains("net.listen") && refused.contains("127.0.0.1:0"),
            "the refusal names the grant that was missing and the endpoint it was asked of: \
             {refused}"
        );

        // And the grant that *is* the bind opens it, holding nothing outbound.
        let mut inbound = Ctx::buffered();
        inbound.set_config(crate::tests::granting(
            "[capabilities.net]\nlisten = true\n",
        ));
        let args = [
            Value::str(NvsStr::new(b"127.0.0.1")),
            Value::uint(0),
            Value::null(),
        ];
        let door = nvs_runtime::call(nvs_core_net_listen, &mut inbound, &args[..2])
            .expect("a granted bind on an ephemeral port");
        released(args);
        #[expect(unsafe_code, reason = "the handle owns the reference it releases")]
        unsafe {
            door.release();
        }

        // The declaration says the same thing, which is what a review reads:
        // one row per member, and the two doors name different grants.
        let asked = |member: &str| {
            crate::registry::CAPABILITIES
                .iter()
                .find(|(class, name, _)| *class == NAME && *name == member)
                .map(|(_, _, cap)| *cap)
                .expect("every member of a capability-bearing class has a row")
        };
        assert_eq!(asked("listen"), Some(nvs_config::Cap::NetListen));
        assert_eq!(asked("connect"), Some(nvs_config::Cap::NetConnect));
    }

    /// The layout each handle class declares and the index its bodies read by
    /// are one decision written twice — the pairing
    /// [`crate::registry::CoreClass::slots`] exists to keep honest.
    #[test]
    fn both_handle_classes_hold_their_socket_in_the_slot_they_declare() {
        assert_eq!(super::SOCKET_SLOT, STREAM.slot("socket"));
        assert_eq!(super::SOCKET_SLOT, LISTENER.slot("socket"));
        assert_eq!(STREAM.name, STREAM_NAME);
        assert_eq!(LISTENER.name, LISTENER_NAME);
        assert_eq!(CLASS.name, NAME);
    }
}
