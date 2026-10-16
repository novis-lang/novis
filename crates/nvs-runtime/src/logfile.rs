//! The rotating file target: what a log written to a path costs on disk, which
//! `rule:http-server/the-floor-cannot-fill-the-disk`
//! requires to be a finite number.
//!
//! # The bound is a product, and it is the only number that matters
//!
//! A [`LogFile`] has two: how large one file may get (`[log] max_size`), and
//! how many rotations are kept beside it (`[log] keep`). Everything the target
//! can occupy is `(keep + 1) * max_size`, and that product is what an operator
//! sizing a partition needs. The defaults are [`nvs_config::log::MAX_SIZE`] and
//! [`nvs_config::log::KEEP`], beside the grammar that reads the two keys.
//!
//! **The bound is enforced before the write, never after.** A record is a line
//! (`rule:errors/renderings`'s JSON Lines), so rotating mid-record would produce two files each
//! holding half of one, and a log reader would be right to reject both. A write
//! that would take the current file past `max_size` rotates first and lands
//! whole in the new one, so a file never passes `max_size` unless one record is
//! larger than it.
//!
//! # One file per path in the process, behind one lock
//!
//! Every context and every `LogWriter` that names the same path gets a
//! handle to the same [`Rotating`] state, found in [`OPEN`]. One lock orders the
//! writers, and one in-memory count is the file's size, so rotation is decided
//! once for the file and never per writer: two writers with their own counts
//! would each rotate the other's file and the bound would be per writer. The
//! size is read from the file once, when it is opened, and counted from there,
//! so a record costs no `stat`.
//!
//! [`OPEN`] holds weak references, so the file is closed when the last handle
//! is dropped and a path nothing writes to holds no descriptor. Two spellings
//! of one path are two entries; the configuration writes one.
//!
//! # A rotation that fails keeps the record
//!
//! A rename can fail: another program holds the file, or the folder's
//! permissions changed. The record is still written, to the live file, and the
//! failure is printed once per process to `stderr`. The next rotation is tried
//! when the file has grown by another `max_size`, so a failure that lasts costs
//! one attempt per `max_size` written and not one per record. Losing records to
//! keep the bound would hide the failure the floor exists to report.
//!
//! # It is opened lazily and closed across a rotation
//!
//! The handle is opened on the first write rather than at construction, so a
//! configured target that is never written costs no descriptor. It is dropped
//! before the renames, because on Windows renaming a file that is still open
//! can fail.
//!
//! # No capability check
//!
//! `rule:security/capability-check-at-the-door`'s
//! doors stand in front of what a *program* asks for. This path is the engine
//! writing its own diagnostics to a path an operator configured, with no
//! program-supplied name anywhere in it, so there is no question for a door to
//! answer.
//!
//! **What it spends:** one entry in [`OPEN`] and one descriptor per path that a
//! live handle names, and nothing per record.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, PoisonError, Weak};

/// Every file a live [`LogFile`] names, by its path as written.
static OPEN: LazyLock<Mutex<HashMap<PathBuf, Weak<Mutex<Rotating>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Set by the first failed rotation in the process, so the failure is printed
/// once.
static REPORTED: AtomicBool = AtomicBool::new(false);

/// A handle to a log file with `rule:http-server/the-floor-cannot-fill-the-disk`'s two bounds on it.
#[derive(Debug)]
pub struct LogFile {
    /// The state every handle to this path shares.
    shared: Arc<Mutex<Rotating>>,
}

impl LogFile {
    /// A target at `path` under [`nvs_config::log::MAX_SIZE`] and
    /// [`nvs_config::log::KEEP`].
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self::with_bounds(path, nvs_config::log::MAX_SIZE, nvs_config::log::KEEP)
    }

    /// A target at `path` under a caller's own bounds.
    ///
    /// A path another live handle already names shares that handle's file, and
    /// these bounds replace the ones it had: the newest configuration wins. The
    /// bounds are arguments so a test can rotate without writing
    /// [`nvs_config::log::MAX_SIZE`] bytes.
    #[must_use]
    pub fn with_bounds(path: PathBuf, max_bytes: u64, keep: usize) -> Self {
        let mut open = OPEN.lock().unwrap_or_else(PoisonError::into_inner);
        open.retain(|_, file| file.strong_count() > 0);
        if let Some(shared) = open.get(&path).and_then(Weak::upgrade) {
            shared
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .bound(max_bytes, keep);
            return Self { shared };
        }
        let shared = Arc::new(Mutex::new(Rotating {
            path: path.clone(),
            file: None,
            written: 0,
            max_bytes,
            keep,
            ceiling: max_bytes,
        }));
        open.insert(path, Arc::downgrade(&shared));
        Self { shared }
    }

    /// Appends `bytes`, rotating first if they would not fit.
    ///
    /// # Errors
    ///
    /// Whatever opening or writing the file returns. A failed rotation is not
    /// an error here: the module doc says why the record is written anyway.
    pub fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .write(bytes)
    }
}

