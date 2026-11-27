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
//! change. Every file is then reached through a door of [`nvs_runtime::capability`]
//! (`rule:security/capability-check-at-the-door`), so each read asks `fs.read` and each change
//! `fs.write` for its own path, and a link inside the directory cannot lead out of it.
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
//! requests in production. A [`Directory`] is copied out of its session for each call, so a door
//! can read the request's `Ctx`: the copy is the directory's path and the block's name.

use std::path::{Path, PathBuf};

use nvs_runtime::capability::{self, Kind};
use nvs_runtime::{Ctx, Fault, ThrownClass};

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
#[derive(Clone)]
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
    pub(super) fn folders(&self, ctx: &Ctx, member: &str) -> Result<Vec<String>, Fault> {
        let mut names: Vec<String> = capability::read_dir(ctx, &self.root, member)?
            .into_iter()
            .filter(|entry| entry.kind == Kind::Dir)
            .filter_map(|entry| entry.name.into_string().ok())
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
    pub(super) fn select(&self, ctx: &Ctx, folder: &str, member: &str) -> Result<u32, Fault> {
        self.folder_path(ctx, folder, member)?;
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
        ctx: &Ctx,
        folder: &str,
        filter: &Filter,
        ceiling: u64,
        member: &str,
    ) -> Result<Vec<u32>, Fault> {
        let dir = self.folder_path(ctx, folder, member)?;
        let reads_headers = filter.since.is_some()
            || filter.before.is_some()
            || filter.from.is_some()
            || filter.subject.is_some();
        let mut found = Vec::new();
        for uid in uids(ctx, &dir, member)? {
            if filter.limit.is_some_and(|limit| found.len() >= limit) {
                break;
            }
            if filter.unseen && has(&flags(ctx, &dir, uid, member)?, "\\Seen") {
                continue;
            }
            if reads_headers {
                let raw = read_message(ctx, &dir, uid, ceiling, member)?;
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
        ctx: &Ctx,
        folder: &str,
        uid: u32,
        ceiling: u64,
        member: &str,
    ) -> Result<Vec<u8>, Fault> {
        let dir = self.folder_path(ctx, folder, member)?;
        read_message(ctx, &dir, uid, ceiling, member)
    }

    /// Adds `flag` to message `uid` of `folder`, or removes it when `add` is false. `flag` is
    /// already checked by [`super::flag_of`].
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when there is no such message or its flags cannot be written.
    pub(super) fn store(
        &self,
        ctx: &Ctx,
        folder: &str,
        uid: u32,
        flag: &str,
        add: bool,
        member: &str,
    ) -> Result<(), Fault> {
        let dir = self.folder_path(ctx, folder, member)?;
        message_path(ctx, &dir, uid, member)?;
        let mut kept = flags(ctx, &dir, uid, member)?;
        let present = has(&kept, flag);
        if add && !present {
            kept.push(flag.to_owned());
        } else if !add && present {
            kept.retain(|each| !each.eq_ignore_ascii_case(flag));
        } else {
            return Ok(());
        }
        let path = dir.join(format!("{uid}.flags"));
        if kept.is_empty() {
            if capability::metadata_if_present(ctx, &path, member)?.is_some() {
                capability::remove_file(ctx, &path, member)?;
            }
            return Ok(());
        }
        let mut text = kept.join("\n");
        text.push('\n');
        if text.len() as u64 > FLAGS_CEILING {
            return Err(io(format!(
                "{member}: the flags of the message with the UID {uid} would be {} bytes, and \
                 a flags file may have {FLAGS_CEILING}. Nothing was written.",
                text.len()
            )));
        }
        capability::write(ctx, &path, text.as_bytes(), member)
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
        ctx: &Ctx,
        folder: &str,
        uid: u32,
        target: &str,
        member: &str,
    ) -> Result<(), Fault> {
        let dir = self.folder_path(ctx, folder, member)?;
        let source = message_path(ctx, &dir, uid, member)?;
        let to = self.folder_path(ctx, target, member)?;
        let next = uids(ctx, &to, member)?
            .last()
            .map_or(Some(1), |last| last.checked_add(1))
            .ok_or_else(|| io(format!("{member}: the folder `{target}` has no free UID")))?;
        let flags = dir.join(format!("{uid}.flags"));
        if is_file(ctx, &flags, member)? {
            capability::rename(ctx, &flags, &to.join(format!("{next}.flags")), member)?;
        }
        capability::rename(ctx, &source, &to.join(format!("{next}.eml")), member)
    }

    /// Removes message `uid` of `folder` and its flags.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when there is no such message or a file cannot be removed.
    pub(super) fn delete(
        &self,
        ctx: &Ctx,
        folder: &str,
        uid: u32,
        member: &str,
    ) -> Result<(), Fault> {
        let dir = self.folder_path(ctx, folder, member)?;
        let source = message_path(ctx, &dir, uid, member)?;
        let flags = dir.join(format!("{uid}.flags"));
        if is_file(ctx, &flags, member)? {
            capability::remove_file(ctx, &flags, member)?;
        }
        capability::remove_file(ctx, &source, member)
    }

    /// The directory of `folder`, once its name is one directory name and it exists.
    fn folder_path(&self, ctx: &Ctx, folder: &str, member: &str) -> Result<PathBuf, Fault> {
        if !is_folder_name(folder) {
            return Err(Fault::thrown(format!(
                "{member}: `{folder}` is not a folder of `[imap.{}]`. A folder of a directory \
                 block is one directory name, without `/`, `\\`, `:` or `..`. It does not end \
                 with a dot or a space.",
                self.block
            )));
        }
        let path = self.root.join(folder);
        if capability::metadata_if_present(ctx, &path, member)?.is_some_and(|found| found.is_dir()) {
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

/// Whether `path` is a regular file.
fn is_file(ctx: &Ctx, path: &Path, member: &str) -> Result<bool, Fault> {
    Ok(capability::metadata_if_present(ctx, path, member)?.is_some_and(|found| found.is_file()))
}

/// The UID of every `<uid>.eml` file in `dir`, sorted.
fn uids(ctx: &Ctx, dir: &Path, member: &str) -> Result<Vec<u32>, Fault> {
    let mut found: Vec<u32> = capability::read_dir(ctx, dir, member)?
        .into_iter()
        .filter(|entry| entry.kind == Kind::File)
        .filter_map(|entry| uid_of_file(&entry.name.to_string_lossy()))
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
fn message_path(ctx: &Ctx, dir: &Path, uid: u32, member: &str) -> Result<PathBuf, Fault> {
    let path = dir.join(format!("{uid}.eml"));
    if is_file(ctx, &path, member)? {
        return Ok(path);
    }
    Err(io(format!(
        "{member}: the folder has no message with the UID {uid}"
    )))
}

/// The bytes of message `uid` in `dir`, when its size is at most `ceiling`.
fn read_message(
    ctx: &Ctx,
    dir: &Path,
    uid: u32,
    ceiling: u64,
    member: &str,
) -> Result<Vec<u8>, Fault> {
    let path = message_path(ctx, dir, uid, member)?;
    let size = capability::metadata(ctx, &path, member)?.len();
    if size > ceiling {
        return Err(io(format!(
            "{member}: the message with the UID {uid} is {size} bytes, and the request can hold \
             {ceiling} more. Its body was not read."
        )));
    }
    let mut file = capability::open_read(ctx, &path, member)?;
    crate::byte_source::read_bounded(ctx, &mut file, member, &path)
}

/// The flags of message `uid` in `dir`: the lines of `<uid>.flags`, or none without the file.
fn flags(ctx: &Ctx, dir: &Path, uid: u32, member: &str) -> Result<Vec<String>, Fault> {
    let path = dir.join(format!("{uid}.flags"));
    let Some(found) = capability::metadata_if_present(ctx, &path, member)? else {
        return Ok(Vec::new());
    };
    let size = found.len();
    if size > FLAGS_CEILING {
        return Err(io(format!(
            "{member}: {} is {size} bytes, and a flags file may take {FLAGS_CEILING}",
            path.display()
        )));
    }
    let mut file = capability::open_read(ctx, &path, member)?;
    let bytes = crate::byte_source::read_bounded(ctx, &mut file, member, &path)?;
    let text = String::from_utf8_lossy(&bytes);
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
