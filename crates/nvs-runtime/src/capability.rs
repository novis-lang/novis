//! `rule:security/capability-check-at-the-door`'s check, and § 5's refusal: the one function a door to the operating system calls
//! before it opens.
//!
//! [`require`] is deliberately the only *decision* here, and [`granted`] is that same decision
//! without the sentence — a door needs to say what it refused, and `rule:security/optional-capability-degrades`'s `Core\Cap::has`
//! needs only the yes or no. The decision procedure is
//! [`nvs_config::capability`] and is pure; this is the half that knows about a request — where the
//! snapshot comes from, and what a denial looks like to the program that hit it. Below are
//! § 2's filesystem doors — [`open_read`], [`metadata`], [`metadata_if_present`], [`exists`],
//! [`canonicalize`], [`resolve_existing`] and
//! [`read_dir`] behind `fs.read`, [`write()`],
//! [`remove_file`], [`remove_dir`] and [`temp_dir`] behind `fs.write`, and [`open`] behind whichever
//! of the two its [`Access`] names — [`exec`] is the process
//! door behind `process.exec`, and [`pin_host`] is the outbound one behind `net.connect`, which
//! answers an address rather than a yes for `rule:http-server/allow-url-pins-the-address`'s reason; each of them calls [`require`]
//! before it names a spelling that
//! performs the effect, which is what makes § 2's claim structural rather than a convention: a member
//! reaches the OS through a door or not at all, and every door has already asked.
//!
//! Every door that reads also records what it read — the file, the directory listed or the path
//! tested — through [`nvs_footprint`], which writes nothing unless `NVS_FOOTPRINT_LOG` names a log.
//! Being the one way a program reaches the filesystem is what makes the doors the place to record it.
//!
//! **A context with no configuration grants nothing.** That is not a special case for tests — it is the
//! same deny-by-default the absent block gets, and a request path that reached a capability check
//! without a snapshot has a bug that should fail closed rather than quietly succeed.
//!
//! **The one call here that leaves the thread is a name lookup.** [`install_resolver`] is that seam:
//! a worker installs a resolver handing the lookup to its blocking pool, because this crate is the
//! bottom of the tree and cannot reach `nvs_host` itself
//! (`rule:http-server/a-core-is-never-blocked-on-a-syscall`). Everything either side of it — the
//! grant, and § 3's table over every address the name answered — stays on the core.
//!

use std::fs::File;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// What [`metadata`] and [`metadata_if_present`] hand back, re-exported so that their callers can
/// name it.
///
/// `nvs-stdlib` may not write `std::fs` anywhere —
/// `nvs_stdlib_reaches_the_os_only_through_the_gate` is the scan that holds `rule:security/capability-check-at-the-door`'s door
/// shut — so a `Core` member that passes one of these to a helper of its own would otherwise have
/// no spelling for the parameter, and would have to re-derive each field at the call site instead.
/// Re-exporting the type grants nothing: every way of *obtaining* one still goes through a door
/// above that has already asked.
pub use std::fs::Metadata;

/// What [`read_dir`] lists, re-exported for the same reason as [`Metadata`].
pub use crate::beneath::{Entry, Kind};

use nvs_config::capability::{Cap, Scope};

use crate::ctx::Ctx;
use crate::{Fault, ThrownClass};

/// § 1's question, asked of `ctx`'s own snapshot, and § 5's `RuntimeError` when the answer is no.
///
/// `member` is what the message names as the thing that wanted the capability — `Core\IO::write`,
/// or `spawn script` for the language construct, which is not a `Core` member at all and is checked
/// through the same function for exactly that reason.
///
/// # Errors
///
/// [`Fault::thrown`] — a `RuntimeError`, catchable, naming the capability in the spelling `nvs.toml`
/// grants it under and, for a scoped check, the argument that fell outside the grant, over a second
/// line naming the file and the table that grant is written in ([`denial`]). A path scope that does
/// not start at a root throws the same class first, before any grant is read
/// ([`relative_refusal`]). It is never a
/// `FATAL`: a denial is known before any work is done and leaves nothing behind, so a program that
/// degrades when a capability is missing is a reasonable program (`rule:security/denial-is-a-runtime-error`).
pub fn require(ctx: &Ctx, cap: Cap, scope: Scope<'_>, member: &str) -> Result<(), Fault> {
    require_as(ctx, cap, scope, ThrownClass::Runtime, member)
}

/// [`require`]'s question with the class of the throw chosen by the door, so that one sentence
/// serves both readings of a missing grant.
///
/// A grant that unlocks an **ability** is `rule:security/denial-is-a-runtime-error`'s
/// `RuntimeError`: the operator said no to something the program set out to do, and a program that
/// degrades instead is a reasonable program. A grant that unlocks a **relaxation** —
/// `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`'s four — is a `LogicError`, which
/// is the class for a program's own mistake: the program asked
/// for its own guarantee to be weakened at a host nobody said it could be, and that is a mistake in
/// the program rather than a verdict on the world.
///
/// The class is the only thing that varies. The message stays [`denial`]'s, because an operator
/// reading either one is about to paste the same grant name into the same table.
///
/// # Errors
///
/// [`require`]'s message, thrown as `class`.
pub fn require_as(
    ctx: &Ctx,
    cap: Cap,
    scope: Scope<'_>,
    class: ThrownClass,
    member: &str,
) -> Result<(), Fault> {
    if let Scope::Path(path) = scope {
        relative_refusal(path, member)?;
        device_refusal(path, member)?;
        alias_refusal(path, member)?;
    }
    match refusal(ctx, cap, scope, member) {
        Some(message) => Err(Fault::thrown_as(class, message)),
        None => Ok(()),
    }
}

/// `rule:programs/relative-paths-resolve-from-their-file`'s run-time half: a path that does not
/// start at a root throws before any grant is asked about it.
///
/// Nothing resolves a path against the process's working directory. Under `nvs serve` that
/// directory is shared by every mounted app and is the system folder under a service manager, so
/// a relative path would name a different file depending on how the server was started. A
/// relative **literal** never reaches here: the compiler has already joined it to the folder of the
/// file that wrote it. What arrives relative is a value the program built, and the message says
/// the two ways to make it absolute.
///
/// "Starts at a root" is [`Path::has_root`], the test the compiler's join makes, so the two halves
/// agree on which paths need no resolving. On Windows it accepts `\data` (the root of the current
/// drive) and refuses `C:data`, which is relative to that drive's own working directory.
///
/// **Cost:** one scan of the path's first component per door call.
///
/// # Errors
///
/// A catchable `RuntimeError` naming `member` and the path.
fn relative_refusal(path: &Path, member: &str) -> Result<(), Fault> {
    match relative(path, member) {
        Some(message) => Err(Fault::thrown(message)),
        None => Ok(()),
    }
}

/// `rule:errors/path-component-refusals`' device names, refused in every path a door is asked
/// about, before any grant is read and on every platform.
///
/// On Windows `C:\data\CON.txt` opens the console and `NUL.` opens the null device, from any
/// folder, so a granted folder does not contain the file the name opens. A read of `CONIN$` waits
/// for a key that never comes. Each component of the path as given is tested with
/// [`nvs_config::capability::is_reserved_device`]; a drive or a root is not a component.
///
/// **Cost:** one scan of the path's components per door call.
///
/// # Errors
///
/// A catchable `RuntimeError` naming `member`, the path and the component.
fn device_refusal(path: &Path, member: &str) -> Result<(), Fault> {
    let device = path.components().find_map(|component| match component {
        Component::Normal(name) => name
            .to_str()
            .filter(|name| nvs_config::capability::is_reserved_device(name)),
        _ => None,
    });
    match device {
        Some(name) => Err(Fault::thrown(format!(
            "{member} cannot use the path {}: `{name}` is a Windows device name. A path may not \
             contain one on any platform.",
            path.display()
        ))),
        None => Ok(()),
    }
}

/// `rule:errors/path-component-refusals`' Windows aliases, refused in every path a door is asked
/// about, by its text, before any grant is read and on every platform.
///
/// The grant's own check canonicalizes the path, and on Windows that opens it. Opening
/// `\\host\share\x` connects to `host` and sends this account's credentials, and the core waits
/// on the network for as long as the redirector takes. So a network, device or verbatim path,
/// a stream after a `:`, a trailing dot or space and an 8.3 short name are refused here, with
/// no system call. [`nvs_config::capability::windows_alias`] is the test.
///
/// **Cost:** one scan of the path per door call.
///
/// # Errors
///
/// A catchable `RuntimeError` naming `member`, the path and what it spells.
pub fn alias_refusal(path: &Path, member: &str) -> Result<(), Fault> {
    let text = path.to_string_lossy();
    match nvs_config::capability::windows_alias(&text) {
        Some(why) => Err(Fault::thrown(format!(
            "{member} cannot use the path {}: it {why}. A path may not be written this way on any \
             platform.",
            path.display()
        ))),
        None => Ok(()),
    }
}

/// [`relative_refusal`]'s message as data: `None` for a path that starts at a root.
///
/// For the places that name the script an isolate runs. `spawn script`, `Core\Socket::upgrade`
/// and `Core\Sse::upgrade` ask it first so the throw names their own member, and
/// [`crate::script::resolve`] asks it again for every caller, so a queued job, a `[[schedule]]`
/// entry and a `[log] handler` whose path arrived relative are refused too.
#[must_use]
pub fn relative(path: &Path, member: &str) -> Option<String> {
    if path.has_root() {
        return None;
    }
    Some(format!(
        "{member} needs an absolute path, and `{}` is relative\nhelp: build the path with \
         `Core\\Path::join` from a folder you know, or with `Core\\Path::fromCwd` for a path \
         typed on the command line",
        path.display()
    ))
}

/// The process's working directory, for `Core\Path::fromCwd` — the one way a program reads it,
/// and only outside a request.
///
/// A command-line program is started from a directory its user chose, and a path typed on its
/// command line means a file in that directory, so the program needs to read it. A request has no
/// such directory: under `nvs serve` every mounted app shares the server's, and under a service
/// manager it is a system folder. So a context answering a request throws instead of answering.
///
/// No grant is asked. The answer names a directory and reads nothing in it; every door that then
/// opens a path under it still asks its own capability.
///
/// # Errors
///
/// A catchable `RuntimeError` naming `member` when `ctx` is answering a request, or [`io_failure`]'s
/// `IOError` when the operating system cannot say what the directory is — it was deleted while the
/// process was in it.
pub fn working_dir(ctx: &Ctx, member: &str) -> Result<PathBuf, Fault> {
    if ctx.inbound().is_some() {
        return Err(Fault::thrown(format!(
            "{member} cannot be used while answering a request, because a server's working \
             directory is not the app's folder\nhelp: write the path directly in the code, in \
             quotes. It is then relative to the file that contains it. You can also build it \
             with `Core\\Path::join`"
        )));
    }
    std::env::current_dir().map_err(|err| io_failure(member, Path::new("."), &err))
}

/// [`require`]'s answer as data: `None` when the capability covers `scope`, and § 5's message
/// otherwise.
///
/// For the doors [`require`] does not fit. [`crate::script::resolve`] cannot hand back a [`Fault`]:
/// it owns an error type of its own, because a spawn's other ways of failing are not capability
/// questions. `Core\Db::open` asks a path-scoped grant about SQLite's `:memory:`, which is a name
/// and not a file, so [`relative_refusal`] has nothing to say about it. Both ask this rather than
/// re-deriving the sentence, so the message a denial prints has exactly one author whichever door
/// produced it.
pub fn refusal(ctx: &Ctx, cap: Cap, scope: Scope<'_>, member: &str) -> Option<String> {
    if granted(ctx, cap, scope) {
        return None;
    }
    Some(denial(cap, scope, member))
}

/// § 1's question as a `bool`, with no message built for the `false` side.
///
/// [`require`]'s own decision, factored out for the one caller that is not a door: `Core\Cap::has`,
/// which is `rule:security/optional-capability-degrades`
/// 's way for a package that declared a capability *optional* to degrade instead of failing a
/// build. Every door goes on calling `require`, because a door needs the sentence a denial prints
/// and this answers only the yes or no.
///
/// **Reporting a grant is not widening one.** § 7's runtime layer may only ever drop, and this
/// reads the same effective configuration a door reads at the same instant — so an answer of `true`
/// is a fact about what the request already holds, never a step towards holding it. Nothing here
/// records the question either: a member that answered and then let the program proceed on the
/// strength of the answer would still meet `require` at the door.
#[must_use]
pub fn granted(ctx: &Ctx, cap: Cap, scope: Scope<'_>) -> bool {
    // `rule:security/isolate-shares-nothing`'s `grants:` narrowing, asked first
    // and beside the overlay rather than instead of it: the list a spawn site
    // wrote can only ever subtract, so a capability this context's own
    // configuration does not hold is not granted by being named in one.
    // The configuration's authority adds the default for an unwritten key,
    // read for this run's kind and app folder.
    granted_with(ctx, cap, scope, ctx.temporary_dirs())
}

/// [`granted`], with `temporary` as the directories this context made: what server work writes
/// by default. [`temp_dir`] passes the list with the directory it is about to create.
fn granted_with(ctx: &Ctx, cap: Cap, scope: Scope<'_>, temporary: &[PathBuf]) -> bool {
    ctx.grants_allow(cap)
        && (ctx.config().is_some_and(|config| {
            config.authority().with_temporary(temporary).allows(
                cap,
                scope,
                &nvs_config::resolve::Disk,
            )
        }) || own_file(ctx, cap, scope))
}

/// Whether a `Core\IO` read of a package's own file passes `fs.read` with no scope that covers it
/// (`rule:packaging/a-package-carries-only-its-source-and-what-it-lists`): `cap` is `fs.read`, and
/// the own-file word the call site was compiled with is the word of `scope`'s path, resolved again
/// now. Asked only after the operator's scope said no, so a read the scope covers resolves nothing
/// twice. **Cost:** one resolution and one hash, for a refused read inside a read that has a word.
fn own_file(ctx: &Ctx, cap: Cap, scope: Scope<'_>) -> bool {
    let Scope::Path(path) = scope else {
        return false;
    };
    cap == Cap::FsRead
        && ctx.has_own_file()
        && nvs_config::capability::resolved(path, &nvs_config::resolve::Disk)
            .is_some_and(|at| ctx.owns_file(&at))
}

