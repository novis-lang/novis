//! The mailbox of a `directory` block: a folder of folders on disk, read and changed the way
//! [`super::session`] reads and changes a server's (`rule:core-classes/imap`).
//!
//! Each subdirectory of the block's `directory` is a folder, and each `<uid>.eml` file in it a
//! message, with `<uid>` written in decimal and without leading zeros. The flags of a message are
//! the lines of `<uid>.flags` beside it, and a message without that file has none. `move` renames
//! both files into the other folder under the next free UID there, and `delete` removes them. The
//! `UIDVALIDITY` of every folder is 1.
//!
//! **Every path is judged before it is joined.** A folder is one directory name: no separator, no
//! `.` or `..`, no `:`, no control character, and no dot or space at its end, which Windows
//! would remove. A UID is a number. So every path this module
//! opens is `<directory>/<folder>/<uid>.eml` or `.flags`, and none leaves the directory.
//! [`super`] asks `fs.read` for the directory when it opens the mailbox, and `fs.write` before each
//! change.
//!
//! **What differs from a server.** `since` and `before` read each message's `Date` header, since
//! a file has no arrival date that a test can set, and a message without one matches neither.
//! `from` and `subject` match the decoded header text, ignoring case. A UID freed by `move` or
//! `delete` is the next free one when it was the highest in its folder. A test that keeps UIDs
//! across a `move` keeps them in the folder that still has them.
//!
//! What it spends: nothing between calls. `search` with a header filter reads and parses one
//! message at a time, each within the memory the request has left. The files are read with
//! blocking calls, because a directory block serves tests and local work and never a server's
//! requests in production.

use std::path::{Path, PathBuf};

use nvs_runtime::{Fault, ThrownClass};

use super::session::{Filter, MAX_FOLDERS};

/// The `UIDVALIDITY` of every folder of a directory block.
pub(super) const VALIDITY: u32 = 1;

/// The largest `.flags` file read: 64 KiB.
const FLAGS_CEILING: u64 = 64 << 10;

/// An `IOError` with `message`.
fn io(message: String) -> Fault {
    Fault::thrown_as(ThrownClass::Io, message)
}

/// The mailbox of `[imap.<block>] directory = "<root>"`.
pub(super) struct Directory {
    /// The block's `directory`, absolute.
    root: PathBuf,
    /// The block's name, `<name>` in `[imap.<name>]`.
    block: String,
}

impl Directory {
    pub(super) fn new(root: PathBuf, block: String) -> Self {
        Self { root, block }
    }

