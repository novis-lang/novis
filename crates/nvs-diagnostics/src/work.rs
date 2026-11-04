//! How much file-system work this thread did, counted and never timed.
//!
//! Five kinds, each counted in the one function that does that work:
//!
//! | kind | counted in | one count is |
//! |---|---|---|
//! | `resolved` | [`crate::canonical_key`], `nvs_hir::requires::canonicalize`, `nvs_config`'s `Disk::trust` and `Disk::canonical` | one path made canonical, a `canonicalize` syscall |
//! | `read` | [`crate::SourceMap::load`] | one source file read from disk |
//! | `listed` | `nvs_hir::autoload`'s listing, `nvs_hir::lenders::sources`, `nvs_config`'s `Disk::list` | one directory listed |
//! | `settings` | `nvs_config`'s `Disk::read_config` | one configuration file read to be parsed |
//! | `programs` | `nvs_hir::lenders::Lender::walk` | one program's graph walked to learn whether it lends its `autoload` map |
//!
//! `nvs check --count` and `nvs run --count` print them after the compile counts, and the language
//! server's index sums them per entry (`rule:testing/bench-counters`). A count is the same on every
//! machine, so a test can bound it per file. They are a feature of `--count`, not a test hook.
//!
//! **The counters are thread-local.** Tests in one binary run on several threads at once, and each
//! reads a delta around the work it runs on its own thread, so one test never sees another's work.
//! A command that runs its work on other threads sums what each thread reports. A read inside a
//! bundled executable is answered from the payload and is not counted, because it touches no disk.
//!
//! Cost: one thread-local add per counted call, beside a syscall that costs about a thousand times
//! more, and five words per thread. Nothing is counted on the request path that is not already a
//! syscall there.

use std::cell::Cell;

/// One kind of file-system work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A path made canonical.
    Resolved,
    /// A source file read.
    Read,
    /// A directory listed.
    Listed,
    /// A configuration file read to be parsed.
    Settings,
    /// A program's graph walked to find whether it lends its `autoload` map.
    Programs,
}

/// The five counts, as one thread's running totals or as the delta between two of them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FileWork {
    /// Paths made canonical.
    pub resolved: u64,
    /// Source files read.
    pub read: u64,
    /// Directories listed.
    pub listed: u64,
    /// Configuration files read to be parsed.
    pub settings: u64,
    /// Programs walked to find a lender.
    pub programs: u64,
}

thread_local! {
    static COUNTS: Cell<FileWork> = const {
        Cell::new(FileWork {
            resolved: 0,
            read: 0,
            listed: 0,
            settings: 0,
            programs: 0,
        })
    };
}

/// Counts one piece of `kind`'s work on this thread.
pub fn count(kind: Kind) {
    COUNTS.with(|counts| {
        let mut now = counts.get();
        let slot = match kind {
            Kind::Resolved => &mut now.resolved,
            Kind::Read => &mut now.read,
            Kind::Listed => &mut now.listed,
            Kind::Settings => &mut now.settings,
            Kind::Programs => &mut now.programs,
        };
        *slot = slot.wrapping_add(1);
        counts.set(now);
    });
}

/// This thread's running totals. Two readings around some work give its count:
/// `so_far() - before`.
#[must_use]
pub fn so_far() -> FileWork {
    COUNTS.with(Cell::get)
}

impl std::ops::Sub for FileWork {
    type Output = Self;

    fn sub(self, before: Self) -> Self {
        Self {
            resolved: self.resolved.wrapping_sub(before.resolved),
            read: self.read.wrapping_sub(before.read),
            listed: self.listed.wrapping_sub(before.listed),
            settings: self.settings.wrapping_sub(before.settings),
            programs: self.programs.wrapping_sub(before.programs),
        }
    }
}

impl std::ops::AddAssign for FileWork {
    fn add_assign(&mut self, other: Self) {
        self.resolved += other.resolved;
        self.read += other.read;
        self.listed += other.listed;
        self.settings += other.settings;
        self.programs += other.programs;
    }
}

impl std::fmt::Display for FileWork {
    /// `--count`'s spelling: `resolved=1 read=2 listed=0 settings=0 programs=0`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "resolved={} read={} listed={} settings={} programs={}",
            self.resolved, self.read, self.listed, self.settings, self.programs
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_delta_counts_only_the_work_between_two_readings() {
        let before = so_far();
        count(Kind::Resolved);
        count(Kind::Resolved);
        count(Kind::Listed);
        let delta = so_far() - before;
        assert_eq!(
            delta,
            FileWork {
                resolved: 2,
                listed: 1,
                ..FileWork::default()
            }
        );
        assert_eq!(
            delta.to_string(),
            "resolved=2 read=0 listed=1 settings=0 programs=0"
        );
    }
}