/// § 5's message. The capability's name comes first after the member because the reader is usually
/// the operator, and that string is what they are about to paste into a configuration file.
///
/// **The second line is the line to add and where**: [`nvs_config::capability::grant_line`]'s
/// line, the table it is written under, which is [`Cap::family`] because a dotted capability name
/// and a `[capabilities.<family>]` block are the same TOML input, and `nvs.toml`. A refusal is the
/// documentation its reader is already looking at, and a line they can paste is what turns "which
/// is not granted" into an instruction.
///
/// It rides on the message rather than on the record an uncaught throw renders, which is what
/// `rule:security/denial-is-a-runtime-error` costs: a denial is catchable, so most of them are read
/// by a program rather than by the floor, and a help line the floor owned would be absent from
/// exactly the path — `catch`, then log `$e->message` — that an operator debugs from. The price is
/// that a program comparing `$e->message` against a literal compares two lines.
fn denial(cap: Cap, scope: Scope<'_>, member: &str) -> String {
    let name = cap.name();
    let subject = match scope {
        Scope::Unscoped => format!("{member} needs the capability `{name}`, which is not granted"),
        Scope::Path(path) => format!(
            "{member} needs the capability `{name}` for {}, which is not granted",
            path.display()
        ),
        Scope::Host(host) | Scope::Name(host) => {
            format!("{member} needs the capability `{name}` for {host}, which is not granted")
        }
        Scope::Endpoint(endpoint) => {
            format!("{member} needs the capability `{name}` for {endpoint}, which is not granted")
        }
    };
    let family = cap.family();
    let line = nvs_config::capability::grant_line(cap, scope);
    format!("{subject}\nhelp: add `{line}` under `[capabilities.{family}]` in nvs.toml")
}

/// `rule:http-server/allow-url-pins-the-address`'s outbound door: the first address
/// `host` is approved to be reached at, once [`Cap::NetConnect`] has been shown to cover the name
/// and § 3's policy has been shown to cover every address that name answered.
///
/// [`pin_host_addresses`] is this same door answering the whole approved set, which is what
/// `Core\Http\Target` carries; this one is for a member that connects to a single address, and the
/// address it hands back is the head of that set.
///
/// **The address is the answer, and that is § 2's load-bearing part.** A door that said only "yes"
/// would leave a gap between this check and the connection in which a second DNS resolution could
/// answer differently — the rebinding attack — so the caller is handed the address that was
/// approved and connects to *that*. Every retry of a call reuses it and only a redirect hop asks
/// again (`rule:http-server/retry-is-opt-in-jittered-and-closed`).
///
/// **Here rather than in `nvs-stdlib`**, for § 5's reason and for this crate's: the policy is one
/// policy across `Core\Http`, `Core\Net` and `Core\Db::open`, and resolution is an operating-system
/// effect, which every `Core` member reaches through a door in this module and through nothing
/// else.
///
/// **The lookup is the only part of this that leaves the thread.** The grant is asked here, on the
/// core, before any name is looked up; the lookup goes through whatever [`install_resolver`] put on
/// this thread, which on a worker is a handoff to the blocking pool
/// (`rule:http-server/a-core-is-never-blocked-on-a-syscall`); and § 3's table is asked back on the
/// core, once per address the name answered.
///
/// `member` is what a refusal names — see [`require`].
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `net.connect` for
/// `host`; a `RuntimeError` when the name resolves to no address at all; and a `RuntimeError`
/// naming the range when **any** address the name answered is one § 3 denies and this deployment's
/// `net.internal` does not except ([`nvs_config::tree::CapNet::internal`]) — one denied address
/// refuses the whole host. The order is the point: a host outside the grant is refused before it is
/// looked up, so an ungranted program cannot use this door as a resolver.
pub fn pin_host(ctx: &Ctx, host: &str, member: &str) -> Result<std::net::IpAddr, Fault> {
    require(ctx, Cap::NetConnect, Scope::Host(host), member)?;
    pinned_address(ctx, host, member)
}

/// [`pin_host`]'s whole answer: every address `host` resolved to that § 3's policy approves, in the
/// resolver's order and at most [`PINNED_ADDRESSES`] of them.
///
/// `rule:http-server/an-outbound-call-tries-every-approved-address` is what the set is for — a call
/// falls back across it when the first address does not connect, and a retry reuses it rather than
/// resolving again, so a retried call still performs exactly one lookup. The set is never empty: a
/// name answering nothing is the error below rather than an approval of no addresses.
///
/// # Errors
///
/// [`pin_host`]'s, in the same order — the grant before the lookup, and one denied address refusing
/// the whole host.
pub fn pin_host_addresses(
    ctx: &Ctx,
    host: &str,
    member: &str,
) -> Result<Vec<std::net::IpAddr>, Fault> {
    require(ctx, Cap::NetConnect, Scope::Host(host), member)?;
    pinned_addresses(ctx, host, member)
}

/// The address `host` resolves to, and nothing asked about it — [`pinned_address`]'s first half,
/// for the doors whose endpoint an operator wrote into root-owned configuration.
///
/// `rule:config/cache-shared-is-the-grant-over-the-configured-store` is why one exists: a store the
/// operator named is authorized by that naming, so the grant over it is asked at the door and § 3's
/// denied-range table is not asked at all — the table exists to keep a *program-supplied* endpoint
/// away from the local machine, which is precisely where such a store usually lives. Splitting it
/// out rather than resolving in `nvs-stdlib` keeps `rule:security/capability-check-at-the-door`'s
/// shape: name resolution is an operating-system effect, and every `Core` member reaches one
/// through this module.
///
/// It answers no policy question, so a caller that has not already shown its own grant is calling
/// the wrong function — [`pin_host`] is the one for an endpoint a program named.
///
/// # Errors
///
/// A `RuntimeError` when the name resolves to no address at all. There is no second failure here:
/// every refusal [`pinned_address`] can add is one this door's callers do not ask.
pub fn resolve_host(host: &str, member: &str) -> Result<std::net::IpAddr, Fault> {
    resolve_addresses(host, member)?
        .into_iter()
        .next()
        .ok_or_else(|| unresolved(host, member))
}

/// How many of the addresses one name answers an approval keeps.
///
/// **Eight**, which is `rule:http-server/an-outbound-call-tries-every-approved-address`'s number:
/// falling back across the set is what a name's second address is for, and a set as long as
/// whatever a resolver felt like answering is a connect budget nothing bounds. The cap is applied in
/// the resolver's order and before the policy is asked, so a caller holds the head of the answer
/// rather than a sample of it.
pub const PINNED_ADDRESSES: usize = 8;

/// How a name becomes addresses, for the crate that has a pool to hand the lookup to.
///
/// `rule:http-server/a-core-is-never-blocked-on-a-syscall` sends name resolution to the blocking
/// pool, and this crate is the bottom of the tree: it cannot reach `nvs_host`, where that pool
/// lives. So the handoff arrives as a function pointer, [`install_resolver`] installs it, and every
/// door here reaches it through [`resolve_addresses`]. An [`std::io::Error`] and an empty answer are
/// the same outcome to a caller — the name has no address — and the sentence it reads is the door's
/// either way.
pub type Resolver = fn(&str) -> std::io::Result<Vec<std::net::IpAddr>>;

thread_local! {
    /// This thread's resolver, or none, in which case [`lookup_on_this_thread`] is it.
    ///
    /// Per thread rather than per process, which is what makes the fallback *exact* rather than
    /// merely safe: a worker installs a resolver that hands the lookup to its own pool, and a thread
    /// that is not a worker — a test, a CLI path — has no core to protect and resolves inline, which
    /// is what `nvs_host::blocking::run` does off a core anyway.
    static RESOLVER: std::cell::Cell<Option<Resolver>> = const { std::cell::Cell::new(None) };
}

/// Installs `resolver` as this thread's, replacing whatever was installed before it.
///
/// Called once per worker thread at boot, by the crate that owns the blocking pool. Nothing on the
/// request path installs anything, so the answer a door gets is fixed for the life of the thread.
pub fn install_resolver(resolver: Resolver) {
    RESOLVER.with(|slot| slot.set(Some(resolver)));
}

/// Every address `host` answers, resolved **on this thread** — the lookup a [`Resolver`] wraps, and
/// the one spelling of it in the tree.
///
/// `nvs-host` calls this from inside its pool, so that the grammar of resolution stays here beside
/// the policy that judges what comes back rather than being written a second time in the crate that
/// owns the handoff.
///
/// # Errors
///
/// Whatever the platform resolver reported. A caller turns it into the door's own sentence, since
/// only the door knows which member to name.
pub fn lookup_on_this_thread(host: &str) -> std::io::Result<Vec<std::net::IpAddr>> {
    use std::net::ToSocketAddrs;

    Ok((host, 0_u16)
        .to_socket_addrs()?
        .map(|socket| socket.ip())
        .collect())
}

/// Every address `host` resolves to that a door will carry: the resolver's order, no address twice,
/// and at most [`PINNED_ADDRESSES`] of them.
///
/// It answers no policy question — [`pinned_addresses`] is the one that does — and a host given as
/// an IP address is a set of one that reaches no resolver at all, on a core or off it, because there is nothing for a
/// lookup to answer differently.
///
/// # Errors
///
/// A `RuntimeError` when the name answers no address, which is also what a resolver's own failure
/// looks like from here.
pub fn resolve_addresses(host: &str, member: &str) -> Result<Vec<std::net::IpAddr>, Fault> {
    use std::net::IpAddr;

    // A bracketed IPv6 address is written `[::1]` inside an authority and is not one anywhere else,
    // so the brackets come off before the address is read and stay off afterwards.
    let bare = host
        .strip_prefix('[')
        .and_then(|held| held.strip_suffix(']'))
        .unwrap_or(host);
    if let Ok(literal) = bare.parse::<IpAddr>() {
        return Ok(vec![literal]);
    }

    let resolver = RESOLVER
        .with(std::cell::Cell::get)
        .unwrap_or(lookup_on_this_thread);
    let mut kept: Vec<IpAddr> = Vec::new();
    for address in resolver(host).unwrap_or_default() {
        if kept.len() == PINNED_ADDRESSES {
            break;
        }
        // The same address twice is one connect attempt and not two: a resolver answering the same
        // machine under two records, or the same record over both families, is ordinary.
        if !kept.contains(&address) {
            kept.push(address);
        }
    }
    if kept.is_empty() {
        return Err(unresolved(host, member));
    }
    Ok(kept)
}

/// The sentence a name with no address gets, from whichever door asked for it.
fn unresolved(host: &str, member: &str) -> Fault {
    Fault::thrown(format!(
        "{member} could not resolve {host}, so there is no address to pin"
    ))
}

/// [`pin_host`]'s second half on its own: [`resolve_addresses`] once, and the whole host refused if
/// § 3's table denies any address the name answered.
///
/// **Split out because one member asks the capability question differently and the address
/// question identically.** `Core\Db::open`'s grant is `db.open`, whose scope is the host its
/// settings object named (`rule:core-classes/db-capabilities`), so asking
/// `net.connect` as well would demand a second grant for the same host; what § 3 does say is that
/// an `open` target "stays subject to that policy in full", and *that* policy is this function.
/// Calling [`pin_host`] there instead would collapse two capabilities into one, and re-implementing
/// the range check beside it would be a second home for the rule — which is the reason this is a
/// door here rather than a few lines in `nvs-stdlib`.
///
/// Every caller still owes its own capability check first, and every caller in this module makes
/// it before it reaches here: a host outside the grant is refused before it is looked up, so an
/// ungranted program cannot use this as a resolver.
///
/// # Errors
///
/// A `RuntimeError` when `host` is a socket path, which is a target this door cannot authorize
/// whatever the roster holds — the grant over one is `Cap::NetLocal`, asked of a path by the member
/// that takes a path; a `RuntimeError` when the name resolves to no address at all; and a
/// `RuntimeError` naming the range when the address it resolves to is one § 3 denies and this deployment's
/// `net.internal` does not except.
pub fn pinned_addresses(
    ctx: &Ctx,
    host: &str,
    member: &str,
) -> Result<Vec<std::net::IpAddr>, Fault> {
    // `rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it`: a socket path is a
    // way onto the local machine that § 3's table cannot see, because there is no address for it to
    // match. It is refused here, in front of the resolution and of every caller's socket, so the
    // answer is the same whether or not anything is bound at the path -- a refusal that had to open
    // the path first would report the difference, and the difference is what a probe reads. A
    // separator is the whole test: no name and no IP address carries one, where a list of the
    // sockets a host keeps would be wrong on the machine nobody tested.
    if host.contains('/') || host.contains('\\') {
        return Err(Fault::thrown(format!(
            "{member} refuses `{host}`: a socket path is not something this door can authorize, \
             since the address policy `net.connect` carries has no address to read — the grant \
             that answers a socket path is `net.local`, and it is asked by the member that takes \
             a path rather than by this one, which takes a host"
        )));
    }

    let addresses = resolve_addresses(host, member)?;

    // § 3's table, less whatever this deployment excepted from it with `net.internal`, asked of
    // every address the name answered: one denied refuses the whole host and names that address,
    // per `rule:http-server/an-outbound-call-tries-every-approved-address`. A context with no
    // snapshot, and one whose snapshot grants no capability at all, both get the table itself -- an
    // exception is something an operator wrote, so its absence is the default and not a reason to
    // skip the question.
    for &address in &addresses {
        let refused = match ctx.config() {
            Some(config) => config.authority().address_refused(address),
            None => nvs_config::capability::denied_by_default(address),
        };
        if let Some(range) = refused {
            let line = nvs_config::capability::internal_line(address);
            return Err(Fault::thrown(format!(
                "{member} cannot connect to {address}: it is {range}, which `net.internal` does \
                 not except\nhelp: add `{line}` under `[capabilities.net]` in nvs.toml"
            )));
        }
    }

    Ok(addresses)
}

/// [`pinned_addresses`]'s first address: the head of the approved set, for a door whose member
/// connects to one address and a caller that has already shown its own grant.
///
/// # Errors
///
/// [`pinned_addresses`]'s, unchanged — every address the name answered is still judged, so the
/// address this hands back is the first of a set none of which was refused.
pub fn pinned_address(ctx: &Ctx, host: &str, member: &str) -> Result<std::net::IpAddr, Fault> {
    pinned_addresses(ctx, host, member)?
        .into_iter()
        .next()
        .ok_or_else(|| unresolved(host, member))
}

