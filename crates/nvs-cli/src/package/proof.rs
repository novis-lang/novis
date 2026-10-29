//! The client's checks of a registry's transparency log
//! (`rule:packaging/the-registry-keeps-an-append-only-log`): the checkpoint this computer saw of
//! each registry, the consistency proof a newer one needs, and the inclusion proof of every
//! archive the client fetches.
//!
//! **The log is under each file host's `log/`**, in C2SP's layout: `log/checkpoint` and
//! `log/tile/...`, as `nvs registry log append` writes a log directory. A checkpoint is opened with
//! the registry's pinned verifier key, the `key` of its `[registries]` line, and its origin is
//! that key's name. A registry with no key has no log or file the client can check, so nothing is
//! read from it.
//!
//! **Every signed root file and index file names a checkpoint**, and [`Seen::covers`] proves the
//! log continues it, once per checkpoint named. A file signed with a checkpoint the log does not
//! continue is the fork error too.
//!
//! **The newest checkpoint seen is kept**, one file per registry, `<name>.checkpoint` beside the
//! kept root file under the data folder's `registry/`, read once that folder passes
//! `rule:config/ownership-is-the-trust-boundary`'s check. A host's checkpoint is used when it is
//! the kept one, or when the host's tiles prove the kept one is a prefix of it, and then it
//! replaces the kept one. A host whose checkpoint does not open, whose log is shorter than the
//! kept one, or whose tiles do not lead to its own root is skipped, and the next host is tried.
//! A checkpoint that opens, whose tiles lead to its root and that the kept one is not a prefix of
//! is the fork error: the registry signed two histories, so nothing more is read from it and the
//! kept checkpoint stays. A host serving tiles that lead to the right root and a wrong proof also
//! ends in that error, which stops the command and lets nothing through.
//!
//! **An archive is used only once the log proves its publish record.** The record the index line
//! names, read from its entry bundle, must publish that package and version with the index's
//! digest, repository and commit, and its inclusion proof must lead to the checkpoint's root. The
//! check runs before the archive is read, and the archive's digest is checked when it is.
//!
//! **`nvs pkg audit` reads the whole log** ([`Seen::advisories`]): the advisory feed is checked
//! against every record, and the records past the feed's size are read for the advisories it does
//! not list yet. The root is computed from the records themselves ([`log::Fetched::walk`]).
//! [`Seen::compare_copy`] proves the second copy of the checkpoint a root file names is this log,
//! a prefix of it or an extension of it, and two copies that are none of these are a split view.
//!
//! Cost: one checkpoint and one consistency proof per registry per command, and one entry bundle
//! and one inclusion proof per archive. An audit also reads the feed, one entry bundle per 256
//! records of the log, and one consistency proof for the copy. Each read is a tile of at most 8 KiB of hashes or one
//! bundle of at most 256 entries, held until its proof is checked. A kept checkpoint is a few
//! hundred bytes per registry. Nothing here runs on the request path.

use std::fs;
use std::path::Path;

use nvs_config::trust::{self, Untrusted};

use super::advisory::{self, Advisory, Feed};
use super::entry::Entry;
use super::index::{Fault, Registry, Release, Stamp, fault};
use super::log::{self, Fetched, Hash};
use super::manifest::Name;
use super::note::{Checkpoint, Verifier};

/// The folder of a file host that serves the log.
const LOG: &str = "log";

/// The most bytes a checkpoint may have, its signatures included.
const MAX_CHECKPOINT_BYTES: u64 = 64 * 1024;

/// The most bytes a tile or an entry bundle may have: 256 entries of the largest size.
const MAX_TILE_BYTES: u64 = 256 * (2 + log::MAX_ENTRY_BYTES as u64);

/// The most bytes an advisory feed may have: tens of thousands of advisories.
const MAX_FEED_BYTES: u64 = 64 * 1024 * 1024;

