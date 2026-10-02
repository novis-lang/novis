//! `Core\Net` — `rule:core-classes/net-one-api-three-transports`'s socket
//! surface, over the runtime's own reactor and nothing else.
//!
//! Spec § 16's one class replacing `socket_*`, `stream_socket_*` and
//! `fsockopen`. All five entry points are here — an outbound TCP connection, a
//! listening TCP socket, a bound datagram socket, an outbound Unix-domain
//! connection and a listening Unix-domain one — with the accept on the listener
//! a member rather than a sixth way in.
//!
//! # Decision: one handle class per shape, two transports inside it
//!
//! A connection is a `Core\Net\Stream` whether it was dialled over TCP or over
//! a socket path, and a listener is a `Core\Net\Listener` the same way:
//! `rule:core-classes/net-one-api-three-transports` gives the connected
//! transports one `Read` and one `Write`, so [`Connected`] and [`Bound`] carry
//! a variant per transport rather than the surface carrying a class per
//! transport. What a program picked is decided at the door and never read back
//! out of an argument.
//!
//! A build with no `AF_UNIX` transport — Windows, where `mio` carries none —
//! answers both local doors with a `RuntimeError` saying so, *after* the grant
//! has been asked. The order is deliberate: a member that reported "no
//! transport" to an ungranted program would answer a question about this host
//! that the program was not authorized to ask.
//!
//! `Core\Net\Listener::port` is the one member the second transport reaches
//! without an answer, and it throws rather than inventing one: a socket bound
//! at a path has no port, and the program that bound it holds the path already.
//! A `?uint` would make every TCP caller unwrap a `null` for a case it cannot
//! reach, which is the shape `rule:core-api/shape-rules` R5 keeps for a value
//! that is genuinely sometimes absent.
//!
//! # Decision: the surface is members, not spellings of one member
//!
//! A host and a socket path are separate members taking separately-typed
//! arguments, so `rule:security/a-path-is-not-a-url`'s refusal holds by
//! construction: nothing here reads the *content* of its argument to decide
//! which transport it opens. That is why [`CLASS`] ends at five entry
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
//! # Decision: a receive answers a message, not octets
//!
//! [`nvs_core_net_datagram_receive`] hands back a `Core\Net\Datagram\Message` —
//! what arrived and who sent it, as three slot-reading members over one object.
//! The migration row it answers is `stream_socket_recvfrom`, which delivers the
//! sender through an out-parameter; Novis has neither out-parameters nor
//! tuples, so an object is the narrowest thing that answers that row at all. A
//! `receive` handing back octets alone would be a datagram socket that cannot
//! reply, which is most of what a datagram socket is for.
//!
//! The sender's address is answered as a plain `string` and not a `tainted`
//! one, and that is what makes the reply compile rather than an oversight:
//! `send`'s `$host` is a sink, a sink refuses a qualified argument, and there is
//! no launderer for an address to pass it through. What guards a reply instead
//! is the grant — every send asks `net.connect` and the denied-range table of
//! the address it was handed, this one included — so an address arriving off the
//! network buys a program nothing its configuration had not already granted.
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
//! `Core\Net::bindDatagram` is the same door under the same grant and takes its
//! address the same way: what decides which capability an opening asks is what
//! the program is doing, never the transport it does it over.
//!
//! A datagram socket then asks the *other* grant at a different moment. It is
//! bound and never connected, so there is no opening at which an outbound
//! address could be named — every `Core\Net\Datagram::send` asks `net.connect`
//! of the address it was handed, which is `rule:security/net-address-policy` in
//! its own words: a datagram sent to a program-supplied address is asked at the
//! send what a TCP connect is asked at the connect.

use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use nvs_config::capability::{Cap, Scope};
use nvs_host::{NvsListener, NvsTcp, NvsUdp};
#[cfg(unix)]
use nvs_host::{NvsUnix, NvsUnixListener};
use nvs_runtime::{Ctx, Fault, HeldSocket, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class's own name, for the rosters in [`crate::registry`] that key on it.
pub(crate) const NAME: &str = r"Core\Net";

/// What [`nvs_core_net_connect`] and [`nvs_core_net_listener_accept`] answer.
pub(crate) const STREAM_NAME: &str = r"Core\Net\Stream";

/// What [`nvs_core_net_listen`] answers.
pub(crate) const LISTENER_NAME: &str = r"Core\Net\Listener";

/// What [`nvs_core_net_bind_datagram`] answers.
pub(crate) const DATAGRAM_NAME: &str = r"Core\Net\Datagram";

/// What [`nvs_core_net_datagram_receive`] answers — this module's fourth
/// decision is why a receive answers one of these rather than octets.
pub(crate) const MESSAGE_NAME: &str = r"Core\Net\Datagram\Message";

/// The one slot all three handle classes hold: the key their socket is filed
/// under in the request (`nvs_runtime::Ctx::hold_open_socket`).
const SOCKET_SLOT: usize = 0;

/// [`MESSAGE`]'s first slot: the octets that arrived.
const PAYLOAD_SLOT: usize = 0;

/// [`MESSAGE`]'s second slot: the address they arrived from, written out.
const HOST_SLOT: usize = 1;

/// [`MESSAGE`]'s third slot: the port they arrived from.
const PORT_SLOT: usize = 2;

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

/// The buffer every [`nvs_core_net_datagram_receive`] allocates, and the ceiling
/// on that member's `$max`.
///
/// **This is a correctness figure and not a footprint one.** A buffer shorter
/// than the datagram waiting on the socket is where the platforms part company:
/// the Unixes keep what fits and drop the rest, while Windows refuses the call
/// outright with `WSAEMSGSIZE` and hands back neither a count nor a sender. A
/// `Core` member cannot observably do two different things on two hosts
/// (`rule:programs/memory-priority`'s ordering puts language semantics above
/// footprint), so the kernel is always handed a buffer no datagram can overflow
/// and the `$max` cut is made here afterwards.
///
/// 64 KiB is that size because a UDP payload cannot exceed 65,507 octets — the
/// length field is sixteen bits — so this is not a policy the way
/// [`READ_CEILING`] is, and a `$max` above it is capped rather than honoured
/// because nothing could ever fill it.
///
/// A `usize` where [`READ_CEILING`] is a `u64`, because this one is a buffer
/// length before it is a bound on an argument.
const DATAGRAM_CEILING: usize = 64 * 1024;

/// `Core\Time\Duration` as this module's parameter type, since every waiting
/// member takes one — see this module's second decision.
const WITHIN: CoreTy = CoreTy::Instance(crate::time::DURATION_NAME);

/// `Core\Net`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Opens network sockets. `connect` and `listen` open a TCP connection or a TCP \
            listener, `bindDatagram` opens a UDP socket, and `connectLocal` and `listenLocal` do \
            the same over a Unix-domain socket path. Each one needs a grant in the configuration.",
};