/// One file's state, shared by every [`LogFile`] that names its path.
#[derive(Debug)]
struct Rotating {
    /// The live file's path. Rotation `n` is this with `.n` appended.
    path: PathBuf,
    /// The live file, once something has been written to it.
    file: Option<File>,
    /// How many bytes the live file holds: its length when it was opened, plus
    /// everything written since.
    written: u64,
    /// The size at which the live file rotates.
    max_bytes: u64,
    /// How many rotations are kept beside it.
    keep: usize,
    /// The size at which the next rotation is tried: `max_bytes`, or one
    /// `max_bytes` past the size at which the last rotation failed.
    ceiling: u64,
}

impl Rotating {
    /// Replaces the bounds, for a handle made under a newer configuration.
    fn bound(&mut self, max_bytes: u64, keep: usize) {
        if (self.max_bytes, self.keep) != (max_bytes, keep) {
            self.max_bytes = max_bytes;
            self.keep = keep;
            self.ceiling = max_bytes;
        }
    }

    /// [`LogFile::write`], under the lock.
    fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        let len = bytes.len() as u64;
        self.open()?;
        if self.written > 0 && self.written.saturating_add(len) > self.ceiling {
            match self.rotate() {
                Ok(()) => self.ceiling = self.max_bytes,
                Err(why) => {
                    self.report(&why);
                    self.ceiling = self.written.saturating_add(self.max_bytes);
                }
            }
            self.open()?;
        }
        let file = self
            .file
            .as_mut()
            .expect("`open` above opened it or returned");
        file.write_all(bytes)?;
        self.written += len;
        Ok(())
    }

    /// Opens the live file for appending if it is not open, and reads its
    /// length once.
    fn open(&mut self) -> io::Result<()> {
        if self.file.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            self.written = file.metadata().map_or(0, |meta| meta.len());
            self.file = Some(file);
        }
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

    /// Ages every kept rotation by one and leaves no live file, so the next
    /// open starts a new one.
    ///
    /// The oldest is removed *first*, so the bound holds at every moment of the
    /// rotation rather than only once it has finished: a crash in the middle
    /// leaves the target inside its bound. With `keep` at zero the live file
    /// itself is the oldest and is removed.
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

    /// Writes the first failed rotation of the process to `stderr`, the
    /// channel the floor writes to when nothing is configured. A write to
    /// `stderr` that fails is ignored, as the floor's own is.
    fn report(&self, why: &io::Error) {
        if !REPORTED.swap(true, Ordering::Relaxed) {
            let _ = writeln!(
                io::stderr(),
                "warning: Novis cannot rotate the log file `{}`: {why}\n  \
                 note: Novis keeps writing to this file and tries again when it has grown by \
                 `[log] max_size`.",
                self.path.display()
            );
        }
    }
}