    /// The block's `directory`, which `fs.write` is asked for before each change.
    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    /// Every subdirectory whose name is a folder name, sorted by name.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the directory cannot be read.
    pub(super) fn folders(&self, member: &str) -> Result<Vec<String>, Fault> {
        let entries = std::fs::read_dir(&self.root).map_err(|why| {
            io(format!(
                "{member}: the directory {} of `[imap.{}]` cannot be read: {why}",
                self.root.display(),
                self.block
            ))
        })?;
        let mut names: Vec<String> = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| is_folder_name(name))
            .collect();
        names.sort();
        names.truncate(MAX_FOLDERS);
        Ok(names)
    }

    /// Checks that `folder` exists, and returns its `UIDVALIDITY`.
    ///
    /// # Errors
    ///
    /// A catchable `RuntimeError` for a name that is not one directory name, and an `IOError`
    /// when the folder does not exist.
    pub(super) fn select(&self, folder: &str, member: &str) -> Result<u32, Fault> {
        self.folder_path(folder, member)?;
        Ok(VALIDITY)
    }

    /// The UIDs in `folder` that match `filter`, the oldest first. `ceiling` is how many bytes
    /// one message may take when a header filter reads it.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the folder cannot be read, or a message a header filter reads
    /// is over `ceiling`.
    pub(super) fn search(
        &self,
        folder: &str,
        filter: &Filter,
        ceiling: u64,
        member: &str,
    ) -> Result<Vec<u32>, Fault> {
        let dir = self.folder_path(folder, member)?;
        let reads_headers = filter.since.is_some()
            || filter.before.is_some()
            || filter.from.is_some()
            || filter.subject.is_some();
        let mut found = Vec::new();
        for uid in uids(&dir, member)? {
            if filter.limit.is_some_and(|limit| found.len() >= limit) {
                break;
            }
            if filter.unseen && has(&flags(&dir, uid, member)?, "\\Seen") {
                continue;
            }
            if reads_headers {
                let raw = read_message(&dir, uid, ceiling, member)?;
                if !matches(&crate::mail::receive::parse(&raw), filter) {
                    continue;
                }
            }
            found.push(uid);
        }
        Ok(found)
    }

    /// The raw bytes of message `uid` in `folder`. `ceiling` is how many bytes the caller can
    /// still hold.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when there is no such message, and when it is larger than `ceiling`,
    /// before it is read.
    pub(super) fn fetch(
        &self,
        folder: &str,
        uid: u32,
        ceiling: u64,
        member: &str,
    ) -> Result<Vec<u8>, Fault> {
        let dir = self.folder_path(folder, member)?;
        read_message(&dir, uid, ceiling, member)
    }

    /// Adds `flag` to message `uid` of `folder`, or removes it when `add` is false. `flag` is
    /// already checked by [`super::flag_of`].
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when there is no such message or its flags cannot be written.
    pub(super) fn store(
        &self,
        folder: &str,
        uid: u32,
        flag: &str,
        add: bool,
        member: &str,
    ) -> Result<(), Fault> {
        let dir = self.folder_path(folder, member)?;
        message_path(&dir, uid, member)?;
        let mut kept = flags(&dir, uid, member)?;
        let present = has(&kept, flag);
        if add && !present {
            kept.push(flag.to_owned());
        } else if !add && present {
            kept.retain(|each| !each.eq_ignore_ascii_case(flag));
        } else {
            return Ok(());
        }
        let path = dir.join(format!("{uid}.flags"));
        let written = if kept.is_empty() {
            match std::fs::remove_file(&path) {
                Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(()),
                other => other,
            }
        } else {
            let mut text = kept.join("\n");
            text.push('\n');
            if text.len() as u64 > FLAGS_CEILING {
                return Err(io(format!(
                    "{member}: the flags of the message with the UID {uid} would be {} bytes, and \
                     a flags file may have {FLAGS_CEILING}. Nothing was written.",
                    text.len()
                )));
            }
            std::fs::write(&path, text)
        };
        written.map_err(|why| {
            io(format!(
                "{member}: the flags of the message with the UID {uid} cannot be written to {}: \
                 {why}",
                path.display()
            ))
        })
    }

    /// Moves message `uid` of `folder`, with its flags, to `target` under the next free UID
    /// there.
    ///
    /// # Errors
    ///
    /// A catchable `RuntimeError` for a target that is not one directory name, and an `IOError`
    /// when there is no such message or target folder, or a file cannot be renamed.
    pub(super) fn move_to(
        &self,
        folder: &str,
        uid: u32,
        target: &str,
        member: &str,
    ) -> Result<(), Fault> {
        let dir = self.folder_path(folder, member)?;
        let source = message_path(&dir, uid, member)?;
        let to = self.folder_path(target, member)?;
        let next = uids(&to, member)?
            .last()
            .map_or(Some(1), |last| last.checked_add(1))
            .ok_or_else(|| io(format!("{member}: the folder `{target}` has no free UID")))?;
        let renamed = |from: &Path, into: PathBuf| {
            std::fs::rename(from, &into).map_err(|why| {
                io(format!(
                    "{member}: {} cannot be moved to {}: {why}",
                    from.display(),
                    into.display()
                ))
            })
        };
        let flags = dir.join(format!("{uid}.flags"));
        if flags.is_file() {
            renamed(&flags, to.join(format!("{next}.flags")))?;
        }
        renamed(&source, to.join(format!("{next}.eml")))
    }

    /// Removes message `uid` of `folder` and its flags.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when there is no such message or a file cannot be removed.
    pub(super) fn delete(&self, folder: &str, uid: u32, member: &str) -> Result<(), Fault> {
        let dir = self.folder_path(folder, member)?;
        let source = message_path(&dir, uid, member)?;
        let removed = |path: &Path| {
            std::fs::remove_file(path).map_err(|why| {
                io(format!(
                    "{member}: {} cannot be removed: {why}",
                    path.display()
                ))
            })
        };
        let flags = dir.join(format!("{uid}.flags"));
        if flags.is_file() {
            removed(&flags)?;
        }
        removed(&source)
    }

    /// The directory of `folder`, once its name is one directory name and it exists.
    fn folder_path(&self, folder: &str, member: &str) -> Result<PathBuf, Fault> {
        if !is_folder_name(folder) {
            return Err(Fault::thrown(format!(
                "{member}: `{folder}` is not a folder of `[imap.{}]`. A folder of a directory \
                 block is one directory name, without `/`, `\\`, `:` or `..`. It does not end \
                 with a dot or a space.",
                self.block
            )));
        }
        let path = self.root.join(folder);
        if std::fs::symlink_metadata(&path).is_ok_and(|found| found.is_dir()) {
            return Ok(path);
        }
        Err(io(format!(
            "{member}: `[imap.{}]` has no folder `{folder}`: there is no directory {}",
            self.block,
            path.display()
        )))
    }
}

