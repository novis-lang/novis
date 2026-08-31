//! The rotating file target: what a log written to a path costs on disk, which
//! [ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
//! § 10 requires to be a finite number.
//!
//! # The bound is a product, and it is the only number that matters
//!
//! A [`LogFile`] holds two: how large one file may get, and how many rotations
//! are retained beside it. Everything the target can ever occupy is
//! `(keep + 1) * max_bytes`, and that product is what an operator sizing a
//! partition needs — the two factors on their own are a tuning question and
//! this module has no opinion about them beyond the defaults below.
//!
//! **The bound is enforced before the write, never after.** A record is a line
//! ([ADR 0092](../../../docs/adr/0092-one-diagnostic-record-three-renderings.md)
//! § 3's JSON Lines), so rotating mid-record would produce two files each
//! holding half of one, and a log reader would be right to reject both. A write
//! that would take the current file past `max_bytes` rotates first and lands
//! whole in the new one, which means a file may exceed `max_bytes` by nothing
//! and fall short of it by at most one record.
//!
//! # It is opened lazily and closed across a rotation
//!
//! The handle is opened on the first write rather than at construction, so a
//! configured target that is never written costs no descriptor. It is dropped
//! again before the renames, because on Windows renaming a file that is still
//! open fails outright — and a rotation that fails is a target that grows
//! without a bound, which is the one thing this module exists to prevent.
//!
//! # No capability check
//!
//! [ADR 0118](../../../docs/adr/0118-every-privileged-operation-answers-one-question.md)'s
//! doors stand in front of what a *program* asks for. This path is the engine
//! writing its own diagnostics to a path an operator configured, with no
//! program-supplied name anywhere in it, so there is no question for a door to
//! answer.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// How large one file may get before it rotates, absent a caller's own bound.
///
/// Eight mebibytes is roughly a day of a busy floor at
/// [`crate::floor::COALESCING_WINDOW`]'s one line per second per record, so the
/// common case is that the retained rotations are never reached at all.
pub const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// How many rotations are retained beside the live file, absent a caller's own
/// bound — so the default target occupies at most `(4 + 1) * 8 MiB`.
pub const KEEP: usize = 4;

/// A log file with ADR 0106 § 10's two bounds on it.
#[derive(Debug)]
pub struct LogFile {
    /// The live file's path. Rotation `n` is this with `.n` appended.
    path: PathBuf,
    /// The live file, once something has been written to it.
    file: Option<File>,
    /// How many bytes the live file holds — its length when it was opened,
    /// plus everything written since.
    written: u64,
    /// The size at which the live file rotates.
    max_bytes: u64,
    /// How many rotations are retained beside it.
    keep: usize,
}

impl LogFile {
    /// A target at `path` under [`MAX_BYTES`] and [`KEEP`].
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self::with_bounds(path, MAX_BYTES, KEEP)
    }

    /// A target at `path` under a caller's own bounds.
    ///
    /// The bounds are arguments rather than constants read from inside for the
    /// usual reason a limit is: a bound nobody can vary is a bound nobody can
    /// test, and asserting rotation against [`MAX_BYTES`] would mean writing
    /// eight mebibytes to learn one thing.
    #[must_use]
    pub fn with_bounds(path: PathBuf, max_bytes: u64, keep: usize) -> Self {
        Self {
            path,
            file: None,
            written: 0,
            max_bytes,
            keep,
        }
    }

    /// Appends `bytes`, rotating first if they would not fit.
    ///
    /// # Errors
    ///
    /// Whatever opening, rotating or writing the file returns. A rotation that
    /// fails is reported rather than swallowed: the alternative is a target
    /// that quietly stops obeying its bound.
    pub fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        let len = bytes.len() as u64;
        if self.written > 0 && self.written + len > self.max_bytes {
            self.rotate()?;
        }
        if self.file.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            self.written = file.metadata().map_or(0, |meta| meta.len());
            self.file = Some(file);
        }
        let file = self
            .file
            .as_mut()
            .expect("the branch above opened it or returned");
        file.write_all(bytes)?;
        self.written += len;
        Ok(())
    }

    /// The path of rotation `n`, where 0 is the live file.
    fn rotation(&self, n: usize) -> PathBuf {
        if n == 0 {
            return self.path.clone();
        }
        let mut name = self.path.clone().into_os_string();
        name.push(format!(".{n}"));
        PathBuf::from(name)
    }

    /// Ages every retained rotation by one and starts a new live file.
    ///
    /// The oldest is removed *first*, so the retention bound holds at every
    /// moment of the rotation rather than only once it has finished — a crash
    /// in the middle leaves the target inside its bound, never one file over.
    /// With `keep` at zero the live file is simply removed, which is the same
    /// rule with nothing to retain.
    fn rotate(&mut self) -> io::Result<()> {
        self.file = None;
        remove(&self.rotation(self.keep))?;
        for n in (1..self.keep).rev() {
            let from = self.rotation(n);
            if from.exists() {
                fs::rename(from, self.rotation(n + 1))?;
            }
        }
        if self.keep > 0 && self.path.exists() {
            fs::rename(&self.path, self.rotation(1))?;
        }
        self.written = 0;
        Ok(())
    }
}

/// Removes `path` if it is there, treating an absent one as done.
fn remove(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(why) if why.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}
