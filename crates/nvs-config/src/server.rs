//! `rule:http-server/the-server-block-is-boot-class`'s `[server]` block, read into what a server starts on: the idle waits and the
//! drain period as durations — refusing the magnitudes that would leave a connection unbounded —
//! and `listen` as the sockets to bind.
//!
//! Those, along with `max_in_flight`, `workers`, `socket_mode`, `health_path` and the
//! `[server.connection]` bounds, are the only parts of `[server]`
//! that resolve to something other than what was written *here*, so this module is small on purpose:
//! everything else in the block is a path or a word read directly off [`crate::tree::Server`]. The
//! mount table resolves as well, and it is [`mod@crate::mount`]'s because it needs a disk to
//! expand a glob against — [`validate`] runs the half of it that does not.
//!
//! **`max_in_flight` resolves to several numbers rather than to one**, which is
//! `rule:http-server/admission-is-arithmetic-not-a-number`
//! : the written ceiling, the cap one request may hold, and what this machine has. [`Capacity`]
//! is those and nothing more — the division is `nvs_server::admit`'s, because the clamp is an
//! admission decision and the counter that enforces it lives beside it. This module is the half
//! that reads a file and asks the operating system one question; it decides nothing.
//!
//! **Each of the waits is an *idle* wait and none is a total.** A slow 2 GB upload completes while
//! a stalled socket does not, which is § 5's own sentence and the reason the server refreshes a
//! deadline on every byte that moves rather than arming one when a connection is accepted.
//! `nvs_server::io`'s phase machine is the home of *which* wait is in force at a given moment;
//! this module only says how long each one is.
//!
//! **`drain_timeout` is the one period here that bounds a connection which is working.** It starts
//! when a connection sees the drain rather than when the drain began
//! (`rule:concurrency/a-drain-closes-a-connection-cleanly`), so it resolves like a wait and is
//! refused like one, and it is a directive of its own because an operator who lengthened keep-alive
//! to suit a proxy would otherwise have lengthened every restart. What makes a parked connection
//! see the drain at all is `nvs_runtime::Drain`'s wake, and what the connection does when it does
//! is `nvs_server::io`'s; this module is only the number.
//!
//! **`[server.connection]` resolves to overrides rather than to numbers**, which is the one shape
//! in this module that is not "read the file and hand back a value". Every bound on an open
//! connection is finite before anything is configured
//! (`rule:concurrency/connection-bounds-are-finite`), and `nvs_server::bounds::Connection` is
//! where each of those numbers is chosen and enforced — so [`connection_bounds_for`] refuses the
//! magnitudes a bound cannot have and says which keys were written, and holds no default of its
//! own. A copy of the shipped numbers here would be a second home for a set whose whole property
//! is that there is one.
//!
//! **`false` and `0` are both refused**, under `E0619`. Everywhere else in this tree `false`
//! removes a ceiling (`rule:config/three-changeability-classes`), and that spelling is exactly what
//! `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` has no version of: its
//! headline is that there is no way to say "wait forever", and a `[server]` wait is the inbound
//! half of it. Reading `false` as "keep the default" would be worse than refusing, because an
//! operator who wrote it asked for the one thing the ADR does not offer and would be told nothing.
//!
//! **A `listen` entry beginning with a path separator is a Unix socket**, which is § 5's own
//! overload and is unambiguous because no `host:port` can be spelled that way. A `host:port` is
//! parsed as a literal address and a host *name* is refused under `E0620`: a name that answers
//! with two addresses is two sockets rather than one, and a boot that resolves one has made the
//! server's start depend on a nameserver. Which of the classified entries a given process actually
//! binds is the caller's — [`Listen`] says what each entry *is* and nothing about how many cores
//! there are.
//!
//! **`socket_mode` is those entries' trust boundary and not a convenience.** A connection over a
//! Unix socket is implicitly trusted for the forwarded headers
//! (`rule:http-server/a-unix-socket-listener`) precisely because this mode decided who could
//! connect, so [`socket_mode_for`] refuses every spelling that is not a permission set rather than
//! reading one as far as it parses: each guess it could make is a different answer to that
//! question. `0660` where the key is absent, which is the one place in this module where the
//! default is narrower than the platform's own — the umask's `022` would create the socket
//! world-connectable.
//!
//! **`workers` is the one key here whose default this machine answers**, and it is a bound rather
//! than a request for one: with nothing written the count is
//! [`std::thread::available_parallelism`], and a written count is taken as it stands in both
//! directions. Nothing clamps it against the machine, because a heuristic that knew better than the
//! block would make the core count something the file cannot state — the same direction
//! `max_in_flight` goes the other way for, where the arithmetic is against a memory budget a
//! deployment cannot see. Only `0` is refused, under `E0636`.
//!
//! **`health_path` is off by default, and an empty string is that same off** — § 5 writes the key
//! out as `""`, so the state where no URL is reserved has to be reachable both by leaving the key
//! out and by transcribing the ADR's own example. What is written there is taken from every mount
//! this server answers, which is why a spelling no request could carry — a relative path, a query
//! or a fragment, a literal space — is refused under `E0623` rather than reserved and then never
//! reached: a probe that never matches leaves the server looking configured while nothing answers.
//! `/` on its own is refused from the other side, because it reserves every mount's own entry.
//!
//! Cost: one pass over one optional block at boot and at reload, and a `Duration` per wait plus
//! one address per written `listen` entry held per configuration generation. Nothing here runs on
//! a request path.
//!

use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Origin, origin_note};
use crate::tree::{Config, Setting};
use crate::value::{Quantity, Unit};