/// § 2's read door: the file at `path`, open for reading, once [`Cap::FsRead`] has been shown to
/// cover it.
///
/// The open handle rather than the bytes, because one door has to serve a whole-file read and an
/// incremental one alike, and the capability question belongs to the *handle*: a descriptor already
/// open is a descriptor already checked, so nothing downstream of this call has to ask again.
///
/// `member` is what a refusal names — see [`require`].
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`, or [`io_failure`]'s `IOError` when the open itself fails. The order is the point: a path
/// outside the grant is refused as a capability whether or not it exists, so a program cannot use
/// the difference between the two messages to probe a directory it was never allowed to read.
/// The open is [`resolve`]'s path through [`crate::beneath`], so a link planted after the check
/// fails it.
pub fn open_read(ctx: &Ctx, path: &Path, member: &str) -> Result<File, Fault> {
    let at = resolve_file(ctx, path, &[Cap::FsRead], member)?;
    nvs_footprint::file(path);
    crate::beneath::open_file(&at, crate::beneath::Mode::Read)
        .map_err(|err| io_failure(member, path, &err))
}

/// What a program asked an open handle for, and so which capability [`open`] has to show.
///
/// Declared here rather than in `nvs-stdlib` because the capability question is this module's and
/// the answer differs per variant: a door that took an already-built [`std::fs::OpenOptions`] could
/// not ask what the caller intended, since nothing on that type reports back what was set. The
/// surface enum a program writes is `Core\IO::FileMode`, which maps onto this one and adds nothing —
/// the two are separate so that `nvs-runtime` does not learn a spelling from the standard library's
/// roster, exactly as [`Scope`] is `nvs_config`'s rather than a `Core` type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// Reading only, from the start of an existing file: `fs.read`.
    Read,
    /// Writing only, truncating what was there and creating the path if it is not: `fs.write`.
    Write,
    /// Writing only, at the end of the file, creating the path if it is not: `fs.write`.
    Append,
    /// Reading and writing, creating the path if it is not and truncating nothing: **both**
    /// `fs.read` and `fs.write`, because a handle that can do either is a handle that can do both.
    ReadWrite,
}

/// § 2's handle door: the file at `path`, open for what `access` names, once every capability that
/// access needs has been shown to cover it.
///
/// This is [`open_read`] generalised to the writing accesses, and the split between them is
/// deliberate: `open_read` is the whole-file read every `Core\IO` reader shares, and this is the one
/// a `Core\IO\File` handle comes out of. A writing open **creates** the path it names, which
/// [`write()`]'s own doc calls a reason to keep the create on the door's side — it is on this side
/// too, because the check below runs before anything is opened. Every access opens the path
/// [`resolve`] checked, through [`crate::beneath`], so no link planted after the check is followed.
///
/// A descriptor this answers with is a descriptor already checked: nothing downstream asks the
/// capability question again, which is why `Core\IO\File`'s own members declare no capability of
/// their own.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant a capability
/// `access` needs for `path`, or [`io_failure`]'s `IOError` when the open itself fails. The
/// capability is checked first for [`open_read`]'s reason, and for [`Access::ReadWrite`] both are
/// checked before either is used, so a path granted for reading and not for writing refuses as a
/// capability rather than as a failed open.
pub fn open(ctx: &Ctx, path: &Path, access: Access, member: &str) -> Result<File, Fault> {
    let caps: &[Cap] = match access {
        Access::Read => &[Cap::FsRead],
        Access::Write | Access::Append => &[Cap::FsWrite],
        Access::ReadWrite => &[Cap::FsRead, Cap::FsWrite],
    };
    let at = resolve_file(ctx, path, caps, member)?;
    if matches!(access, Access::Read | Access::ReadWrite) {
        nvs_footprint::file(path);
    }
    let mode = match access {
        Access::Read => crate::beneath::Mode::Read,
        Access::Write => crate::beneath::Mode::Truncate,
        Access::Append => crate::beneath::Mode::Append,
        // No truncate: a read-write handle that emptied the file before its
        // holder had read a byte is `fopen`'s `w+`, and the mode a program
        // reaches for when it wants both is the one that keeps what is there.
        Access::ReadWrite => crate::beneath::Mode::ReadWrite,
    };
    crate::beneath::open_file(&at, mode).map_err(|err| io_failure(member, path, &err))
}

/// § 2's streaming-write door: a handle on `path`, ready to become the whole of its content, once
/// [`Cap::FsWrite`] has been shown to cover it.
///
/// [`write()`] hands back the finished effect because it already holds every byte; this is the same
/// door for a caller that does not — `Core\IO::writeStream` writes what it is handed as it is handed
/// it, so the handle has to cross. The create is still on this side of it, which is [`write()`]'s own
/// reason for taking the bytes.
///
/// **`overwrite` is a parameter rather than another [`Access`] case**, because that enum is
/// `Core\IO::FileMode`'s cases and nothing else: a case no mode spells would be a variant the
/// surface enum could never produce. And **`overwrite == false` refuses through the operating
/// system** — `create_new`, which is `O_EXCL` — rather than through an [`exists`] call first: a check
/// followed by a create is a window another process can create the file in, and `rule:core-classes/io-write-stream` makes
/// this the default precisely because the destination is usually named by a client.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// `path`, checked before anything is created for [`write()`]'s reason. [`io_failure`]'s `IOError`
/// when the create itself fails — including `AlreadyExists`, which is what a refused overwrite is.
pub fn create(ctx: &Ctx, path: &Path, overwrite: bool, member: &str) -> Result<File, Fault> {
    let at = resolve_file(ctx, path, &[Cap::FsWrite], member)?;
    crate::beneath::create_file(&at, overwrite).map_err(|err| io_failure(member, path, &err))
}

/// The write doors' check: [`resolve`] for [`Cap::FsWrite`].
///
/// # Errors
///
/// [`resolve`]'s.
fn resolve_write(ctx: &Ctx, path: &Path, member: &str) -> Result<PathBuf, Fault> {
    resolve(ctx, path, &[Cap::FsWrite], member)
}

/// The opening doors' check: `path` resolved once, and each of `caps` asked about that resolved
/// path as it is written, which is the path the caller then opens with [`crate::beneath`].
///
/// **One resolution, and the open follows no link** (`rule:security/writes-open-beneath-a-handle`).
/// [`require`] resolves the path inside the grant check, and a door that then opened the path by
/// name would resolve it a second time. A folder replaced by a link between the two would be
/// followed outside the grant. Here the check and the open use the same resolved path, and a link
/// planted after the check fails the open.
///
/// # Errors
///
/// [`relative_refusal`]'s `RuntimeError` for a path with no root, and [`require`]'s `RuntimeError`
/// when the grant does not cover the resolved path. When no ancestor of `path` resolves at all,
/// [`require`]'s answer on `path` decides: its denial when the grant does not cover it, or an
/// `IOError` when it does, because then there is nothing to open.
fn resolve(ctx: &Ctx, path: &Path, caps: &[Cap], member: &str) -> Result<PathBuf, Fault> {
    relative_refusal(path, member)?;
    device_refusal(path, member)?;
    alias_refusal(path, member)?;
    let at = nvs_config::capability::resolved(path, &nvs_config::resolve::Disk);
    allow(ctx, path, at, caps, member)
}

/// [`resolve`] for a door that opens a file, which a path ending with a separator never names.
///
/// The resolved path has no separator at its end, so it would open a file named like the folder
/// that path spells. This checks the path as the program wrote it, after the grant, so a path
/// outside the grant is still refused as a capability.
///
/// # Errors
///
/// [`resolve`]'s, or an `IOError` for a path that ends with a separator.
fn resolve_file(ctx: &Ctx, path: &Path, caps: &[Cap], member: &str) -> Result<PathBuf, Fault> {
    let at = resolve(ctx, path, caps, member)?;
    let written = path.as_os_str().to_string_lossy();
    if written.ends_with(std::path::is_separator) {
        let err = std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "a path that ends with a separator names a folder, not a file",
        );
        return Err(io_failure(member, path, &err));
    }
    Ok(at)
}

/// [`resolve_write`] for a door that changes the name itself rather than the file it names: the
/// parent of `path` resolved, and its last name kept as written.
///
/// [`rename`] moves the entry, and [`remove_file`] and [`remove_dir`] remove it, so a link at the
/// name is the entry to move, replace or remove. Resolving it would put the link's target in its
/// place, and a rename onto a link would then replace the file the link points at.
///
/// # Errors
///
/// [`resolve`]'s.
fn resolve_name(ctx: &Ctx, path: &Path, member: &str) -> Result<PathBuf, Fault> {
    relative_refusal(path, member)?;
    device_refusal(path, member)?;
    alias_refusal(path, member)?;
    let at = match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => {
            nvs_config::capability::resolved(parent, &nvs_config::resolve::Disk)
                .map(|parent| parent.join(name))
        }
        _ => None,
    };
    allow(ctx, path, at, &[Cap::FsWrite], member)
}

/// The grant check [`resolve`] and [`resolve_name`] share: each of `caps` on `at`, the resolved
/// spelling of `path`, or `None` when it did not resolve. The first capability not granted is the
/// one the denial names.
fn allow(
    ctx: &Ctx,
    path: &Path,
    at: Option<PathBuf>,
    caps: &[Cap],
    member: &str,
) -> Result<PathBuf, Fault> {
    let Some(at) = at else {
        for &cap in caps {
            require(ctx, cap, Scope::Path(path), member)?;
        }
        let err = std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no ancestor of this path could be resolved",
        );
        return Err(io_failure(member, path, &err));
    };
    for &cap in caps {
        let allowed = ctx.grants_allow(cap)
            && ctx.config().is_some_and(|config| {
                config
                    .authority()
                    .with_temporary(ctx.temporary_dirs())
                    .allows_resolved(cap, &at)
            })
            || (cap == Cap::FsRead && ctx.grants_allow(cap) && ctx.owns_file(&at));
        if !allowed {
            return Err(Fault::thrown(denial(cap, Scope::Path(path), member)));
        }
    }
    Ok(at)
}

/// § 2's write door: `bytes` become the whole content of `path`, once [`Cap::FsWrite`] has been
/// shown to cover it.
///
/// The finished effect rather than an open handle, unlike [`open_read`], because a write *creates*
/// the path it names: `nvs_config::capability` § 4's argument side resolves the deepest existing
/// ancestor precisely so this check can happen before anything is created, and handing back a
/// handle would put the create on the caller's side of the door.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// `path`, or [`io_failure`]'s `IOError` when the write itself fails.
pub fn write(ctx: &Ctx, path: &Path, bytes: &[u8], member: &str) -> Result<(), Fault> {
    let at = resolve_file(ctx, path, &[Cap::FsWrite], member)?;
    crate::beneath::create_file(&at, true)
        .and_then(|mut file| std::io::Write::write_all(&mut file, bytes))
        .map_err(|err| io_failure(member, path, &err))
}

/// § 2's copy door: `to` becomes a duplicate of `from`, once [`Cap::FsRead`] has been shown to cover
/// the source and [`Cap::FsWrite`] the destination.
///
/// **Two paths, two capabilities, and both are checked before either is used**, for [`open`]'s
/// reason: a source a program may read and a destination it may not write refuses as a capability
/// rather than half-way through an effect. The split is the honest one — a copy reads one name and
/// writes another — so a grant that opens a directory for reading never becomes a way to fill it.
///
/// **The destination is replaced if it is there**, exactly as [`write()`] replaces, and unlike
/// [`create`]'s `overwrite: false`: the difference is who names the path. A stream's destination is
/// usually a name a client supplied, and this one is a name the program wrote beside a source it
/// already holds.
///
/// The source is opened as [`open_read`] opens one, and the destination as [`write()`] opens one
/// (`rule:security/writes-open-beneath-a-handle`), and the source's permissions are copied onto it after its bytes. The byte count is not returned,
/// because `Core\IO::copy` has nothing to say about it.
///
/// **A destination that is the source itself is refused**, under any spelling and through a hard
/// link. The destination is opened without emptying it, the two handles are compared, and only a
/// destination that is another file is emptied. Emptying first would copy an empty file onto itself.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `from` or `fs.write` for `to`, or [`io_failure`]'s `IOError` when the source is not a file, the
/// two paths name one file, or the copy itself fails. The message names [`pair`]'s both-ends
/// spelling, because either end can be the one at fault.
pub fn copy(ctx: &Ctx, from: &Path, to: &Path, member: &str) -> Result<(), Fault> {
    let from_at = resolve_file(ctx, from, &[Cap::FsRead], member)?;
    let at = resolve_file(ctx, to, &[Cap::FsWrite], member)?;
    let fail = |err: &std::io::Error| io_failure(member, &pair(from, to), err);
    let refuse = |why: &str| fail(&std::io::Error::new(std::io::ErrorKind::InvalidInput, why));
    let mut source = crate::beneath::open_file(&from_at, crate::beneath::Mode::Read)
        .map_err(|err| fail(&err))?;
    let permissions = source.metadata().map_err(|err| fail(&err))?;
    if !permissions.is_file() {
        return Err(refuse("the source is not a file"));
    }
    let mut destination =
        crate::beneath::open_file(&at, crate::beneath::Mode::Write).map_err(|err| fail(&err))?;
    if crate::beneath::same_file(&source, &destination).map_err(|err| fail(&err))? {
        return Err(refuse("the source and the destination are the same file"));
    }
    destination
        .set_len(0)
        .and_then(|()| std::io::copy(&mut source, &mut destination))
        .and_then(|_| destination.set_permissions(permissions.permissions()))
        .map_err(|err| fail(&err))
}

/// § 2's rename door: the name `from` becomes the name `to`, once [`Cap::FsWrite`] has been shown to
/// cover **both**.
///
/// `fs.write` on the source and not [`copy`]'s `fs.read`, which is the whole difference between the
/// two doors: a move takes the source away, and taking a file away is destroying it. A grant that
/// let a program read a directory would otherwise let it empty one.
///
/// **A rename across filesystems fails rather than falling back to a copy and a removal.** The
/// operating system's rename is atomic — the destination is the whole file or the old one — and a
/// copy followed by a delete is neither atomic nor the same failure surface. A program that wants
/// the fallback spells it with [`copy`] and [`remove_file`], which is two capability checks in the
/// order it chose.
///
/// Both ends are checked by [`resolve_name`] and renamed by [`crate::beneath::rename`]
/// (`rule:security/writes-open-beneath-a-handle`): a link at either end is the name that moves or is
/// replaced, and a link planted above either end after the check fails the rename.
///
/// # Errors
///
/// [`resolve_name`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// either path, or [`io_failure`]'s `IOError` when the rename itself fails — nothing at the source,
/// a destination directory that is not there, or the two paths on different filesystems.
pub fn rename(ctx: &Ctx, from: &Path, to: &Path, member: &str) -> Result<(), Fault> {
    let source = resolve_name(ctx, from, member)?;
    let destination = resolve_name(ctx, to, member)?;
    crate::beneath::rename(&source, &destination)
        .map_err(|err| io_failure(member, &pair(from, to), &err))
}

/// § 2's mkdir door: a directory exists at `path` when this returns, once [`Cap::FsWrite`] has been
/// shown to cover it.
///
/// **Missing parents are created, where [`remove_dir`] refuses to recurse**, and the asymmetry is
/// the point rather than an inconsistency. A recursive removal is one grant check standing in for a
/// whole tree of *destructions*, any one of which is unrecoverable; a recursive creation makes empty
/// directories that are all, necessarily, under the path the check just covered — a grant is a root,
/// so an ancestor of a granted path that this creates is one the grant already reaches through.
/// Nothing is destroyed and nothing outside the grant is touched.
///
/// **A directory that is already there is success, not a refusal.** The member's contract is that
/// the directory exists afterwards, and a refusal would leave every caller writing an [`exists`]
/// check in front of it — which is the window between a question and an act that [`create`]'s own
/// doc refuses to open. A *file* at the path is still a failure: that is not the directory the
/// caller asked for.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// `path`, checked before anything is created for [`write()`]'s reason, or [`io_failure`]'s
/// `IOError` when the creation itself fails — a component that exists and is not a directory, a
/// folder replaced by a link after the check, or a permission the process lacks. The levels are
/// created from the root down by [`crate::beneath::create_dirs`], for [`resolve_write`]'s reason.
pub fn create_dir(ctx: &Ctx, path: &Path, member: &str) -> Result<(), Fault> {
    let at = resolve_write(ctx, path, member)?;
    crate::beneath::create_dirs(&at).map_err(|err| io_failure(member, path, &err))
}

/// The two ends of a [`copy`] or a [`rename`], as the one path [`io_failure`] names.
///
/// A two-path member has two candidate culprits and the operating system's error says which kind of
/// failure it was without saying which end it was about, so the message carries both, in the
/// direction the member reads. This is a spelling for a message and never a path anything opens.
fn pair(from: &Path, to: &Path) -> PathBuf {
    PathBuf::from(format!("{} -> {}", from.display(), to.display()))
}

/// § 2's metadata door: what the operating system knows about `path`, once [`Cap::FsRead`] has been
/// shown to cover it.
///
/// Reading a file's size, kind or timestamps is reading the file, so this is the same capability
/// [`open_read`] asks for and not a weaker one: a program that can measure a path it was not granted
/// can enumerate a directory it was never allowed to open.
///
/// The whole [`Metadata`] rather than the one field a caller wants, because every question a
/// `Core\IO` metadata member asks is answered by one `stat` and a second door per field would be a
/// second syscall for the same permission.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`, or [`io_failure`]'s `IOError` when the `stat` itself fails — a path that is not there is
/// a failure here, because a member asking for a size has no answer for one.
///
/// The path is resolved once by [`resolve`] and read by [`crate::beneath::metadata`]
/// (`rule:security/writes-open-beneath-a-handle`), so a folder replaced by a link after the check
/// fails the read, and a link at the last name is not followed.
pub fn metadata(ctx: &Ctx, path: &Path, member: &str) -> Result<Metadata, Fault> {
    let at = resolve(ctx, path, &[Cap::FsRead], member)?;
    nvs_footprint::exists(path);
    crate::beneath::metadata(&at).map_err(|err| io_failure(member, path, &err))
}

