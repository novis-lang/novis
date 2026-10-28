//! A root git dependency: one exact commit of a repository, written out and packed as a
//! registry archive is (`rule:packaging/a-git-dependency-is-root-only`).
//!
//! **Only the commit's tree is read**, never its history, and only the paths the package's archive
//! can carry are written: `package.toml`, the files at the root, everything under `src/`, and the
//! `[package] files` list (`rule:packaging/a-package-carries-only-its-source-and-what-it-lists`).
//! [`pack::file_set`] then reads that directory exactly as `nvs pkg pack` reads a package, so a git
//! package's digest is the digest of the same bytes a registry would serve. A symlink or a
//! submodule at one of those paths is refused, as `nvs pkg pack` refuses a symlink, and every
//! other path of the repository is never written.
//!
//! **A `file:` remote is opened where it is**, with gix's isolated options, so no configuration
//! file, hook or helper program of that repository or of this user is read or run. It reaches only
//! the user's own disk. An `https:` remote is fetched at the commit alone, through the connection
//! `nvs` resolves and checks; this version of `nvs` does not fetch one yet.
//!
//! **Two limits bound the clone** ([`Limits`]): the bytes of the objects read, and the time since
//! the clone started. Both are checked before each object is read.
//!
//! Cost: the archive in memory, at most [`pack::MAX_ARCHIVE_BYTES`], and the written files in a
//! scratch folder that is removed on every way out. Nothing here runs on the request path.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use gix::bstr::ByteSlice as _;
use nvs_diagnostics::SourceMap;

use super::index::{Fault, fault};
use super::manifest::{self, Manifest};
use super::pack;

/// The limits a clone runs inside.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Limits {
    /// The most bytes the clone may read, counted over every object.
    pub(crate) bytes: u64,
    /// The longest the clone may take.
    pub(crate) time: Duration,
}

/// The limits of `rule:packaging/the-registry-builds-the-archive-from-git`: 512 MiB, and five
/// minutes, which fetches the size limit at about 2 MiB/s.
pub(crate) const LIMITS: Limits = Limits {
    bytes: 512 * 1024 * 1024,
    time: Duration::from_secs(300),
};

/// A cloned package: its manifest, its archive and the archive's digest.
#[derive(Debug)]
pub(crate) struct Cloned {
    pub(crate) manifest: Manifest,
    pub(crate) archive: Vec<u8>,
    pub(crate) digest: blake3::Hash,
}

/// The package at `url`'s commit `rev`, written out under a fresh folder in `scratch` and packed.
/// A relative `file:` URL resolves against `base`.
///
/// # Errors
///
/// A URL that is neither `file:` nor `https:`, a repository that cannot be opened, a commit it
/// does not have, a clone over `limits`, a symlink or a submodule where the archive reads, a
/// package that does not pack, and a scratch folder that cannot be written.
pub(crate) fn clone(
    url: &str,
    rev: &str,
    base: &Path,
    scratch: &Path,
    limits: Limits,
) -> Result<Cloned, Fault> {
    let started = Instant::now();
    let id = gix::ObjectId::from_hex(rev.as_bytes())
        .map_err(|err| fault(format!("`{rev}` is not a commit hash: {err}")))?;
    let Some(path) = url.strip_prefix("file:") else {
        if url.starts_with("https:") {
            return Err(fault(format!(
                "`{url}` is an HTTPS repository, and this version of `nvs` cannot fetch one yet"
            )));
        }
        return Err(fault(format!("`{url}` is not a `file:` or `https:` URL")));
    };
    let repo = gix::open_opts(base.join(path), gix::open::Options::isolated())
        .map_err(|err| fault(format!("cannot open the repository `{url}`: {err}")))?;
    let folder = Folder::new(scratch)?;
    let mut walk = Walk {
        repo: &repo,
        url,
        limits,
        started,
        read: 0,
    };
    let manifest = walk.write(id, &folder.0)?;
    let entries = pack::file_set(&folder.0, &manifest)
        .map_err(|refusal| fault(format!("`{url}` at `{rev}` does not pack: {refusal}")))?;
    let mut archive = Vec::new();
    let digest = pack::write(&entries, &mut archive)
        .map_err(|refusal| fault(format!("`{url}` at `{rev}` does not pack: {refusal}")))?;
    Ok(Cloned {
        manifest,
        archive,
        digest,
    })
}