/// Spec § 16's `Core\Net` —
/// `rule:core-classes/net-one-api-three-transports`'s five entry points, each
/// saying in its name what it opens.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
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
        CoreMethod {
            name: "bindDatagram",
            names: &["address", "port"],
            // `listen`'s classification and for `listen`'s reason: an address
            // written here decides which network this program is reachable
            // from, and a tainted one is how a program is made to expose a
            // surface its operator never chose.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Instance(DATAGRAM_NAME),
            symbol: "nvs_core_net_bind_datagram",
            doc: Some(&BIND_DATAGRAM_DOC),
        },
        CoreMethod {
            name: "connectLocal",
            names: &["path", "within"],
            // `connect`'s classification over the other transport: a socket
            // path is where this program's traffic goes, so a tainted one is
            // the same sink a tainted host is
            // (`rule:security/outbound-url-is-a-sink`). That it is a path and
            // not a host is what makes it a second member rather than a wider
            // first one — `rule:security/a-path-is-not-a-url` holding by
            // construction.
            params: &[CoreTy::Path(Qual::Sink), WITHIN],
            defaults: &[],
            return_ty: CoreTy::Instance(STREAM_NAME),
            symbol: "nvs_core_net_connect_local",
            doc: Some(&CONNECT_LOCAL_DOC),
        },
        CoreMethod {
            name: "listenLocal",
            names: &["path"],
            // `listen`'s classification over the other transport: the path
            // decides who on this host can reach the program, and a tainted one
            // is how a surface its operator never chose gets exposed.
            params: &[CoreTy::Path(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Instance(LISTENER_NAME),
            symbol: "nvs_core_net_listen_local",
            doc: Some(&LISTEN_LOCAL_DOC),
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
                   to no address, the address it resolves to is in a denied range, `$port` is not \
                   a port, or `$within` is not a positive length of time.",
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

/// `Core\Net::bindDatagram`'s reference card — `rule:core-api/reference-card`.
const BIND_DATAGRAM_DOC: MethodDoc = MethodDoc {
    short: "Binds a datagram socket to `$address` on `$port`. Needs `net.listen` for that exact \
            endpoint, exactly as a TCP bind does; sending to anywhere is a separate grant asked at \
            `Core\\Net\\Datagram::send`.",
    params: &[
        ParamDoc {
            name: "address",
            desc: "An address literal — `127.0.0.1`, `::1`, `0.0.0.0`. Not a hostname, for \
                   `Core\\Net::listen`'s reason: a grant names one endpoint.",
            shape: &[],
        },
        ParamDoc {
            name: "port",
            desc: "The port to bind, or `0` to let the operating system pick one — which \
                   `Core\\Net\\Datagram::port` then reports.",
            shape: &[],
        },
    ],
    ret: "A bound `Core\\Net\\Datagram`, closed with this request if the program does not close it \
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

/// `Core\Net::connectLocal`'s reference card — `rule:core-api/reference-card`.
const CONNECT_LOCAL_DOC: MethodDoc = MethodDoc {
    short: "Connects to the Unix-domain socket bound at `$path`, parking on the runtime's reactor \
            while the connect is in flight. Needs `net.local` for that path, which is a grant of \
            its own: `net.connect` never covers a path, and carries no policy that could read one.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "An absolute path to a bound socket. It is never read as a host and never as a \
                   URL — this is the member that takes a path, and no other one does.",
            shape: &[],
        },
        ParamDoc {
            name: "within",
            desc: "How long to wait for the connect. It bounds this call alone, and a local \
                   connect waits at all only when the listener's backlog is full.",
            shape: &[],
        },
    ],
    ret: "A connected `Core\\Net\\Stream`, closed with this request if the program does not close \
          it first.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `net.local` for this path, this build has no \
                   Unix-domain transport, or `$within` is not a positive length of time.",
        },
        ErrorDoc {
            error: "TimeoutError",
            desc: "The connect was still in flight after `$within`.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The operating system refused the path — nothing is bound there, no permission, \
                   or a name too long for the platform.",
        },
    ],
};

/// `Core\Net::listenLocal`'s reference card — `rule:core-api/reference-card`.
const LISTEN_LOCAL_DOC: MethodDoc = MethodDoc {
    short: "Binds a listening Unix-domain socket at `$path`, creating it. Needs `net.local` for \
            that path: the grant governs both ends, because a socket a program creates is one \
            whatever finds it on this host may speak to.",
    params: &[ParamDoc {
        name: "path",
        desc: "An absolute path the socket is created at. It must not exist yet, and it is not \
               removed when the socket closes — the name may by then be something else's.",
        shape: &[],
    }],
    ret: "A bound `Core\\Net\\Listener`, closed with this request if the program does not close it \
          first. Its `port` throws: a socket bound at a path has none.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `net.local` for this path, or this build has \
                   no Unix-domain transport.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The operating system refused the path — it exists already, the directory does \
                   not, or there is no permission to create it.",
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
    doc: Some(&STREAM_CARD),
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

/// `Core\Net\Stream`'s class card — `rule:core-api/reference-card`.
const STREAM_CARD: ClassDoc = ClassDoc {
    short: "A connection to one client or server, returned by `Core\\Net::connect`, \
            `Core\\Net::connectLocal` and `Core\\Net\\Listener::accept`. `read` and `write` \
            move bytes, and each waits no longer than its `$within`.",
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
            desc: "This handle is closed, or `$within` is not a positive length of time.",
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
            desc: "This handle is closed, or `$within` is not a positive length of time.",
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
    doc: Some(&LISTENER_CARD),
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

/// `Core\Net\Listener`'s class card — `rule:core-api/reference-card`.
const LISTENER_CARD: ClassDoc = ClassDoc {
    short: "A TCP server socket, returned by `Core\\Net::listen` and `Core\\Net::listenLocal`. \
            `accept` waits for the next client and returns a `Core\\Net\\Stream` for it.",
};

/// `Core\Net\Listener::accept`'s reference card — `rule:core-api/reference-card`.
const LISTENER_ACCEPT_DOC: MethodDoc = MethodDoc {
    short: "Waits for the next client to connect, for no longer than `$within`, and returns a \
            `Core\\Net\\Stream` for that client. It needs no setting of its own: `nvs.toml` was \
            checked when `Core\\Net::listen` opened this socket.",
    params: &[ParamDoc {
        name: "within",
        desc: "How long to wait for a client. It is the limit for this call only.",
        shape: &[],
    }],
    ret: "The connection to the client. The request closes it when it ends, if the program has \
          not closed it before.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This handle is closed, or `$within` is not a positive length of time.",
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
            desc: "This handle is closed, or it is a listener `Core\\Net::listenLocal` bound at a \
                   path, which has no port at all.",
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

/// `rule:core-classes/net-one-api-three-transports`'s third transport, which
/// answers neither `Read` nor `Write` because a datagram socket has no stream
/// to read: it sends and receives whole messages, addressed one at a time.
///
/// Its slot is the two above's — a key into the request's own table of open
/// sockets — and it is the one handle class here whose members do not all reach
/// what the bind already granted. See this module's grant section for why
/// [`nvs_core_net_datagram_send`] asks a second one.
pub(crate) const DATAGRAM: CoreClass = CoreClass {
    name: DATAGRAM_NAME,
    doc: Some(&DATAGRAM_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "send",
            names: &["host", "port", "payload", "within"],
            // `Core\Net::connect`'s classification on the same argument, and
            // for the whole of its reason: this is the outbound address, and it
            // is where a tainted value must not arrive.
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Uint,
                CoreTy::Blob(Qual::Neutral),
                WITHIN,
            ],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_net_datagram_send",
            doc: Some(&DATAGRAM_SEND_DOC),
        },
        CoreMethod {
            name: "receive",
            names: &["max", "within"],
            params: &[CoreTy::Uint, WITHIN],
            defaults: &[],
            return_ty: CoreTy::Instance(MESSAGE_NAME),
            symbol: "nvs_core_net_datagram_receive",
            doc: Some(&DATAGRAM_RECEIVE_DOC),
        },
        CoreMethod {
            name: "port",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_net_datagram_port",
            doc: Some(&DATAGRAM_PORT_DOC),
        },
        CoreMethod {
            name: "close",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_net_datagram_close",
            doc: Some(&DATAGRAM_CLOSE_DOC),
        },
    ],
    slots: &["socket"],
    constants: &[],
};

/// `Core\Net\Datagram`'s class card — `rule:core-api/reference-card`.
const DATAGRAM_CARD: ClassDoc = ClassDoc {
    short: "A UDP socket, returned by `Core\\Net::bindDatagram`. `send` sends one message to an \
            address, and `receive` waits for one message and returns it as a \
            `Core\\Net\\Datagram\\Message`.",
};

/// `Core\Net\Datagram::send`'s reference card — `rule:core-api/reference-card`.
const DATAGRAM_SEND_DOC: MethodDoc = MethodDoc {
    short: "Sends one datagram to `$host` on `$port` and answers how many octets went. Needs \
            `net.connect` for the host, and the address it resolves to must not be one the address \
            policy denies — the same question a TCP connect is asked, asked here because this is \
            where the address is named.",
    params: &[
        ParamDoc {
            name: "host",
            desc: "A hostname or an address literal. It is checked against the grant before it is \
                   resolved, and the datagram goes to the one address it resolved to.",
            shape: &[],
        },
        ParamDoc {
            name: "port",
            desc: "The port to send to, 1 to 65535.",
            shape: &[],
        },
        ParamDoc {
            name: "payload",
            desc: "The octets of one message. There is no partial send: a message too large for \
                   the path is refused rather than split.",
            shape: &[],
        },
        ParamDoc {
            name: "within",
            desc: "How long to wait for the socket to take the message. It bounds this call alone.",
            shape: &[],
        },
    ],
    ret: "How many octets went, which is `$payload`'s length on every success.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `net.connect` for this host, the host resolves \
                   to no address, the address it resolves to is in a denied range, `$port` is not \
                   a port, this handle is closed, or `$within` is not a positive length of time.",
        },
        ErrorDoc {
            error: "TimeoutError",
            desc: "The socket would not take the message within `$within`.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The operating system refused the send — the message is over the maximum size, \
                   or the network is unreachable.",
        },
    ],
};