/// § 2's kind door: the [`Metadata`] at `path` if there is anything there, once [`Cap::FsRead`] has
/// been shown to cover it, and `None` if there is not.
///
/// Separate from [`metadata`] for the same reason [`exists`] is: **absence is an answer here, not a
/// failure**, because the members over this door — `Core\IO::isFile` and `Core\IO::isDir` — ask what
/// kind of thing is at a name, and *nothing* is a complete answer to that. It is one door rather
/// than an [`exists`] followed by a [`metadata`] so that the two questions are one `stat`: the pair
/// answers the wrong thing for a name something else removes between them, and a member whose
/// falsity depends on losing that race is not one this class will ship.
///
/// Only [`std::io::ErrorKind::NotFound`] becomes `None`. Every other failure stays a failure, so a
/// parent directory the process may not traverse throws here rather than reporting the name as
/// absent — which is the same line [`exists`] draws, and for the same reason: a `false` that can
/// mean *not allowed* tells a program nothing.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`, or [`io_failure`]'s `IOError` when the `stat` failed for any reason other than the path
/// not being there. The read is [`metadata`]'s, so a link at the last name that leads nowhere is
/// `None`.
pub fn metadata_if_present(
    ctx: &Ctx,
    path: &Path,
    member: &str,
) -> Result<Option<Metadata>, Fault> {
    let at = resolve(ctx, path, &[Cap::FsRead], member)?;
    nvs_footprint::exists(path);
    match crate::beneath::metadata(&at) {
        Ok(found) => Ok(Some(found)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(io_failure(member, path, &err)),
    }
}

/// § 2's `realpath` door: what `path` resolves to **when every component of it already exists**,
/// once [`Cap::FsRead`] has been shown to cover it.
///
/// The other resolution door, and the difference from [`canonicalize`] is **only** what it does
/// about a path that is not there: that one answers by pinning the deepest existing ancestor,
/// because `Core\IO::within` has to prove containment for a name about to be created, and this one
/// refuses, because `Core\IO::canonicalize` is `realpath` and `realpath` has no answer for a name
/// with nothing at it.
///
/// **The walk is the same walk**, and that is the load-bearing part rather than an implementation
/// detail: a class whose two resolving members answered different *spellings* of one file — a
/// verbatim `\\?\C:\…` from [`std::fs::canonicalize`] against a plain path from
/// [`nvs_config::capability::resolved`] — would hand a program comparing them a wrong answer on one
/// platform and a right one on the next. So existence is asked here as its own question and the
/// resolution is delegated, rather than reaching for the standard library's resolver and getting a
/// second spelling with it.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`, or [`io_failure`]'s `IOError` when there is nothing at `path` — including a symbolic
/// link that leads nowhere, since the `stat` follows it — or when nothing about the path could be
/// resolved.
pub fn resolve_existing(ctx: &Ctx, path: &Path, member: &str) -> Result<PathBuf, Fault> {
    require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    nvs_footprint::exists(path);
    std::fs::metadata(path).map_err(|err| io_failure(member, path, &err))?;
    nvs_config::capability::resolved(path, &nvs_config::resolve::Disk).ok_or_else(|| {
        io_failure(
            member,
            path,
            &std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no ancestor of this path could be resolved",
            ),
        )
    })
}

/// § 2's existence door: whether anything is at `path`, once [`Cap::FsRead`] has been shown to cover
/// it.
///
/// Separate from [`metadata`] for the one reason that matters to a caller: **absence is an answer
/// here, not a failure.** Everything else about the two is the same, including which capability is
/// asked and that it is asked first — so a path outside the grant is refused whether or not it
/// exists, and the difference between a missing file and an unreadable one leaks nothing.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`, or [`io_failure`]'s `IOError` when the operating system could answer neither yes nor no —
/// a parent directory it will not traverse, for instance, which is not the same as "no".
///
/// It is [`metadata_if_present`] with the metadata dropped, so the read is [`metadata`]'s.
pub fn exists(ctx: &Ctx, path: &Path, member: &str) -> Result<bool, Fault> {
    metadata_if_present(ctx, path, member).map(|found| found.is_some())
}

/// § 2's access door for reading: whether the operating system would let **this process** read what
/// is at `path`, once [`Cap::FsRead`] has been shown to cover it.
///
/// **The two gates answer differently on purpose, and this is the whole design of the pair.** The
/// capability is the configuration's answer to "may this program touch that name at all", and it
/// **refuses** — a path outside the grant throws here exactly as it does at every other door. Only
/// then does the member ask the operating system's question, which is about this process's uid, the
/// mode bits and the mount, and that one answers `false`. Folding the first into the second would
/// hand a program a boolean it could sweep the filesystem with to map its own configuration, which
/// is precisely the enumeration [`exists`] is behind a capability to prevent.
///
/// Absence is `false` and not a failure — a name that is not there is not readable, which is the
/// same answer PHP's `is_readable` gives and is what makes this member usable as a guard before a
/// read rather than a second thing to wrap in a `try`.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`. Nothing else: every operating-system outcome, absence included, is one of the two
/// booleans.
pub fn readable(ctx: &Ctx, path: &Path, member: &str) -> Result<bool, Fault> {
    require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    nvs_footprint::exists(path);
    Ok(permitted(path, false))
}

/// § 2's access door for writing: whether the operating system would let **this process** write what
/// is at `path`, once [`Cap::FsWrite`] has been shown to cover it.
///
/// [`readable`]'s doc is the home of why the capability refuses where the operating system answers
/// `false`. What is decided *here* is which capability: `fs.write` and not `fs.read`, by the same
/// reading that puts `size` behind `fs.read` — a member's capability is about the effect its
/// question is *about*, and this question is entirely about writing. A program granted only reads
/// therefore cannot ask where it could write, which is the answer a read-only program has no use for
/// and an escaping one has every use for.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// `path`, and nothing else.
pub fn writable(ctx: &Ctx, path: &Path, member: &str) -> Result<bool, Fault> {
    require(ctx, Cap::FsWrite, Scope::Path(path), member)?;
    nvs_footprint::exists(path);
    Ok(permitted(path, true))
}

/// Whether this process may write (or, for `write` false, read) what is at `path`, as the operating
/// system itself would decide it at the moment of the call.
///
/// **`access(2)` on Unix, and the read-only attribute on Windows**, which is exactly the split PHP's
/// own `is_readable`/`is_writable` make and for the same reason: only one of the two platforms has a
/// question to ask. Unix permission is a function of the process's real uid and gid against the mode
/// bits of the file *and* of every directory above it, so nothing short of the syscall answers it —
/// `std::fs::Permissions::readonly` is true only when no write bit is set for *anybody*, which says
/// nothing about whether this process is the owner. Windows has no uid in that sense at this layer:
/// a handle-based check would need a full access-token comparison against the DACL, and the
/// attribute is what its own CRT's `_waccess` reports.
///
/// **This is inherently a snapshot**, on both platforms and in PHP alike: the answer is about the
/// instant it was asked, and anything may change the permission before the caller acts on it. That
/// is a reason to prefer attempting the operation and catching the failure, and it is why this is a
/// door under a member rather than something any `Core` writer consults on a caller's behalf.
///
/// A path that cannot be spelled for the platform call — a Unix path holding an interior NUL — is
/// `false`, because there is nothing there for the answer to be about.
#[cfg(unix)]
fn permitted(path: &Path, write: bool) -> bool {
    use std::os::unix::ffi::OsStrExt;

    let Ok(name) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    let mode = if write { libc::W_OK } else { libc::R_OK };
    #[expect(
        unsafe_code,
        reason = "`access` reads the NUL-terminated string it is handed and nothing else, and \
                  `name` owns that allocation for the length of the call"
    )]
    let answer = unsafe { libc::access(name.as_ptr(), mode) };
    answer == 0
}

/// See the `unix` half above, which is the home of this pair's reasoning.
#[cfg(windows)]
fn permitted(path: &Path, write: bool) -> bool {
    let Ok(stat) = std::fs::metadata(path) else {
        return false;
    };
    !write || !stat.permissions().readonly()
}

/// § 2's resolution door: what `path` actually names, once [`Cap::FsRead`] has been shown to cover
/// it — every `..` collapsed by the operating system and every symlink followed.
///
/// Behind `fs.read` and not behind nothing, because resolving a name *reads* the directories above
/// it: a program that can canonicalize a path it was never granted can learn which of its
/// components exist, which is the enumeration [`exists`] is refused for.
///
/// The walk is [`nvs_config::capability::resolved`] and not a second one. It answers for a path that
/// does not exist yet by pinning its deepest existing ancestor, which is what a `Core\IO::within`
/// over a name about to be created needs — and, more importantly, it is the **same** resolution
/// [`require`] just compared against the grant, so a member cannot prove containment about a
/// different path than the one it was allowed to touch.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`, or [`io_failure`]'s `IOError` when nothing about the path could be resolved — not even
/// an ancestor of it exists, or a still-unresolved component is `..`, which the walk refuses rather
/// than collapsing textually.
pub fn canonicalize(ctx: &Ctx, path: &Path, member: &str) -> Result<PathBuf, Fault> {
    require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    nvs_config::capability::resolved(path, &nvs_config::resolve::Disk).ok_or_else(|| {
        io_failure(
            member,
            path,
            &std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no ancestor of this path could be resolved",
            ),
        )
    })
}

/// § 2's enumeration door: what the directory at `path` holds, once [`Cap::FsRead`] has been shown
/// to cover the directory itself.
///
/// The grant is asked about the directory and about nothing under it, because reading a directory
/// is one read of one path — its entries are its content, exactly as a file's octets are its
/// content. A name the listing hands back is a *second* path, and every other door still asks about
/// that one, so `fs.read` over a root lets a program learn what is in the root and buys it nothing
/// else.
///
/// This is the enumeration [`exists`] is careful about, arriving as a door of its own rather than
/// as a widening of one: `exists` answers about a path the caller already named, and this answers
/// about paths it could not name yet. Which is why the check is over the directory and never over
/// an ancestor of it — a program granted one disk's root cannot list the one beside it.
///
/// The directory is resolved once by [`resolve`] and listed by [`crate::beneath::list`]
/// (`rule:security/writes-open-beneath-a-handle`), so a folder replaced by a link after the check
/// fails the listing. Each entry's kind is read without following a link.
///
/// **Memory:** the whole listing, read at once, one name and one kind per entry, until the caller
/// drops it. Every caller collects the names anyway.
///
/// # Errors
///
/// [`resolve`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`, or [`io_failure`]'s `IOError` when the directory could not be listed — it is not there,
/// it is not a directory, or the listing failed partway.
pub fn read_dir(ctx: &Ctx, path: &Path, member: &str) -> Result<Vec<Entry>, Fault> {
    let at = resolve(ctx, path, &[Cap::FsRead], member)?;
    nvs_footprint::dir(path);
    crate::beneath::list(&at).map_err(|err| io_failure(member, path, &err))
}

