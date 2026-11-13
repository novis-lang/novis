//! Per-thread counts of the writes a request makes to cache lines that every core shares.
//!
//! A lock taken for reading writes its state word twice, once to take it and once to give it back.
//! An `Arc` clone writes its count once, and the clone's drop writes it again. Each counted site
//! names the [`Line`] it writes and how many writes it makes there, both of them at once, so a site
//! is counted where it is taken and never again where it is dropped. `nvs-cli`'s keep-alive test
//! reads the delta per request on the thread that serves it, and the bound it asserts is what a
//! change to the door or the handler has to keep or lower.
//!
//! **Debug builds only**, for the reason `nvs_host::counts` gives: a crate's `cfg(test)` is
//! invisible to another crate's tests, and a cargo feature would build every crate above this one
//! twice. A counted site sits under `#[cfg(debug_assertions)]`, so the release binary has no
//! counter on its request path.
//!
//! The counts are thread-local. The core that serves a request is the thread that runs its door and
//! its handler, so a test running beside it on another thread adds nothing. A write the
//! publisher or the revalidating thread makes is counted on that thread.
//!
//! Memory: one `u64` per [`Line`] per thread, in debug builds.

use std::cell::Cell;
use std::fmt;

/// The shared cache lines a request writes, one name per kind of value written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    /// The lock of the configuration holder, `nvs_config::Current`.
    CurrentLock,
    /// The lock of one publish's resolved entries and folders.
    PublishLock,
    /// The lock of the server's rendered response policy.
    PolicyLock,
    /// The locks of the compile cache's maps.
    UnitsLock,
    /// The count of one publish, `nvs_config::Published`.
    Publish,
    /// The count of one configuration snapshot.
    Snapshot,
    /// The count of the rendered policy and of each header value it copies into a response.
    Policy,
    /// The count of an app folder's path.
    Folder,
    /// The counts of the compile cache, of a compiled unit and of the tables it installs.
    Unit,
    /// The counts a request's context copies from the context it is made from.
    Context,
    /// The server's count of requests in flight.
    Admit,
}

impl Line {
    /// Every line, in the order a reading prints them.
    pub const ALL: [Self; 11] = [
        Self::CurrentLock,
        Self::PublishLock,
        Self::PolicyLock,
        Self::UnitsLock,
        Self::Publish,
        Self::Snapshot,
        Self::Policy,
        Self::Folder,
        Self::Unit,
        Self::Context,
        Self::Admit,
    ];

    /// The name a reading prints for this line.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::CurrentLock => "current-lock",
            Self::PublishLock => "publish-lock",
            Self::PolicyLock => "policy-lock",
            Self::UnitsLock => "units-lock",
            Self::Publish => "publish",
            Self::Snapshot => "snapshot",
            Self::Policy => "policy",
            Self::Folder => "folder",
            Self::Unit => "unit",
            Self::Context => "context",
            Self::Admit => "admit",
        }
    }
}

const LINES: usize = Line::ALL.len();

thread_local! {
    static WRITES: [Cell<u64>; LINES] = const { [const { Cell::new(0) }; LINES] };
}

/// Counts `writes` shared writes to `line` on this thread.
pub fn wrote(line: Line, writes: u64) {
    WRITES.with(|counts| {
        let count = &counts[line as usize];
        count.set(count.get() + writes);
    });
}

/// What this thread has counted, per line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shared([u64; LINES]);

impl Shared {
    /// This thread's counts now.
    #[must_use]
    pub fn now() -> Self {
        Self(WRITES.with(|counts| std::array::from_fn(|at| counts[at].get())))
    }

    /// What was counted between `earlier` and `self`.
    #[must_use]
    pub fn since(self, earlier: Self) -> Self {
        Self(std::array::from_fn(|at| self.0[at] - earlier.0[at]))
    }

    /// The writes counted on `line`.
    #[must_use]
    pub fn on(self, line: Line) -> u64 {
        self.0[line as usize]
    }

    /// The writes counted on every line together.
    #[must_use]
    pub fn total(self) -> u64 {
        self.0.iter().sum()
    }

    /// Each count divided by `by`, rounded down, for a reading taken over `by` requests.
    ///
    /// # Panics
    ///
    /// If `by` is zero.
    #[must_use]
    pub fn per(self, by: u64) -> Self {
        Self(self.0.map(|count| count / by))
    }

    /// Each count multiplied by `by`, for comparing a per-request reading with a longer one.
    #[must_use]
    pub fn times(self, by: u64) -> Self {
        Self(self.0.map(|count| count * by))
    }
}

/// One line per counted line that is not zero, then the total.
impl fmt::Display for Shared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in Line::ALL {
            let count = self.on(line);
            if count > 0 {
                writeln!(f, "{:>14} {count}", line.name())?;
            }
        }
        write!(f, "{:>14} {}", "total", self.total())
    }
}

#[cfg(test)]
mod tests {
    use super::{Line, Shared, wrote};

    #[test]
    fn a_reading_counts_each_line_apart_and_sums_them() {
        let before = Shared::now();
        wrote(Line::CurrentLock, 2);
        wrote(Line::Snapshot, 2);
        wrote(Line::Snapshot, 2);
        let delta = Shared::now().since(before);
        assert_eq!(delta.on(Line::CurrentLock), 2);
        assert_eq!(delta.on(Line::Snapshot), 4);
        assert_eq!(delta.on(Line::Admit), 0);
        assert_eq!(delta.total(), 6);
        assert_eq!(delta.per(2).total(), 3);
        assert!(delta.to_string().contains("snapshot 4"));
    }
}
