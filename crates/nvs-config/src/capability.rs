//! [ADR 0118] § 1's capability question: a grant, a scope, and the `bool` the two of them answer.
//!
//! This module is the whole decision procedure and it is **pure** — a [`Capabilities`], a [`Cap`], an
//! argument, a `bool`. It takes no context, throws nothing, and reports nothing, so it is testable
//! without a compiler or a request in front of it. The refusal a program sees is
//! `nvs_runtime::capability::require`, which asks this and turns a `false` into § 5's `RuntimeError`;
//! keeping the two apart is what lets `-p nvs-config` assert the *rule* and `-p nvs-stdlib` assert the
//! *diagnostic*.
//!
//! **Deny by default, at every step.** A capability whose block is absent is denied, one whose grant is
//! `false` is denied, one granted an empty list is denied, and a scoped argument that cannot be resolved
//! to a canonical path is denied. There is no path through [`Capabilities::allows`] that returns `true`
//! without an operator having written something that says so.
//!
//! [`Cap`] is also the one place a capability's configuration *name* maps to the field of
//! [`Capabilities`] that grants it. A new capability is a variant, a `name` arm and a `grant` arm —
//! never a string compared in a second module.
//!
//! [ADR 0118]: ../../../docs/adr/0118-a-capability-is-checked-at-the-door-to-the-effect.md

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::resolve::Files;
use crate::tree::{Capabilities, Setting};

/// One capability, by the name `nvs.toml` grants it under.
///
/// The roster is closed: a member needing something not in it has no `Cap` to pass, which is
/// [ADR 0118](../../../docs/adr/0118-a-capability-is-checked-at-the-door-to-the-effect.md)'s last
/// consequence — a capability the configuration cannot express fails visibly at the door rather than
/// quietly at a review.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cap {
    /// `fs.read` — the roots readable.
    FsRead,
    /// `fs.write` — the roots writable.
    FsWrite,
    /// `script.spawn` — the roots a `spawn script` target may live under (ADR 0006 § 5). Being able
    /// to read a file is not permission to run it, which is why this is not implied by `fs.read`.
    ScriptSpawn,
    /// `net.connect` — the hosts an outbound connection may reach.
    NetConnect,
    /// `process.exec` — the programs a subprocess may be started from.
    ProcessExec,
    /// `debug.trace` — where a trace may be written (ADR 0018).
    DebugTrace,
    /// `debug.profile` — where a profile may be written (ADR 0018).
    DebugProfile,
    /// `db.connect` — which `[db.<name>]` blocks a program may open by name (ADR 0067 § 3).
    DbConnect,
    /// `db.open` — which hosts a program-supplied `Db\Settings` may reach.
    DbOpen,
}