/// `rule:http-server/the-server-block-is-boot-class`'s waits, each a number — the whole clock one connection is bounded by.
///
/// Held by value and copied per configuration generation rather than borrowed from the tree, for
/// [`crate::queue::QueueBounds`]'s reason: `rule:config/the-config-is-an-immutable-snapshot`
/// 's reload replaces the tree whole, and these are `Boot`-class anyway — a connection already
/// being served keeps the waits it was accepted under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Waits {
    /// How long the request head may take to arrive, refreshed as it arrives.
    pub header: Duration,
    /// How long the body may go without a byte.
    pub body_idle: Duration,
    /// How long the response may go without the socket taking one.
    pub write_idle: Duration,
    /// How long a kept-alive connection may sit between one response and the next request's first
    /// byte. § 5: this must exceed the proxy's own upstream keep-alive, or the proxy writes into a
    /// socket the origin has already closed and the client gets an intermittent `502`.
    pub keepalive: Duration,
    /// How long a connection keeps being served once it has **seen** the drain, after which it
    /// closes itself with a defined code (`rule:concurrency/a-drain-closes-a-connection-cleanly`).
    ///
    /// The one field here that is not an idle wait: it bounds a connection that is working, which
    /// is why it is a directive of its own rather than a reuse of
    /// [`keepalive`](Self::keepalive) — an operator who lengthened keep-alive would otherwise have
    /// lengthened every restart ([ADR 0186](/docs/decisions/0186.md) § 3). A stopping
    /// process is bounded by this plus the write of whatever response was in flight, which
    /// [`write_idle`](Self::write_idle) already bounds.
    pub drain: Duration,
}

impl Default for Waits {
    /// § 5's own example, transcribed rather than chosen — the ADR writes every number out. The
    /// drain period is [ADR 0186](/docs/decisions/0186.md) § 3's, written out the same
    /// way, and sized so that a stop finishing in `drain` plus one response's write stays inside
    /// systemd's own default with room for a distribution that lowered it.
    fn default() -> Self {
        Self {
            header: Duration::from_secs(10),
            body_idle: Duration::from_secs(30),
            write_idle: Duration::from_secs(30),
            keepalive: Duration::from_secs(75),
            drain: Duration::from_secs(30),
        }
    }
}

/// The block resolves — the boot half of [`waits_for`], of [`connection_bounds_for`], of
/// [`listen_on`], of [`health_path`] and of [`crate::mount::check`].
///
/// # Errors
///
/// Whatever any of them refuses, the waits first. A tree is refused for the first thing wrong
/// with it everywhere else in this crate, so the order decides nothing but which sentence an
/// operator who wrote two mistakes reads first. The mount half is only the part that needs no
/// disk — [`crate::mount::expand`] is where the globs are walked, and its module doc says why that
/// is the server's boot step rather than the resolver's.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    waits_for(config, origins)?;
    connection_bounds_for(config, origins)?;
    listen_on(config, origins)?;
    workers_for(config, origins)?;
    health_path(config, origins)?;
    crate::mount::check(config, origins)
}

/// The waits the tree's `[server]` asks for, over [`Waits::default`].
///
/// A tree with no `[server]` block at all is the default set and not an absence: § 5's waits are
/// what makes the server finite, so "unconfigured" and "unbounded" must not be the same state.
/// Every unwritten key keeps its own default, since `rule:config/later-wins-and-every-override-is-recorded`'s override record is per key and
/// a partly-written `[server]` is a decision per key rather than one. `origins` names the file a
/// refusal points at, and an empty map simply leaves the note off.
///
/// # Errors
///
/// `E0619` for a wait written as `false` or as `0`, both of which the module doc owns. A value
/// that is not a duration at all is `E0601` from [`mod@crate::value`], in that module's words
/// rather than this one's.
pub fn waits_for(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<Waits, Diagnostic> {
    let defaults = Waits::default();
    let Some(server) = config.server.as_ref() else {
        return Ok(defaults);
    };
    Ok(Waits {
        header: wait(
            "server.header_timeout",
            server.header_timeout.as_ref(),
            defaults.header,
            "the request head is what says which request this is, so a connection that never \
             finishes one is holding a coroutine to say nothing",
            origins,
        )?,
        body_idle: wait(
            "server.body_idle_timeout",
            server.body_idle_timeout.as_ref(),
            defaults.body_idle,
            "this is the wait between two bytes of a body and not the time a body may take, so a \
             slow upload is already allowed by it and only a stalled one is not",
            origins,
        )?,
        write_idle: wait(
            "server.write_idle_timeout",
            server.write_idle_timeout.as_ref(),
            defaults.write_idle,
            "a peer that stops reading its response is otherwise a coroutine, a buffered body and \
             a socket held until it goes away",
            origins,
        )?,
        keepalive: wait(
            "server.keepalive_timeout",
            server.keepalive_timeout.as_ref(),
            defaults.keepalive,
            "an idle kept-alive connection costs one coroutine and one socket, and nothing else \
             ever closes it — the peer is by definition not speaking",
            origins,
        )?,
        drain: wait(
            "server.drain_timeout",
            server.drain_timeout.as_ref(),
            defaults.drain,
            "a stop is only as bounded as this period is, and a connection that may be served \
             forever after the drain begins is a process that never exits",
            origins,
        )?,
    })
}

/// One written wait, or the default for a key the block left out.
///
/// [`mod@crate::value`] is the parser, so `"10s"`, `10` and `"10000ms"` all read the same and a
/// suffix it does not know is refused in its own words. What this adds is the magnitude question,
/// and `why` completes the sentence that says what an unbounded one would cost.
fn wait(
    key: &str,
    written: Option<&Setting>,
    default: Duration,
    why: &str,
    origins: &BTreeMap<String, Origin>,
) -> Result<Duration, Diagnostic> {
    let Some(setting) = written else {
        return Ok(default);
    };
    let quantity = Quantity::parse(key, Unit::Duration, setting)
        .map_err(|invalid| invalid.diagnostic(origins.get(key)))?;
    // Zero is only reachable when the block wrote it, so the refusal always has the operator's own
    // spelling to quote back — which is why this reads the parsed value and the written one
    // together, as `[queue] visibility` does one subsystem over.
    match quantity {
        Quantity::Nanos(0) => Err(refuse(key, setting, why, origins)),
        Quantity::Nanos(nanos) => Ok(Duration::from_nanos(nanos)),
        // `Unit::Duration` yields nothing else, and the reachable one is `false`.
        _ => Err(refuse(key, setting, why, origins)),
    }
}

/// A `[server]` wait that would never end, under `E0619`.
///
/// Built here rather than through [`crate::value::Invalid`] for [`mod@crate::queue`]'s reason:
/// that type says which *unit* a value failed to be, and both values refused here are already
/// durations — and its help would offer `false`, which is the very spelling this refuses.
fn refuse(
    key: &str,
    written: &Setting,
    why: &str,
    origins: &BTreeMap<String, Origin>,
) -> Diagnostic {
    Diagnostic::error(
        code::E_UNBOUNDED_WAIT,
        format!(
            "`{key}` is `{}`, which is a wait that never ends",
            crate::value::as_written(written)
        ),
    )
    .with_note(format!("{why}{}", origin_note(origins.get(key))))
    .with_help(format!(
        "write how long the server waits, as `10s` — there is no spelling for waiting forever, \
         and leaving `{key}` out keeps `rule:http-server/the-server-block-is-boot-class`'s own default"
    ))
}

/// What a `[server.connection]` block wrote, key by key, with `None` for a key it left out.
///
/// **The numbers are not here, and that is the point.** Every bound on an open connection is
/// finite before anything is configured (`rule:concurrency/connection-bounds-are-finite`), and
/// `nvs_server::bounds::Connection::default` is where each of those numbers is chosen, argued
/// against what one connection may hold, and enforced. A default repeated in this crate would be a
/// second home for a number the server already states, so this type carries the *override* and
/// nothing else: a `None` means the shipped bound stands, not that the bound is absent.
///
/// Copied rather than borrowed, for [`Waits`]' reason: `[server.connection]` is `Boot`-class, so a
/// connection already open keeps the bounds it was accepted under across a reload.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConnectionBounds {
    /// `max_open` — connections this process may hold open at once.
    pub max_open: Option<u64>,
    /// `max_frame` — the largest frame payload the codec accepts, in bytes.
    pub frame: Option<u64>,
    /// `max_message` — the largest reassembled message the codec accepts, in bytes.
    pub message: Option<u64>,
    /// `idle_timeout` — the longest a connection may go without a frame from its peer.
    pub idle: Option<Duration>,
    /// `max_lifetime` — the longest a connection may stay open at all.
    pub lifetime: Option<Duration>,
    /// `send_timeout` — the longest one `send` may take before it throws.
    pub send: Option<Duration>,
}