/// § 2's unlink door: `path` stops existing, once [`Cap::FsWrite`] has been shown to cover it.
///
/// Removal is a write and not a capability of its own, for the reason § 3 gives for not splitting
/// one: an account that may replace a file's whole content can already destroy it, so a separate
/// grant would name a distinction the filesystem does not make.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// `path`, or [`io_failure`]'s `IOError` when the unlink itself fails — the path is not there, or is
/// a directory, which [`remove_dir`] is the door for.
///
/// The name is checked by [`resolve_name`] and removed by [`crate::beneath::remove_file`]
/// (`rule:security/writes-open-beneath-a-handle`): a link at the name is the entry removed, and a
/// link planted above it after the check fails the removal.
pub fn remove_file(ctx: &Ctx, path: &Path, member: &str) -> Result<(), Fault> {
    let at = resolve_name(ctx, path, member)?;
    crate::beneath::remove_file(&at).map_err(|err| io_failure(member, path, &err))
}

/// § 2's rmdir door: the **empty** directory at `path` stops existing, once [`Cap::FsWrite`] has
/// been shown to cover it.
///
/// Empty deliberately, and this is the door's own decision rather than the caller's: a recursive
/// removal is one grant check standing in for a whole tree of them, so a single wrong argument
/// deletes everything under it. A program that means to empty a directory first walks it, and every
/// entry it removes is a path the capability was asked about.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// `path`, or [`io_failure`]'s `IOError` when the removal itself fails — the directory is not there,
/// is not a directory, or still has entries in it.
///
/// The name is checked and removed as [`remove_file`] checks and removes one, by
/// [`crate::beneath::remove_dir`].
pub fn remove_dir(ctx: &Ctx, path: &Path, member: &str) -> Result<(), Fault> {
    let at = resolve_name(ctx, path, member)?;
    crate::beneath::remove_dir(&at).map_err(|err| io_failure(member, path, &err))
}

/// § 2's temporary-directory door: a new, empty, private directory under the root Novis owns, once
/// [`Cap::FsWrite`] has been shown to cover **the path it is about to create**.
///
/// The check is the ordinary one and the argument is the ordinary argument, which is the whole
/// decision here: a member that creates a directory the program never named could plausibly have
/// been exempt from the grant, or have widened it to cover what it created, and both would make a
/// capability something a running program can enlarge. So the name is chosen first and asked about
/// second, exactly as [`write()`] asks about a path that does not exist yet. A written `fs.write`
/// must cover the temporary root. With none written, a CLI run may write anywhere, and server work's
/// default covers the new directory because it is asked with this context's list plus that one.
///
/// **Nothing here relies on the name being unpredictable.** The defence is that creating a directory
/// is atomic: a name an attacker has already taken, including as a symlink, fails with
/// `AlreadyExists` and is retried rather than adopted. Each directory is made by
/// [`nvs_config::data::create_private_dir`]: on Unix it is `0o700` from the moment it exists, so
/// there is no window in which anyone else can read it, and on Windows it inherits the root's DACL,
/// which is private to this account wherever Novis created the root.
///
/// **The root is Novis's own** (`rule:core-classes/temporary-dir-sweep`): `[io] temp_root` when an operator configured one,
/// else `tmp/` in the data folder ([`nvs_config::data`]), created private on first use.
/// That the runtime is the only writer there is the whole safety argument for `rule:core-classes/temporary-dir-orphan-sweep`'s orphan sweep,
/// which deletes entries a dead process left behind — sweeping a shared `/tmp`, with anyone's names
/// and anyone's symlinks in it, is the classic TOCTOU surface and is what the owned root forbids.
///
/// A root that **already exists** is used as it stands and its ownership is not re-examined here.
/// That costs nothing an entry relies on — each one is still created atomically and privately, so a
/// root somebody else made cannot expose what a program puts inside one — but `rule:core-classes/temporary-dir-orphan-sweep`'s sweep is the
/// part that leans on exclusivity, and examining the root is that slice's to own.
///
/// **Every directory this hands back is recorded on the context** ([`Ctx::track_temporary_dir`]),
/// which is what makes `rule:core-classes/temporary-dir-sweep`'s end-of-script sweep possible at all — the program is never asked to
/// remember, and the runtime cannot delete what it did not write down. The record is taken after the
/// directory exists and before the caller sees the path, so there is no ordering in which a program
/// holds a directory the sweep does not know about. That is also why this takes `&mut Ctx` where
/// every other door in this module takes `&Ctx`: it is the one that leaves something behind.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for the
/// path being created, or an `IOError` when there is no root at all ([`temp_root`] is `None`), the
/// root could not be created, or every attempt to create a directory under it failed.
///
pub fn temp_dir(ctx: &mut Ctx, member: &str) -> Result<PathBuf, Fault> {
    // The snapshot, never the request's overlay — [`temp_root`]'s doc owns why that is the only
    // tree this may be asked of.
    let Some(root) = temp_root(ctx.config().map(|request| &request.snapshot().config)) else {
        return Err(Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{member} failed: there is no folder for temporary files. Set `[io] temp_root`, \
                 or pass `--data <folder>` to name a data folder Novis can use."
            ),
        ));
    };
    // Canonical, so the directory it hands back is the spelling a later door's resolved path
    // starts with, which is how server work's default write finds it.
    let root = nvs_config::capability::resolved(&root, &nvs_config::resolve::Disk).unwrap_or(root);
    let path = create_entry(
        &root,
        // Asked of the entry rather than of the root, because that is the path the member hands
        // back (§ 2), and before the root is created, so a program the grant refuses leaves
        // nothing behind. The entry counts as one of this run's directories, so server work's
        // default write covers creating it and nothing else under the root.
        |path| {
            relative_refusal(path, member)?;
            let granted = ctx.with_candidate_temporary_dir(path, |ctx, made| {
                granted_with(ctx, Cap::FsWrite, Scope::Path(path), made)
            });
            if granted {
                Ok(())
            } else {
                Err(Fault::thrown(denial(
                    Cap::FsWrite,
                    Scope::Path(path),
                    member,
                )))
            }
        },
        |path, err| io_failure(member, path, err),
    )?;
    ctx.track_temporary_dir(path.clone());
    Ok(path)
}

/// A private directory under `root`, named the way [`crate::sweep::orphans`] reads an entry, for a
/// caller with no [`Ctx`]: the `.nvst` runner and `nvs lsp-test`, which write their cases where
/// [`temp_root`] says.
///
/// The same create [`temp_dir`] runs, without the grant check and without the end-of-script record.
/// The runner is the operator's own tool rather than a program, so no grant applies, and the caller
/// deletes the directory itself when it finishes. A run that is killed first leaves it behind under
/// the owned root, where its dead pid is what `nvs tmp clean` sweeps.
///
/// # Errors
///
/// The `std::io::Error` of the create that failed, naming its path, when the root could not be
/// created or every attempt to create a directory under it failed.
///
pub fn private_dir(root: &Path) -> std::io::Result<PathBuf> {
    create_entry(
        root,
        |_| Ok(()),
        |path, err| std::io::Error::new(err.kind(), format!("{}: {err}", path.display())),
    )
}

/// The create loop [`temp_dir`] and [`private_dir`] share: `check` is asked of each candidate
/// before anything is written, the root is created on the first attempt, and a name already taken
/// is retried with a new one.
fn create_entry<E>(
    root: &Path,
    mut check: impl FnMut(&Path) -> Result<(), E>,
    fail: impl Fn(&Path, &std::io::Error) -> E,
) -> Result<PathBuf, E> {
    /// Enough attempts that exhausting them means something other than a collision — a full disk, a
    /// root that is not writable, a temporary directory someone has filled with our names.
    const ATTEMPTS: u32 = 16;

    for attempt in 0..ATTEMPTS {
        // The pid at ten digits, the widest a `u32` is, so every process's directory name has
        // the same length and a counted run allocates the same bytes for its paths
        // (`rule:testing/bench-counters`).
        let path = root.join(format!("nvs-{:010}-{:016x}", std::process::id(), nonce()));
        check(&path)?;
        if attempt == 0 {
            // After the check and never before it: creating the root is itself a write.
            nvs_config::data::create_private_root(root).map_err(|err| fail(root, &err))?;
        }
        match nvs_config::data::create_private_dir(&path) {
            Ok(()) => return Ok(path),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(fail(&path, &err)),
        }
    }
    Err(fail(
        root,
        &std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("no unused name after {ATTEMPTS} attempts"),
        ),
    ))
}

/// A value unlikely to repeat within a process or between two of them, for [`temp_dir`]'s candidate
/// name. Not a secret and not required to be one — see that function's own paragraph on why.
fn nonce() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let ticks = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| u64::from(since.subsec_nanos()));
    let counted = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // An odd multiplier so consecutive counter values do not produce consecutive names, which is
    // what would let one process's directories be guessed from another's.
    ticks.wrapping_add(counted.wrapping_mul(0x9e37_79b9_7f4a_7c15))
}

/// `rule:core-classes/temporary-dir-sweep`'s owned root: `[io] temp_root` where a tree set it, else `tmp/` in this process's
/// data folder ([`nvs_config::data::current`]).
///
/// `None` when there is neither: no key, and no data folder to use — the binary's own path could
/// not be read, or the CLI's startup `prepare` found the folder unusable. Nothing is written then,
/// and [`temp_dir`] throws only for a program that asks for a directory.
///
/// **Pass the snapshot's tree, never a request's overlay.** `io.temp_root` is `System`-class and
/// `Boot` (`nvs_config::directive`), so there is no spelling by which a request could have written
/// one, and a root a request could move is a sweep pointed wherever that request liked. `None` — a
/// context with no configuration at all, or a door outside a context — gets the default rather than
/// a refusal, since this decides *where* and the grant still decides *whether*.
///
/// **Public because `rule:core-classes/temporary-dir-orphan-sweep`'s doors need the same answer this one does.** The `nvs serve` boot and
/// `nvs tmp clean` walk the root that [`temp_dir`] created under, and a second reading of `[io]
/// temp_root` in the CLI is how the walk and the writer come to disagree; [`crate::sweep::orphans`]
/// takes the path this hands back. It reads a tree rather than a [`Ctx`] for the same reason: those
/// two doors have a configuration and no context.
///
/// An empty string is treated as unset. It is what a `temp_root = ""` in a file means to every other
/// path key here, and joining a name onto it would otherwise create the directory in the process's
/// working directory, which is the one place a temporary must never land.
///
#[must_use]
pub fn temp_root(config: Option<&nvs_config::Config>) -> Option<PathBuf> {
    let written = config
        .and_then(|config| config.io.as_ref())
        .and_then(|io| io.temp_root.as_deref())
        .filter(|root| !root.is_empty());
    match written {
        Some(root) => Some(PathBuf::from(root)),
        None => nvs_config::data::current().map(nvs_config::data::Folder::tmp),
    }
}

/// § 2's process door: `program` started as a child with `argv`, once [`Cap::ProcessExec`] has been
/// shown to cover it and `rule:core-classes/process-refuses-a-shell-target`'s shell targets have been refused.
///
/// The started child rather than its output, for [`open_read`]'s reason: one door has to serve
/// `Core\Process::run`'s captured wait and `::spawn`'s streamed handle alike, and the capability
/// question belongs to the *child* — a process already started is a process already checked, so
/// nothing downstream of this call has to ask again.
///
/// `argv` is what the program receives after its own name, which the operating system supplies:
/// there is no command line anywhere in this function, and so nothing for a quoting rule to be
/// wrong about. `rule:core-classes/process-run` is why that is the only shape offered.
///
/// **All three standard streams are pipes, and that is this door's decision rather than the
/// caller's.** A child that inherited them would read the server's own stdin and write to the
/// server's own stdout — a request reaching a descriptor no capability named, and one that no
/// `Core\Process` member would have to ask for.
///
/// **The child inherits no other handle of this process's, and it ends when this process ends,
/// however it ends.** `crates/nvs-runtime/src/child_start.rs` owns how a child is started, and
/// says what each platform does. A child that will not be ended if this process is killed starts
/// anyway, and a `Warn` record in the log says so.
///
/// **A shell target is refused here, on every platform**, `rule:core-classes/process-refuses-a-shell-target`: a `.bat`, `.cmd` or `.ps1`
/// runs by handing a command line to `cmd.exe` or `powershell.exe`, which re-parses the arguments
/// this door never built, so an argv Novis passed correctly becomes a shell string again by the time
/// the target sees it. The check runs on Unix too, where the risk is not real — a `#!` line is read
/// by the same `execve` that already has the split argv — because a refusal that exists on one
/// platform only is a behaviour no test on the other can pin.
///
/// **A bare name such as `git` is looked up on `PATH`, and only once the grant allows some
/// program** (`rule:security/process-exec-capability`). A CLI run allows every program by default,
/// so its bare name is looked up straight away. Server work with no `process.exec` line is refused
/// before the lookup, so it never learns which programs exist. With a line, the program found is
/// asked about by its absolute path like any other, so the file the operating system starts is the
/// file the check approved. A bare-name entry in the grant was looked up on the same `PATH` when
/// the configuration loaded. Every other path that does not start at a root is refused by
/// [`require`].
///
/// The capability is asked **first**, before the target's kind, so the rule every other door here
/// states holds without an exception: the grant is consulted before anything else is looked at.
/// Both refusals are the same catchable class, since neither is a condition a program can recover
/// from by trying something adjacent.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `process.exec` for
/// `program`, a `RuntimeError` naming the extension for § 4's refused target kinds, an `IOError`
/// when a bare name is in no folder of `PATH`, or [`io_failure`]'s `IOError` when the spawn itself
/// fails — nothing is at `program`, or it is not executable.
///
pub fn exec(ctx: &mut Ctx, program: &Path, argv: &[&str], member: &str) -> Result<Child, Fault> {
    exec_with(ctx, program, argv, &Launch::default(), member)
}

/// The variables a child [`exec_with`] starts with no `env` takes from this process, and the only
/// ones: `rule:core-classes/process-options`' base set.
///
/// They are what an ordinary program needs to find other programs, its user's folder, a temporary
/// folder and its language. Everything else in this process's environment, such as a database
/// password or a cloud key, stays here. Windows also needs `SYSTEMROOT`, without which a child
/// cannot open a socket. A variable this process does not have is not set.
#[cfg(windows)]
const BASE_ENV: &[&str] = &["PATH", "USERPROFILE", "TEMP", "TMP", "SYSTEMROOT", "LANG"];
/// See the Windows list.
#[cfg(not(windows))]
const BASE_ENV: &[&str] = &["PATH", "HOME", "TEMP", "TMP", "LANG"];