/// The verifier key of the registry at `url`, read from `key`, its `[registries]` line's.
///
/// # Errors
///
/// No key, and a key that is not a verifier key.
pub(crate) fn verifier(url: &str, key: Option<&str>) -> Result<Verifier, Fault> {
    let Some(key) = key else {
        return Err(fault(format!(
            "the registry `{url}` has no verifier key, so `nvs pkg` cannot check its files. Add `key = \"<verifier key>\"` to its line under `[registries]` in \
             `package.toml`."
        )));
    };
    Verifier::parse(key).map_err(|err| {
        fault(format!(
            "the verifier key of the registry `{url}` is not valid: {err}"
        ))
    })
}

/// A registry's log as one of its file hosts serves it, at a checkpoint consistent with every one
/// this computer saw before.
pub(crate) struct Seen {
    checkpoint: Checkpoint,
    /// The file host that served the checkpoint, which the proofs are read from.
    host: usize,
    /// Each checkpoint a signed file named that this log continues, so each is proved once.
    covered: Vec<(u64, Hash)>,
}

/// Why a file a file host served was not used.
pub(crate) enum Skip {
    /// The host did not answer, or served something that does not check. The next host is tried.
    Host(String),
    /// The registry signed something wrong, such as a log the kept checkpoint is not a prefix of.
    /// The command stops, and no other host is tried.
    Stop(Fault),
}

impl Seen {
    /// The log of `registry` under `verifier`, from its first file host whose checkpoint opens
    /// and extends the one kept in `keep`. That checkpoint is kept in its place.
    ///
    /// # Errors
    ///
    /// The kept checkpoint's folder fails the ownership check or the kept file does not open, no
    /// file host serves a usable log, and the fork error.
    pub(crate) fn open(
        registry: &Registry,
        verifier: &Verifier,
        keep: Option<&Path>,
    ) -> Result<Self, Fault> {
        let kept_path = keep.map(|dir| dir.join(format!("{}.checkpoint", registry.kept)));
        let kept = match &kept_path {
            Some(path) => read_kept(path, &registry.url, verifier)?,
            None => None,
        };
        let mut failures = Vec::new();
        for host in registry.hosts() {
            match from_host(registry, host, verifier, kept.as_ref()) {
                Ok((checkpoint, note)) => {
                    let same = kept.as_ref().is_some_and(|kept| {
                        kept.size() == checkpoint.size() && kept.root() == checkpoint.root()
                    });
                    if let (Some(path), false) = (&kept_path, same) {
                        keep_checkpoint(path, &note)?;
                    }
                    return Ok(Self {
                        checkpoint,
                        host,
                        covered: Vec::new(),
                    });
                }
                Err(Skip::Host(why)) => {
                    failures.push(format!("{}: {why}", registry.host_name(host)));
                }
                Err(Skip::Stop(fork)) => return Err(fork),
            }
        }
        let mut message = format!(
            "cannot read the log of the registry `{}` from any of its file hosts:",
            registry.url
        );
        for failure in failures {
            message.push_str("\n  ");
            message.push_str(&failure);
        }
        Err(fault(message))
    }