/// What a capability is being asked *about* — the second half of § 1's question.
#[derive(Clone, Copy, Debug)]
pub enum Scope<'a> {
    /// The grant is the whole answer: there is no argument to place inside it.
    Unscoped,
    /// A filesystem path, resolved by § 4's canonicalise-then-prefix rule.
    Path(&'a Path),
    /// A hostname, matched case-insensitively because DNS is.
    Host(&'a str),
    /// A configured name — a `[db.<name>]` block — matched exactly, because a name is not a hostname
    /// and two blocks differing only in case are two blocks.
    Name(&'a str),
}

impl Cap {
    /// Every capability, for a guard test and for `nvs meta`.
    pub const ALL: &'static [Self] = &[
        Self::FsRead,
        Self::FsWrite,
        Self::ScriptSpawn,
        Self::NetConnect,
        Self::ProcessExec,
        Self::DebugTrace,
        Self::DebugProfile,
        Self::DbConnect,
        Self::DbOpen,
    ];

    /// The name `nvs.toml` grants it under, which is also the name a refusal prints — the operator
    /// reading that message is about to paste this string into a configuration file.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::FsRead => "fs.read",
            Self::FsWrite => "fs.write",
            Self::ScriptSpawn => "script.spawn",
            Self::NetConnect => "net.connect",
            Self::ProcessExec => "process.exec",
            Self::DebugTrace => "debug.trace",
            Self::DebugProfile => "debug.profile",
            Self::DbConnect => "db.connect",
            Self::DbOpen => "db.open",
        }
    }

    /// The capability that [`name`](Self::name) spells, or `None` for a name no capability has.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|cap| cap.name() == name)
    }

    /// Whether this capability's grant is a list of **paths**, and so whether
    /// [`Capabilities::canonicalize`] has work to do on it.
    #[must_use]
    pub const fn is_path_scoped(self) -> bool {
        matches!(
            self,
            Self::FsRead
                | Self::FsWrite
                | Self::ScriptSpawn
                | Self::ProcessExec
                | Self::DebugTrace
                | Self::DebugProfile
        )
    }

    /// What `caps` grants for this capability, or `None` when the block is absent — which is a
    /// refusal, not an omission.
    #[must_use]
    pub fn grant(self, caps: &Capabilities) -> Option<&Setting> {
        match self {
            Self::FsRead => caps.fs.as_ref()?.read.as_ref(),
            Self::FsWrite => caps.fs.as_ref()?.write.as_ref(),
            Self::ScriptSpawn => caps.script.as_ref()?.spawn.as_ref(),
            Self::NetConnect => caps.net.as_ref()?.connect.as_ref(),
            Self::ProcessExec => caps.process.as_ref()?.exec.as_ref(),
            Self::DebugTrace => caps.debug.as_ref()?.trace.as_ref(),
            Self::DebugProfile => caps.debug.as_ref()?.profile.as_ref(),
            Self::DbConnect => caps.db.as_ref()?.connect.as_ref(),
            Self::DbOpen => caps.db.as_ref()?.open.as_ref(),
        }
    }

    /// [`grant`](Self::grant)'s mirror, for [`Capabilities::canonicalize`] alone.
    ///
    /// Written out a second time rather than derived: the arms *are* the name-to-field mapping this
    /// type exists to hold once, and a mapping expressed as a shared traversal would be a third thing
    /// to keep in step with both.
    fn grant_mut(self, caps: &mut Capabilities) -> Option<&mut Setting> {
        match self {
            Self::FsRead => caps.fs.as_mut()?.read.as_mut(),
            Self::FsWrite => caps.fs.as_mut()?.write.as_mut(),
            Self::ScriptSpawn => caps.script.as_mut()?.spawn.as_mut(),
            Self::NetConnect => caps.net.as_mut()?.connect.as_mut(),
            Self::ProcessExec => caps.process.as_mut()?.exec.as_mut(),
            Self::DebugTrace => caps.debug.as_mut()?.trace.as_mut(),
            Self::DebugProfile => caps.debug.as_mut()?.profile.as_mut(),
            Self::DbConnect => caps.db.as_mut()?.connect.as_mut(),
            Self::DbOpen => caps.db.as_mut()?.open.as_mut(),
        }
    }
}

/// What a [`Setting`] means when it is read as a grant.
enum Grant<'a> {
    /// Nothing is granted — an absent block, `false`, an empty list, or a number, which is not a
    /// spelling any capability has.
    Nothing,
    /// Everything is granted: the operator wrote `true`.
    Everything,
    /// These entries and no others.
    These(&'a [String]),
}

fn grant_of(setting: &Setting) -> Grant<'_> {
    match setting {
        Setting::Bool(true) => Grant::Everything,
        Setting::Text(one) => Grant::These(std::slice::from_ref(one)),
        Setting::List(many) if !many.is_empty() => Grant::These(many),
        Setting::Bool(false) | Setting::List(_) | Setting::Integer(_) | Setting::Float(_) => {
            Grant::Nothing
        }
    }
}