/// Where a child [`exec_with`] starts runs, and what environment it sees —
/// `rule:core-classes/process-options`' two fields that reach the operating system. The default
/// is the parent's folder and [`BASE_ENV`] from the parent's environment.
#[derive(Debug, Default)]
pub struct Launch<'a> {
    /// The folder the child starts in. It must start at a root, for [`relative_refusal`]'s
    /// reason: a folder resolved against the server's own working directory names a different
    /// place depending on how the server was started.
    pub dir: Option<&'a Path>,
    /// The child's whole environment, name and value. `Some(&[])` starts a child with no
    /// variables at all, and `None` with [`BASE_ENV`] alone.
    pub env: Option<&'a [(String, String)]>,
}

/// [`exec`] with a [`Launch`]: the same capability question and the same refused targets, asked
/// first, and then the folder and the environment checked before anything is started.
///
/// A folder grants nothing and needs no capability of its own. The child is the program
/// [`Cap::ProcessExec`] approved, and it can change its own folder as soon as it runs. With a
/// folder given, the program path is made absolute against this process's folder first, so the
/// file the operating system starts is the file the capability was asked about on every
/// platform.
///
/// # Errors
///
/// [`exec`]'s, plus a catchable `RuntimeError` for a relative folder, and for a variable whose
/// name is empty or has a `=` or a NUL in it, or whose value has a NUL in it. The operating
/// system cannot pass any of those on as written.
pub fn exec_with(
    ctx: &mut Ctx,
    program: &Path,
    argv: &[&str],
    launch: &Launch<'_>,
    member: &str,
) -> Result<Child, Fault> {
    let found;
    let program = match nvs_config::capability::bare_program(program) {
        Some(name) => {
            if !granted(ctx, Cap::ProcessExec, Scope::Unscoped) {
                return Err(Fault::thrown(denial(
                    Cap::ProcessExec,
                    Scope::Path(program),
                    member,
                )));
            }
            found = nvs_config::capability::on_path(name).ok_or_else(|| {
                Fault::thrown_as(
                    ThrownClass::Io,
                    format!(
                        "{member} cannot run `{name}`: no folder of `PATH` has a program with \
                         this name\nhelp: write the full path of the program"
                    ),
                )
            })?;
            found.as_path()
        }
        None => program,
    };
    require(ctx, Cap::ProcessExec, Scope::Path(program), member)?;
    if let Some(extension) = shell_target(program) {
        return Err(Fault::thrown(format!(
            "{member} will not run {}: a `.{extension}` target is started by handing a command line \
             to a second parser, which re-quotes an argv this API passed across whole — start that \
             interpreter yourself if it is what you mean",
            program.display()
        )));
    }
    if let Some(dir) = launch.dir {
        relative_refusal(dir, member)?;
    }
    if let Some(env) = launch.env {
        for (name, value) in env {
            if name.is_empty() || name.contains(['=', '\0']) || value.contains('\0') {
                return Err(Fault::thrown(format!(
                    "{member} cannot pass the environment variable `{}`: a name must not be \
                     empty or contain `=` or a NUL, and a value must not contain a NUL",
                    name.escape_debug()
                )));
            }
        }
    }
    let mut command = match launch.dir {
        Some(dir) => {
            let absolute =
                std::path::absolute(program).map_err(|err| io_failure(member, program, &err))?;
            let mut command = Command::new(absolute);
            command.current_dir(dir);
            command
        }
        None => Command::new(program),
    };
    command.env_clear();
    match launch.env {
        Some(env) => {
            command.envs(env.iter().map(|(name, value)| (name, value)));
        }
        None => {
            command.envs(
                BASE_ENV
                    .iter()
                    .filter_map(|&name| std::env::var_os(name).map(|value| (name, value))),
            );
        }
    }
    command
        .args(argv)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let (child, warning) =
        crate::child_start::spawn(&mut command).map_err(|err| io_failure(member, program, &err))?;
    if let Some(warning) = warning {
        crate::floor::report(ctx, &crate::floor::note(nvs_render::Level::Warn, &warning));
    }
    Ok(child)
}

/// The lower-cased extension of a target [`exec`] refuses, or `None` for one it will start.
///
/// By extension and not by content, `rule:core-classes/process-refuses-a-shell-target`: what makes a `.bat` unsafe to start is which
/// program the operating system hands the command line to, and that is decided by the name alone —
/// so this answers the same way for a file that is not there, which is what lets the refusal be
/// about the kind of target rather than about the filesystem.
///
/// The name is read the way Windows reads it before it opens the file: everything from the first
/// `:` is an alternate stream and goes, and then every trailing dot and space goes. `say.bat.`,
/// `say.bat ` and `say.bat::$DATA` all start `say.bat`, so all three are refused, on every platform
/// for the rule's reason.
fn shell_target(program: &Path) -> Option<String> {
    let name = program.file_name()?.to_str()?;
    let name = name.split(':').next().unwrap_or(name);
    let name = name.trim_end_matches(['.', ' ']);
    let extension = Path::new(name).extension()?.to_str()?.to_ascii_lowercase();
    matches!(extension.as_str(), "bat" | "cmd" | "ps1").then_some(extension)
}