    /// Checks that this log continues the checkpoint a signed root file or index file of
    /// `registry` names in `stamp`: it is this log at that size, proved by this log's tiles.
    ///
    /// # Errors
    ///
    /// [`Skip::Host`] for a checkpoint past this log's end and for tiles that do not lead to this
    /// log's root, and [`Skip::Stop`] with the fork error for a checkpoint this log does not
    /// continue.
    pub(crate) fn covers(&mut self, registry: &Registry, stamp: &Stamp) -> Result<(), Skip> {
        let (size, old) = (self.checkpoint.size(), stamp.size);
        if self.covered.contains(&(old, stamp.root)) {
            return Ok(());
        }
        if old > size {
            return Err(Skip::Host(format!(
                "it is signed with a log of {old} records, and the log has {size} records"
            )));
        }
        let mut tiles = tiles(registry, self.host, size);
        let consistent = if old == size {
            stamp.root == *self.checkpoint.root()
        } else if old == 0 {
            stamp.root == log::empty_root()
        } else {
            let proof = log::prove_tree(size, old, &mut tiles)
                .map_err(|err| Skip::Host(format!("cannot read the log: {err}")))?;
            log::check_tree(&proof, size, self.checkpoint.root(), old, &stamp.root).is_ok()
        };
        if consistent {
            self.covered.push((old, stamp.root));
            return Ok(());
        }
        if log::tree_hash(size, &mut tiles).ok().as_ref() != Some(self.checkpoint.root()) {
            return Err(Skip::Host(
                "the log's tiles do not match its checkpoint".to_owned(),
            ));
        }
        Err(Skip::Stop(fault(format!(
            "the log of the registry `{}` was forked: a file it signed names a checkpoint of {old} \
             records that its log of {size} records does not continue. Nothing was fetched. Tell \
             the people who run the registry.",
            registry.url
        ))))
    }

    /// The checkpoint of the log this computer checked.
    pub(crate) fn checkpoint(&self) -> &Checkpoint {
        &self.checkpoint
    }

    /// Every advisory in the log of `registry`: its advisory feed, read from the file host this
    /// log came from and checked against every record of the log, and each advisory past the
    /// feed's size, read from the log itself. The log's records must lead to its checkpoint's
    /// root, and its first records to the root the feed names.
    ///
    /// # Errors
    ///
    /// No feed, a feed that does not open, names a log this one does not continue, or disagrees
    /// with the log, and a log that cannot be read or does not lead to its root.
    pub(crate) fn advisories(&mut self, registry: &Registry) -> Result<Vec<Advisory>, Fault> {
        let url = &registry.url;
        let refused =
            |why: String| fault(format!("the advisory feed of `{url}` is not used: {why}"));
        let path = format!("{LOG}/{}", advisory::FILE);
        let bytes = registry
            .get(self.host, &path, MAX_FEED_BYTES)
            .map_err(|failure| refused(failure.0))?
            .ok_or_else(|| refused(format!("`{path}` is not on the file host")))?;
        let feed = Feed::open(&bytes, registry.verifier(), registry.now()).map_err(refused)?;
        self.covers(registry, &feed.stamp)
            .map_err(|skip| match skip {
                Skip::Host(why) => refused(why),
                Skip::Stop(stop) => stop,
            })?;
        let (size, root) = (self.checkpoint.size(), *self.checkpoint.root());
        let mut against = feed.against();
        let mut past = Vec::new();
        tiles(registry, self.host, size)
            .walk(&root, &[(feed.stamp.size, feed.stamp.root)], |n, record| {
                against.record(n, record)?;
                if n >= feed.stamp.size {
                    past.extend(Advisory::from_record(n, record));
                }
                Ok(())
            })
            .map_err(|err| fault(format!("the log of `{url}` does not check: {err}")))?;
        against.finish().map_err(refused)?;
        let mut all = feed.advisories;
        all.extend(past);
        Ok(all)
    }

