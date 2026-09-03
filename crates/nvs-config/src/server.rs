//! [ADR 0097] § 5's `[server]` block, read into what a server starts on: the four waits as
//! durations — with the two magnitudes that would leave a connection unbounded — and `listen` as
//! the sockets to bind.
//!
//! Those two are the only parts of `[server]` that resolve to something other than what was
//! written *here*, so this module is small on purpose: everything else in the block is a path or a
//! word read directly off [`crate::tree::Server`]. The third thing that resolves is the mount
//! table, and it is [`mod@crate::mount`]'s because it needs a disk to expand a glob against —
//! [`validate`] runs the half of it that does not.
//!
//! **All four are *idle* waits and none of them is a total.** A slow 2 GB upload completes while a
//! stalled socket does not, which is § 5's own sentence and the reason the server refreshes a
//! deadline on every byte that moves rather than arming one when a connection is accepted.
//! `nvs_server::io`'s phase machine is the home of *which* wait is in force at a given moment;
//! this module only says how long each one is.
//!
//! **`false` and `0` are both refused**, under `E0619`. Everywhere else in this tree `false`
//! removes a ceiling ([ADR 0005]), and that spelling is exactly what
//! [ADR 0074](../../../docs/adr/0074-http-defaults-safe-and-finite.md) has no version of: its
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
//! Cost: one pass over one optional block at boot and at reload, and four `Duration`s plus one
//! address per written `listen` entry held per configuration generation. Nothing here runs on a
//! request path.
//!
//! [ADR 0005]: ../../../docs/adr/0005-config-changeability.md
//! [ADR 0097]: ../../../docs/adr/0097-development-server-and-proxied-origin.md

use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Origin, origin_note};
use crate::tree::{Config, Setting};
use crate::value::{Quantity, Unit};

/// ADR 0097 § 5's four waits, each a number — the whole clock one connection is bounded by.
///
/// Held by value and copied per configuration generation rather than borrowed from the tree, for
/// [`crate::queue::QueueBounds`]'s reason: [ADR 0078](../../../docs/adr/0078-config-reload-and-control-socket.md)
/// § 1's reload replaces the tree whole, and these are `Boot`-class anyway — a connection already
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
}

impl Default for Waits {
    /// § 5's own example, transcribed rather than chosen — the ADR writes all four numbers out.
    fn default() -> Self {
        Self {
            header: Duration::from_secs(10),
            body_idle: Duration::from_secs(30),
            write_idle: Duration::from_secs(30),
            keepalive: Duration::from_secs(75),
        }
    }
}

/// The block resolves — the boot half of [`waits_for`], of [`listen_on`] and of
/// [`crate::mount::check`].
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
    listen_on(config, origins)?;
    crate::mount::check(config, origins)
}

/// The waits the tree's `[server]` asks for, over [`Waits::default`].
///
/// A tree with no `[server]` block at all is the default set and not an absence: § 5's waits are
/// what makes the server finite, so "unconfigured" and "unbounded" must not be the same state.
/// Every unwritten key keeps its own default, since ADR 0103 § 3's override record is per key and
/// a partly-written `[server]` is four decisions rather than one. `origins` names the file a
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
         and leaving `{key}` out keeps ADR 0097 § 5's own default"
    ))
}

/// One classified `[server] listen` entry: the two transports § 5's flat array spells.
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
            "leave `server.listen` out to keep ADR 0097 § 5's own `127.0.0.1:8000`, or write the \
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(text: &str) -> Config {
        toml::from_str(text).expect("the fixture did not deserialize")
    }

    /// A tree that writes no `[server]` block is bounded anyway: § 5's four numbers are what the
    /// server runs on, and an operator configuring nothing is the deployment they describe.
    #[test]
    fn a_tree_with_no_server_block_still_has_all_four_waits() {
        let waits = waits_for(&tree(""), &BTreeMap::new()).expect("an empty tree was refused");
        assert_eq!(waits, Waits::default());
        assert_eq!(waits.keepalive, Duration::from_secs(75));
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
        // The other three keep their defaults independently: ADR 0103 § 3's override is per key.
        assert_eq!(suffixed.keepalive, Waits::default().keepalive);
    }

    /// ADR 0074's headline, as the two refusals that hold it up. Both sides are named in one case
    /// because a resolution that refused only `false` would still accept the `0` that closes every
    /// connection as it arrives.
    #[test]
    fn neither_false_nor_zero_is_a_wait_this_server_accepts() {
        for written in [
            "[server]\nkeepalive_timeout = false\n",
            "[server]\nkeepalive_timeout = 0\n",
            "[server]\nbody_idle_timeout = \"0s\"\n",
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

    /// § 5's overload, asserted on both sides of it in one case: the two transports are told apart
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

    /// The two refusals, named together because each is plausible on its own: a name is the entry
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
}