/// The bounds the tree's `[server.connection]` writes over the server's own.
///
/// A tree with no block, and a block that leaves a key out, are both "the shipped bound stands"
/// rather than an absence to be filled in later — which is [`waits_for`]'s reason and § 7's: an
/// unconfigured connection is already bounded, and a reader that treated an unwritten key as
/// unbounded would invert the one property that rule exists for.
///
/// # Errors
///
/// `E0649` for any key written as `false` or as zero, both of which are a bound with no bound in
/// it. A value that is not a size, a count or a duration at all is `E0601` from
/// [`mod@crate::value`], in that module's words rather than this one's.
pub fn connection_bounds_for(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
) -> Result<ConnectionBounds, Diagnostic> {
    let Some(block) = config
        .server
        .as_ref()
        .and_then(|server| server.connection.as_ref())
    else {
        return Ok(ConnectionBounds::default());
    };
    Ok(ConnectionBounds {
        max_open: bound(
            "server.connection.max_open",
            Unit::Count,
            block.max_open.as_ref(),
            "a connection outlives the request that upgraded it, so the only thing that bounds how \
             many this process holds is this count",
            origins,
        )?,
        frame: bound(
            "server.connection.max_frame",
            Unit::Bytes,
            block.max_frame.as_ref(),
            "a frame is buffered before it can be looked at, so this is the peer's own say over \
             what one connection makes this process hold",
            origins,
        )?,
        message: bound(
            "server.connection.max_message",
            Unit::Bytes,
            block.max_message.as_ref(),
            "a message is frames reassembled, so removing this cap lets a peer spend the \
             connection's whole budget one continuation frame at a time",
            origins,
        )?,
        idle: bound(
            "server.connection.idle_timeout",
            Unit::Duration,
            block.idle_timeout.as_ref(),
            "nothing else closes a connection whose peer stopped speaking — there is no request to \
             end and no response to finish writing",
            origins,
        )?
        .map(Duration::from_nanos),
        lifetime: bound(
            "server.connection.max_lifetime",
            Unit::Duration,
            block.max_lifetime.as_ref(),
            "this is what catches the process that never reloads, so without it a busy connection \
             can outlive every deploy that followed it",
            origins,
        )?
        .map(Duration::from_nanos),
        send: bound(
            "server.connection.send_timeout",
            Unit::Duration,
            block.send_timeout.as_ref(),
            "a peer that stopped reading is otherwise a coroutine parked in `send` for as long as \
             it cares to leave it there",
            origins,
        )?
        .map(Duration::from_nanos),
    })
}

/// One written `[server.connection]` bound, or `None` for a key the block left out.
///
/// [`mod@crate::value`] is the parser, so `"4MB"`, `4194304`, `"30s"` and `30` each read as what
/// they spell and a suffix it does not know is refused in its own words. What is left for this
/// function is the magnitude question, and it is the same question in all three units: `false`
/// removes a ceiling everywhere else in the tree, and zero is that value written the other way
/// round — no connection open, no frame accepted, no wait elapsed.
fn bound(
    key: &str,
    unit: Unit,
    written: Option<&Setting>,
    why: &str,
    origins: &BTreeMap<String, Origin>,
) -> Result<Option<u64>, Diagnostic> {
    let Some(setting) = written else {
        return Ok(None);
    };
    let quantity = Quantity::parse(key, unit, setting)
        .map_err(|invalid| invalid.diagnostic(origins.get(key)))?;
    match quantity {
        Quantity::Bytes(0) | Quantity::Nanos(0) | Quantity::Count(0) | Quantity::Unbounded => {
            Err(unbounded(key, setting, why, origins))
        }
        Quantity::Bytes(magnitude) | Quantity::Nanos(magnitude) | Quantity::Count(magnitude) => {
            Ok(Some(magnitude))
        }
        // None of the three units above yields a ratio.
        Quantity::Ratio(_) => Err(unbounded(key, setting, why, origins)),
    }
}