    /// The size of the second copy of the checkpoint that the root file of `registry` names,
    /// once it opens with the registry's key and is this log or a prefix or an extension of it,
    /// proved by this log's tiles. `None` when the root file names no copy.
    ///
    /// # Errors
    ///
    /// The copy cannot be read, does not open, or cannot be proved against this log, and the
    /// error for two logs that disagree.
    pub(crate) fn compare_copy(&self, registry: &Registry) -> Result<Option<u64>, Fault> {
        let Some(copy_url) = registry.checkpoint_copy() else {
            return Ok(None);
        };
        let url = &registry.url;
        let unusable = |why: String| {
            fault(format!(
                "the second copy of the checkpoint of `{url}` at `{copy_url}` cannot be \
                 compared: {why}"
            ))
        };
        let note = registry
            .get_copy(MAX_CHECKPOINT_BYTES)
            .map_err(|failure| unusable(failure.0))?
            .ok_or_else(|| unusable("its host does not have it".to_owned()))?;
        let verifier = registry.verifier();
        let copy = Checkpoint::open(&note, verifier.name(), std::slice::from_ref(verifier))
            .map_err(|err| unusable(format!("it does not open with the registry's key: {err}")))?;
        let (ours, theirs) = (&self.checkpoint, &copy);
        let (long, short) = if theirs.size() > ours.size() {
            (theirs, ours)
        } else {
            (ours, theirs)
        };
        let consistent = if long.size() == short.size() {
            long.root() == short.root()
        } else if short.size() == 0 {
            *short.root() == log::empty_root()
        } else {
            let mut tiles = tiles(registry, self.host, long.size());
            let proof = log::prove_tree(long.size(), short.size(), &mut tiles)
                .map_err(|err| unusable(format!("cannot read the log: {err}")))?;
            let proved =
                log::check_tree(&proof, long.size(), long.root(), short.size(), short.root())
                    .is_ok();
            // Tiles that do not lead to the longer root are the host's fault, not a second log.
            if !proved && log::tree_hash(long.size(), &mut tiles).ok().as_ref() != Some(long.root())
            {
                return Err(unusable(
                    "the log's tiles do not match the longer checkpoint".to_owned(),
                ));
            }
            proved
        };
        if consistent {
            return Ok(Some(copy.size()));
        }
        Err(fault(format!(
            "the registry `{url}` shows two different logs: its file host has a log of {} \
             records, and the copy at `{copy_url}` has a log of {} records that does not \
             match it. Somebody may show you a different log than other users see. Tell the \
             people who run the registry.",
            ours.size(),
            theirs.size()
        )))
    }

    /// Checks that the log publishes `release` of `name`: the record its index line names is a
    /// publish of that version with the same digest, repository and commit, and its inclusion
    /// proof leads to the checkpoint's root.
    ///
    /// # Errors
    ///
    /// The record is past the log's end, is another record, or its proof does not check. The
    /// message names the package and the registry.
    pub(crate) fn include(
        &self,
        registry: &Registry,
        name: &Name,
        release: &Release,
    ) -> Result<(), Fault> {
        let absent = |why: String| {
            fault(format!(
                "`{name}` {} is not in the log of the registry `{}`, so it is not used: {why}",
                release.version, registry.url
            ))
        };
        let size = self.checkpoint.size();
        let Some(n) = release.log else {
            return Err(absent("its index names no record of the log".to_owned()));
        };
        if n >= size {
            return Err(absent(format!(
                "its index names record {n}, and the log has {size} records"
            )));
        }
        let mut tiles = tiles(registry, self.host, size);
        let record = tiles.record(n).map_err(|err| absent(err.to_string()))?;
        let publish = Entry::Publish {
            package: name.slug(),
            version: release.version.to_string(),
            digest: release.digest.to_hex().to_string(),
            repository: release.repository.clone(),
            commit: release.commit.clone(),
        };
        if Entry::parse(&record).ok() != Some(publish) {
            return Err(absent(format!(
                "record {n} does not publish this version with the digest {}",
                release.digest.to_hex()
            )));
        }
        let proof =
            log::prove_record(size, n, &mut tiles).map_err(|err| absent(err.to_string()))?;
        log::check_record(
            &proof,
            size,
            self.checkpoint.root(),
            n,
            &log::record_hash(&record),
        )
        .map_err(|_| {
            absent(format!(
                "the proof of record {n} does not match the checkpoint"
            ))
        })
    }
}