/// Whether `name` is one directory name: not empty, not `.` or `..`, without a separator, a `:`
/// or a control character, and not ending with a dot or a space. Windows removes a trailing dot
/// or space when it opens a path, so `INBOX.` would open `INBOX` there and fail on Linux.
fn is_folder_name(name: &str) -> bool {
    !name.is_empty()
        && !name.ends_with(['.', ' '])
        && !name
            .chars()
            .any(|each| matches!(each, '/' | '\\' | ':') || each.is_control())
}

/// The UID of every `<uid>.eml` file in `dir`, sorted.
fn uids(dir: &Path, member: &str) -> Result<Vec<u32>, Fault> {
    let entries = std::fs::read_dir(dir)
        .map_err(|why| io(format!("{member}: {} cannot be read: {why}", dir.display())))?;
    let mut found: Vec<u32> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| uid_of_file(&entry.file_name().to_string_lossy()))
        .collect();
    found.sort_unstable();
    Ok(found)
}

/// The UID a file named `<uid>.eml` has, when the number is 1 or more and has no leading zero.
fn uid_of_file(name: &str) -> Option<u32> {
    let digits = name.strip_suffix(".eml")?;
    let uid: u32 = digits.parse().ok()?;
    (uid != 0 && uid.to_string() == digits).then_some(uid)
}

/// The path of message `uid` in `dir`, once the file exists.
fn message_path(dir: &Path, uid: u32, member: &str) -> Result<PathBuf, Fault> {
    let path = dir.join(format!("{uid}.eml"));
    if path.is_file() {
        return Ok(path);
    }
    Err(io(format!(
        "{member}: the folder has no message with the UID {uid}"
    )))
}

/// The bytes of message `uid` in `dir`, when its size is at most `ceiling`.
fn read_message(dir: &Path, uid: u32, ceiling: u64, member: &str) -> Result<Vec<u8>, Fault> {
    let path = message_path(dir, uid, member)?;
    let size = std::fs::metadata(&path)
        .map_err(|why| {
            io(format!(
                "{member}: {} cannot be read: {why}",
                path.display()
            ))
        })?
        .len();
    if size > ceiling {
        return Err(io(format!(
            "{member}: the message with the UID {uid} is {size} bytes, and the request can hold \
             {ceiling} more. Its body was not read."
        )));
    }
    std::fs::read(&path).map_err(|why| {
        io(format!(
            "{member}: {} cannot be read: {why}",
            path.display()
        ))
    })
}

/// The flags of message `uid` in `dir`: the lines of `<uid>.flags`, or none without the file.
fn flags(dir: &Path, uid: u32, member: &str) -> Result<Vec<String>, Fault> {
    let path = dir.join(format!("{uid}.flags"));
    let size = match std::fs::metadata(&path) {
        Ok(found) => found.len(),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => {
            return Err(io(format!(
                "{member}: {} cannot be read: {why}",
                path.display()
            )));
        }
    };
    if size > FLAGS_CEILING {
        return Err(io(format!(
            "{member}: {} is {size} bytes, and a flags file may take {FLAGS_CEILING}",
            path.display()
        )));
    }
    let text = std::fs::read_to_string(&path).map_err(|why| {
        io(format!(
            "{member}: {} cannot be read: {why}",
            path.display()
        ))
    })?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Whether `flags` has `flag`, ignoring case as IMAP does.
fn has(flags: &[String], flag: &str) -> bool {
    flags.iter().any(|each| each.eq_ignore_ascii_case(flag))
}

/// Whether the header filters of `filter` match `parsed`.
fn matches(parsed: &crate::mail::receive::Parsed, filter: &Filter) -> bool {
    let day = parsed.date.as_ref().map(|(date, _)| {
        (
            i32::from(date.year),
            u32::from(date.month),
            u32::from(date.day),
        )
    });
    if let Some(since) = filter.since
        && !day.is_some_and(|day| day >= since)
    {
        return false;
    }
    if let Some(before) = filter.before
        && !day.is_some_and(|day| day < before)
    {
        return false;
    }
    let contains = |text: &str, part: &str| text.to_lowercase().contains(&part.to_lowercase());
    if let Some(from) = &filter.from {
        let written = parsed
            .from
            .as_ref()
            .map(|sender| format!("{} <{}>", sender.name, sender.address))
            .unwrap_or_default();
        if !contains(&written, from) {
            return false;
        }
    }
    if let Some(subject) = &filter.subject
        && !contains(&parsed.subject, subject)
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests;