/// A `[server.connection]` bound with no bound in it, under `E0649`.
///
/// One code for the block rather than one per unit, because the mistake is one mistake: what an
/// operator wrote is a connection this server would hold with nothing to close it, and whether
/// they spelled it as a size, a count or a wait changes only which sentence completes the note.
fn unbounded(
    key: &str,
    written: &Setting,
    why: &str,
    origins: &BTreeMap<String, Origin>,
) -> Diagnostic {
    Diagnostic::error(
        code::E_CONNECTION_BOUND_REMOVED,
        format!(
            "`{key}` is `{}`, which is a bound with no bound in it",
            crate::value::as_written(written)
        ),
    )
    .with_note(format!("{why}{}", origin_note(origins.get(key))))
    .with_help(format!(
        "write the bound this deployment wants, or leave `{key}` out to keep \
         `rule:concurrency/connection-bounds-are-finite`'s own finite default — there is no \
         spelling here for an unbounded one"
    ))
}

/// One classified `[server] listen` entry: whichever transport § 5's flat array spells.
///
/// A classification and not a socket — nothing here binds anything, and a process holding this
/// has not yet decided how many of the entries it will take. The module doc owns why the overload
/// is unambiguous.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Listen {
    /// A `host:port` entry, parsed as a literal address.
    Tcp(SocketAddr),
    /// An entry beginning with a path separator: the Unix-domain socket § 5 says a proxy should
    /// prefer, and the one that makes FastCGI unnecessary.
    Unix(PathBuf),
}

/// § 5's own default, in both modes: loopback is the proxied shape as well as the development one,
/// so requiring an explicit `listen` in production would be friction with no safety in it.
const DEFAULT_PORT: u16 = 8000;

/// The sockets the tree's `[server] listen` asks for, in the order written.
///
/// A tree with no `[server]` block, and one that leaves the key out, are both § 5's default —
/// `127.0.0.1:8000` — for [`waits_for`]'s reason: an unconfigured server is the deployment the ADR
/// describes rather than an absence to be filled in later.
///
/// # Errors
///
/// `E0620` for an entry that is neither a literal `host:port` nor an absolute path, and for a
/// written array with nothing in it; the code's own declaration owns why a host name is one of
/// them. `origins` names the file a refusal points at, exactly as it does for a wait.
pub fn listen_on(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
) -> Result<Vec<Listen>, Diagnostic> {
    let default = || {
        vec![Listen::Tcp(SocketAddr::from((
            Ipv4Addr::LOCALHOST,
            DEFAULT_PORT,
        )))]
    };
    let Some(written) = config
        .server
        .as_ref()
        .and_then(|server| server.listen.as_ref())
    else {
        return Ok(default());
    };
    if written.is_empty() {
        return Err(Diagnostic::error(
            code::E_BAD_LISTEN,
            "`server.listen` is written as an empty array, which is a server that accepts nothing",
        )
        .with_note(format!(
            "a process with no listening socket is not a smaller deployment, it is one that \
             answers no request at all{}",
            origin_note(origins.get("server.listen"))
        ))
        .with_help(
            "leave `server.listen` out to keep `rule:http-server/the-server-block-is-boot-class`'s own `127.0.0.1:8000`, or write the \
             address this deployment answers on",
        ));
    }
    written
        .iter()
        .map(|entry| classify(entry, origins))
        .collect()
}

/// One entry, read as whichever transport it spells.
fn classify(entry: &str, origins: &BTreeMap<String, Origin>) -> Result<Listen, Diagnostic> {
    if entry.starts_with(std::path::is_separator) {
        return Ok(Listen::Unix(PathBuf::from(entry)));
    }
    entry.parse::<SocketAddr>().map(Listen::Tcp).map_err(|_| {
        Diagnostic::error(
            code::E_BAD_LISTEN,
            format!("`server.listen` entry `{entry}` is not an address and a port"),
        )
        .with_note(format!(
            "every socket this server binds is chosen before it accepts anything, so an entry \
             that has to be looked up is one the start depends on a nameserver for{}",
            origin_note(origins.get("server.listen"))
        ))
        .with_help(
            "write a literal address, as `127.0.0.1:8000` or `[::1]:8000`, or an absolute path \
             for a Unix-domain socket",
        )
    })
}

/// § 5's own `socket_mode`: the owner's account and the socket's group, and nobody else.
///
/// Not the umask's answer, for `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s
/// reason — an ordinary `022` would create the socket world-connectable, and on this transport
/// that is world-trusted.
const DEFAULT_SOCKET_MODE: u32 = 0o660;

/// The mode every Unix-domain entry of `listen` is created with, and [`DEFAULT_SOCKET_MODE`]
/// where the block writes none.
///
/// **This is a trust boundary and not a convenience.** A connection over a Unix socket is
/// implicitly trusted for the forwarded headers (`rule:http-server/a-unix-socket-listener`),
/// because what decided who may connect is this mode — so `0660` means *the owner and that group
/// may name their own client address*, and adding a tenant to the socket's group on a
/// multi-tenant host grants them exactly that.
///
/// A `String` in the tree and a number here, because a file mode is octal and TOML has no octal
/// integer: an unquoted `0660` would be read as six hundred and sixty. Read once at boot, because
/// the key is `Boot`-class, so a reload never moves the mode under a socket already bound.
///
/// # Errors
///
/// `E0648` for a value that is not a permission set — a digit outside `0-7`, anything longer than
/// four digits, and a fourth digit above `0`, which is where `setuid`, `setgid` and the sticky bit
/// would be. Each is refused rather than masked down to what parses, because every reading this
/// function could invent for a malformed mode is a different answer to who may connect.
pub fn socket_mode_for(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
) -> Result<u32, Diagnostic> {
    let Some(written) = config
        .server
        .as_ref()
        .and_then(|server| server.socket_mode.as_ref())
    else {
        return Ok(DEFAULT_SOCKET_MODE);
    };
    let refuse = |why: &str| {
        Err(Diagnostic::error(
            code::E_BAD_SOCKET_MODE,
            format!("`server.socket_mode` is `{written}`, which {why}"),
        )
        .with_note(format!(
            "the mode on a Unix-domain socket is the whole of who may connect to it, and a \
             connection that arrives over one is trusted to name its own client address{}",
            origin_note(origins.get("server.socket_mode"))
        ))
        .with_help(
            "write the permissions as octal in quotes, as `\"0660\"` — the proxy's account and its \
             group, which is `rule:http-server/a-unix-socket-listener`'s own default",
        ))
    };
    if written.is_empty() || written.len() > 4 {
        return refuse("is not a file mode written as three or four octal digits");
    }
    let Ok(mode) = u32::from_str_radix(written, 8) else {
        return refuse("is not octal");
    };
    if mode > 0o777 {
        return refuse("sets a bit that is not a permission — `setuid`, `setgid` or sticky");
    }
    Ok(mode)
}