impl Capabilities {
    /// § 1's question: does this configuration grant `cap` for `scope`?
    ///
    /// `files` is the canonicalizer, and it is a parameter rather than the filesystem because
    /// [`Files::canonical`] is [`trust::canonical`](crate::trust::canonical) in production and a fake
    /// under test — the same seam [`app`](crate::app) resolves an `[[app]]` key through, and for the
    /// same reason: a second canonicalizer is how a `..` gets through.
    #[must_use]
    pub fn allows(&self, cap: Cap, scope: Scope<'_>, files: &dyn Files) -> bool {
        let Some(setting) = cap.grant(self) else {
            return false;
        };
        match (grant_of(setting), scope) {
            (Grant::Nothing, _) => false,
            (Grant::Everything, _) => true,
            (Grant::These(_), Scope::Unscoped) => true,
            (Grant::These(list), Scope::Host(host)) => {
                list.iter().any(|entry| entry.eq_ignore_ascii_case(host))
            }
            (Grant::These(list), Scope::Name(name)) => list.iter().any(|entry| entry == name),
            (Grant::These(list), Scope::Path(path)) => {
                let Some(path) = resolved(path, files) else {
                    return false;
                };
                list.iter().any(|root| path.starts_with(Path::new(root)))
            }
        }
    }

    /// § 4's grant side: replaces every path-scoped root with its canonical spelling, once.
    ///
    /// Called when the snapshot is built, exactly as an `[[app]]` block's key is canonicalized at the
    /// same point and for the same reason — a root still spelled the way the operator typed it is a
    /// comparison against the wrong thing, and doing it per call would put a `realpath` on the grant
    /// side of every check rather than only on the argument's.
    ///
    /// A root that cannot be canonicalized is **left as written**, which fails closed: a canonical
    /// argument will not be under it, so the entry grants nothing until the directory exists. Dropping
    /// it instead would silently discard what an operator asked for.
    pub fn canonicalize(&mut self, files: &dyn Files) {
        for cap in Cap::ALL.iter().copied().filter(|cap| cap.is_path_scoped()) {
            let Some(setting) = cap.grant_mut(self) else {
                continue;
            };
            match setting {
                Setting::Text(one) => canonical_root(one, files),
                Setting::List(many) => {
                    for one in many.iter_mut() {
                        canonical_root(one, files);
                    }
                }
                Setting::Bool(_) | Setting::Integer(_) | Setting::Float(_) => {}
            }
        }
    }
}

fn canonical_root(root: &mut String, files: &dyn Files) {
    if let Ok(found) = files.canonical(Path::new(root.as_str())) {
        *root = found.to_string_lossy().into_owned();
    }
}

/// § 4's argument side: `path` canonicalized, or its **deepest existing ancestor** canonicalized with
/// the rest re-appended.
///
/// A write creates the file it names, so its path cannot be canonicalized before the check — and
/// creating it to find out whether creating it is allowed is the wrong order. Pinning the deepest
/// ancestor that does exist resolves every symlink and every `..` above that point, which is where an
/// escape would have to come from.
///
/// `None` when even the deepest ancestor cannot be resolved, and when a component that is still
/// unresolved is `..` — [`Path::file_name`] is `None` for one, so the loop exits on it. That is the
/// single component that could still escape after the ancestor is pinned, and there is no legitimate
/// spelling of a new file's path that needs one.
fn resolved(path: &Path, files: &dyn Files) -> Option<PathBuf> {
    if let Ok(found) = files.canonical(path) {
        return Some(found);
    }
    let mut tail: Vec<&OsStr> = Vec::new();
    let mut cursor = path;
    loop {
        let name = cursor.file_name()?;
        let parent = cursor.parent()?;
        tail.push(name);
        if let Ok(base) = files.canonical(parent) {
            let mut resolved = base;
            resolved.extend(tail.iter().rev());
            return Some(resolved);
        }
        cursor = parent;
    }
}