/// `Core\Net\Datagram::receive`'s reference card — `rule:core-api/reference-card`.
const DATAGRAM_RECEIVE_DOC: MethodDoc = MethodDoc {
    short: "Waits for one datagram, no longer than `$within`, and answers it together with who \
            sent it. It asks no capability: the bind was granted when this socket was opened, and \
            who sent the message was not this program's choice.",
    params: &[
        ParamDoc {
            name: "max",
            desc: "How many octets to keep, capped at 64 KiB because no datagram is larger. A \
                   datagram arrives whole or not at all, so a message longer than this keeps what \
                   fits and the rest are gone — there is no second call that answers the tail.",
            shape: &[],
        },
        ParamDoc {
            name: "within",
            desc: "How long to wait for a message. It bounds this call alone.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Net\\Datagram\\Message` carrying the octets and the endpoint they came from.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "This handle is closed, or `$within` is not a positive length of time.",
        },
        ErrorDoc {
            error: "TimeoutError",
            desc: "No datagram arrived within `$within`.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The operating system failed the receive — on Windows, this is also how an \
                   earlier send of this socket's is reported unreachable.",
        },
    ],
};

/// `Core\Net\Datagram::port`'s reference card — `rule:core-api/reference-card`.
const DATAGRAM_PORT_DOC: MethodDoc = MethodDoc {
    short: "The port this socket is bound to, which is how a program that asked for `0` learns the \
            one the operating system picked.",
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

/// `Core\Net\Datagram::close`'s reference card — `rule:core-api/reference-card`.
const DATAGRAM_CLOSE_DOC: MethodDoc = MethodDoc {
    short: "Closes this socket and gives its port and its reactor registration back. A request \
            that forgets closes every socket it opened when it ends.",
    params: &[],
    ret: "Nothing.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "This handle is already closed.",
    }],
};

/// One received datagram: what arrived, and the endpoint it arrived from.
///
/// Three slots and three readers, because this module's fourth decision is that
/// a receive cannot answer octets alone — a program that could not name the
/// sender could not reply to it, and replying is most of what a datagram socket
/// is for. Nothing here waits and nothing here reaches anything: the values are
/// already in hand by the time one of these exists.
pub(crate) const MESSAGE: CoreClass = CoreClass {
    name: MESSAGE_NAME,
    doc: Some(&MESSAGE_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "payload",
            names: &[],
            params: &[],
            // `rule:security/tainted-qualifier`, on `Core\Net\Stream::read`'s
            // reading: octets off a socket are as untrusted as a request
            // body's, and a datagram's sender is even less accountable for
            // them.
            return_ty: CoreTy::TaintedBytes,
            defaults: &[],
            symbol: "nvs_core_net_message_payload",
            doc: Some(&MESSAGE_PAYLOAD_DOC),
        },
        CoreMethod {
            name: "host",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_net_message_host",
            doc: Some(&MESSAGE_HOST_DOC),
        },
        CoreMethod {
            name: "port",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_net_message_port",
            doc: Some(&MESSAGE_PORT_DOC),
        },
    ],
    slots: &["payload", "host", "port"],
    constants: &[],
};

/// `Core\Net\Datagram\Message`'s class card — `rule:core-api/reference-card`.
const MESSAGE_CARD: ClassDoc = ClassDoc {
    short: "One message that `Core\\Net\\Datagram::receive` returned. `payload` gives its bytes, \
            and `host` and `port` give the address it came from, which is where a reply goes.",
};

/// `Core\Net\Datagram\Message::payload`'s reference card —
/// `rule:core-api/reference-card`.
const MESSAGE_PAYLOAD_DOC: MethodDoc = MethodDoc {
    short: "The octets this datagram carried, as `tainted bytes`. Truncated to the `$max` the \
            receive named, if the message was longer than that.",
    params: &[],
    ret: "What arrived, which may be empty: a zero-length datagram is a message and not an \
          absence.",
    errors: &[],
};

/// `Core\Net\Datagram\Message::host`'s reference card —
/// `rule:core-api/reference-card`.
const MESSAGE_HOST_DOC: MethodDoc = MethodDoc {
    short: "The address this datagram came from, written out — `127.0.0.1`, `::1`. A plain \
            `string`, so it can be handed straight back to `send`, which asks the grant and the \
            address policy about it exactly as it would about any other address.",
    params: &[],
    ret: "An address literal, never a hostname: nothing here is resolved backwards.",
    errors: &[],
};

/// `Core\Net\Datagram\Message::port`'s reference card —
/// `rule:core-api/reference-card`.
const MESSAGE_PORT_DOC: MethodDoc = MethodDoc {
    short: "The port this datagram came from, which is where a reply goes.",
    params: &[],
    ret: "The sender's port, 1 to 65535.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_net_connect" => (nvs_core_net_connect as *const ()).cast(),
        "nvs_core_net_listen" => (nvs_core_net_listen as *const ()).cast(),
        "nvs_core_net_bind_datagram" => (nvs_core_net_bind_datagram as *const ()).cast(),
        "nvs_core_net_connect_local" => (nvs_core_net_connect_local as *const ()).cast(),
        "nvs_core_net_listen_local" => (nvs_core_net_listen_local as *const ()).cast(),
        "nvs_core_net_stream_read" => (nvs_core_net_stream_read as *const ()).cast(),
        "nvs_core_net_stream_write" => (nvs_core_net_stream_write as *const ()).cast(),
        "nvs_core_net_stream_close" => (nvs_core_net_stream_close as *const ()).cast(),
        "nvs_core_net_listener_accept" => (nvs_core_net_listener_accept as *const ()).cast(),
        "nvs_core_net_listener_port" => (nvs_core_net_listener_port as *const ()).cast(),
        "nvs_core_net_listener_close" => (nvs_core_net_listener_close as *const ()).cast(),
        "nvs_core_net_datagram_send" => (nvs_core_net_datagram_send as *const ()).cast(),
        "nvs_core_net_datagram_receive" => (nvs_core_net_datagram_receive as *const ()).cast(),
        "nvs_core_net_datagram_port" => (nvs_core_net_datagram_port as *const ()).cast(),
        "nvs_core_net_datagram_close" => (nvs_core_net_datagram_close as *const ()).cast(),
        "nvs_core_net_message_payload" => (nvs_core_net_message_payload as *const ()).cast(),
        "nvs_core_net_message_host" => (nvs_core_net_message_host as *const ()).cast(),
        "nvs_core_net_message_port" => (nvs_core_net_message_port as *const ()).cast(),
        _ => return None,
    })
}

/// A connected socket, in the shape the request's table holds — over TCP or
/// over a socket path, which is one class to the program either way.
///
/// A type of this crate's rather than an impl on [`NvsTcp`]: neither that type
/// nor [`HeldSocket`] is local here, so the impl has to be written on something
/// that is. What it buys back is that the handle classes are distinct types, so
/// a listener's key read as a stream's fails the downcast instead of answering.
///
/// An enum and not a `dyn Read + Write`, for the reason `nvs_host::net`'s docs
/// § *One type over the source* give for a generic: the parking path takes no
/// vtable, and a socket costs exactly what it did
/// (`rule:programs/memory-priority`). This module's first decision is why the
/// two transports are one class at all.
#[derive(Debug)]
enum Connected {
    /// A TCP connection — [`nvs_core_net_connect`]'s, or one an accept took off
    /// a listening port.
    Tcp(NvsTcp),
    /// A Unix-domain connection — [`nvs_core_net_connect_local`]'s, or one an
    /// accept took off a local listener.
    #[cfg(unix)]
    Local(NvsUnix),
}

impl Connected {
    /// Bounds the next wait on this socket by `at`, or lifts the bound —
    /// `nvs_host`'s `NvsStream::set_deadline`, over whichever transport this is.
    fn set_deadline(&mut self, at: Option<Instant>) {
        match self {
            Self::Tcp(stream) => stream.set_deadline(at),
            #[cfg(unix)]
            Self::Local(stream) => stream.set_deadline(at),
        }
    }
}

impl Read for Connected {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.read(buffer),
            #[cfg(unix)]
            Self::Local(stream) => stream.read(buffer),
        }
    }
}

impl Write for Connected {
    fn write(&mut self, payload: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.write(payload),
            #[cfg(unix)]
            Self::Local(stream) => stream.write(payload),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Tcp(stream) => stream.flush(),
            #[cfg(unix)]
            Self::Local(stream) => stream.flush(),
        }
    }
}

