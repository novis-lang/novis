//! The client's checks of a registry's transparency log
//! (`rule:packaging/the-registry-keeps-an-append-only-log`): the checkpoint this computer saw of
//! each registry, the consistency proof a newer one needs, and the inclusion proof of every
//! archive the client fetches.
//!
//! **The log is under each file host's `log/`**, in C2SP's layout: `log/checkpoint` and
//! `log/tile/...`, as `nvs registry log append` writes a log directory. A checkpoint is opened with
//! the registry's pinned verifier key, the `key` of its `[registries]` line, and its origin is
//! that key's name. A registry with no key has no log the client can check, so no archive is read
//! from it.
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
//! Cost: one checkpoint and one consistency proof per registry per command, and one entry bundle
//! and one inclusion proof per archive. Each read is a tile of at most 8 KiB of hashes or one
//! bundle of at most 256 entries, held until its proof is checked. A kept checkpoint is a few
//! hundred bytes per registry. Nothing here runs on the request path.

use std::fs;
use std::path::Path;

use nvs_config::trust::{self, Untrusted};

use super::entry::Entry;
use super::index::{Fault, Registry, Release, fault};
use super::log::{self, Fetched};
use super::manifest::Name;
use super::note::{Checkpoint, Verifier};

/// The folder of a file host that serves the log.
const LOG: &str = "log";

/// The most bytes a checkpoint may have, its signatures included.
const MAX_CHECKPOINT_BYTES: u64 = 64 * 1024;

/// The most bytes a tile or an entry bundle may have: 256 entries of the largest size.
const MAX_TILE_BYTES: u64 = 256 * (2 + log::MAX_ENTRY_BYTES as u64);

/// The verifier key of the registry at `url`, read from `key`, its `[registries]` line's.
///
/// # Errors
///
/// No key, and a key that is not a verifier key.
pub(crate) fn verifier(url: &str, key: Option<&str>) -> Result<Verifier, Fault> {
    let Some(key) = key else {
        return Err(fault(format!(
            "the registry `{url}` has no verifier key, so `nvs pkg` cannot check that a package \
             is in its log. Add `key = \"<verifier key>\"` to its line under `[registries]` in \
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
}

/// Why a file host's log was not used.
enum Skip {
    /// The host did not answer, or served something that does not check. The next host is tried.
    Host(String),
    /// The registry signed a log the kept checkpoint is not a prefix of.
    Fork(Fault),
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
                    return Ok(Self { checkpoint, host });
                }
                Err(Skip::Host(why)) => {
                    failures.push(format!("{}: {why}", registry.host_name(host)));
                }
                Err(Skip::Fork(fork)) => return Err(fork),
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
    Err(Skip::Fork(fault(format!(
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