/// The checkpoint file host `host` of `registry` serves, and its note, once it opens and extends
/// `kept`.
fn from_host(
    registry: &Registry,
    host: usize,
    verifier: &Verifier,
    kept: Option<&Checkpoint>,
) -> Result<(Checkpoint, Vec<u8>), Skip> {
    let note = match registry.get(host, &format!("{LOG}/checkpoint"), MAX_CHECKPOINT_BYTES) {
        Ok(Some(note)) => note,
        Ok(None) => return Err(Skip::Host("it has no log".to_owned())),
        Err(failure) => return Err(Skip::Host(failure.0)),
    };
    let checkpoint = Checkpoint::open(&note, verifier.name(), std::slice::from_ref(verifier))
        .map_err(|err| Skip::Host(format!("its checkpoint does not open: {err}")))?;
    let Some(kept) = kept.filter(|kept| kept.size() > 0) else {
        return Ok((checkpoint, note));
    };
    let (size, old) = (checkpoint.size(), kept.size());
    if size < old {
        return Err(Skip::Host(format!(
            "its log has {size} records, and this computer saw {old}"
        )));
    }
    let mut tiles = tiles(registry, host, size);
    let consistent = if size == old {
        checkpoint.root() == kept.root()
    } else {
        let proof = log::prove_tree(size, old, &mut tiles)
            .map_err(|err| Skip::Host(format!("cannot read its log: {err}")))?;
        log::check_tree(&proof, size, checkpoint.root(), old, kept.root()).is_ok()
    };
    if consistent {
        return Ok((checkpoint, note));
    }
    // Tiles that do not lead to the host's own signed root are the host's fault, not the log's.
    if size > old && log::tree_hash(size, &mut tiles).ok().as_ref() != Some(checkpoint.root()) {
        return Err(Skip::Host(
            "its tiles do not match its checkpoint".to_owned(),
        ));
    }
    Err(Skip::Stop(fault(format!(
        "the log of the registry `{}` was forked: its checkpoint of {size} records does not \
         continue the log of {old} records this computer saw before. Nothing was fetched. Tell \
         the people who run the registry.",
        registry.url
    ))))
}

/// The log file host `host` of `registry` serves at `size` records.
fn tiles(
    registry: &Registry,
    host: usize,
    size: u64,
) -> Fetched<impl FnMut(&str) -> Result<Option<Vec<u8>>, log::Error> + '_> {
    Fetched::new(size, move |path: &str| {
        let path = format!("{LOG}/{path}");
        registry
            .get(host, &path, MAX_TILE_BYTES)
            .map_err(|failure| log::Error::Fetch {
                path,
                reason: failure.0,
            })
    })
}

/// The checkpoint kept at `path` for the registry at `url`, opened with `verifier`, or `None` when
/// none is kept.
fn read_kept(path: &Path, url: &str, verifier: &Verifier) -> Result<Option<Checkpoint>, Fault> {
    let dir = path.parent().expect("a kept file is in a folder");
    trust::check(crate::cache::existing_root(dir)).map_err(|why| {
        let head = format!("the folder `{}`: {}", dir.display(), why.message());
        fault(match why {
            Untrusted::Breach { help, .. } => format!("{head}\n  help: {help}"),
            Untrusted::Unreadable(_) => head,
        })
    })?;
    match fs::read(path) {
        Ok(note) => Checkpoint::open(&note, verifier.name(), std::slice::from_ref(verifier))
            .map(Some)
            .map_err(|err| {
                fault(format!(
                    "the checkpoint `{}` kept for the registry `{url}` does not open with its \
                     verifier key: {err}. If the registry changed its key, delete that file.",
                    path.display()
                ))
            }),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(fault(format!("cannot read `{}`: {err}", path.display()))),
    }
}

/// `note` written to `path` through a temporary file and a rename, so a reader never sees half.
fn keep_checkpoint(path: &Path, note: &[u8]) -> Result<(), Fault> {
    let failed = |err: std::io::Error| {
        fault(format!(
            "cannot keep the checkpoint at `{}`: {err}",
            path.display()
        ))
    };
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(failed)?;
    }
    let partial = path.with_extension("checkpoint.partial");
    fs::write(&partial, note).map_err(failed)?;
    fs::rename(&partial, path).map_err(failed)
}