impl HeldSocket for Connected {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A bound listening socket, in the shape the request's table holds —
/// [`Connected`]'s twin, over the same two transports and for its reasons.
#[derive(Debug)]
enum Bound {
    /// A listening TCP port — [`nvs_core_net_listen`]'s.
    Tcp(NvsListener),
    /// A listening Unix-domain socket — [`nvs_core_net_listen_local`]'s.
    #[cfg(unix)]
    Local(NvsUnixListener),
}

impl Bound {
    /// Bounds the next wait on this listener by `at`, or lifts the bound.
    fn set_deadline(&mut self, at: Option<Instant>) {
        match self {
            Self::Tcp(listener) => listener.set_deadline(at),
            #[cfg(unix)]
            Self::Local(listener) => listener.set_deadline(at),
        }
    }

    /// The next connection, in the transport this listener is bound over.
    ///
    /// The peer is dropped rather than answered: what a program does with an
    /// accepted connection is read and write it, and the address it arrived
    /// from is the one member `rule:core-api/shape-rules` R5 would have this
    /// class answer only if a caller could do something with it.
    ///
    /// # Errors
    ///
    /// The platform's, `TimedOut` included once the deadline above has passed.
    fn accept(&mut self) -> std::io::Result<Connected> {
        match self {
            Self::Tcp(listener) => listener
                .accept()
                .map(|(stream, _peer)| Connected::Tcp(stream)),
            #[cfg(unix)]
            Self::Local(listener) => listener
                .accept()
                .map(|(stream, _peer)| Connected::Local(stream)),
        }
    }

    /// The port this listener is bound to, or `None` for one bound at a path,
    /// which has no port — this module's first decision is why that is a
    /// refusal at the member and not a `null`.
    ///
    /// # Errors
    ///
    /// The platform's answer for a socket it no longer holds.
    fn port(&self) -> std::io::Result<Option<u16>> {
        match self {
            Self::Tcp(listener) => listener.local_addr().map(|bound| Some(bound.port())),
            #[cfg(unix)]
            Self::Local(_) => Ok(None),
        }
    }
}

impl HeldSocket for Bound {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A bound datagram socket, in the shape the request's table holds — the third
/// of [`Connected`]'s family, whose doc is the home of why each is a newtype.
#[derive(Debug)]
struct Datagrams(NvsUdp);

impl HeldSocket for Datagrams {
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
            "{member} expected {:?} for its port, got tag {}",
            Tag::Uint,
            args[at].tag_byte()
        ))
    })?;
    let port = u16::try_from(asked).unwrap_or(0);
    if u64::from(port) != asked || (port == 0 && !zero) {
        return Err(Fault::thrown(format!("{member}: {asked} is not a port")));
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
                "{member}: a bound must be a positive length of time"
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
fn stream_of<'a>(ctx: &'a mut Ctx, key: u64, member: &str) -> Result<&'a mut Connected, Fault> {
    let held = ctx
        .open_socket_mut(key)
        .ok_or_else(|| already_closed(&STREAM, member))?;
    held.as_any_mut()
        .downcast_mut::<Connected>()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{STREAM_NAME}::{member} found a listener at its key"
            ))
        })
}

/// The listening socket `key` names, borrowed for one call — [`stream_of`]'s
/// twin, with its errors.
fn listener_of<'a>(ctx: &'a mut Ctx, key: u64, member: &str) -> Result<&'a mut Bound, Fault> {
    let held = ctx
        .open_socket_mut(key)
        .ok_or_else(|| already_closed(&LISTENER, member))?;
    held.as_any_mut().downcast_mut::<Bound>().ok_or_else(|| {
        Fault::fatal(format!(
            "{LISTENER_NAME}::{member} found a connection at its key"
        ))
    })
}

/// The datagram socket `key` names, borrowed for one call — [`stream_of`]'s
/// twin over the third transport, with its errors.
fn datagram_of<'a>(ctx: &'a mut Ctx, key: u64, member: &str) -> Result<&'a mut NvsUdp, Fault> {
    let held = ctx
        .open_socket_mut(key)
        .ok_or_else(|| already_closed(&DATAGRAM, member))?;
    held.as_any_mut()
        .downcast_mut::<Datagrams>()
        .map(|datagrams| &mut datagrams.0)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{DATAGRAM_NAME}::{member} found a connected socket at its key"
            ))
        })
}