/// The cores this server accepts on, as the tree's `[server] workers` bounds them.
///
/// With the key left out the count is what this machine answers
/// [`std::thread::available_parallelism`] with, and one core where it answers nothing at all — an
/// unconfigured server takes the box it was started on, which is
/// `rule:http-server/the-accept-fan-out-is-one-worker-per-core`'s default and the reason a
/// deployment needs no directive to scale. A written count is the bound in both directions: it is
/// neither raised to the machine's parallelism nor clamped down to it, because a number the file
/// states is the operator's answer to a question this function is not asked to have an opinion on.
///
/// # Errors
///
/// `E0636` for a `workers` of `0`. Nothing else here can refuse: the code's own declaration owns
/// why a count above the machine's parallelism is started rather than clamped.
pub fn workers_for(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
) -> Result<usize, Diagnostic> {
    let Some(written) = config.server.as_ref().and_then(|server| server.workers) else {
        return Ok(this_machine());
    };
    if written == 0 {
        return Err(Diagnostic::error(
            code::E_NO_WORKERS,
            "`server.workers` is `0`, which is a server with no core to accept on",
        )
        .with_note(format!(
            "every listening socket is accepted on by a worker, so a count of zero binds the \
             addresses this tree names and then answers nobody on any of them{}",
            origin_note(origins.get("server.workers"))
        ))
        .with_help(
            "write how many cores this server accepts on, as `4`, or leave `server.workers` out \
             to take this machine's own parallelism",
        ));
    }
    // Saturating rather than refusing: a count this platform cannot hold in a `usize` is a number
    // no machine has cores for, and the fan-out asking for every one of them is what the tree said.
    Ok(usize::try_from(written).unwrap_or(usize::MAX))
}

/// What this machine answers, and one core where it answers nothing.
fn this_machine() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

/// The one URL § 5's probe answers on, or `None` where this server reserves none.
///
/// A `String` and not a [`std::path::PathBuf`]: this is the path inside a request target and it
/// never reaches a disk, which is the whole difference between it and every other path in the
/// block. Matching it is `nvs_server::mount`'s — § 4's steps run after the probe, not around
/// it — and what this function decides is only that the written value is a target a request could
/// carry.
///
/// # Errors
///
/// `E0623` for a written value that is not an absolute path: a relative one, one carrying a query
/// or a fragment, one holding a space or a control character, and `/` on its own. The module doc
/// owns why each is refused here rather than reserved and never matched.
pub fn health_path(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
) -> Result<Option<String>, Diagnostic> {
    let Some(written) = config
        .server
        .as_ref()
        .and_then(|server| server.health_path.as_ref())
        .filter(|path| !path.is_empty())
    else {
        return Ok(None);
    };
    let wrong = if !written.starts_with('/') {
        "a request target begins at the root, so this is a path no request can carry"
    } else if written == "/" {
        "the root is every mount's own entry, so reserving it answers the probe to whoever asked \
         for the site itself"
    } else if written.contains(['?', '#']) {
        "a query and a fragment are not part of the path a request is matched on, so this is a URL \
         rather than the path inside one"
    } else if written
        .chars()
        .any(|char| char.is_whitespace() || char.is_control())
    {
        "a space or a control character reaches this server percent-encoded, so a path holding one \
         literally is one no probe can ask for"
    } else {
        return Ok(Some(written.clone()));
    };
    Err(Diagnostic::error(
        code::E_BAD_HEALTH_PATH,
        format!("`server.health_path` is `{written}`, which is not a path this server can answer"),
    )
    .with_note(format!(
        "{wrong}{}",
        origin_note(origins.get("server.health_path"))
    ))
    .with_help(
        "write the one absolute path the probe asks for, as `/healthz`, or leave \
         `server.health_path` out to keep `rule:http-server/the-server-block-is-boot-class`'s own off",
    ))
}

/// § 5's own number, transcribed rather than chosen — the ADR writes it out.
const DEFAULT_MAX_IN_FLIGHT: u64 = 10_000;

/// The numbers `rule:http-server/admission-is-arithmetic-not-a-number`'s admission arithmetic is over: what the file asked for, what
/// one request may hold, and what this machine has.
///
/// Deliberately **not** the answer — the division, the clamp and the log line are
/// `nvs_server::admit`'s, because the ceiling is enforced by a counter that has to live beside the
/// thing it refuses. What this type says is that every input exists and where each came from;
/// a configuration crate that also decided the ceiling would be deciding an admission policy from
/// the wrong end of the process.
///
/// `per_request` and `budget` are both `Option` because both are genuinely absent on real
/// deployments, and the absences mean different things: a tree that states no `[limits] memory` has
/// no per-request cap for a concurrency ceiling to have a relationship *with*, and a host that
/// answers no budget has not been asked a question this crate can put to it. § 13's arithmetic is
/// inert either way, and inert means the configured number stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capacity {
    /// `[server] max_in_flight`, or § 5's own `10000` where the key is unwritten.
    pub configured: u64,
    /// The bytes one request may hold at its most: `[limits.hard] memory` where a ceiling is
    /// written, and otherwise the `[limits] memory` a request starts with. `None` where the tree
    /// states neither, and where `[limits.hard] memory = false` removed the ceiling — `rule:config/three-changeability-classes`'s
    /// spelling for "no ceiling" is exactly the case § 13 has nothing to divide by.
    pub per_request: Option<u64>,
    /// What this machine affords the process, as [`memory_budget`] read it.
    pub budget: Option<u64>,
}

