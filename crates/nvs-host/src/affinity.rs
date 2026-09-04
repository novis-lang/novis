//! Pinning a worker thread to one CPU, and what is allowed to go wrong.
//!
//! [ADR 0106](/docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
//! § 6 wants "one single-threaded scheduler of our own pinned per core", and
//! `docs/plan/design.md` § *Thread-per-core, shared-nothing runtime* says why:
//! a request never migrates, so a refcount is a non-atomic increment and an
//! arena is reached without a lock. Pinning is what turns "never migrates
//! between workers" into "never migrates between CPUs" as well, which is what
//! makes the L1 and L2 lines a request touches stay its own.
//!
//! # Pinning is best-effort, and that is a decision
//!
//! [`pin_current_thread`] returns whether the OS agreed, and **no caller
//! treats a refusal as fatal**. A container with a restrictive cpuset, a
//! platform with no affinity call at all, and a scheduler policy that owns the
//! mask are all real deployments, and in every one of them Novis's correctness
//! is unchanged: the shared-nothing invariant is held by *the run queue*,
//! which never hands a task to another thread, not by the affinity mask. What
//! is lost is cache locality — a priority-3 cost, not a priority-1 one — so
//! refusing to start would be trading the ordering in `AGENTS.md` the wrong
//! way round.
//!
//! The consequence a deployment has to know: an unpinned worker is a
//! *slower* worker, never an unsafe one, and [`Worker::pinned`] is how a
//! caller finds out which it got.
//!
//! [`Worker::pinned`]: crate::Worker::pinned

/// A CPU a worker can be pinned to, as the OS numbers them.
///
/// Deliberately not "the *n*th core": on a machine with SMT, with an
/// asymmetric topology, or inside a cpuset, the ids the OS hands back are the
/// only ones that mean anything, and they are neither dense nor ordered by
/// any property Novis could infer. [`cpus`] is the only place they come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CpuId(core_affinity::CoreId);

impl CpuId {
    /// The OS's own number for this CPU, for logging and for a configuration
    /// file that names one.
    #[must_use]
    pub const fn raw(self) -> usize {
        self.0.id
    }
}

/// The CPUs this process is allowed to run on, in the order the OS lists them.
///
/// Empty rather than an error when the platform cannot say — the caller's
/// answer to "how many workers should I start" is then one, which is the same
/// answer it would give on a single-CPU machine and needs no second code path.
#[must_use]
pub fn cpus() -> Vec<CpuId> {
    core_affinity::get_core_ids()
        .unwrap_or_default()
        .into_iter()
        .map(CpuId)
        .collect()
}

/// Pins the calling thread to `cpu`, reporting whether the OS agreed.
///
/// See the module docs: a `false` here is a locality loss and never a
/// correctness one, so nothing in this crate turns it into an error.
pub fn pin_current_thread(cpu: CpuId) -> bool {
    core_affinity::set_for_current(cpu.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cpu_the_os_lists_can_be_pinned_to() {
        // Both halves in one test on purpose: a `cpus()` that answered with an
        // id no `pin_current_thread` accepts would pass two separate tests and
        // still be useless. Skipped rather than failed on a platform that
        // lists none, which the module docs say is a supported deployment.
        for cpu in cpus() {
            assert!(
                pin_current_thread(cpu),
                "the OS listed CPU {} and then refused it",
                cpu.raw()
            );
        }
    }

    #[test]
    fn a_cpu_id_reports_the_number_the_os_gave_it() {
        let listed = cpus();
        for cpu in &listed {
            assert_eq!(cpu.raw(), cpu.0.id);
        }
        // Ids are distinct: a duplicate would silently pin two workers to one
        // CPU while the caller believed it had spread them.
        let mut raw: Vec<_> = listed.iter().map(|c| c.raw()).collect();
        raw.sort_unstable();
        let before = raw.len();
        raw.dedup();
        assert_eq!(before, raw.len(), "the OS listed a CPU twice");
    }
}
