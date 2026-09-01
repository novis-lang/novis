//! [ADR 0118] § 2's check, and § 5's refusal: the one function a door to the operating system calls
//! before it opens.
//!
//! [`require`] is deliberately the only *decision* here. The decision procedure is
//! [`nvs_config::capability`] and is pure; this is the half that knows about a request — where the
//! snapshot comes from, and what a denial looks like to the program that hit it. Twelve below are
//! § 2's filesystem doors — [`open_read`], [`metadata`], [`metadata_if_present`], [`exists`],
//! [`canonicalize`], [`resolve_existing`] and
//! [`read_dir`] behind `fs.read`, [`write()`],
//! [`remove_file`], [`remove_dir`] and [`temp_dir`] behind `fs.write`, and [`open`] behind whichever
//! of the two its [`Access`] names — [`exec`] is the process
//! door behind `process.exec`, and [`pin_host`] is the outbound one behind `net.connect`, which
//! answers an address rather than a yes for ADR 0058 § 2's reason; each of them calls [`require`]
//! before it names a spelling that
//! performs the effect, which is what makes § 2's claim structural rather than a convention: a member
//! reaches the OS through a door or not at all, and every door has already asked.
//!
//! **A context with no configuration grants nothing.** That is not a special case for tests — it is the
//! same deny-by-default the absent block gets, and a request path that reached a capability check
//! without a snapshot has a bug that should fail closed rather than quietly succeed.
//!
//! [ADR 0118]: ../../../docs/adr/0118-a-capability-is-checked-at-the-door-to-the-effect.md

use std::fs::{File, ReadDir};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// What [`metadata`] and [`metadata_if_present`] hand back, re-exported so that their callers can
/// name it.
///
/// `nvs-stdlib` may not write `std::fs` anywhere —
/// `nvs_stdlib_reaches_the_os_only_through_the_gate` is the scan that holds ADR 0118 § 2's door
/// shut — so a `Core` member that passes one of these to a helper of its own would otherwise have
/// no spelling for the parameter, and would have to re-derive each field at the call site instead.
/// Re-exporting the type grants nothing: every way of *obtaining* one still goes through a door
/// above that has already asked.
pub use std::fs::Metadata;

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
/// grants it under and, for a scoped check, the argument that fell outside the grant. It is never a
/// `FATAL`: a denial is known before any work is done and leaves nothing behind, so a program that
/// degrades when a capability is missing is a reasonable program (ADR 0118 § 5).
pub fn require(ctx: &Ctx, cap: Cap, scope: Scope<'_>, member: &str) -> Result<(), Fault> {
    match refusal(ctx, cap, scope, member) {
        Some(message) => Err(Fault::thrown(message)),
        None => Ok(()),
    }
}

/// [`require`]'s answer as data: `None` when the capability covers `scope`, and § 5's message
/// otherwise.
///
/// For the one door that cannot hand back a [`Fault`] — [`crate::script::resolve`] owns an error
/// type of its own, because a spawn's two other ways of failing are not capability questions. It
/// asks this rather than re-deriving the sentence, so the message a denial prints has exactly one
/// author whichever door produced it.
pub(crate) fn refusal(ctx: &Ctx, cap: Cap, scope: Scope<'_>, member: &str) -> Option<String> {
    let granted = ctx.config().is_some_and(|config| {
        config
            .snapshot()
            .config
            .capabilities
            .as_ref()
            .is_some_and(|caps| caps.allows(cap, scope, &nvs_config::resolve::Disk))
    });
    if granted {
        return None;
    }
    Some(denial(cap, scope, member))
}

/// § 5's message. The capability's name comes first after the member because the reader is usually
/// the operator, and that string is what they are about to paste into a configuration file.
fn denial(cap: Cap, scope: Scope<'_>, member: &str) -> String {
    let name = cap.name();
    match scope {
        Scope::Unscoped => format!("{member} needs the capability `{name}`, which is not granted"),
        Scope::Path(path) => format!(
            "{member} needs the capability `{name}` for {}, which is not granted",
            path.display()
        ),
        Scope::Host(host) | Scope::Name(host) => {
            format!("{member} needs the capability `{name}` for {host}, which is not granted")
        }
    }
}