/// A scratch folder of its own, removed when it is dropped.
struct Folder(PathBuf);

impl Folder {
    /// A new, empty folder under `scratch`, named after this process and a counter.
    fn new(scratch: &Path) -> Result<Self, Fault> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = scratch.join(format!("nvs-git-{}-{n}", std::process::id()));
        // A folder left by an earlier process with the same id is not this clone's.
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path)
            .map_err(|err| fault(format!("cannot make `{}`: {err}", path.display())))?;
        Ok(Self(path))
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        // A folder that cannot be removed is left for the user's temporary-file cleanup.
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// One commit's tree being written out, with the bytes read so far.
struct Walk<'a> {
    repo: &'a gix::Repository,
    url: &'a str,
    limits: Limits,
    started: Instant,
    read: u64,
}

impl Walk<'_> {
    /// The commit `id`'s `package.toml`, read, and every path the archive can carry written
    /// under `dir`.
    fn write(&mut self, id: gix::ObjectId, dir: &Path) -> Result<Manifest, Fault> {
        let url = self.url;
        let commit = self
            .repo
            .find_commit(id)
            .map_err(|err| fault(format!("`{url}` has no commit `{id}`: {err}")))?;
        let tree = commit
            .tree_id()
            .map_err(|err| fault(format!("cannot read the commit `{id}` of `{url}`: {err}")))?
            .detach();
        let root = self.entries(tree)?;
        let Some(toml) = root
            .iter()
            .find(|(name, _, _)| name == nvs_config::grants::FILE)
        else {
            return Err(fault(format!(
                "`{url}` has no `{}` at the commit `{id}`",
                nvs_config::grants::FILE
            )));
        };
        let text = self.blob(toml, nvs_config::grants::FILE)?;
        let text = String::from_utf8(text).map_err(|_| {
            fault(format!(
                "the `{}` of `{url}` is not UTF-8",
                nvs_config::grants::FILE
            ))
        })?;
        let path = dir.join(nvs_config::grants::FILE);
        let manifest =
            manifest::parse(&mut SourceMap::new(), &path, &text).map_err(|diagnostic| {
                fault(format!(
                    "the `{}` of `{url}` does not read: {}",
                    nvs_config::grants::FILE,
                    diagnostic.message
                ))
            })?;
        for entry in &root {
            self.place(entry, "", dir, &manifest.files)?;
        }
        Ok(manifest)
    }

    /// The entries of the tree `id`: each name, its mode and its object.
    fn entries(
        &mut self,
        id: gix::ObjectId,
    ) -> Result<Vec<(String, gix::object::tree::EntryMode, gix::ObjectId)>, Fault> {
        let data = self.read(id, "a folder")?;
        let tree = gix::objs::TreeRef::from_bytes(&data, gix::hash::Kind::Sha1)
            .map_err(|err| fault(format!("cannot read a folder of `{}`: {err}", self.url)))?;
        tree.entries
            .iter()
            .map(|entry| {
                let name = entry.filename.to_str().map_err(|_| {
                    fault(format!("`{}` has a file name that is not UTF-8", self.url))
                })?;
                Ok((name.to_owned(), entry.mode, entry.oid.to_owned()))
            })
            .collect()
    }

    /// `entry`, at `prefix`, written under `dir` when the archive can carry it, and each entry
    /// of a folder the archive reads.
    fn place(
        &mut self,
        (name, mode, id): &(String, gix::object::tree::EntryMode, gix::ObjectId),
        prefix: &str,
        dir: &Path,
        files: &[String],
    ) -> Result<(), Fault> {
        let path = format!("{prefix}{name}");
        let in_src = path == "src" || path.starts_with("src/");
        let listed = files.contains(&path);
        let folder = format!("{path}/");
        let leads = files.iter().any(|file| file.starts_with(&folder));
        let at_root = prefix.is_empty();
        if mode.is_tree() {
            if !in_src && !leads {
                return Ok(());
            }
            check(&path, self.url)?;
            fs::create_dir_all(dir.join(&path))
                .map_err(|err| fault(format!("cannot write `{path}`: {err}")))?;
            for entry in self.entries(*id)? {
                self.place(&entry, &format!("{path}/"), dir, files)?;
            }
            return Ok(());
        }
        if !in_src && !listed && !at_root {
            return Ok(());
        }
        if mode.is_link() {
            return Err(fault(format!(
                "`{path}` in `{}` is a symlink. A package's files must be regular files.",
                self.url
            )));
        }
        if mode.is_commit() {
            return Err(fault(format!(
                "`{path}` in `{}` is a submodule. A package's files must be in its own repository.",
                self.url
            )));
        }
        check(&path, self.url)?;
        let bytes = self.blob(&(name.clone(), *mode, *id), &path)?;
        fs::write(dir.join(&path), bytes)
            .map_err(|err| fault(format!("cannot write `{path}`: {err}")))
    }

    /// The contents of the blob `entry`, at `path`.
    fn blob(
        &mut self,
        (_, _, id): &(String, gix::object::tree::EntryMode, gix::ObjectId),
        path: &str,
    ) -> Result<Vec<u8>, Fault> {
        self.read(*id, path)
    }

    /// The object `id`'s data, once the limits allow it. `what` names it in a fault.
    fn read(&mut self, id: gix::ObjectId, what: &str) -> Result<Vec<u8>, Fault> {
        let url = self.url;
        if self.started.elapsed() > self.limits.time {
            return Err(fault(format!(
                "the clone of `{url}` took longer than {} seconds, so it was stopped",
                self.limits.time.as_secs()
            )));
        }
        let header = self
            .repo
            .find_header(id)
            .map_err(|err| fault(format!("cannot read {what} of `{url}`: {err}")))?;
        self.read = self.read.saturating_add(header.size());
        if self.read > self.limits.bytes {
            return Err(fault(format!(
                "the clone of `{url}` is larger than {} bytes, so it was stopped",
                self.limits.bytes
            )));
        }
        let object = self
            .repo
            .find_object(id)
            .map_err(|err| fault(format!("cannot read {what} of `{url}`: {err}")))?;
        Ok(object.detach().data)
    }
}

