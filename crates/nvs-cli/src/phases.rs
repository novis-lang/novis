//! The work each phase of one start did, counted and never timed, in a debug build only.
//!
//! `nvs check --count` and `nvs run --count` print one `phases:` line after the `compile:` line.
//! It names the six phases [`crate::front_end_in`] runs before a program can be compiled, in order:
//!
//! | phase | what it is |
//! |---|---|
//! | `configuration` | the configuration tree read, merged and turned into grants (`config::grants`) |
//! | `built_in` | the built-in extension set added and its classes made (`extensions::with_built_in`) |
//! | `grants` | the namespace grant table read from the nearest `package.toml` |
//! | `core` | the `Core` signature base and its roster, built once per process |
//! | `collection` | the `require` and `autoload` walk, and the package checks over what it loaded |
//! | `check` | the type check and the class layouts |
//!
//! Each phase gets the allocations and bytes this thread allocated
//! ([`nvs_runtime::budget::allocations`]) and its file-system work ([`nvs_diagnostics::work`]).
//! Those counters exist in every build, and this module only takes the difference across a phase.
//! A phase that runs twice, as a lent check does, adds both runs. A start that stops at an error
//! records the phases it finished. `bun nv start-cost` prints the line beside its timings, and
//! `tests/start_cost.rs` bounds each figure.
//!
//! **Debug only** (`rule:testing/perf-two-mechanisms`): a release build compiles [`Mark`] to an
//! empty value and [`line`] to `None`, so the start path has no extra work there.
//!
//! # What it costs
//!
//! In a debug build, three thread-local reads at each phase boundary and six records of seven words
//! per thread. Nothing in a release build.

/// One phase of a start, in the order a start runs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Configuration,
    BuiltIn,
    Grants,
    Core,
    Collection,
    Check,
}

impl Phase {
    const ALL: [Self; 6] = [
        Self::Configuration,
        Self::BuiltIn,
        Self::Grants,
        Self::Core,
        Self::Collection,
        Self::Check,
    ];

    /// The name `--count` prints.
    const fn name(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::BuiltIn => "built_in",
            Self::Grants => "grants",
            Self::Core => "core",
            Self::Collection => "collection",
            Self::Check => "check",
        }
    }
}

#[cfg(debug_assertions)]
mod counted {
    use std::cell::Cell;

    use nvs_diagnostics::work::FileWork;

    /// One phase's totals on this thread.
    #[derive(Debug, Default, Clone, Copy)]
    pub(super) struct Work {
        pub(super) allocations: usize,
        pub(super) bytes: usize,
        pub(super) files: FileWork,
    }

    impl Work {
        pub(super) fn now() -> Self {
            Self {
                allocations: nvs_runtime::budget::allocations(),
                bytes: nvs_runtime::budget::allocated_bytes(),
                files: nvs_diagnostics::work::so_far(),
            }
        }
    }

    thread_local! {
        pub(super) static TOTALS: Cell<[Work; 6]> = const { Cell::new([Work {
            allocations: 0,
            bytes: 0,
            files: FileWork {
                resolved: 0,
                read: 0,
                listed: 0,
                settings: 0,
                programs: 0,
            },
        }; 6]) };
    }
}

/// The counters' reading at the start of a phase. [`Mark::end`] adds what changed since.
#[must_use]
pub(crate) struct Mark {
    #[cfg(debug_assertions)]
    at: counted::Work,
}

/// Starts counting one phase on this thread.
pub(crate) fn start() -> Mark {
    Mark {
        #[cfg(debug_assertions)]
        at: counted::Work::now(),
    }
}

impl Mark {
    /// Adds the work done since [`start`] to `phase`'s totals.
    #[cfg_attr(not(debug_assertions), allow(clippy::unused_self))]
    pub(crate) fn end(self, phase: Phase) {
        #[cfg(debug_assertions)]
        {
            let now = counted::Work::now();
            let index = Phase::ALL
                .iter()
                .position(|p| *p == phase)
                .unwrap_or_default();
            counted::TOTALS.with(|totals| {
                let mut all = totals.get();
                let slot = &mut all[index];
                slot.allocations += now.allocations.wrapping_sub(self.at.allocations);
                slot.bytes += now.bytes.wrapping_sub(self.at.bytes);
                slot.files += now.files - self.at.files;
                totals.set(all);
            });
        }
        #[cfg(not(debug_assertions))]
        let _ = phase;
    }
}

/// `--count`'s `phases:` line, such as `phases: configuration.allocations=12 configuration.bytes=640
/// configuration.read=0 ...`, with every phase's allocations, bytes, `read`, `settings`, `listed` and
/// `resolved`. `None` in a release build.
pub(crate) fn line() -> Option<String> {
    #[cfg(debug_assertions)]
    {
        use std::fmt::Write as _;
        let totals = counted::TOTALS.with(std::cell::Cell::get);
        let mut line = String::from("phases:");
        for (phase, work) in Phase::ALL.iter().zip(totals) {
            let name = phase.name();
            let files = work.files;
            let _ = write!(
                line,
                " {name}.allocations={} {name}.bytes={} {name}.read={} {name}.settings={} \
                 {name}.listed={} {name}.resolved={}",
                work.allocations,
                work.bytes,
                files.read,
                files.settings,
                files.listed,
                files.resolved
            );
        }
        Some(line)
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = Phase::ALL.map(Phase::name);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A phase's figures are what was allocated between its start and its end, and the line
    /// names every phase in order.
    #[test]
    fn a_phase_counts_what_was_allocated_inside_it() {
        let mark = start();
        let held: Vec<Vec<u8>> = (0..4).map(|n| vec![0u8; 16 + n]).collect();
        mark.end(Phase::Grants);
        drop(held);
        let line = line().expect("a test build is a debug build");
        let grants = line
            .split(' ')
            .find_map(|pair| pair.strip_prefix("grants.allocations="))
            .and_then(|value| value.parse::<usize>().ok())
            .expect("a grants count");
        assert!(grants >= 5, "four vectors and their outer vector: {line}");
        let names: Vec<&str> = line
            .split(' ')
            .skip(1)
            .filter_map(|pair| pair.split_once('.').map(|(name, _)| name))
            .collect();
        let mut order = names.clone();
        order.dedup();
        assert_eq!(
            order,
            [
                "configuration",
                "built_in",
                "grants",
                "core",
                "collection",
                "check"
            ]
        );
    }
}