/// [ADR 0058](../../../docs/adr/0058-outbound-request-policy.md)'s outbound door: the one address
/// `host` is approved to be reached at, once [`Cap::NetConnect`] has been shown to cover the name
/// and § 3's policy has been shown to cover the address.
///
/// **The address is the answer, and that is § 2's load-bearing part.** A door that said only "yes"
/// would leave a gap between this check and the connection in which a second DNS resolution could
/// answer differently — the rebinding attack — so the caller is handed the address that was
/// approved and connects to *that*. Every retry of a call reuses it and only a redirect hop asks
/// again ([ADR 0074](../../../docs/adr/0074-http-defaults-safe-and-finite.md) § 6).
///
/// **Here rather than in `nvs-stdlib`**, for § 5's reason and for this crate's: the policy is one
/// policy across `Core\Http`, `Core\Net` and `Core\Db::open`, and resolution is an operating-system
/// effect, which every `Core` member reaches through a door in this module and through nothing
/// else.
///
/// The resolution is synchronous. `Core\Http\Client`'s own transport will run over the parking
/// stream, and moving this call onto the blocking pool belongs with it rather than ahead of it —
/// what it costs today is one core parked in the resolver for the length of a lookup, which is the
/// same cost the file doors above already pay.
///
/// `member` is what a refusal names — see [`require`].
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `net.connect` for
/// `host`; a `RuntimeError` when the name resolves to no address at all; and a `RuntimeError`
/// naming the range when the address it resolves to is one § 3 denies and this deployment's
/// `net.internal` does not except ([`nvs_config::tree::CapNet::internal`]). The order is the point: a
/// host outside the grant is refused before it is looked up, so an ungranted program cannot use
/// this door as a resolver.
pub fn pin_host(ctx: &Ctx, host: &str, member: &str) -> Result<std::net::IpAddr, Fault> {
    use std::net::{IpAddr, ToSocketAddrs};

    require(ctx, Cap::NetConnect, Scope::Host(host), member)?;

    // A bracketed IPv6 literal is written `[::1]` inside an authority and is not one anywhere else,
    // so the brackets come off before the address is read and stay off afterwards.
    let bare = host
        .strip_prefix('[')
        .and_then(|held| held.strip_suffix(']'))
        .unwrap_or(host);
    let address = match bare.parse::<IpAddr>() {
        Ok(literal) => literal,
        Err(_) => (host, 0_u16)
            .to_socket_addrs()
            .ok()
            .and_then(|mut found| found.next())
            .map(|socket| socket.ip())
            .ok_or_else(|| {
                Fault::thrown(format!(
                    "{member} could not resolve {host}, so there is no address to pin"
                ))
            })?,
    };

    // § 3's table, less whatever this deployment excepted from it with `net.internal`. A context
    // with no snapshot, and one whose snapshot grants no capability at all, both get the table
    // itself -- an exception is something an operator wrote, so its absence is the default and not
    // a reason to skip the question.
    let refused = match ctx.config() {
        Some(config) => match config.snapshot().config.capabilities.as_ref() {
            Some(caps) => caps.address_refused(address),
            None => nvs_config::capability::denied_by_default(address),
        },
        None => nvs_config::capability::denied_by_default(address),
    };

    match refused {
        Some(range) => Err(Fault::thrown(format!(
            "{member} refuses {address}: it is {range}, which `net.internal` does not except"
        ))),
        None => Ok(address),
    }
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
pub fn open_read(ctx: &Ctx, path: &Path, member: &str) -> Result<File, Fault> {
    require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    File::open(path).map_err(|err| io_failure(member, path, &err))
}

/// What a program asked an open handle for, and so which capability [`open`] has to show.
///
/// Declared here rather than in `nvs-stdlib` because the capability question is this module's and
/// the answer differs per variant: a door that took an already-built [`std::fs::OpenOptions`] could
/// not ask what the caller intended, since nothing on that type reports back what was set. The
/// surface enum a program writes is `Core\IO\FileMode`, which maps onto this one and adds nothing —
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
/// This is [`open_read`] generalised to the three writing accesses, and the split between them is
/// deliberate: `open_read` is the whole-file read every `Core\IO` reader shares, and this is the one
/// a `Core\IO\File` handle comes out of. A writing open **creates** the path it names, which
/// [`write()`]'s own doc calls a reason to keep the create on the door's side — it is on this side
/// too, because the `require` below runs before the `OpenOptions` does anything at all.
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
    if matches!(access, Access::Read | Access::ReadWrite) {
        require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    }
    if matches!(access, Access::Write | Access::Append | Access::ReadWrite) {
        require(ctx, Cap::FsWrite, Scope::Path(path), member)?;
    }
    let mut options = std::fs::OpenOptions::new();
    match access {
        Access::Read => options.read(true),
        Access::Write => options.write(true).create(true).truncate(true),
        Access::Append => options.append(true).create(true),
        // No `truncate`: a read-write handle that emptied the file before its
        // holder had read a byte is `fopen`'s `w+`, and the mode a program
        // reaches for when it wants both is the one that keeps what is there.
        Access::ReadWrite => options.read(true).write(true).create(true),
    };
    options
        .open(path)
        .map_err(|err| io_failure(member, path, &err))
}