/// What a door reports when the operating system refuses something the capability allowed: an
/// `IOError`, catchable, naming the member, the path and what the OS said.
///
/// Public because [`open_read`] hands back a handle rather than a result, so the caller that reads
/// from it owes the same message for the same kind of failure; one function is how the two agree
/// rather than drifting into two spellings of "could not read".
///
/// A file that is not there adds a `help:` line saying which folder a relative path written in the
/// code starts at, because such a path joined to the folder of its own file is the usual reason a
/// path names a place the author did not expect. Every other kind of failure is the one line.
#[must_use]
pub fn io_failure(member: &str, path: &Path, err: &std::io::Error) -> Fault {
    let help = if err.kind() == std::io::ErrorKind::NotFound {
        "\nhelp: a relative path written in the source starts at the folder of the file that \
         contains it"
    } else {
        ""
    };
    Fault::thrown_as(
        ThrownClass::Io,
        format!("{member} failed on {}: {err}{help}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use std::net::{IpAddr, Ipv4Addr};

    use super::{
        Access, Cap, Ctx, Fault, PINNED_ADDRESSES, Path, Scope, ThrownClass, copy, create, exec,
        granted, install_resolver, open, open_read, pin_host, pin_host_addresses, rename, require,
        shell_target, temp_dir, working_dir, write,
    };

    /// The member a case refuses on behalf of. `run` and not `spawn` for no reason beyond being the
    /// one the fixture calls; the door does not know which it is serving.
    const MEMBER: &str = "Core\\Process::run";

    /// A snapshot built from the text an operator would have written, rather than from the typed
    /// tree — the boot path deserializes, so a case that constructed the struct directly would pin
    /// a grant no configuration file can express.
    fn snapshot_of(written: &str) -> Arc<nvs_config::Snapshot> {
        let table: toml::Table = written.parse().expect("the case writes valid TOML");
        Arc::new(nvs_config::Snapshot {
            config: table
                .clone()
                .try_into()
                .expect("the case writes a block this tree has"),
            table,
            ..nvs_config::Snapshot::default()
        })
    }

    thread_local! {
        /// What [`scripted`] answers next. Thread-locals rather than shared state, for the same
        /// reason the resolver seam itself is one: a case owns its thread, so two of them scripting
        /// different answers cannot reach each other's.
        static ANSWER: std::cell::RefCell<Vec<IpAddr>> =
            const { std::cell::RefCell::new(Vec::new()) };
        /// How many times a door has asked [`scripted`] for it.
        static LOOKUPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    /// A resolver answering what [`scripted_to_answer`] wrote, and counting the asking — which is
    /// what lets a case assert that a lookup did *not* happen.
    fn scripted(_host: &str) -> std::io::Result<Vec<IpAddr>> {
        LOOKUPS.with(|count| count.set(count.get() + 1));
        Ok(ANSWER.with_borrow(Clone::clone))
    }

    /// Installs [`scripted`] on this thread with `addresses` as its answer and nothing asked yet.
    fn scripted_to_answer(addresses: &[IpAddr]) {
        install_resolver(scripted);
        ANSWER.with_borrow_mut(|slot| *slot = addresses.to_vec());
        LOOKUPS.with(|count| count.set(0));
    }

    /// How many lookups [`scripted`] has been asked for since it was installed.
    fn lookups() -> usize {
        LOOKUPS.with(std::cell::Cell::get)
    }

    /// The `n`th address of TEST-NET-3, which § 3's table does not deny and which no machine a case
    /// can reach answers on.
    fn public(n: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(203, 0, 113, n))
    }

    /// A context granting `net.connect` for `host` and excepting no address, written as an operator
    /// would write it per [`snapshot_of`].
    fn reaching(host: &str) -> Ctx {
        let mut ctx = Ctx::buffered();
        ctx.set_config(snapshot_of(&format!(
            "[capabilities.net]\nconnect = [\"{host}\"]\ninternal = []\n"
        )));
        ctx
    }

    /// `rule:http-server/a-core-is-never-blocked-on-a-syscall`, asserted as an **order**: the grant
    /// is asked on the core before the lookup leaves the thread, so a program outside `net.connect`
    /// cannot use this door as a resolver for a name it may not reach.
    ///
    /// The resolver seam is what makes that observable. A case cannot see which thread ran a lookup,
    /// but it can see whether one happened at all, and a refused host that asked for none is the
    /// whole claim. The IP address at the end is the other half: a set of one reaches no resolver, so
    /// nothing is handed off for an address that was already written down.
    #[test]
    fn the_grant_is_asked_before_the_lookup_leaves_the_core() {
        const HOST: &str = "names.test";
        const OUTBOUND: &str = "Core\\Http::allowUrl";
        scripted_to_answer(&[public(7)]);

        let outside = Ctx::buffered();
        let refused = pin_host(&outside, HOST, OUTBOUND)
            .expect_err("a context with no configuration grants nothing");
        assert!(
            format!("{refused:?}").contains("net.connect"),
            "the host is refused as a capability: {refused:?}"
        );
        assert_eq!(
            lookups(),
            0,
            "and refused before the lookup, or this door is a resolver for an ungranted program"
        );

        assert_eq!(
            pin_host(&reaching(HOST), HOST, OUTBOUND)
                .expect("a public address § 3's table does not deny"),
            public(7)
        );
        assert_eq!(
            lookups(),
            1,
            "the lookup ran once, through the resolver this thread installed"
        );

        assert_eq!(
            pin_host(&reaching("203.0.113.7"), "203.0.113.7", OUTBOUND)
                .expect("an IP address is its own approval"),
            public(7)
        );
        assert_eq!(lookups(), 1, "and an IP address asked no resolver at all");
    }

    /// `rule:http-server/an-outbound-call-tries-every-approved-address`: a name answering one
    /// address § 3 denies refuses the **whole host**, naming that address, whichever position it
    /// arrived in — a door that dropped it and used the rest would approve exactly the answer a
    /// rebinding attack produces.
    #[test]
    fn a_host_resolving_to_one_denied_address_is_refused_whole() {
        const HOST: &str = "split.test";
        const OUTBOUND: &str = "Core\\Http::allowUrl";
        let loopback = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let private = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 4));

        for (answer, named) in [
            (vec![public(1), loopback], "127.0.0.1"),
            (vec![loopback, public(1)], "127.0.0.1"),
            (vec![public(1), public(2), private], "10.0.0.4"),
        ] {
            scripted_to_answer(&answer);
            let refused = pin_host_addresses(&reaching(HOST), HOST, OUTBOUND)
                .expect_err("one denied address refuses the host");
            let sentence = format!("{refused:?}");
            assert!(
                sentence.contains(named),
                "the refusal names the address that failed, not the host: {sentence}"
            );
            assert!(
                sentence.contains("net.internal"),
                "and the key an operator would have to write: {sentence}"
            );
        }

        scripted_to_answer(&[public(1), public(2)]);
        assert_eq!(
            pin_host_addresses(&reaching(HOST), HOST, OUTBOUND)
                .expect("nothing but public addresses"),
            vec![public(1), public(2)],
            "a set the table approves is approved whole"
        );
    }

    /// `rule:http-server/an-outbound-call-tries-every-approved-address`'s set: the resolver's order
    /// kept, no address twice, and at most `PINNED_ADDRESSES` of it — a cap because falling back
    /// across the set is a connect budget, and an uncapped one is whatever a resolver answered.
    #[test]
    fn every_approved_address_is_kept_in_the_resolvers_order_up_to_eight() {
        const HOST: &str = "many.test";
        const OUTBOUND: &str = "Core\\Http::allowUrl";

        let answered: Vec<IpAddr> = (1_u8..=12).map(public).collect();
        scripted_to_answer(&answered);
        assert_eq!(
            pin_host_addresses(&reaching(HOST), HOST, OUTBOUND).expect("every address is public"),
            answered[..PINNED_ADDRESSES].to_vec(),
            "the head of the resolver's answer, in its order"
        );
        assert_eq!(
            pin_host(&reaching(HOST), HOST, OUTBOUND).expect("every address is public"),
            public(1),
            "and the single-address door is the head of that set"
        );

        scripted_to_answer(&[public(1), public(2), public(1)]);
        assert_eq!(
            pin_host_addresses(&reaching(HOST), HOST, OUTBOUND).expect("every address is public"),
            vec![public(1), public(2)],
            "the same machine under two records is one address and one connect attempt"
        );
    }

    /// `rule:security/denial-is-a-runtime-error`, asked of the process door: an unconfigured context starts nothing, and the
    /// message names the capability in the spelling `nvs.toml` grants it under.
    #[test]
    fn a_child_starts_only_where_process_exec_is_granted() {
        let mut ctx = Ctx::buffered();
        let denied = exec(
            &mut ctx,
            Path::new("/usr/bin/convert"),
            &["-version"],
            MEMBER,
        )
        .expect_err("a context with no configuration grants nothing");
        let Fault::Thrown(class, message) = denied else {
            panic!(
                "a denial is a throw and never a fatal — `rule:security/denial-is-a-runtime-error`"
            );
        };
        assert_eq!(class, ThrownClass::Runtime);
        assert!(
            message.contains("process.exec")
                && message.contains(MEMBER)
                && message.contains("convert"),
            "the denial names the capability, who wanted it and what for: {message}"
        );
    }

    /// The refusal's second line, which is the only part of it an operator can act on: the line to
    /// add, the table it goes under, and the file. Four capabilities by hand for the exact
    /// sentence, then every one of them, so that a capability added to a family with no
    /// `[capabilities.<family>]` block of its own fails here rather than pointing a reader at a
    /// table that does not exist.
    #[test]
    fn an_ungranted_call_names_the_config_file_and_the_capability_table() {
        let ctx = Ctx::buffered();
        for (cap, line, table) in [
            (Cap::FsRead, "read = true", "[capabilities.fs]"),
            (Cap::ScriptSpawn, "spawn = true", "[capabilities.script]"),
            (Cap::DbOpen, "open = true", "[capabilities.db]"),
            (Cap::CacheShared, "shared = true", "[capabilities.cache]"),
        ] {
            let denied = require(&ctx, cap, Scope::Unscoped, MEMBER)
                .expect_err("a context with no configuration grants nothing");
            let Fault::Thrown(class, message) = denied else {
                panic!(
                    "a denial is a throw and never a fatal — `rule:security/denial-is-a-runtime-error`"
                );
            };
            assert_eq!(class, ThrownClass::Runtime);
            let want = format!("help: add `{line}` under `{table}` in nvs.toml");
            assert_eq!(
                message.lines().nth(1),
                Some(want.as_str()),
                "the line under the refusal says where the grant goes: {message}"
            );
        }

        for cap in Cap::ALL.iter().copied() {
            let denied = require(&ctx, cap, Scope::Unscoped, MEMBER)
                .expect_err("a context with no configuration grants nothing");
            let Fault::Thrown(_, message) = denied else {
                panic!("every door's denial is the same throw")
            };
            let help = message
                .lines()
                .nth(1)
                .expect("every denial carries the help line, whatever the capability");
            let named = help
                .split_once("` under `[capabilities.")
                .and_then(|(_, rest)| rest.strip_suffix("]` in nvs.toml"))
                .unwrap_or_else(|| panic!("one wording for every capability: {help}"));
            assert_eq!(
                named,
                cap.family(),
                "the table is the capability's own family half: {}",
                cap.name()
            );
        }
    }

    /// A file that is not there is an `IOError` whose second line says where a relative path
    /// written in the code starts. Any other failure is one line.
    #[test]
    fn a_missing_file_names_the_folder_a_written_relative_path_starts_at() {
        let path = Path::new("/srv/app/data/note.txt");
        let missing = std::io::Error::from(std::io::ErrorKind::NotFound);
        let Fault::Thrown(class, message) = super::io_failure("Core\\IO::read", path, &missing)
        else {
            panic!("the failure is a catchable throw");
        };
        assert_eq!(class, ThrownClass::Io);
        assert!(
            message.starts_with("Core\\IO::read failed on "),
            "{message}"
        );
        assert_eq!(
            message.lines().nth(1),
            Some(
                "help: a relative path written in the source starts at the folder of the file \
                 that contains it"
            ),
            "{message}"
        );
        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let Fault::Thrown(_, message) = super::io_failure("Core\\IO::read", path, &denied) else {
            panic!("the failure is a catchable throw");
        };
        assert_eq!(message.lines().count(), 1, "{message}");
    }

    /// `rule:programs/relative-paths-resolve-from-their-file`'s run-time half: a relative path throws
    /// at every path door before a grant is read, and the message names the path and both ways to
    /// make it absolute. A path that starts at a root reaches the grant check instead.
    // covers: lang:programs/file-paths-a-relative-path-starts-at-the-folder-of-its-file
    #[test]
    fn a_relative_path_throws_before_any_grant_is_asked() {
        let ctx = Ctx::buffered();
        for cap in [Cap::FsRead, Cap::FsWrite, Cap::ProcessExec, Cap::NetLocal] {
            let refused = require(&ctx, cap, Scope::Path(Path::new("data/note.txt")), MEMBER)
                .expect_err("a relative path is never resolved against the working directory");
            let Fault::Thrown(class, message) = refused else {
                panic!("the refusal is a catchable throw");
            };
            assert_eq!(class, ThrownClass::Runtime);
            assert!(message.contains("`data/note.txt` is relative"), "{message}");
            assert!(message.contains("Core\\Path::join"), "{message}");
            assert!(message.contains("Core\\Path::fromCwd"), "{message}");
            assert!(
                !message.contains(cap.name()),
                "the grant was not asked about: {message}"
            );
        }
        let denied = require(
            &ctx,
            Cap::FsRead,
            Scope::Path(Path::new("/data/note.txt")),
            MEMBER,
        )
        .expect_err("a context with no configuration grants nothing");
        let Fault::Thrown(_, message) = denied else {
            panic!("the denial is a catchable throw");
        };
        assert!(message.contains("fs.read"), "{message}");
    }

    /// A context answering no request reads the working directory, and the answer is an absolute
    /// path to a directory that exists.
    // covers: Core\Path::fromCwd
    #[test]
    fn the_working_directory_is_read_outside_a_request() {
        let ctx = Ctx::buffered();
        let found = working_dir(&ctx, "Core\\Path::fromCwd").expect("no request is being answered");
        assert!(found.is_absolute(), "{}", found.display());
        assert!(found.is_dir(), "{}", found.display());
    }

    /// The line above it does not move. Every scope's wording, character for character, because the
    /// conformance corpus compares this sentence against a literal a program built — so a refusal
    /// that gained a line is what this change is, and a refusal that gained a word is a break.
    #[test]
    fn the_denials_own_subject_and_wording_are_unchanged() {
        let ctx = Ctx::buffered();
        let endpoint: std::net::SocketAddr = "127.0.0.1:8080"
            .parse()
            .expect("an endpoint written as an address, not a name to resolve");
        for (cap, scope, want) in [
            (
                Cap::ProcessExec,
                Scope::Unscoped,
                "Core\\Process::run needs the capability `process.exec`, which is not granted",
            ),
            (
                Cap::FsRead,
                Scope::Path(Path::new("/data/note.txt")),
                "Core\\Process::run needs the capability `fs.read` for /data/note.txt, which is not granted",
            ),
            (
                Cap::NetConnect,
                Scope::Host("example.com"),
                "Core\\Process::run needs the capability `net.connect` for example.com, which is not granted",
            ),
            (
                Cap::DbConnect,
                Scope::Name("main"),
                "Core\\Process::run needs the capability `db.connect` for main, which is not granted",
            ),
            (
                Cap::NetListen,
                Scope::Endpoint(endpoint),
                "Core\\Process::run needs the capability `net.listen` for 127.0.0.1:8080, which is not granted",
            ),
        ] {
            let denied = require(&ctx, cap, scope, MEMBER)
                .expect_err("a context with no configuration grants nothing");
            let Fault::Thrown(_, message) = denied else {
                panic!("every door's denial is the same throw")
            };
            assert_eq!(
                message.lines().next(),
                Some(want),
                "the subject is untouched: {message}"
            );
            assert_eq!(
                message.lines().count(),
                2,
                "a refusal is its sentence and one line under it: {message}"
            );
        }
    }

    /// `rule:core-classes/process-refuses-a-shell-target`, on a context that grants everything: the refusal is about the kind of target,
    /// so a grant cannot buy it and no platform is exempt from it.
    #[test]
    fn a_shell_target_is_refused_however_wide_the_grant_is() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(snapshot_of("[capabilities.process]\nexec = true\n"));
        // The positive control: without it every refusal below could be the capability denial in
        // disguise, which is the same class and would satisfy a weaker assertion.
        require(
            &ctx,
            Cap::ProcessExec,
            Scope::Path(Path::new("/srv/examples/process/say.bat")),
            MEMBER,
        )
        .expect("`exec = true` covers every program, this one included");

        for (target, named) in [
            ("/srv/examples/process/say.bat", "bat"),
            ("/deploy/RELEASE.CMD", "cmd"),
            ("/srv/build.ps1", "ps1"),
        ] {
            let refused = exec(&mut ctx, Path::new(target), &[], MEMBER)
                .expect_err("a second command-line parser is not a target this API has");
            let Fault::Thrown(class, message) = refused else {
                panic!("§ 4's refusal is catchable, like every other one this module writes");
            };
            assert_eq!(class, ThrownClass::Runtime);
            assert!(
                message.contains(&format!(".{named}")) && message.contains(MEMBER),
                "the refusal names the extension and the member: {message}"
            );
            assert!(
                !message.contains("process.exec"),
                "and is not the capability denial, which this context does not produce: {message}"
            );
        }
    }

    /// The same rule on the spellings Windows cuts before it opens a file: a trailing dot, a
    /// trailing space and an alternate stream all start the batch file, so each one is refused.
    #[test]
    fn a_shell_target_is_refused_however_its_name_is_spelled() {
        for (target, named) in [
            ("/srv/say.bat.", "bat"),
            ("/srv/say.bat...", "bat"),
            ("/srv/say.bat ", "bat"),
            ("/srv/say.CMD . .", "cmd"),
            ("/srv/say.bat::$DATA", "bat"),
            ("/srv/say.ps1:stream", "ps1"),
            ("/srv/say.bat. :x", "bat"),
        ] {
            assert_eq!(
                shell_target(Path::new(target)).as_deref(),
                Some(named),
                "`{target}` starts a `.{named}` file on Windows"
            );
        }
    }

    /// `rule:errors/path-component-refusals`' device names at the file doors: refused under a grant
    /// that covers everything, with an extension, a trailing dot or a trailing space, on every
    /// platform, and before the filesystem is asked anything.
    #[cfg_attr(
        miri,
        ignore = "opens a file with `openat`, which Miri does not emulate"
    )]
    #[test]
    fn a_device_name_is_refused_by_every_file_door_however_wide_the_grant_is() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(snapshot_of(
            "[capabilities.fs]\nread = true\nwrite = true\n",
        ));
        for target in [
            "/srv/data/CON",
            "/srv/data/nul.txt",
            "/srv/data/COM1.",
            "/srv/data/com1.log.",
            "/srv/data/CONIN$",
            "/srv/data/conout$.txt",
            "/srv/data/CON .log",
            "/srv/NUL/inside.txt",
            "/srv/data/LPT²",
        ] {
            let path = Path::new(target);
            let refusals = [
                open_read(&ctx, path, MEMBER).map(drop),
                write(&ctx, path, b"x", MEMBER),
                create(&ctx, path, true, MEMBER).map(drop),
                rename(&ctx, Path::new("/srv/data/a.txt"), path, MEMBER),
            ];
            for refused in refusals {
                let Err(Fault::Thrown(class, message)) = refused else {
                    panic!("`{target}` names a device, so the door refuses it");
                };
                assert_eq!(class, ThrownClass::Runtime, "for `{target}`");
                assert!(
                    message.contains("Windows device name") && message.contains(MEMBER),
                    "the refusal names the reason and the member: {message}"
                );
            }
        }
        // A name that only contains a device name is a name: the door reaches the filesystem,
        // which has no such file.
        let Err(Fault::Thrown(class, message)) =
            open_read(&ctx, Path::new("/srv/data/console.txt"), MEMBER)
        else {
            panic!("there is no `/srv/data/console.txt` to open");
        };
        assert_eq!(class, ThrownClass::Io, "{message}");
    }

    /// A path that ends with a separator names a folder, so no file door opens or creates a file
    /// by the name in front of it. The resolved path has lost the separator, and only the path as
    /// written still shows it.
    #[test]
    fn a_path_ending_with_a_separator_is_refused_by_every_file_door() {
        let root = scratch("trailing-separator");
        let mut ctx = Ctx::buffered();
        ctx.set_config(snapshot_of(
            "[capabilities.fs]\nread = true\nwrite = true\n",
        ));
        let source = root.join("source.txt");
        std::fs::write(&source, b"x").expect("the scratch folder is writable");
        let folder = format!("{}/", root.join("folder").display());
        let path = Path::new(&folder);
        let refusals = [
            open_read(&ctx, path, MEMBER).map(drop),
            open(&ctx, path, Access::Write, MEMBER).map(drop),
            write(&ctx, path, b"x", MEMBER),
            create(&ctx, path, true, MEMBER).map(drop),
            copy(&ctx, &source, path, MEMBER),
        ];
        for refused in refusals {
            let Err(Fault::Thrown(class, message)) = refused else {
                panic!("`{folder}` ends with a separator, so the door refuses it");
            };
            assert_eq!(class, ThrownClass::Io, "{message}");
            assert!(message.contains("ends with a separator"), "{message}");
        }
        assert!(!root.join("folder").exists(), "no file was created");
    }

    /// The other half of the same rule, which the cases above cannot show: every other target kind
    /// reaches the spawn, including the shebang script Unix runs through `execve` itself.
    #[test]
    fn nothing_but_those_three_extensions_is_a_shell_target() {
        for allowed in [
            "/usr/bin/convert",
            "bin/nvs.exe",
            "tools/deploy.sh",
            "batch",
            "archive.bat.gz",
            "say.bat.gz.",
            "notes:say.bat",
        ] {
            assert!(
                shell_target(Path::new(allowed)).is_none(),
                "`{allowed}` is started by the operating system, not by a command-line parser"
            );
        }
    }

    /// `rule:security/process-exec-capability`: a CLI run starts `git` by name, found on `PATH`.
    /// Server work with no line refuses the name before it looks it up, and starts it once the
    /// line names it. A `.cmd` name stays refused however wide the grant is, and a relative path
    /// with a folder in it is still refused on both kinds of run.
    #[cfg_attr(miri, ignore = "starts a process, which Miri cannot do")]
    #[test]
    fn a_bare_name_is_found_on_path_on_a_cli_run_and_needs_a_line_in_server_work() {
        let mut cli = Ctx::buffered();
        cli.set_config(snapshot_of(""));
        let mut child = exec(&mut cli, Path::new("git"), &["--version"], MEMBER)
            .expect("a CLI run starts any program, and `git` is on PATH");
        let mut out = String::new();
        std::io::Read::read_to_string(child.stdout.as_mut().expect("piped"), &mut out)
            .expect("the child writes its version");
        assert!(child.wait().expect("the child ends").success());
        assert!(out.starts_with("git version"), "{out}");

        let mut server = Ctx::buffered();
        server.set_config(snapshot_of(""));
        server.set_run_kind(crate::ctx::RunKind::Job);
        let Fault::Thrown(class, message) = exec(&mut server, Path::new("git"), &[], MEMBER)
            .expect_err("server work runs no program without a line")
        else {
            panic!("a denial is a throw");
        };
        assert_eq!(class, ThrownClass::Runtime);
        assert!(message.contains("exec = [\"git\"]"), "{message}");

        let mut granted_server = Ctx::buffered();
        let mut table: toml::Table = "[capabilities.process]\nexec = [\"git\"]\n"
            .parse()
            .expect("valid TOML");
        let mut config: nvs_config::tree::Config =
            table.clone().try_into().expect("a block this tree has");
        nvs_config::capability::anchor(&mut config, &mut table, &Default::default());
        let mut snapshot = nvs_config::Snapshot {
            config,
            table,
            ..nvs_config::Snapshot::default()
        };
        if let Some(caps) = snapshot.config.capabilities.as_mut() {
            caps.canonicalize(&nvs_config::resolve::Disk);
        }
        granted_server.set_config(Arc::new(snapshot));
        granted_server.set_run_kind(crate::ctx::RunKind::Job);
        let mut child = exec(
            &mut granted_server,
            Path::new("git"),
            &["--version"],
            MEMBER,
        )
        .expect("`exec = [\"git\"]` was looked up when the configuration loaded");
        assert!(child.wait().expect("the child ends").success());

        let Err(Fault::Thrown(_, message)) = exec(&mut cli, Path::new("/srv/say.cmd"), &[], MEMBER)
        else {
            panic!("a `.cmd` target is refused on a CLI run");
        };
        assert!(message.contains(".cmd"), "{message}");
        let Err(Fault::Thrown(_, message)) = exec(&mut cli, Path::new("bin/git"), &[], MEMBER)
        else {
            panic!("a relative path with a folder is refused on a CLI run");
        };
        assert!(message.contains("relative"), "{message}");
    }

    /// `rule:concurrency/a-child-belongs-to-the-calling-task`'s children "share the request", and this is the half every capability-gated
    /// member depends on: [`Ctx::child`] carries the request's configuration, so a grant the
    /// request holds is a grant inside a task of it. Without that field crossing, [`granted`] would
    /// answer `false` to *everything* inside a child — `Core\Db::connect` succeeding in a program's
    /// main body and refused verbatim inside a `Core\Task::all` child.
    #[test]
    fn a_task_child_is_granted_what_its_request_was_granted() {
        let mut request = Ctx::buffered();
        request.set_config(snapshot_of("[capabilities.process]\nexec = true\n"));
        // The negative control, on a context nobody configured: without it the assertion below
        // would pass on a `granted` that had simply stopped reading the configuration at all.
        let bare = Ctx::buffered();
        // SAFETY: each child is dropped at the end of this scope, before the context it borrows
        // static-property storage from, and nothing runs on it after that.
        #[expect(unsafe_code, reason = "the child is dropped before its parent")]
        let bare_child = unsafe { bare.child() };
        assert!(
            bare_child.config().is_none()
                && !granted(
                    &bare_child,
                    Cap::ProcessExec,
                    Scope::Path(Path::new("/bin/ls"))
                ),
            "a child of an unconfigured request is as unconfigured as its request"
        );

        // SAFETY: as above — `child` is dropped before `request`.
        #[expect(unsafe_code, reason = "the child is dropped before its parent")]
        let child = unsafe { request.child() };
        assert!(
            child.config().is_some(),
            "the request's configuration is request-wide, so a task of it holds the same view"
        );
        assert!(
            granted(&child, Cap::ProcessExec, Scope::Path(Path::new("/bin/ls"))),
            "`exec = true` is in force for the request, and a child is inside that request"
        );
        require(
            &child,
            Cap::ProcessExec,
            Scope::Path(Path::new("/bin/ls")),
            MEMBER,
        )
        .expect("the door agrees with the reporter, which is what makes `granted` honest");
    }

    /// A directory this case alone is using, for it to point `[io] temp_root` at. It is under
    /// `target/` and never under the default root, so that a case asserting where a temporary
    /// landed cannot pass by accident, and dropping it deletes it.
    fn scratch(name: &str) -> nvs_repo::Scratch {
        nvs_repo::scratch(&format!("capability-{name}"))
    }

    /// A context granting every write and naming `root` as the owned root. The grant is written as
    /// an operator would write it, per [`snapshot_of`]; the root goes in as a TOML *literal* string,
    /// because a Windows path in a basic string is a sequence of escapes.
    fn rooted_at(root: &Path) -> Ctx {
        let mut ctx = Ctx::buffered();
        ctx.set_config(snapshot_of(&format!(
            "[capabilities.fs]\nwrite = true\n\n[io]\ntemp_root = '{}'\n",
            root.display()
        )));
        ctx
    }

    /// `rule:core-classes/temporary-dir-sweep`: the root is the configured one, and the platform default is not consulted when
    /// a tree named one. The negative half matters as much as the positive — a member that ignored
    /// `[io] temp_root` would still hand back a directory that exists and is private, and only
    /// *where* it is says whether the sweeps can ever reach it.
    #[test]
    fn temporary_dir_creates_under_the_configured_temp_root_not_the_platform_default() {
        let root = scratch("configured");
        let mut ctx = rooted_at(&root);

        let made = temp_dir(&mut ctx, "Core\\IO::temporaryDir").expect("`write = true` covers it");

        assert!(
            made.starts_with(&root),
            "`rule:core-classes/temporary-dir-sweep` creates under `[io] temp_root`: {} is not under {}",
            made.display(),
            root.display()
        );
        assert!(
            super::temp_root(None).is_none_or(|default| !made.starts_with(default)),
            "and the default root is not consulted at all when one is configured"
        );
        assert!(
            made.is_dir()
                && made
                    .read_dir()
                    .is_ok_and(|mut entries| entries.next().is_none()),
            "the answer is a directory that exists and holds nothing"
        );
    }

    /// `rule:core-classes/temporary-dir-sweep`'s "created private on first use": the root need not exist, and creating it is the
    /// runtime's own doing rather than something an operator has to prepare. The Unix half asserts
    /// the mode, which is the whole reason the root is created by the private create — a root left
    /// at the process umask would make every entry under it enumerable by anyone on the machine,
    /// even though each entry is itself `0o700`.
    #[cfg_attr(miri, ignore = "Miri's `mkdir` does not apply the mode it is given")]
    #[test]
    fn the_owned_root_is_created_private_on_first_use() {
        // A component below the scratch directory as well, so this also pins that a `temp_root`
        // pointing somewhere not yet on disk is made rather than refused.
        let dir = scratch("first-use");
        let root = dir.join("owned");
        assert!(!root.exists(), "the case starts with nothing on disk");
        let mut ctx = rooted_at(&root);

        let made = temp_dir(&mut ctx, "Core\\IO::temporaryDir").expect("`write = true` covers it");

        assert!(root.is_dir(), "the first use created the root");
        assert!(made.starts_with(&root), "and put the directory inside it");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = |path: &Path| {
                std::fs::metadata(path)
                    .expect("both exist, having just been created")
                    .permissions()
                    .mode()
                    & 0o777
            };
            assert_eq!(mode(&root), 0o700, "the root is owner-only from creation");
            assert_eq!(mode(&made), 0o700, "and so is the directory under it");
        }
    }

    /// `rule:core-classes/temporary-dir-sweep`'s per-script list, from the only side that writes it: what the member hands back
    /// is what the context holds, in order, and a call that created nothing leaves nothing behind.
    ///
    /// The refused half is the one worth the case. A list written before the capability check — or
    /// before the directory existed — would still make every sweep test pass, and would have the
    /// runtime try to delete a path no program was ever given; asserting the count *after* a refusal
    /// is what separates "records what it created" from "records what it was asked for".
    #[test]
    fn every_directory_the_member_hands_back_is_recorded_and_a_refused_call_records_nothing() {
        let root = scratch("tracked");
        let mut ctx = rooted_at(&root);

        let first = temp_dir(&mut ctx, "Core\\IO::temporaryDir").expect("`write = true` covers it");
        let second = temp_dir(&mut ctx, "Core\\IO::temporaryDir").expect("and covers the second");

        assert_eq!(
            ctx.temporary_dirs(),
            [first, second],
            "the list is what the member answered, in the order it answered it"
        );

        // The same root, and a tree that writes `fs.write` as `[]`: `rule:core-classes/temporary-dir-sweep` chooses the name first and asks about
        // it second, so this reaches the check and stops there.
        let mut refused = Ctx::buffered();
        refused.set_config(snapshot_of(&format!(
            "[capabilities.fs]\nwrite = []\n\n[io]\ntemp_root = '{}'\n",
            root.display()
        )));
        temp_dir(&mut refused, "Core\\IO::temporaryDir")
            .expect_err("`write = []` is a written key that grants nothing");
        assert!(
            refused.temporary_dirs().is_empty(),
            "a call that created no directory has nothing for the sweep to delete"
        );

        assert_eq!(
            ctx.take_temporary_dirs().len(),
            2,
            "the sweep drains the list once"
        );
        assert!(
            ctx.temporary_dirs().is_empty(),
            "and a second sweep of the same context has nothing left to do"
        );
    }

    /// `rule:core-classes/temporary-dir-sweep`, end to end and from the outside: what the member handed
    /// out is gone once the script that asked for it is over, and the program
    /// did nothing to make that happen.
    ///
    /// The two halves the sweep is written around are both here — a directory
    /// the program filled and a file it left open inside one. The second is the
    /// Windows case `Ctx::drop` closes the descriptors for: a held handle is a
    /// refused deletion on that platform, so a sweep that ran before the files
    /// went would log about a program that had merely forgotten to close.
    #[test]
    fn a_temporary_dir_still_standing_at_script_end_is_removed() {
        let root = scratch("script-end");
        let (filled, held) = {
            let mut ctx = rooted_at(&root);
            let filled = temp_dir(&mut ctx, "Core\\IO::temporaryDir").expect("`write = true`");
            std::fs::write(filled.join("note.txt"), b"what the program wrote")
                .expect("the directory was just created");

            let held = temp_dir(&mut ctx, "Core\\IO::temporaryDir").expect("and the second");
            ctx.hold_open_file(
                std::fs::File::create(held.join("open.txt")).expect("the directory exists"),
            );

            assert!(
                filled.is_dir() && held.is_dir(),
                "both stand while the script is running — the sweep is the ending, not the call"
            );
            (filled, held)
        };

        assert!(!filled.exists(), "the directory the program filled is gone");
        assert!(
            !held.exists(),
            "and so is the one holding a file the program never closed"
        );
        assert!(
            root.is_dir(),
            "the owned root itself stays: `rule:core-classes/temporary-dir-orphan-sweep`'s sweeps are what empty it, and the next script \
             creates under it rather than remaking it"
        );
    }

    /// Server work with nothing written under `[capabilities.fs]`, rooted at `root`.
    fn serving_at(root: &Path) -> Ctx {
        let mut ctx = Ctx::buffered();
        ctx.set_config(snapshot_of(&format!(
            "[io]\ntemp_root = '{}'\n",
            root.display()
        )));
        ctx.set_run_kind(crate::ctx::RunKind::Job);
        ctx
    }

    /// `rule:core-classes/temporary-dir-sweep`'s directories are the only place server work
    /// writes with nothing written, and only the context that made one writes or reads it. Two
    /// requests share the root, and neither reaches the other's directory or anything outside its
    /// own. A CLI run on the same tree writes outside the root.
    #[cfg_attr(
        miri,
        ignore = "writes a file with `openat`, which Miri does not emulate"
    )]
    #[test]
    fn server_work_writes_its_own_temporary_dirs_and_not_another_requests() {
        let root = scratch("server-default");
        let mut mine = serving_at(&root);
        let mut theirs = serving_at(&root);
        let made = temp_dir(&mut mine, MEMBER).expect("server work creates its own directory");
        let other = temp_dir(&mut theirs, MEMBER).expect("and so does the other request");

        write(&mine, &made.join("note.txt"), b"mine", MEMBER).expect("its own directory");
        open_read(&mine, &made.join("note.txt"), MEMBER).expect("and reads it back");
        write(&theirs, &other.join("note.txt"), b"theirs", MEMBER).expect("its own directory");

        let refused = write(&mine, &other.join("note.txt"), b"mine", MEMBER)
            .expect_err("another request's directory");
        let Fault::Thrown(_, message) = refused else {
            panic!("a denial is thrown: {refused:?}");
        };
        assert!(message.contains("`fs.write`"), "{message}");
        open_read(&mine, &other.join("note.txt"), MEMBER)
            .expect_err("another request's directory is not read either");
        write(&mine, &root.join("beside.txt"), b"x", MEMBER)
            .expect_err("the root itself is not this request's");

        let mut cli = Ctx::buffered();
        cli.set_config(snapshot_of(&format!(
            "[io]\ntemp_root = '{}'\n",
            root.display()
        )));
        write(&cli, &root.join("beside.txt"), b"x", MEMBER)
            .expect("a CLI run writes outside its temporary directories");
        temp_dir(&mut cli, MEMBER).expect("and creates one");
    }

    /// `rule:security/script-spawn-capability`: with nothing written, a script in the app folder
    /// spawns, in server work and on a CLI run. A script outside it, below a dot-folder, or in a
    /// temporary directory the run made does not, even where that directory is inside the app
    /// folder. The method form needs no path and spawns.
    #[test]
    fn a_script_in_the_app_folder_spawns_and_one_outside_it_does_not() {
        let scratch = scratch("spawn-default");
        let app = scratch.join("app");
        for folder in [app.join(".jobs"), app.join("tmp"), scratch.join("outside")] {
            std::fs::create_dir_all(folder).expect("the case makes its folders");
        }
        for file in [
            app.join("job.nvs"),
            app.join(".jobs").join("job.nvs"),
            scratch.join("outside").join("job.nvs"),
        ] {
            std::fs::write(file, "").expect("the case writes its scripts");
        }
        let app = nvs_config::capability::resolved(&app, &nvs_config::resolve::Disk)
            .expect("the app folder exists");

        for serving in [true, false] {
            let mut ctx = serving_at(&app.join("tmp"));
            if !serving {
                ctx.set_run_kind(crate::ctx::RunKind::Command);
            }
            ctx.set_app_folder(Arc::from(app.as_path()));
            let made = temp_dir(&mut ctx, MEMBER).expect("the run makes its own directory");
            std::fs::write(made.join("job.nvs"), "").expect("the run writes a script there");

            let spawns = |path: &Path| granted(&ctx, Cap::ScriptSpawn, Scope::Path(path));
            assert!(
                spawns(&app.join("job.nvs")),
                "the app folder, serving: {serving}"
            );
            assert!(
                granted(&ctx, Cap::ScriptSpawn, Scope::Unscoped),
                "the method form, serving: {serving}"
            );
            for refused in [
                app.join(".jobs").join("job.nvs"),
                scratch.join("outside").join("job.nvs"),
                made.join("job.nvs"),
            ] {
                assert!(
                    !spawns(&refused),
                    "{}, serving: {serving}",
                    refused.display()
                );
            }
        }
    }
}