/// § 5's `max_in_flight` and the numbers § 13 divides against it.
///
/// A tree with no `[server]` block is § 5's default ceiling and not an absence, for [`waits_for`]'s
/// reason. The per-request cap is read from `[limits]` rather than from `[server]` because that is
/// where it is written; this function is the one place the two blocks are read together, which is
/// the whole of what § 13 added.
///
/// # Errors
///
/// `E0622` for a `max_in_flight` of `0`, and whatever [`mod@crate::value`] refuses a `[limits]`
/// memory setting for. Nothing here refuses a configured ceiling the budget cannot afford: § 13
/// clamps that one and logs it, which is `nvs_server::admit`'s and is a decision rather than an
/// omission — a server that will not boot because two directives disagree is the worse outage.
pub fn capacity_for(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
) -> Result<Capacity, Diagnostic> {
    let configured = match config
        .server
        .as_ref()
        .and_then(|server| server.max_in_flight)
    {
        None => DEFAULT_MAX_IN_FLIGHT,
        Some(0) => {
            return Err(Diagnostic::error(
                code::E_NO_ADMISSION,
                "`server.max_in_flight` is `0`, which is a server that refuses every request",
            )
            .with_note(format!(
                "the ceiling is the point at which a request is answered `503` instead of being \
                 run, so a ceiling of zero is that answer to all of them{}",
                origin_note(origins.get("server.max_in_flight"))
            ))
            .with_help(
                "write how many requests may be in flight at once, as `10000`, or leave \
                 `server.max_in_flight` out to keep `rule:http-server/the-server-block-is-boot-class`'s own default",
            ));
        }
        Some(written) => written,
    };
    Ok(Capacity {
        configured,
        per_request: per_request_cap(config, origins)?,
        budget: memory_budget(),
    })
}

/// The bytes one request may hold, as the tree's `[limits]` states them.
///
/// The *hard* ceiling wins where one is written, because that is the number a request can actually
/// reach: `[limits] memory` is only where it starts, and a request that raises itself to the
/// ceiling is what the machine has to hold. A cap of zero is read as no cap rather than divided by
/// — a request that may hold nothing is a tree that is wrong about something else, and this
/// function is not the place that says so.
fn per_request_cap(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
) -> Result<Option<u64>, Diagnostic> {
    let Some(limits) = config.limits.as_ref() else {
        return Ok(None);
    };
    let (key, written) = match limits.hard.as_ref().and_then(|hard| hard.memory.as_ref()) {
        Some(hard) => ("limits.hard.memory", hard),
        None => match limits.memory.as_ref() {
            Some(start) => ("limits.memory", start),
            None => return Ok(None),
        },
    };
    let quantity = Quantity::parse(key, Unit::Bytes, written)
        .map_err(|invalid| invalid.diagnostic(origins.get(key)))?;
    Ok(match quantity {
        Quantity::Bytes(0) | Quantity::Unbounded => None,
        Quantity::Bytes(bytes) => Some(bytes),
        // `Unit::Bytes` yields nothing else.
        _ => None,
    })
}

/// What this machine affords the process, in bytes, or `None` where it does not answer.
///
/// **A container's limit is the answer where there is one**, and the machine's own memory only
/// where there is not. A cgroup-limited process on a 256 GB host has 256 MB, and an admission
/// arithmetic that read the host's number there would compute a ceiling whose whole purpose —
/// keeping the out-of-memory killer from being the real admission control — it had already given
/// away. That is why this reads the cgroup files before `/proc/meminfo`.
///
/// `None` is not a failure: it is a host this function has no question for, and § 13's arithmetic
/// treats it as an absent bound rather than as a zero. Every path here is read once at boot and
/// never on a request.
#[must_use]
pub fn memory_budget() -> Option<u64> {
    platform::memory_budget()
}

#[cfg(unix)]
mod platform {
    //! The files a Unix host answers with, in the order a container makes correct.
    //!
    //! Each is a plain read, so this half needs no `unsafe` and no `libc`: cgroup v2's
    //! `memory.max` is a decimal number or the word `max`, cgroup v1's `memory.limit_in_bytes` is a
    //! decimal number with [`NO_LIMIT`]'s sentinel for the same thing, and `/proc/meminfo` states
    //! `MemTotal` in kibibytes. A host with none of them — macOS is the one that matters — answers
    //! `None`, which is § 13's inert case and not a wrong number.

    /// Anything at or above this is a sentinel rather than a limit: cgroup v1 spells "no limit" as
    /// a number near `u64::MAX` rounded down to a page, and no machine has four exabytes.
    const NO_LIMIT: u64 = 1 << 62;

    /// The first of them that answers.
    pub(super) fn memory_budget() -> Option<u64> {
        cgroup("/sys/fs/cgroup/memory.max")
            .or_else(|| cgroup("/sys/fs/cgroup/memory/memory.limit_in_bytes"))
            .or_else(mem_total)
    }

    /// One cgroup limit file: a decimal count of bytes, `max`, or the v1 sentinel.
    fn cgroup(path: &str) -> Option<u64> {
        let bytes: u64 = std::fs::read_to_string(path).ok()?.trim().parse().ok()?;
        (bytes > 0 && bytes < NO_LIMIT).then_some(bytes)
    }

    /// `/proc/meminfo`'s `MemTotal`, which is stated in kibibytes.
    fn mem_total() -> Option<u64> {
        let file = std::fs::read_to_string("/proc/meminfo").ok()?;
        let line = file.lines().find(|line| line.starts_with("MemTotal:"))?;
        let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
        kib.checked_mul(1024)
    }
}