/// A fault when `path`, from the repository at `url`, is not a path every platform can write.
fn check(path: &str, url: &str) -> Result<(), Fault> {
    pack::check_path(path).map_err(|refusal| fault(format!("`{url}`: {refusal}")))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A repository at `dir` with one commit of `files`, each a path and its contents. The
    /// commit's hash.
    pub(crate) fn repository(dir: &Path, files: &[(&str, &[u8])]) -> String {
        let repo = gix::init_bare(dir).expect("the test makes a repository");
        let mut editor = repo
            .edit_tree(gix::ObjectId::empty_tree(gix::hash::Kind::Sha1))
            .expect("the test edits a tree");
        for (path, contents) in files {
            let blob = repo.write_blob(contents).expect("the test writes a file");
            editor
                .upsert(*path, gix::object::tree::EntryKind::Blob, blob)
                .expect("the test adds a file");
        }
        let tree = editor.write().expect("the test writes the tree");
        let person = gix::actor::Signature {
            name: "Test".into(),
            email: "test@example.com".into(),
            time: gix::date::Time::new(0, 0),
        };
        let mut committer = gix::date::parse::TimeBuf::default();
        let mut author = gix::date::parse::TimeBuf::default();
        let commit = repo
            .new_commit_as(
                person.to_ref(&mut committer),
                person.to_ref(&mut author),
                "one",
                tree,
                std::iter::empty::<gix::ObjectId>(),
            )
            .expect("the test commits");
        commit.id.to_hex().to_string()
    }

    /// A package `Acme\Git` with a class, a license and one file the archive does not carry.
    pub(crate) fn package(dir: &Path, extra: &[(&str, &[u8])]) -> String {
        let mut files: Vec<(&str, &[u8])> = vec![
            (
                "package.toml",
                b"[package]\nname = 'Acme\\Git'\nversion = \"1.0.0\"\n",
            ),
            ("LICENSE", b"MIT\n"),
            (
                "src/Main.nvs",
                b"<?nvs\nnamespace Acme\\Git;\n\nfinal class Main\n{\n}\n",
            ),
            ("docs/notes.txt", b"not in the archive\n"),
        ];
        files.extend_from_slice(extra);
        repository(dir, &files)
    }

    /// The clone of a `file:` remote is the package at that commit, packed: only the archive's
    /// files, with the digest the same directory packs to.
    #[test]
    fn a_file_remote_is_cloned_at_its_exact_commit() {
        let scratch = nvs_repo::scratch("git-exact");
        let rev = package(&scratch.join("repo"), &[]);
        let url = format!("file:{}", scratch.join("repo").display());
        let cloned = clone(&url, &rev, &scratch, &scratch, LIMITS).expect("it clones");
        assert_eq!(
            cloned.manifest.name.as_ref().map(ToString::to_string),
            Some("Acme\\Git".to_owned())
        );
        let paths: Vec<String> = tar::Archive::new(cloned.archive.as_slice())
            .entries()
            .expect("the archive reads")
            .map(|entry| {
                entry
                    .expect("an entry reads")
                    .path()
                    .expect("a path reads")
                    .display()
                    .to_string()
            })
            .collect();
        assert_eq!(paths, ["LICENSE", "package.toml", "src/Main.nvs"]);
        assert_eq!(cloned.digest, blake3::hash(&cloned.archive));
        let again = clone(&url, &rev, &scratch, &scratch, LIMITS).expect("it clones again");
        assert_eq!(again.digest, cloned.digest);
    }

    /// A commit the repository does not have is a fault that names it.
    #[test]
    fn a_missing_commit_is_refused() {
        let scratch = nvs_repo::scratch("git-missing");
        package(&scratch.join("repo"), &[]);
        let url = format!("file:{}", scratch.join("repo").display());
        let rev = "1".repeat(40);
        let fault = clone(&url, &rev, &scratch, &scratch, LIMITS).expect_err("it is refused");
        assert!(fault.0.contains(&rev), "{fault}");
    }

    /// A repository over the size limit stops the clone.
    #[test]
    fn an_over_size_repository_is_stopped() {
        let scratch = nvs_repo::scratch("git-size");
        let big = vec![b'x'; 64 * 1024];
        let rev = package(&scratch.join("repo"), &[("src/Big.nvs", &big)]);
        let url = format!("file:{}", scratch.join("repo").display());
        let limits = Limits {
            bytes: 32 * 1024,
            ..LIMITS
        };
        let fault = clone(&url, &rev, &scratch, &scratch, limits).expect_err("it is stopped");
        assert!(fault.0.contains("larger than 32768 bytes"), "{fault}");
    }

    /// An `https:` remote is a fault until the checked connection fetches one.
    #[test]
    fn an_https_remote_is_not_fetched_yet() {
        let scratch = nvs_repo::scratch("git-https");
        let fault = clone(
            "https://git.example.com/acme/git",
            &"1".repeat(40),
            &scratch,
            &scratch,
            LIMITS,
        )
        .expect_err("it is refused");
        assert!(fault.0.contains("cannot fetch one yet"), "{fault}");
    }
}