/// Removes `path` if it is there, treating an absent one as done.
fn remove(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(why) if why.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rotation `n` of `path`, as the target names it.
    fn rotation(path: &Path, n: usize) -> PathBuf {
        let mut name = path.as_os_str().to_owned();
        name.push(format!(".{n}"));
        PathBuf::from(name)
    }

    /// The size of `path`, or zero where there is no file.
    fn size(path: &Path) -> u64 {
        fs::metadata(path).map_or(0, |meta| meta.len())
    }

    /// A full file is renamed to `.1`, the older ones move up one, and the one
    /// past `keep` is deleted, so the files on disk stay under
    /// `(keep + 1) * max_size`.
    // covers: directive:log.max_size, directive:log.keep
    #[test]
    fn a_full_file_shifts_up_and_the_oldest_past_keep_is_deleted() {
        let dir = nvs_repo::scratch_private("logfile-shift");
        let path = dir.join("nvs.log");
        let mut log = LogFile::with_bounds(path.clone(), 20, 2);
        for n in 0..10 {
            log.write(format!("record {n:02}\n").as_bytes())
                .expect("a write to a private folder succeeds");
        }
        drop(log);

        // Ten 10-byte records, two to a file: the live file has the last two,
        // `.1` and `.2` the two pairs before, and nothing older is kept.
        assert_eq!(fs::read_to_string(&path).unwrap(), "record 08\nrecord 09\n");
        assert_eq!(
            fs::read_to_string(rotation(&path, 1)).unwrap(),
            "record 06\nrecord 07\n"
        );
        assert_eq!(
            fs::read_to_string(rotation(&path, 2)).unwrap(),
            "record 04\nrecord 05\n"
        );
        assert!(!rotation(&path, 3).exists(), "`keep = 2` keeps two");
    }

    /// `keep = 0` deletes the full file and starts a new one, so the target is
    /// never larger than `max_size`.
    #[test]
    fn keep_zero_deletes_the_full_file() {
        let dir = nvs_repo::scratch_private("logfile-keep-zero");
        let path = dir.join("nvs.log");
        let mut log = LogFile::with_bounds(path.clone(), 20, 0);
        for n in 0..5 {
            log.write(format!("record {n:02}\n").as_bytes()).unwrap();
        }
        drop(log);
        assert_eq!(fs::read_to_string(&path).unwrap(), "record 04\n");
        assert!(!rotation(&path, 1).exists());
    }

    /// A file a previous process left behind counts toward the size, so the
    /// first record after a restart rotates a full file.
    #[test]
    fn a_full_file_from_before_is_rotated_by_the_first_record() {
        let dir = nvs_repo::scratch_private("logfile-restart");
        let path = dir.join("nvs.log");
        fs::write(&path, "0123456789012345678\n").unwrap();
        let mut log = LogFile::with_bounds(path.clone(), 20, 1);
        log.write(b"new\n").unwrap();
        drop(log);
        assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
        assert_eq!(size(&rotation(&path, 1)), 20);
    }

    /// A rotation that cannot happen keeps every record in the live file, and
    /// the next attempt after the cause is gone rotates as usual.
    #[test]
    fn a_failed_rotation_keeps_writing_to_the_live_file() {
        let dir = nvs_repo::scratch_private("logfile-stuck");
        let path = dir.join("nvs.log");
        // A folder where `.1` goes: it cannot be removed as a file or renamed
        // over, so the rotation fails.
        let blocker = rotation(&path, 1);
        fs::create_dir(&blocker).unwrap();
        fs::write(blocker.join("inside"), "x").unwrap();

        let mut log = LogFile::with_bounds(path.clone(), 10, 1);
        for _ in 0..3 {
            log.write(b"8 bytes\n")
                .expect("a failed rotation does not fail the write");
        }
        assert_eq!(size(&path), 24, "every record is in the live file");

        fs::remove_dir_all(&blocker).unwrap();
        // The failure at 16 bytes set the next attempt to 26, which this write
        // passes, so the rotation is tried again and succeeds.
        log.write(b"8 bytes\n").unwrap();
        drop(log);
        assert_eq!(size(&rotation(&path, 1)), 24);
        assert_eq!(size(&path), 8);
    }

    /// Two handles to one path share one file and one count: many threads
    /// writing at once keep every line whole and stay inside the bound.
    #[test]
    fn writers_on_one_path_share_the_file_and_its_bound() {
        const MAX: u64 = 200;
        const KEEP: usize = 3;
        let dir = nvs_repo::scratch_private("logfile-shared");
        let path = dir.join("nvs.log");
        let handles: Vec<_> = (0..4)
            .map(|writer| {
                let mut log = LogFile::with_bounds(path.clone(), MAX, KEEP);
                std::thread::spawn(move || {
                    for n in 0..100 {
                        log.write(format!("writer {writer} record {n:03}\n").as_bytes())
                            .unwrap();
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }

        let mut held = 0;
        for n in 0..=KEEP {
            let file = if n == 0 {
                path.clone()
            } else {
                rotation(&path, n)
            };
            let text = fs::read_to_string(&file).unwrap();
            assert!(
                text.len() as u64 <= MAX,
                "{} passes max_size",
                file.display()
            );
            for line in text.lines() {
                assert!(line.starts_with("writer ") && line.len() == 19, "{line:?}");
            }
            held += text.len() as u64;
        }
        assert!(!rotation(&path, KEEP + 1).exists());
        assert!(held <= (KEEP as u64 + 1) * MAX);
    }
}
