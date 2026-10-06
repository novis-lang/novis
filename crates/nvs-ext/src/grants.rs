//! `rule:security/extension-grants-are-an-intersection`: the files and hosts a guest may reach, as
//! the one function [`effective`] computes from the three parties that each only narrow.
//!
//! The parties are the operator's `[[extension]] grants` ([`Granted`], every root already absolute
//! against the file that wrote it), the manifest's [`Requests`], and the [`Caller`]: the calling
//! context's own `[capabilities]`, narrowed by an isolate's `grants:` list when one spawned it. The
//! preopens and the outbound handler read only the [`Effective`] this returns, which is a type of
//! its own so the entry's unintersected grant cannot be handed to them by mistake.
//!
//! Decisions ADR 0246 § 7 left to this module, under the priority ordering:
//!
//! - **The manifest's `read` and `write` say whether it asks for that kind at all.** An author
//!   cannot know the operator's folders, so a list that is not empty requests the kind, its entries
//!   are what `nvs ext inspect` prints for the operator to read, and the roots come from the entry.
//!   `connect` names hosts, and an entry host is kept only where the manifest names it, compared
//!   case-insensitively as `net.connect` compares. Neither side has a host pattern.
//! - **The caller narrows by path, not by membership.** An entry root under a root the caller holds
//!   is kept whole, and a caller root under an entry root is kept in its place, so the result covers
//!   exactly the paths both cover. A caller that holds `fs.read = true` keeps every entry root.
//! - **A `write` root is opened read-write, so the caller must hold both `fs.write` and `fs.read`
//!   over it.** Holding a write grant alone would otherwise let the guest read what its caller
//!   cannot. A `write` root is not repeated in `read`.
//! - **Roots are canonicalised here**, by [`nvs_config::capability::resolved`], the one
//!   implementation `rule:security/path-scope-canonicalise-then-prefix` allows, and compared over
//!   whole components. The caller's roots are canonical already, because the snapshot is. An entry
//!   root that does not resolve is dropped, which only narrows.
//!
//! Cost: one canonicalisation per entry root and one list walk per caller root, paid when an
//! instance is created, never by a request that calls no extension. The result is a few paths and
//! host names per instance, freed with it.

use std::path::{Path, PathBuf};

use nvs_config::capability::{Cap, Grant};
use nvs_config::extension::Granted;
use nvs_config::resolve::Files;
use nvs_config::tree::Capabilities;

use crate::manifest::Requests;

/// The calling code's own authority at the call.
#[derive(Clone, Copy, Debug, Default)]
pub struct Caller<'a> {
    /// The `[capabilities]` of the caller's configuration snapshot, with its roots canonical.
    /// `None` holds nothing.
    pub capabilities: Option<&'a Capabilities>,
    /// An isolate's `grants:` list, when a spawn narrowed the caller: a capability it does not
    /// name is not held, whatever `capabilities` grants. `None` narrows nothing.
    pub narrowed: Option<&'a [Cap]>,
}

impl Caller<'_> {
    fn keeps(&self, cap: Cap) -> bool {
        self.narrowed.is_none_or(|kept| kept.contains(&cap))
    }

    fn holds(&self, cap: Cap) -> Grant<'_> {
        match self.capabilities {
            Some(caps) if self.keeps(cap) => caps.holds(cap),
            _ => Grant::Nothing,
        }
    }

    fn connects(&self, host: &str) -> bool {
        self.keeps(Cap::NetConnect)
            && self
                .capabilities
                .is_some_and(|caps| caps.allows_host(Cap::NetConnect, host))
    }
}

/// What a guest may reach: the roots it may read under, the roots it may read and write under, and
/// the hosts its outbound HTTP may reach. Every root is canonical.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Effective {
    /// Roots opened read-only.
    pub read: Vec<PathBuf>,
    /// Roots opened read-write.
    pub write: Vec<PathBuf>,
    /// Hosts the outbound handler may connect to.
    pub connect: Vec<String>,
}

impl Effective {
    /// Whether the guest may reach nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.read.is_empty() && self.write.is_empty() && self.connect.is_empty()
    }
}

/// The intersection of `entry`, `requests` and `caller`. `files` is the canonicaliser, the disk in
/// production and a fake under test.
#[must_use]
pub fn effective(
    entry: &Granted,
    requests: &Requests,
    caller: Caller<'_>,
    files: &dyn Files,
) -> Effective {
    let read = if requests.read.is_empty() {
        Vec::new()
    } else {
        narrow(canonical(&entry.read, files), caller.holds(Cap::FsRead))
    };
    let write = if requests.write.is_empty() {
        Vec::new()
    } else {
        let writable = narrow(canonical(&entry.write, files), caller.holds(Cap::FsWrite));
        narrow(writable, caller.holds(Cap::FsRead))
    };
    let connect = entry
        .connect
        .iter()
        .filter(|host| {
            requests
                .connect
                .iter()
                .any(|asked| asked.eq_ignore_ascii_case(host))
        })
        .filter(|host| caller.connects(host))
        .cloned()
        .collect();
    Effective {
        read,
        write,
        connect,
    }
}

fn canonical(roots: &[PathBuf], files: &dyn Files) -> Vec<PathBuf> {
    roots
        .iter()
        .filter(|root| !root.as_os_str().is_empty())
        .filter_map(|root| nvs_config::capability::resolved(root, files))
        .collect()
}

/// The paths both `roots` and `held` cover, as roots.
fn narrow(roots: Vec<PathBuf>, held: Grant<'_>) -> Vec<PathBuf> {
    let mut kept = match held {
        Grant::Nothing => Vec::new(),
        Grant::Everything => roots,
        Grant::These(list) => {
            let mut kept = Vec::new();
            for root in &roots {
                for granted in list.iter().map(Path::new) {
                    if root.starts_with(granted) {
                        kept.push(root.clone());
                    } else if granted.starts_with(root) {
                        kept.push(granted.to_path_buf());
                    }
                }
            }
            kept
        }
    };
    kept.sort();
    kept.dedup();
    kept
}