/// A slot of the receiving [`MESSAGE`], retained because it is being answered.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not an object — unreachable from
/// source, since an instance member's receiver is typed and the checker refuses
/// a call on anything else.
fn message_slot(args: &[Value], index: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &MESSAGE, member)?;
    let held = crate::instance::slot(receiver, index);
    #[expect(
        unsafe_code,
        reason = "the slot's reference belongs to the receiver, which is live for \
                  the length of the call, and this value is being handed to the \
                  caller — which is exactly `Value::retain`'s obligation"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
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
        let port = port_of(args, 1, MEMBER, false)?;
        let bound = within(args, 2, MEMBER)?;
        let address = nvs_runtime::capability::pin_host(ctx, host, MEMBER)?;
        let socket = NvsTcp::connect_timeout(SocketAddr::new(address, port), bound)
            .map_err(|err| failed(MEMBER, &err))?;
        Ok(handle(ctx, &STREAM, Box::new(Connected::Tcp(socket))))
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
        let port = port_of(args, 1, MEMBER, true)?;
        let address: IpAddr = written.parse().map_err(|_| {
            Fault::thrown(format!(
                "{MEMBER}: `{written}` is not an address literal — a bind names an endpoint, and \
                 a hostname is not one"
            ))
        })?;
        let endpoint = SocketAddr::new(address, port);
        nvs_runtime::capability::require(ctx, Cap::NetListen, Scope::Endpoint(endpoint), MEMBER)?;
        let socket = NvsListener::bind(endpoint).map_err(|err| failed(MEMBER, &err))?;
        Ok(handle(ctx, &LISTENER, Box::new(Bound::Tcp(socket))))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net::connectLocal(string $path, Core\Time\Duration $within): Core\Net\Stream`
    /// — replacing `stream_socket_client`'s `unix://` half and `socket_connect`
    /// over `AF_UNIX`.
    ///
    /// **`net.local` and not `net.connect`**, asked at `Scope::Path` under
    /// `rule:security/path-scope-canonicalise-then-prefix`:
    /// `rule:config/net-local-is-named-and-not-on-the-roster` is the grant, and
    /// it is separate because `net.connect` carries an address policy and a
    /// path has no address for that policy to read. A grant of one kind
    /// covering entries of two would be one name carrying two guarantees.
    ///
    /// It is a **separate member** and not `connect` widened. Nothing here
    /// reads its argument to decide a transport, so
    /// `rule:security/a-path-is-not-a-url` holds by construction rather than by
    /// a check — and `Core\Net::connect` still refuses a path outright, since
    /// its door cannot authorize one.
    ///
    /// The grant is asked **before** the transport is looked for, so a build
    /// with no `AF_UNIX` answers an ungranted program with the missing grant
    /// and not with a fact about this host.
    fn nvs_core_net_connect_local(ctx, args: [2]) {
        const MEMBER: &str = r"Core\Net::connectLocal";

        let path = text(&args[0], MEMBER, "path")?;
        let bound = within(args, 1, MEMBER)?;
        nvs_runtime::capability::require(
            ctx,
            Cap::NetLocal,
            Scope::Path(std::path::Path::new(path)),
            MEMBER,
        )?;
        let socket = local_connection(path, bound, MEMBER)?;
        Ok(handle(ctx, &STREAM, Box::new(socket)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net::listenLocal(string $path): Core\Net\Listener` — replacing
    /// `stream_socket_server`'s `unix://` half.
    ///
    /// **The same grant as [`nvs_core_net_connect_local`]**, and that is the
    /// rule rather than a convenience:
    /// `rule:config/net-local-is-named-and-not-on-the-roster` governs both ends
    /// of a path, because a socket a program creates is one that whatever else
    /// on this host finds it may speak to, and a path the operator did not
    /// write down is one they cannot have intended.
    ///
    /// **This member takes no bound**, for [`nvs_core_net_listen`]'s reason: a
    /// bind either succeeds or is refused on the spot. Waiting for a connection
    /// is `Core\Net\Listener::accept`, which takes its own.
    fn nvs_core_net_listen_local(ctx, args: [1]) {
        const MEMBER: &str = r"Core\Net::listenLocal";

        let path = text(&args[0], MEMBER, "path")?;
        nvs_runtime::capability::require(
            ctx,
            Cap::NetLocal,
            Scope::Path(std::path::Path::new(path)),
            MEMBER,
        )?;
        let socket = local_listener(path, MEMBER)?;
        Ok(handle(ctx, &LISTENER, Box::new(socket)))
    }
}

/// The local connection at `path`, waited out under `bound`.
///
/// A `cfg` split and not a run-time check, because there is nothing to check at
/// run time: `mio` carries no `AF_UNIX` transport on Windows, so a local socket
/// cannot be parked on this runtime's reactor there at all. `nvs_host::net`'s
/// `NvsUnix` is the home of why a type that compiles and cannot be polled is
/// worse than a name that is not there.
///
/// # Errors
///
/// The platform's, as [`failed`] shapes it — nothing bound at the path, no
/// permission, a name too long — or [`no_local_transport`] where there is no
/// transport to open one over.
#[cfg(unix)]
fn local_connection(path: &str, bound: Duration, member: &str) -> Result<Connected, Fault> {
    NvsUnix::connect_timeout(path, bound)
        .map(Connected::Local)
        .map_err(|err| failed(member, &err))
}

/// [`local_connection`] where the platform has no Unix-domain transport.
///
/// # Errors
///
/// Always [`no_local_transport`], which is that whole answer.
#[cfg(not(unix))]
fn local_connection(_path: &str, _bound: Duration, member: &str) -> Result<Connected, Fault> {
    Err(no_local_transport(member))
}

/// The listening socket bound at `path` — [`local_connection`]'s twin, and the
/// home of why the split is a `cfg` is that function's doc.
///
/// # Errors
///
/// The platform's, as [`failed`] shapes it — the path exists, its directory
/// does not, no permission — or [`no_local_transport`].
#[cfg(unix)]
fn local_listener(path: &str, member: &str) -> Result<Bound, Fault> {
    NvsUnixListener::bind(path)
        .map(Bound::Local)
        .map_err(|err| failed(member, &err))
}

/// [`local_listener`] where the platform has no Unix-domain transport.
///
/// # Errors
///
/// Always [`no_local_transport`].
#[cfg(not(unix))]
fn local_listener(_path: &str, member: &str) -> Result<Bound, Fault> {
    Err(no_local_transport(member))
}

/// What both local doors answer on a build with no `AF_UNIX` transport.
///
/// A `RuntimeError` and not an `IOError` for [`already_closed`]'s reason:
/// nothing about a socket went wrong, because there was no socket to open — the
/// program asked this build for something it does not carry.
#[cfg(not(unix))]
fn no_local_transport(member: &str) -> Fault {
    Fault::thrown(format!(
        "{member}: this build has no Unix-domain transport, so there is no socket path it could \
         open — a program that needs a local transport on every platform reaches loopback with \
         `Core\\Net::connect`"
    ))
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
        let bound = within(args, 2, MEMBER)?;
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
        let bound = within(args, 2, MEMBER)?;
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
        let bound = within(args, 1, MEMBER)?;
        let listener = listener_of(ctx, key, "accept")?;
        listener.set_deadline(Some(Instant::now() + bound));
        let outcome = listener.accept();
        listener.set_deadline(None);
        let socket = outcome.map_err(|err| failed(MEMBER, &err))?;
        Ok(handle(ctx, &STREAM, Box::new(socket)))
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
    ///
    /// A listener `Core\Net::listenLocal` bound is the one this cannot answer,
    /// and it throws: a socket at a path has no port, and this module's first
    /// decision is why that is a refusal rather than a nullable answer every
    /// TCP caller would carry.
    fn nvs_core_net_listener_port(ctx, args: [1]) {
        const MEMBER: &str = r"Core\Net\Listener::port";

        let key = key_of(args[0], &LISTENER, "port")?;
        let listener = listener_of(ctx, key, "port")?;
        match listener.port().map_err(|err| failed(MEMBER, &err))? {
            Some(port) => Ok(Value::uint(u64::from(port))),
            None => Err(Fault::thrown(format!(
                "{MEMBER}: this listener is bound at a socket path, which has no port — a local \
                 socket is reached by the path the program passed to `Core\\Net::listenLocal`"
            ))),
        }
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

nvs_runtime::nvs_helper! {
    /// `Core\Net::bindDatagram(string $address, uint $port): Core\Net\Datagram`
    /// — replacing `stream_socket_server`'s `udp://` half and `socket_bind`.
    ///
    /// [`nvs_core_net_listen`]'s body over [`NvsUdp`], asking the same grant of
    /// the same parsed endpoint and resolving no name for the same reason. What
    /// decides the capability is what the program is doing —
    /// `rule:security/net-listen-is-a-separate-grant-from-net-connect` — and
    /// binding an endpoint is binding an endpoint whatever transport is bound
    /// at it.
    ///
    /// **A datagram socket is bound and never connected**, which is why there
    /// is no outbound entry point beside this one: the address a program sends
    /// to is named at the send, and [`nvs_core_net_datagram_send`] is where the
    /// outbound grant is therefore asked.
    fn nvs_core_net_bind_datagram(ctx, args: [2]) {
        const MEMBER: &str = r"Core\Net::bindDatagram";

        let written = text(&args[0], MEMBER, "address")?;
        let port = port_of(args, 1, MEMBER, true)?;
        let address: IpAddr = written.parse().map_err(|_| {
            Fault::thrown(format!(
                "{MEMBER}: `{written}` is not an address literal — a bind names an endpoint, and \
                 a hostname is not one"
            ))
        })?;
        let endpoint = SocketAddr::new(address, port);
        nvs_runtime::capability::require(ctx, Cap::NetListen, Scope::Endpoint(endpoint), MEMBER)?;
        let socket = NvsUdp::bind(endpoint).map_err(|err| failed(MEMBER, &err))?;
        Ok(handle(ctx, &DATAGRAM, Box::new(Datagrams(socket))))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net\Datagram::send(string $host, uint $port, bytes $payload, Core\Time\Duration $within): uint`
    /// — replacing `stream_socket_sendto` and `socket_sendto`.
    ///
    /// **The outbound grant is asked here rather than at the bind**, which is
    /// `rule:security/net-address-policy`'s own sentence: a datagram sent to an
    /// address the program supplied is asked at the send what a TCP connect is
    /// asked at the connect. `nvs_runtime::capability::pin_host` is the one call
    /// [`nvs_core_net_connect`] makes and does the same three things in the same
    /// order — `net.connect` about the host, resolve, then the denied-range
    /// table about the one address that came back — and the datagram goes to
    /// **that address** rather than to the name again.
    ///
    /// One call is one datagram and there is no partial send. The count comes
    /// back rather than being asserted because it is what the platform said, and
    /// a message too large for the path is refused rather than split — so unlike
    /// [`nvs_core_net_stream_write`], a caller has nothing to loop over.
    fn nvs_core_net_datagram_send(ctx, args: [5]) {
        const MEMBER: &str = r"Core\Net\Datagram::send";

        let key = key_of(args[0], &DATAGRAM, "send")?;
        let host = text(&args[1], MEMBER, "host")?;
        let port = port_of(args, 2, MEMBER, false)?;
        let payload = args[3].as_bytes().ok_or_else(|| {
            Fault::fatal(format!(
                "{MEMBER} expected {:?} for its payload, got tag {}",
                Tag::Bytes,
                args[3].tag_byte()
            ))
        })?;
        let bound = within(args, 4, MEMBER)?;
        let address = nvs_runtime::capability::pin_host(ctx, host, MEMBER)?;
        let target = SocketAddr::new(address, port);
        let socket = datagram_of(ctx, key, "send")?;
        socket.set_deadline(Some(Instant::now() + bound));
        let outcome = socket.send_to(payload, target);
        socket.set_deadline(None);
        let sent = outcome.map_err(|err| failed(MEMBER, &err))?;
        Ok(Value::uint(sent as u64))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net\Datagram::receive(uint $max, Core\Time\Duration $within): Core\Net\Datagram\Message`
    /// — replacing `stream_socket_recvfrom` and `socket_recvfrom`.
    ///
    /// **It asks no capability**, for [`nvs_core_net_listener_accept`]'s reason
    /// over the transport that has no accept: the bind was granted when this
    /// socket was opened, and who sent a datagram that arrived was not this
    /// program's choice to make.
    ///
    /// A datagram is delivered whole or not at all, so a message longer than
    /// `$max` keeps what fits and **the rest are gone**. That is the difference
    /// from [`nvs_core_net_stream_read`], where a short answer is ordinary
    /// because the remainder arrives next time, and it is the caller's to size
    /// for.
    ///
    /// **The cut is made here rather than by the kernel**, which is
    /// [`DATAGRAM_CEILING`]'s whole reason: handed a short buffer, the Unixes
    /// truncate and Windows refuses the call, and a member answering differently
    /// on two hosts is the one thing this may not do. So the socket is always
    /// given a buffer no datagram can overflow, and `$max` is applied to what
    /// came back.
    ///
    /// **What it spends:** one [`DATAGRAM_CEILING`] buffer for the length of the
    /// call, plus the message it answers with, both charged to the request that
    /// asked. It is a fixed 64 KiB rather than the `$max` a program named,
    /// because that is what the paragraph above costs.
    fn nvs_core_net_datagram_receive(ctx, args: [3]) {
        const MEMBER: &str = r"Core\Net\Datagram::receive";

        let key = key_of(args[0], &DATAGRAM, "receive")?;
        let max = args[1].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "{MEMBER} expected {:?} for its max, got tag {}",
                Tag::Uint,
                args[1].tag_byte()
            ))
        })?;
        let bound = within(args, 2, MEMBER)?;
        let wanted = usize::try_from(max)
            .unwrap_or(DATAGRAM_CEILING)
            .min(DATAGRAM_CEILING);
        let socket = datagram_of(ctx, key, "receive")?;
        let mut buffer = vec![0u8; DATAGRAM_CEILING];
        socket.set_deadline(Some(Instant::now() + bound));
        let outcome = socket.recv_from(&mut buffer);
        socket.set_deadline(None);
        let (read, from) = outcome.map_err(|err| failed(MEMBER, &err))?;
        let kept = read.min(wanted);
        let sender = from.ip().to_string();
        Ok(crate::instance::build(
            &MESSAGE,
            [
                Value::bytes(NvsStr::new(&buffer[..kept])),
                Value::str(NvsStr::new(sender.as_bytes())),
                Value::uint(u64::from(from.port())),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net\Datagram::port(): uint` — replacing `stream_socket_get_name`'s
    /// local half over the third transport.
    ///
    /// [`nvs_core_net_listener_port`]'s member and its reasoning: it is what
    /// makes a `0` port usable, and the address is deliberately not answered
    /// beside it because the program wrote that itself.
    fn nvs_core_net_datagram_port(ctx, args: [1]) {
        const MEMBER: &str = r"Core\Net\Datagram::port";

        let key = key_of(args[0], &DATAGRAM, "port")?;
        let socket = datagram_of(ctx, key, "port")?;
        let bound = socket.local_addr().map_err(|err| failed(MEMBER, &err))?;
        Ok(Value::uint(u64::from(bound.port())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net\Datagram::close(): void` — replacing `fclose` on a datagram
    /// socket.
    ///
    /// [`nvs_core_net_stream_close`]'s body and its reasoning over the third
    /// handle class. Messages already received are untouched: each is an object
    /// holding copies of what arrived, and nothing in one points at the socket.
    fn nvs_core_net_datagram_close(ctx, args: [1]) {
        let key = key_of(args[0], &DATAGRAM, "close")?;
        let socket = ctx
            .take_open_socket(key)
            .ok_or_else(|| already_closed(&DATAGRAM, "close"))?;
        drop(socket);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net\Datagram\Message::payload(): tainted bytes` — the octets half
    /// of what `stream_socket_recvfrom` returns.
    ///
    /// A slot read and nothing else: the truncation `$max` decides happened at
    /// the receive, so what is here is already the whole of the answer.
    fn nvs_core_net_message_payload(_ctx, args: [1]) {
        message_slot(args, PAYLOAD_SLOT, "payload")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net\Datagram\Message::host(): string` — the address half of what
    /// `stream_socket_recvfrom` delivers through an out-parameter.
    ///
    /// A plain `string` rather than a `tainted` one, which this module's fourth
    /// decision is the home of: `send`'s host is a sink and there is no
    /// launderer for an address, so a qualified answer here would be a datagram
    /// socket that cannot reply. The grant is what guards the reply instead.
    fn nvs_core_net_message_host(_ctx, args: [1]) {
        message_slot(args, HOST_SLOT, "host")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Net\Datagram\Message::port(): uint` — the other half of the
    /// endpoint a reply is addressed to.
    fn nvs_core_net_message_port(_ctx, args: [1]) {
        message_slot(args, PORT_SLOT, "port")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CLASS, DATAGRAM, DATAGRAM_NAME, LISTENER, LISTENER_NAME, MESSAGE, MESSAGE_NAME, NAME,
        STREAM, STREAM_NAME, nvs_core_net_bind_datagram, nvs_core_net_connect,
        nvs_core_net_connect_local, nvs_core_net_datagram_close, nvs_core_net_datagram_port,
        nvs_core_net_datagram_receive, nvs_core_net_datagram_send, nvs_core_net_listen,
        nvs_core_net_listen_local, nvs_core_net_listener_accept, nvs_core_net_listener_close,
        nvs_core_net_listener_port, nvs_core_net_message_host, nvs_core_net_message_payload,
        nvs_core_net_message_port, nvs_core_net_stream_close, nvs_core_net_stream_read,
        nvs_core_net_stream_write,
    };
    use nvs_runtime::{Ctx, NvsStr, Value};

    /// One second, as the bound every waiting member takes. Short because no
    /// case here waits it out: what is being asserted is which door refuses,
    /// and a refusal arrives before anything is waited on.
    fn a_second() -> Value {
        crate::time::duration_of(1_000_000_000)
    }

    /// Gives back the references a case built for its arguments.
    fn released<const N: usize>(values: [Value; N]) {
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
    // covers: Core\Net::connect
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
    // covers: Core\Net::listen
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

    /// The datagram half, end to end and through the doors a program uses: two
    /// bound sockets, a message from one to the other, and a reply addressed
    /// with what the message answered.
    ///
    /// **Off a core there is no coroutine to suspend**, so what waits here is
    /// `nvs_host::net`'s blocking path rather than the reactor's park — the same
    /// four functions either way, which is the whole of why `NvsUdp` is an alias
    /// over `NvsStream` rather than a second transport. That the park itself
    /// works is `nvs-host`'s own
    /// `a_udp_socket_registers_with_the_reactor_and_parks_the_coroutine`; what
    /// is asserted here is that `Core\Net`'s doors reach it.
    ///
    /// The reply leg is not a second copy of the first: it is what makes
    /// `Core\Net\Datagram\Message::host` a plain `string` rather than a
    /// `tainted` one, since a qualified answer could not be handed to `send`'s
    /// sink at all.
    // covers: Core\Net::bindDatagram, Core\Net\Datagram::send, Core\Net\Datagram::receive, Core\Net\Datagram::port, Core\Net\Datagram::close, Core\Net\Datagram\Message::payload, Core\Net\Datagram\Message::host, Core\Net\Datagram\Message::port
    #[test]
    fn a_udp_socket_sends_and_receives_over_the_runtimes_own_reactor() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(
            "[capabilities.net]\nlisten = true\nconnect = [\"127.0.0.1\"]\ninternal = [\"127.0.0.1\"]\n",
        ));

        // Five seconds rather than [`a_second`]: this case genuinely waits on a
        // socket, and the bound is a ceiling on a failure rather than a figure
        // anything here is measuring.
        let within = crate::time::duration_of(5_000_000_000);
        let loopback = Value::str(NvsStr::new(b"127.0.0.1"));
        let bind = |ctx: &mut Ctx| {
            nvs_runtime::call(nvs_core_net_bind_datagram, ctx, &[loopback, Value::uint(0)])
                .expect("a granted bind on an ephemeral port")
        };
        let bound_port = |ctx: &mut Ctx, socket: Value| {
            nvs_runtime::call(nvs_core_net_datagram_port, ctx, &[socket])
                .expect("a bound socket answers its port")
                .as_uint()
                .expect("a port is a uint")
        };

        let alpha = bind(&mut ctx);
        let beta = bind(&mut ctx);
        let alpha_port = bound_port(&mut ctx, alpha);
        let beta_port = bound_port(&mut ctx, beta);
        assert!(
            alpha_port > 0 && beta_port > 0 && alpha_port != beta_port,
            "port `0` asks the operating system to pick, and it picks two"
        );

        let ping = Value::bytes(NvsStr::new(b"ping"));
        let sent = nvs_runtime::call(
            nvs_core_net_datagram_send,
            &mut ctx,
            &[alpha, loopback, Value::uint(beta_port), ping, within],
        )
        .expect("a granted send to a granted address")
        .as_uint()
        .expect("a count is a uint");
        assert_eq!(
            sent, 4,
            "one call is one datagram, sent whole or not at all"
        );

        let message = nvs_runtime::call(
            nvs_core_net_datagram_receive,
            &mut ctx,
            &[beta, Value::uint(64), within],
        )
        .expect("the datagram that was just sent");
        let heard = nvs_runtime::call(nvs_core_net_message_payload, &mut ctx, &[message])
            .expect("a message answers its payload");
        assert_eq!(heard.as_bytes(), Some(&b"ping"[..]));
        let from = nvs_runtime::call(nvs_core_net_message_host, &mut ctx, &[message])
            .expect("a message answers where it came from");
        assert_eq!(from.as_text(), Some("127.0.0.1"));
        let from_port = nvs_runtime::call(nvs_core_net_message_port, &mut ctx, &[message])
            .expect("a message answers the port it came from")
            .as_uint();
        assert_eq!(
            from_port,
            Some(alpha_port),
            "a receive that could not name its sender could not be replied to"
        );

        let pong = Value::bytes(NvsStr::new(b"pong"));
        nvs_runtime::call(
            nvs_core_net_datagram_send,
            &mut ctx,
            &[beta, from, Value::uint(alpha_port), pong, within],
        )
        .expect("the reply goes back to the endpoint the message named");
        let back = nvs_runtime::call(
            nvs_core_net_datagram_receive,
            &mut ctx,
            &[alpha, Value::uint(64), within],
        )
        .expect("the reply arrived");
        let echoed = nvs_runtime::call(nvs_core_net_message_payload, &mut ctx, &[back])
            .expect("a message answers its payload");
        assert_eq!(echoed.as_bytes(), Some(&b"pong"[..]));

        // Closing is taking the socket out of the request's table, and a second
        // close finds nothing there: a key is never reused, so this can only be
        // the same handle twice.
        nvs_runtime::call(nvs_core_net_datagram_close, &mut ctx, &[alpha])
            .expect("an open socket closes");
        nvs_runtime::call(nvs_core_net_datagram_close, &mut ctx, &[beta])
            .expect("an open socket closes");
        nvs_runtime::call(nvs_core_net_datagram_close, &mut ctx, &[alpha])
            .expect_err("a second close throws rather than succeeding quietly");

        released([within, loopback, ping, pong, heard, from, echoed]);
        released([message, back, alpha, beta]);
    }

    /// `Core\Net\Listener`'s three members over one socket, end to end: an
    /// accept with nobody connecting stops at its bound, the next one takes the
    /// connection that is waiting, and a closed listener answers nothing while
    /// the connection it accepted still carries bytes.
    ///
    /// The timeout comes first on purpose. `accept` sets the deadline for its
    /// one call and clears it after, so a listener that ran out of time once
    /// must still take the next connection — a deadline left behind would make
    /// the second accept fail at once.
    // covers: Core\Net\Listener::accept, Core\Net\Listener::port, Core\Net\Listener::close, Core\Net\Stream::read, Core\Net\Stream::write, Core\Net\Stream::close
    #[test]
    fn a_listener_times_out_takes_a_waiting_connection_and_refuses_once_closed() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(
            "[capabilities.net]\nlisten = true\nconnect = [\"127.0.0.1\"]\ninternal = [\"127.0.0.1\"]\n",
        ));
        let within = crate::time::duration_of(5_000_000_000);
        let briefly = crate::time::duration_of(50_000_000);
        let loopback = Value::str(NvsStr::new(b"127.0.0.1"));

        let door = nvs_runtime::call(nvs_core_net_listen, &mut ctx, &[loopback, Value::uint(0)])
            .expect("a granted bind on an ephemeral port");
        let port = nvs_runtime::call(nvs_core_net_listener_port, &mut ctx, &[door])
            .expect("a TCP listener answers its port")
            .as_uint()
            .expect("a port is a uint");
        assert!(
            (1..=65535).contains(&port),
            "port `0` asks the operating system to pick: {port}"
        );

        nvs_runtime::call(nvs_core_net_listener_accept, &mut ctx, &[door, briefly])
            .expect_err("nobody has connected yet");
        let waited = ctx.take_pending().expect("a message").into_owned();
        assert!(
            waited.contains(r"Core\Net\Listener::accept ran out of time"),
            "an empty backlog is a timeout, not an I/O failure: {waited}"
        );

        let client = nvs_runtime::call(
            nvs_core_net_connect,
            &mut ctx,
            &[loopback, Value::uint(port), within],
        )
        .expect("a granted connect to the listener's own port");
        let accepted = nvs_runtime::call(nvs_core_net_listener_accept, &mut ctx, &[door, within])
            .expect("the connection that is waiting");

        // The accepted handle is the server's end of the client's connection.
        let hello = Value::bytes(NvsStr::new(b"hello"));
        nvs_runtime::call(
            nvs_core_net_stream_write,
            &mut ctx,
            &[client, hello, within],
        )
        .expect("the client writes");
        let heard = nvs_runtime::call(
            nvs_core_net_stream_read,
            &mut ctx,
            &[accepted, Value::uint(64), within],
        )
        .expect("the server reads");
        assert_eq!(heard.as_bytes(), Some(&b"hello"[..]));

        // Closing the listener takes it out of the request's table, so each of
        // its members now finds nothing, and a second close is not quiet.
        nvs_runtime::call(nvs_core_net_listener_close, &mut ctx, &[door])
            .expect("an open listener closes");
        for (member, args) in [
            ("accept", &[door, within][..]),
            ("port", &[door][..]),
            ("close", &[door][..]),
        ] {
            let body = match member {
                "accept" => nvs_core_net_listener_accept,
                "port" => nvs_core_net_listener_port,
                _ => nvs_core_net_listener_close,
            };
            nvs_runtime::call(body, &mut ctx, args).expect_err("the listener is closed");
            let refused = ctx.take_pending().expect("a message").into_owned();
            assert_eq!(
                refused,
                format!(r"Core\Net\Listener::{member}: this handle is closed")
            );
        }

        // What came through the door is its own socket and outlives it.
        let reply = Value::bytes(NvsStr::new(b"bye"));
        nvs_runtime::call(
            nvs_core_net_stream_write,
            &mut ctx,
            &[accepted, reply, within],
        )
        .expect("an accepted connection outlives its listener");
        let back = nvs_runtime::call(
            nvs_core_net_stream_read,
            &mut ctx,
            &[client, Value::uint(64), within],
        )
        .expect("the client reads the reply");
        assert_eq!(back.as_bytes(), Some(&b"bye"[..]));

        nvs_runtime::call(nvs_core_net_stream_close, &mut ctx, &[accepted])
            .expect("an open connection closes");
        nvs_runtime::call(nvs_core_net_stream_close, &mut ctx, &[client])
            .expect("an open connection closes");
        released([within, briefly, loopback, hello, heard, reply, back]);
        released([door, client, accepted]);
    }

    /// The layout each handle class declares and the index its bodies read by
    /// are one decision written twice — the pairing
    /// [`crate::registry::CoreClass::slots`] exists to keep honest.
    #[test]
    fn every_handle_class_holds_its_socket_in_the_slot_it_declares() {
        assert_eq!(super::SOCKET_SLOT, STREAM.slot("socket"));
        assert_eq!(super::SOCKET_SLOT, LISTENER.slot("socket"));
        assert_eq!(super::SOCKET_SLOT, DATAGRAM.slot("socket"));
        assert_eq!(super::PAYLOAD_SLOT, MESSAGE.slot("payload"));
        assert_eq!(super::HOST_SLOT, MESSAGE.slot("host"));
        assert_eq!(super::PORT_SLOT, MESSAGE.slot("port"));
        assert_eq!(STREAM.name, STREAM_NAME);
        assert_eq!(LISTENER.name, LISTENER_NAME);
        assert_eq!(DATAGRAM.name, DATAGRAM_NAME);
        assert_eq!(MESSAGE.name, MESSAGE_NAME);
        assert_eq!(CLASS.name, NAME);
    }

    /// `rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it`
    /// and `rule:config/net-local-is-named-and-not-on-the-roster`: a socket
    /// path a *program* supplied is refused at every door that takes a host or
    /// an endpoint, and the two doors that do take a path ask `net.local` for
    /// it — which neither network grant buys.
    ///
    /// Asked of every door as agreement rather than of one member at a time: a
    /// member that grew its own reading of a path would still look right on its
    /// own line. What the first assertion also pins is that the sentence the
    /// outbound door gives has **one home**,
    /// `nvs_runtime::capability::pinned_address`, which is the call
    /// `Core\Db::open`'s program-supplied target is refused by as well.
    #[test]
    fn a_program_supplied_unix_path_is_refused_as_a_target_at_every_door() {
        const PATH: &str = "/run/novis/app.sock";
        const GRANTED: &str = "[capabilities.net]\nconnect = true\nlisten = true\n";

        // Takes a host, and a host carries no separator: refused in front of
        // the resolution, so the answer is the same whether or not anything is
        // bound at the path.
        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(GRANTED));
        let args = [
            Value::str(NvsStr::new(PATH.as_bytes())),
            Value::uint(80),
            a_second(),
        ];
        nvs_runtime::call(nvs_core_net_connect, &mut ctx, &args)
            .expect_err("the widest `net.connect` authorizes no path");
        let by_connect = ctx.pending().expect("a message").into_owned();
        released(args);
        assert!(
            by_connect.contains("net.local") && by_connect.contains(PATH),
            "the refusal names the path and the grant that does answer one: {by_connect}"
        );

        let by_door = nvs_runtime::capability::pinned_address(&ctx, PATH, r"Core\Net::connect")
            .expect_err("the door refuses a path whichever member reached it");
        let nvs_runtime::Fault::Thrown(class, by_door) = by_door else {
            panic!("a path refusal is catchable — `rule:security/denial-is-a-runtime-error`");
        };
        assert_eq!(class, nvs_runtime::ThrownClass::Runtime);
        assert_eq!(
            by_connect, by_door,
            "the sentence is the shared door's own, so there is no second copy to keep in step"
        );

        // Takes an endpoint, which is parsed and never resolved. A path is not
        // one, and that is answered before any grant is consulted.
        for door in [nvs_core_net_listen, nvs_core_net_bind_datagram] {
            let mut ctx = Ctx::buffered();
            ctx.set_config(crate::tests::granting(GRANTED));
            let args = [Value::str(NvsStr::new(PATH.as_bytes())), Value::uint(0)];
            nvs_runtime::call(door, &mut ctx, &args).expect_err("a path is not an endpoint");
            let refused = ctx.pending().expect("a message").into_owned();
            released(args);
            assert!(
                refused.contains("is not an address literal") && refused.contains(PATH),
                "a bind names an endpoint, and says so of what it was handed: {refused}"
            );
        }

        // And the two that take a path ask the grant that governs one — at both
        // ends, since binding a path is granted exactly as connecting to one is.
        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(GRANTED));
        let args = [Value::str(NvsStr::new(PATH.as_bytes())), a_second()];
        nvs_runtime::call(nvs_core_net_connect_local, &mut ctx, &args)
            .expect_err("`net.connect` does not widen to admit a path");
        let by_local = ctx.pending().expect("a message").into_owned();
        released(args);

        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(GRANTED));
        let args = [Value::str(NvsStr::new(PATH.as_bytes()))];
        nvs_runtime::call(nvs_core_net_listen_local, &mut ctx, &args)
            .expect_err("and `net.listen` does not widen to admit one either");
        let by_bind = ctx.pending().expect("a message").into_owned();
        released(args);
        for refused in [&by_local, &by_bind] {
            assert!(
                refused.contains("net.local") && refused.contains(PATH),
                "both ends name the one grant a path has, and the path: {refused}"
            );
        }

        // The declaration says the same thing, which is what a review reads.
        let asked = |member: &str| {
            crate::registry::CAPABILITIES
                .iter()
                .find(|(class, name, _)| *class == NAME && *name == member)
                .map(|(_, _, cap)| *cap)
                .expect("every member of a capability-bearing class has a row")
        };
        assert_eq!(asked("connectLocal"), Some(nvs_config::Cap::NetLocal));
        assert_eq!(asked("listenLocal"), Some(nvs_config::Cap::NetLocal));
    }

    /// A path `net.local` names gets past the grant at both ends and reaches
    /// the transport. On Unix the listener binds it and the connection meets
    /// it; where this build has no Unix-domain transport, both members answer
    /// with that fact, which they can only give once the grant has held.
    // covers: Core\Net::connectLocal, Core\Net::listenLocal
    #[test]
    fn a_granted_socket_path_reaches_the_transport_at_both_ends() {
        let path = std::env::temp_dir().join(format!("nvs-local-{}.sock", std::process::id()));
        // A run that was killed leaves the node behind, and a bind refuses it.
        let _ = std::fs::remove_file(&path);
        let path = path
            .to_str()
            .expect("the temporary directory is text")
            .to_owned();
        let written = format!("[capabilities.net]\nlocal = [{path:?}]\n");

        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(&written));
        let args = [Value::str(NvsStr::new(path.as_bytes()))];
        let bound = nvs_runtime::call(nvs_core_net_listen_local, &mut ctx, &args)
            .map_err(|_| ctx.take_pending().expect("a message").into_owned());
        released(args);
        let args = [Value::str(NvsStr::new(path.as_bytes())), a_second()];
        let connected = nvs_runtime::call(nvs_core_net_connect_local, &mut ctx, &args)
            .map_err(|_| ctx.take_pending().expect("a message").into_owned());
        released(args);

        if cfg!(unix) {
            let bound = bound.expect("a granted path the program may create");
            let connected = connected.expect("a granted path something is listening at");
            #[expect(unsafe_code, reason = "each handle owns the reference it releases")]
            unsafe {
                connected.release();
                bound.release();
            }
            let _ = std::fs::remove_file(&path);
        } else {
            for (member, answer) in [("listenLocal", bound), ("connectLocal", connected)] {
                let refused = answer.expect_err("no transport to open a path over");
                assert!(
                    refused.contains(member) && refused.contains("no Unix-domain transport"),
                    "the grant held, so the answer is about the transport: {refused}"
                );
            }
        }
    }

    /// `rule:security/a-path-is-not-a-url`: no door reads a scheme prefix, so
    /// `unix:`, `tcp:` and `php:` are ordinary text inside whatever argument
    /// they arrived in.
    ///
    /// The evidence is that the whole string survives into every refusal,
    /// scheme and all: a door that dispatched on the prefix would have consumed
    /// it, and one that stripped it would be answering about a target the
    /// program did not name. Asked of all five doors at once, as agreement.
    #[test]
    fn no_door_dispatches_on_a_url_scheme() {
        const GRANTED: &str = "[capabilities.net]\nconnect = true\nlisten = true\n";

        for written in [
            "unix:/run/novis/app.sock",
            "tcp://127.0.0.1:80",
            "php://filter/read=convert.base64-encode/resource=/etc/passwd",
        ] {
            let mut ctx = Ctx::buffered();
            ctx.set_config(crate::tests::granting(GRANTED));
            let args = [
                Value::str(NvsStr::new(written.as_bytes())),
                Value::uint(80),
                a_second(),
            ];
            nvs_runtime::call(nvs_core_net_connect, &mut ctx, &args)
                .expect_err("a scheme is not a transport selector and this is not a host");
            let refused = ctx.pending().expect("a message").into_owned();
            released(args);
            assert!(
                refused.contains(written),
                "the outbound door names what it was handed, whole: {refused}"
            );

            for door in [nvs_core_net_listen, nvs_core_net_bind_datagram] {
                let mut ctx = Ctx::buffered();
                ctx.set_config(crate::tests::granting(GRANTED));
                let args = [Value::str(NvsStr::new(written.as_bytes())), Value::uint(0)];
                nvs_runtime::call(door, &mut ctx, &args)
                    .expect_err("an endpoint is parsed, and a URL does not parse as one");
                let refused = ctx.pending().expect("a message").into_owned();
                released(args);
                assert!(
                    refused.contains("is not an address literal") && refused.contains(written),
                    "a bind reads its argument as one address and nothing else: {refused}"
                );
            }

            // The two path doors are the ones that could plausibly strip a
            // scheme, and do not: the grant is asked about the whole string.
            let mut ctx = Ctx::buffered();
            ctx.set_config(crate::tests::granting(GRANTED));
            let args = [Value::str(NvsStr::new(written.as_bytes())), a_second()];
            nvs_runtime::call(nvs_core_net_connect_local, &mut ctx, &args)
                .expect_err("no grant here covers a path");
            let by_local = ctx.pending().expect("a message").into_owned();
            released(args);

            let mut ctx = Ctx::buffered();
            ctx.set_config(crate::tests::granting(GRANTED));
            let args = [Value::str(NvsStr::new(written.as_bytes()))];
            nvs_runtime::call(nvs_core_net_listen_local, &mut ctx, &args)
                .expect_err("nor at the other end");
            let by_bind = ctx.pending().expect("a message").into_owned();
            released(args);
            for refused in [&by_local, &by_bind] {
                assert!(
                    refused.contains(written),
                    "the path is the argument as written, scheme included: {refused}"
                );
            }
        }
    }
}