/// § 2's streaming-write door: a handle on `path`, ready to become the whole of its content, once
/// [`Cap::FsWrite`] has been shown to cover it.
///
/// [`write()`] hands back the finished effect because it already holds every byte; this is the same
/// door for a caller that does not — `Core\IO::writeStream` writes what it is handed as it is handed
/// it, so the handle has to cross. The create is still on this side of it, which is [`write()`]'s own
/// reason for taking the bytes.
///
/// **`overwrite` is a parameter rather than a fifth [`Access`] case**, because that enum is
/// `Core\IO\FileMode`'s four cases and nothing else: a case no mode spells would be a variant the
/// surface enum could never produce. And **`overwrite == false` refuses through the operating
/// system** — `create_new`, which is `O_EXCL` — rather than through an [`exists`] call first: a check
/// followed by a create is a window another process can create the file in, and ADR 0105 § 4 makes
/// this the default precisely because the destination is usually named by a client.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// `path`, checked before anything is created for [`write()`]'s reason. [`io_failure`]'s `IOError`
/// when the create itself fails — including `AlreadyExists`, which is what a refused overwrite is.
pub fn create(ctx: &Ctx, path: &Path, overwrite: bool, member: &str) -> Result<File, Fault> {
    require(ctx, Cap::FsWrite, Scope::Path(path), member)?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true);
    if overwrite {
        options.create(true).truncate(true);
    } else {
        options.create_new(true);
    }
    options
        .open(path)
        .map_err(|err| io_failure(member, path, &err))
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
    require(ctx, Cap::FsWrite, Scope::Path(path), member)?;
    std::fs::write(path, bytes).map_err(|err| io_failure(member, path, &err))
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
/// The byte count `std::fs::copy` answers with is dropped here rather than returned, because
/// `Core\IO::copy` has nothing to say about it and a door that answered one would be inviting a
/// second member to report it.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `from` or `fs.write` for `to`, or [`io_failure`]'s `IOError` when the copy itself fails. The
/// message names [`pair`]'s both-ends spelling, because either end can be the one at fault.
pub fn copy(ctx: &Ctx, from: &Path, to: &Path, member: &str) -> Result<(), Fault> {
    require(ctx, Cap::FsRead, Scope::Path(from), member)?;
    require(ctx, Cap::FsWrite, Scope::Path(to), member)?;
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|err| io_failure(member, &pair(from, to), &err))
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
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// either path, or [`io_failure`]'s `IOError` when the rename itself fails — nothing at the source,
/// a destination directory that is not there, or the two paths on different filesystems.
pub fn rename(ctx: &Ctx, from: &Path, to: &Path, member: &str) -> Result<(), Fault> {
    require(ctx, Cap::FsWrite, Scope::Path(from), member)?;
    require(ctx, Cap::FsWrite, Scope::Path(to), member)?;
    std::fs::rename(from, to).map_err(|err| io_failure(member, &pair(from, to), &err))
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
/// `IOError` when the creation itself fails — a component that exists and is not a directory, or a
/// permission the process lacks.
pub fn create_dir(ctx: &Ctx, path: &Path, member: &str) -> Result<(), Fault> {
    require(ctx, Cap::FsWrite, Scope::Path(path), member)?;
    std::fs::create_dir_all(path).map_err(|err| io_failure(member, path, &err))
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
pub fn metadata(ctx: &Ctx, path: &Path, member: &str) -> Result<Metadata, Fault> {
    require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    std::fs::metadata(path).map_err(|err| io_failure(member, path, &err))
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
/// not being there.
pub fn metadata_if_present(
    ctx: &Ctx,
    path: &Path,
    member: &str,
) -> Result<Option<Metadata>, Fault> {
    require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    match std::fs::metadata(path) {
        Ok(found) => Ok(Some(found)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(io_failure(member, path, &err)),
    }
}

/// § 2's `realpath` door: what `path` resolves to **when every component of it already exists**,
/// once [`Cap::FsRead`] has been shown to cover it.
///
/// The second resolution door, and the difference from [`canonicalize`] is **only** what it does
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
pub fn exists(ctx: &Ctx, path: &Path, member: &str) -> Result<bool, Fault> {
    require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    path.try_exists()
        .map_err(|err| io_failure(member, path, &err))
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
/// The [`ReadDir`] is handed back lazily, as [`open_read`] hands back a [`File`]: one check, at the
/// door, over the one path the whole walk stays inside.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`, or [`io_failure`]'s `IOError` when the directory could not be opened at all — it is not
/// there, or it is not a directory. A failure on one *entry* during the walk arrives later and is
/// the caller's, since only the caller knows whether an entry it cannot read is fatal to what it
/// was asking.
pub fn read_dir(ctx: &Ctx, path: &Path, member: &str) -> Result<ReadDir, Fault> {
    require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    std::fs::read_dir(path).map_err(|err| io_failure(member, path, &err))
}

/// § 2's unlink door: `path` stops existing, once [`Cap::FsWrite`] has been shown to cover it.
///
/// Removal is a write and not a fifth capability, for the reason § 3 gives for not splitting one:
/// an account that may replace a file's whole content can already destroy it, so a separate grant
/// would name a distinction the filesystem does not make.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// `path`, or [`io_failure`]'s `IOError` when the unlink itself fails — the path is not there, or is
/// a directory, which [`remove_dir`] is the door for.
pub fn remove_file(ctx: &Ctx, path: &Path, member: &str) -> Result<(), Fault> {
    require(ctx, Cap::FsWrite, Scope::Path(path), member)?;
    std::fs::remove_file(path).map_err(|err| io_failure(member, path, &err))
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
pub fn remove_dir(ctx: &Ctx, path: &Path, member: &str) -> Result<(), Fault> {
    require(ctx, Cap::FsWrite, Scope::Path(path), member)?;
    std::fs::remove_dir(path).map_err(|err| io_failure(member, path, &err))
}

/// § 2's temporary-directory door: a new, empty, private directory under the system temporary root,
/// once [`Cap::FsWrite`] has been shown to cover **the path it is about to create**.
///
/// The check is the ordinary one and the argument is the ordinary argument, which is the whole
/// decision here: a member that creates a directory the program never named could plausibly have
/// been exempt from the grant, or have widened it to cover what it created, and both would make a
/// capability something a running program can enlarge. So the name is chosen first and asked about
/// second, exactly as [`write()`] asks about a path that does not exist yet, and an operator grants
/// the temporary root — or `true` — or the member does not run.
///
/// **Nothing here relies on the name being unpredictable.** The defence is that creating a directory
/// is atomic: a name an attacker has already taken, including as a symlink, fails with
/// `AlreadyExists` and is retried rather than adopted. On Unix the mode is `0o700` at creation
/// rather than after it, so there is no window in which the directory is readable by anyone else; on
/// Windows the per-user temporary root already carries that ACL and the directory inherits it.
///
/// The caller owns what it gets. Nothing here registers the directory for later cleanup — a program
/// removes what it made, and a runtime that swept temporary directories at request end would be
/// deciding the lifetime of data it knows nothing about.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for the
/// temporary root, or [`io_failure`]'s `IOError` when every attempt to create one failed.
pub fn temp_dir(ctx: &Ctx, member: &str) -> Result<PathBuf, Fault> {
    /// Enough attempts that exhausting them means something other than a collision — a full disk, a
    /// root that is not writable, a temporary directory someone has filled with our names.
    const ATTEMPTS: u32 = 16;

    let root = std::env::temp_dir();
    for _ in 0..ATTEMPTS {
        let path = root.join(format!("nvs-{}-{:016x}", std::process::id(), nonce()));
        require(ctx, Cap::FsWrite, Scope::Path(&path), member)?;
        match create_private_dir(&path) {
            Ok(()) => return Ok(path),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(io_failure(member, &path, &err)),
        }
    }
    Err(io_failure(
        member,
        &root,
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

/// [`temp_dir`]'s one create, with the mode applied by the create itself rather than after it.
fn create_private_dir(path: &Path) -> std::io::Result<()> {
    private_builder().create(path)
}

/// A builder that creates owner-only directories: `0o700` from the moment the directory exists, so
/// there is no window in which anyone else on the machine can read it.
#[cfg(unix)]
fn private_builder() -> std::fs::DirBuilder {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700);
    builder
}

/// The same builder where the mode is not a concept: Windows has no `mode` bits to set, and the
/// per-user temporary root already carries the ACL a new directory under it inherits.
#[cfg(not(unix))]
fn private_builder() -> std::fs::DirBuilder {
    std::fs::DirBuilder::new()
}

/// § 2's process door: `program` started as a child with `argv`, once [`Cap::ProcessExec`] has been
/// shown to cover it and [ADR 0044] § 4's shell targets have been refused.
///
/// The started child rather than its output, for [`open_read`]'s reason: one door has to serve
/// `Core\Process::run`'s captured wait and `::spawn`'s streamed handle alike, and the capability
/// question belongs to the *child* — a process already started is a process already checked, so
/// nothing downstream of this call has to ask again.
///
/// `argv` is what the program receives after its own name, which the operating system supplies:
/// there is no command line anywhere in this function, and so nothing for a quoting rule to be
/// wrong about. ADR 0044 § 1 is why that is the only shape offered.
///
/// **All three standard streams are pipes, and that is this door's decision rather than the
/// caller's.** A child that inherited them would read the server's own stdin and write to the
/// server's own stdout — a request reaching a descriptor no capability named, and one that no
/// `Core\Process` member would have to ask for.
///
/// **A shell target is refused here, on every platform**, [ADR 0044] § 4: a `.bat`, `.cmd` or `.ps1`
/// runs by handing a command line to `cmd.exe` or `powershell.exe`, which re-parses the arguments
/// this door never built, so an argv Novis passed correctly becomes a shell string again by the time
/// the target sees it. The check runs on Unix too, where the risk is not real — a `#!` line is read
/// by the same `execve` that already has the split argv — because a refusal that exists on one
/// platform only is a behaviour no test on the other can pin.
///
/// The capability is asked **first**, before the target's kind, so the rule every other door here
/// states holds without an exception: the grant is consulted before anything else is looked at.
/// Both refusals are the same catchable class, since neither is a condition a program can recover
/// from by trying something adjacent.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `process.exec` for
/// `program`, a `RuntimeError` naming the extension for § 4's refused target kinds, or
/// [`io_failure`]'s `IOError` when the spawn itself fails — nothing is at `program`, or it is not
/// executable.
///
/// [ADR 0044]: ../../../docs/adr/0044-core-process-argv-only-no-shell.md
pub fn exec(ctx: &Ctx, program: &Path, argv: &[&str], member: &str) -> Result<Child, Fault> {
    require(ctx, Cap::ProcessExec, Scope::Path(program), member)?;
    if let Some(extension) = shell_target(program) {
        return Err(Fault::thrown(format!(
            "{member} will not run {}: a `.{extension}` target is started by handing a command line \
             to a second parser, which re-quotes an argv this API passed across whole — start that \
             interpreter yourself if it is what you mean",
            program.display()
        )));
    }
    Command::new(program)
        .args(argv)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| io_failure(member, program, &err))
}

/// The lower-cased extension of a target [`exec`] refuses, or `None` for one it will start.
///
/// By extension and not by content, ADR 0044 § 4: what makes a `.bat` unsafe to start is which
/// program the operating system hands the command line to, and that is decided by the name alone —
/// so this answers the same way for a file that is not there, which is what lets the refusal be
/// about the kind of target rather than about the filesystem.
fn shell_target(program: &Path) -> Option<String> {
    let extension = program.extension()?.to_str()?.to_ascii_lowercase();
    matches!(extension.as_str(), "bat" | "cmd" | "ps1").then_some(extension)
}

/// What a door reports when the operating system refuses something the capability allowed: an
/// `IOError`, catchable, naming the member, the path and what the OS said.
///
/// Public because [`open_read`] hands back a handle rather than a result, so the caller that reads
/// from it owes the same message for the same kind of failure; one function is how the two agree
/// rather than drifting into two spellings of "could not read".
#[must_use]
pub fn io_failure(member: &str, path: &Path, err: &std::io::Error) -> Fault {
    Fault::thrown_as(
        ThrownClass::Io,
        format!("{member} failed on {}: {err}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{Cap, Ctx, Fault, Path, Scope, ThrownClass, exec, require, shell_target};

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

    /// ADR 0118 § 5, asked of the process door: an unconfigured context starts nothing, and the
    /// message names the capability in the spelling `nvs.toml` grants it under.
    #[test]
    fn a_child_starts_only_where_process_exec_is_granted() {
        let ctx = Ctx::buffered();
        let denied = exec(&ctx, Path::new("/usr/bin/convert"), &["-version"], MEMBER)
            .expect_err("a context with no configuration grants nothing");
        let Fault::Thrown(class, message) = denied else {
            panic!("a denial is a throw and never a fatal — ADR 0118 § 5");
        };
        assert_eq!(class, ThrownClass::Runtime);
        assert!(
            message.contains("process.exec")
                && message.contains(MEMBER)
                && message.contains("convert"),
            "the denial names the capability, who wanted it and what for: {message}"
        );
    }

    /// ADR 0044 § 4, on a context that grants everything: the refusal is about the kind of target,
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
            Scope::Path(Path::new("examples/process/say.bat")),
            MEMBER,
        )
        .expect("`exec = true` covers every program, this one included");

        for (target, named) in [
            ("examples/process/say.bat", "bat"),
            ("C:/deploy/RELEASE.CMD", "cmd"),
            ("./build.ps1", "ps1"),
        ] {
            let refused = exec(&ctx, Path::new(target), &[], MEMBER)
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

    /// The other half of the same rule, which the cases above cannot show: every other target kind
    /// reaches the spawn, including the shebang script Unix runs through `execve` itself.
    #[test]
    fn nothing_but_those_three_extensions_is_a_shell_target() {
        for allowed in [
            "/usr/bin/convert",
            "target/debug/nvs.exe",
            "tools/deploy.sh",
            "batch",
            "archive.bat.gz",
        ] {
            assert!(
                shell_target(Path::new(allowed)).is_none(),
                "`{allowed}` is started by the operating system, not by a command-line parser"
            );
        }
    }
}