#[cfg(windows)]
mod platform {
    //! `GlobalMemoryStatusEx`, which is the whole of the question on Windows.
    //!
    //! There is no container limit to prefer here: a Windows container reports its own quota
    //! through this same call, so the one answer is already the right one.

    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    /// The installed physical memory, or `None` where the call fails.
    #[expect(
        unsafe_code,
        reason = "`GlobalMemoryStatusEx` fills a caller-owned struct whose `dwLength` says how big \
                  it is; the call is unsafe only because it is `extern`"
    )]
    pub(super) fn memory_budget() -> Option<u64> {
        // A plain-old-data struct of integers, so an all-zero one is a valid value of it and the
        // call is what fills it.
        let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        // The API's own version check: a zero here is what makes the call fail.
        status.dwLength = u32::try_from(size_of::<MEMORYSTATUSEX>()).ok()?;
        // `status` is a live, correctly sized `MEMORYSTATUSEX` owned by this frame, and the call
        // writes only inside it.
        if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
            return None;
        }
        (status.ullTotalPhys > 0).then_some(status.ullTotalPhys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(text: &str) -> Config {
        toml::from_str(text).expect("the fixture did not deserialize")
    }

    /// § 5's `socket_mode`, from the three answers it has: the default, a written mode, and every
    /// spelling that is not a mode at all.
    ///
    /// The default is asserted as `0660` rather than as "whatever the constant says", because the
    /// number is the reachable-by-default set on a transport whose connections are trusted — a
    /// reading that fell back to the umask would pass a test that only checked the key was read.
    /// The refusals are each a value that *nearly* parses: a `0999` that is decimal, a `1777` whose
    /// fourth digit is the sticky bit, and a `06600` that is a mode with a digit too many. Each is
    /// one an operator could plausibly type, and each would otherwise become a different answer to
    /// who may connect.
    #[test]
    fn socket_mode_is_owner_and_group_by_default_and_every_non_mode_is_refused() {
        let origins = BTreeMap::new();
        for written in ["", "[server]\nlisten = [\"/run/nvs.sock\"]\n"] {
            assert_eq!(
                socket_mode_for(&tree(written), &origins).expect("an unwritten mode was refused"),
                0o660,
                "for {written:?}"
            );
        }
        for (written, mode) in [("\"0660\"", 0o660), ("\"600\"", 0o600), ("\"0666\"", 0o666)] {
            assert_eq!(
                socket_mode_for(
                    &tree(&format!("[server]\nsocket_mode = {written}\n")),
                    &origins
                )
                .expect("a written mode was refused"),
                mode,
                "for {written}"
            );
        }
        for written in ["\"\"", "\"0999\"", "\"1777\"", "\"06600\"", "\"rw-rw----\""] {
            let refused = socket_mode_for(
                &tree(&format!("[server]\nsocket_mode = {written}\n")),
                &origins,
            )
            .expect_err("a value that is not a file mode was accepted");
            assert_eq!(refused.code, Some(code::E_BAD_SOCKET_MODE), "for {written}");
        }
    }

    /// A tree that writes no `[server]` block is bounded anyway: § 5's own numbers are what the
    /// server runs on, and an operator configuring nothing is the deployment they describe.
    #[test]
    fn a_tree_with_no_server_block_still_has_every_wait() {
        let waits = waits_for(&tree(""), &BTreeMap::new()).expect("an empty tree was refused");
        assert_eq!(waits, Waits::default());
        assert_eq!(waits.keepalive, Duration::from_secs(75));
        assert_eq!(waits.drain, Duration::from_secs(30));
    }

    /// The drain period is a directive of its own and not keep-alive under another name, asserted
    /// as the property an operator relies on: lengthening the wait a proxy needs leaves the
    /// restart where it was.
    #[test]
    fn the_drain_period_is_written_and_read_apart_from_the_keepalive_wait() {
        let waits = waits_for(
            &tree("[server]\nkeepalive_timeout = \"300s\"\ndrain_timeout = 5\n"),
            &BTreeMap::new(),
        )
        .expect("a written drain period was refused");
        assert_eq!(waits.drain, Duration::from_secs(5));
        assert_eq!(waits.keepalive, Duration::from_secs(300));
    }

    /// The two spellings `mod@crate::value` makes equal, asserted together rather than one at a
    /// time: a wait is a duration like every other one in the tree.
    #[test]
    fn a_written_wait_reads_the_same_as_a_bare_number_of_seconds() {
        let suffixed = waits_for(
            &tree("[server]\nheader_timeout = \"5s\"\n"),
            &BTreeMap::new(),
        )
        .expect("`5s` was refused");
        let bare = waits_for(&tree("[server]\nheader_timeout = 5\n"), &BTreeMap::new())
            .expect("a bare `5` was refused");
        assert_eq!(suffixed.header, Duration::from_secs(5));
        assert_eq!(suffixed, bare);
        // Every other wait keeps its default independently: `rule:config/later-wins-and-every-override-is-recorded`'s override is per key.
        assert_eq!(suffixed.keepalive, Waits::default().keepalive);
    }

    /// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s headline, as the refusals that hold it up. Each side is named in one case
    /// because a resolution that refused only `false` would still accept the `0` that closes every
    /// connection as it arrives.
    #[test]
    fn neither_false_nor_zero_is_a_wait_this_server_accepts() {
        for written in [
            "[server]\nkeepalive_timeout = false\n",
            "[server]\nkeepalive_timeout = 0\n",
            "[server]\nbody_idle_timeout = \"0s\"\n",
            "[server]\ndrain_timeout = false\n",
            "[server]\ndrain_timeout = 0\n",
        ] {
            let refused = waits_for(&tree(written), &BTreeMap::new())
                .expect_err("a wait that never ends was accepted");
            assert_eq!(
                refused.code,
                Some(code::E_UNBOUNDED_WAIT),
                "for {written:?}"
            );
        }
    }

    /// § 13's inputs, read from the blocks they are written in. The *hard* ceiling wins
    /// where one is there, because that is the number a request can actually reach — a cap read
    /// from `[limits] memory` under a `[limits.hard] memory` four times its size would afford four
    /// times the concurrency the machine can hold.
    #[test]
    fn the_per_request_cap_is_the_ceiling_a_request_can_reach() {
        let none = capacity_for(&tree(""), &BTreeMap::new()).expect("an empty tree was refused");
        assert_eq!(none.configured, DEFAULT_MAX_IN_FLIGHT);
        assert_eq!(none.per_request, None, "an unwritten cap became a number");

        let started = capacity_for(&tree("[limits]\nmemory = \"64M\"\n"), &BTreeMap::new())
            .expect("a written `[limits]` was refused");
        assert_eq!(started.per_request, Some(64 * 1024 * 1024));

        let ceilinged = capacity_for(
            &tree("[limits]\nmemory = \"64M\"\n\n[limits.hard]\nmemory = \"256M\"\n"),
            &BTreeMap::new(),
        )
        .expect("a written ceiling was refused");
        assert_eq!(
            ceilinged.per_request,
            Some(256 * 1024 * 1024),
            "the cap a request can raise itself to did not win"
        );

        // `rule:config/three-changeability-classes`'s spelling for "no ceiling" is exactly the case § 13 has nothing to divide by,
        // and reading it as the `[limits] memory` underneath would afford a concurrency the
        // request is free to exceed.
        let unbounded = capacity_for(
            &tree("[limits]\nmemory = \"64M\"\n\n[limits.hard]\nmemory = false\n"),
            &BTreeMap::new(),
        )
        .expect("a removed ceiling was refused");
        assert_eq!(
            unbounded.per_request, None,
            "a removed ceiling became a cap"
        );
    }

    /// The one magnitude of `max_in_flight` that is a refusal rather than a clamp: a ceiling the
    /// budget cannot afford is § 13's clamp-and-log, but a ceiling the *operator* wrote as zero is
    /// a server that accepts a connection and answers `503` to everything on it.
    #[test]
    fn a_ceiling_of_zero_is_a_server_that_refuses_every_request() {
        let refused = capacity_for(&tree("[server]\nmax_in_flight = 0\n"), &BTreeMap::new())
            .expect_err("a ceiling of zero was accepted");
        assert_eq!(refused.code, Some(code::E_NO_ADMISSION));

        let written = capacity_for(&tree("[server]\nmax_in_flight = 32\n"), &BTreeMap::new())
            .expect("a written ceiling was refused");
        assert_eq!(written.configured, 32);
    }

    /// A value that is not a duration at all stays `mod@crate::value`'s refusal and does not
    /// become this module's: what is wrong with `"soon"` is the unit, and `E0601` is where every
    /// directive in the tree says so.
    #[test]
    fn a_wait_that_is_not_a_duration_is_refused_in_the_parsers_own_words() {
        let refused = waits_for(
            &tree("[server]\nwrite_idle_timeout = \"soon\"\n"),
            &BTreeMap::new(),
        )
        .expect_err("`soon` was read as a duration");
        assert_eq!(refused.code, Some(code::E_BAD_DIRECTIVE));
    }

    /// § 5's default is the same in both modes, and an unwritten key is it — the same reading the
    /// waits get one function up, asserted here because a server that defaulted to nothing would
    /// have to be told where to listen before it could be started at all.
    #[test]
    fn a_tree_that_writes_no_listen_answers_with_loopback_on_8000() {
        let entries = listen_on(&tree(""), &BTreeMap::new()).expect("an empty tree was refused");
        assert_eq!(
            entries,
            vec![Listen::Tcp(SocketAddr::from((Ipv4Addr::LOCALHOST, 8000)))]
        );
        assert_eq!(
            listen_on(&tree("[server]\nroot = \"/www\"\n"), &BTreeMap::new())
                .expect("a `[server]` without the key was refused"),
            entries
        );
    }

    /// § 5's overload, asserted on both sides of it in one case: the transports are told apart
    /// by the first character and by nothing else, so a case that only looked at an address would
    /// pass against a reading that classified everything as one.
    #[test]
    fn an_entry_beginning_with_a_separator_is_a_socket_and_everything_else_is_an_address() {
        let entries = listen_on(
            &tree("[server]\nlisten = [\"0.0.0.0:80\", \"/run/nvs.sock\", \"[::1]:8080\"]\n"),
            &BTreeMap::new(),
        )
        .expect("§ 5's own spellings were refused");
        assert_eq!(
            entries,
            vec![
                Listen::Tcp("0.0.0.0:80".parse().expect("a literal address")),
                Listen::Unix(PathBuf::from("/run/nvs.sock")),
                Listen::Tcp("[::1]:8080".parse().expect("a literal address")),
            ]
        );
    }

    /// The refusals, named together because each is plausible on its own: a name is the entry
    /// an operator is most likely to write, and an empty array is the one spelling of "listen on
    /// nothing" that § 5 has no version of.
    #[test]
    fn a_host_name_and_an_empty_array_are_both_refused() {
        for written in [
            "[server]\nlisten = [\"localhost:8000\"]\n",
            "[server]\nlisten = [\"8000\"]\n",
            "[server]\nlisten = []\n",
        ] {
            let refused = listen_on(&tree(written), &BTreeMap::new())
                .expect_err("an unbindable `listen` was accepted");
            assert_eq!(refused.code, Some(code::E_BAD_LISTEN), "for {written:?}");
        }
    }

    /// § 5's "off by default, so no URL is silently reserved", asserted through every spelling
    /// of off — no block, a block without the key, and the ADR's own `""` — because a reading that
    /// answered `Some("")` for the last one would reserve the empty path from every mount while
    /// looking like the example it was copied from.
    #[test]
    fn a_health_path_is_off_until_one_is_written() {
        for written in [
            "",
            "[server]\nroot = \"/www\"\n",
            "[server]\nhealth_path = \"\"\n",
        ] {
            assert_eq!(
                health_path(&tree(written), &BTreeMap::new()).expect("an off probe was refused"),
                None,
                "for {written:?}"
            );
        }
        assert_eq!(
            health_path(
                &tree("[server]\nhealth_path = \"/healthz\"\n"),
                &BTreeMap::new()
            )
            .expect("a written probe was refused"),
            Some("/healthz".to_owned())
        );
    }

    /// The bound on both sides: every spelling that could not be the path a request carries, named
    /// beside the ones that can. A probe that is merely never matched is worse than one refused —
    /// the server boots looking configured and nothing answers — so the accepted half is asserted
    /// in the same case, or a resolution that refused everything would pass against the first.
    #[test]
    fn a_reserved_health_path_is_an_absolute_path_and_nothing_else() {
        for written in [
            "healthz",
            "./healthz",
            "/",
            "/healthz?verbose=1",
            "/healthz#live",
            "/health z",
        ] {
            let refused = health_path(
                &tree(&format!("[server]\nhealth_path = \"{written}\"\n")),
                &BTreeMap::new(),
            )
            .expect_err("a path no request can carry was reserved");
            assert_eq!(
                refused.code,
                Some(code::E_BAD_HEALTH_PATH),
                "for {written:?}"
            );
        }
        for written in ["/healthz", "/-/health", "/internal/health/", "/%20"] {
            assert_eq!(
                health_path(
                    &tree(&format!("[server]\nhealth_path = \"{written}\"\n")),
                    &BTreeMap::new()
                )
                .expect("an absolute path was refused"),
                Some(written.to_owned()),
                "for {written:?}"
            );
        }
    }
}
