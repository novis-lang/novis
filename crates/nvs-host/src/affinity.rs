//! Pinning a worker thread to one CPU, and what is allowed to go wrong.
//!
//! `rule:http-server/a-core-is-never-blocked-on-a-syscall`
//! wants "one single-threaded scheduler of our own pinned per core", and
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
//! is lost is cache locality — a latency cost, not a security one — so
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
///
/// A listed CPU is not a promise that [`pin_current_thread`] will take it.
/// arm64 macOS lists every CPU the machine has and refuses all of them,
/// because `THREAD_AFFINITY_POLICY` is not implemented there — the "platform
/// with no affinity call at all" of the module doc, arrived at through a full
/// list rather than an empty one.
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
    fn the_os_answers_the_same_way_for_every_cpu_it_lists() {
        // Both halves in one test on purpose: a `cpus()` that answered with an
        // id no `pin_current_thread` accepts would pass two separate tests and
        // still be useless. The answer asserted is a *uniform* one rather than
        // a positive one, because a platform that lists every CPU and refuses
        // all of them is supported and is in the test matrix. What is left to
        // catch is the mismatch that matters: a `cpus()` whose ids the pinning
        // call disagrees with, one by one. Skipped where none are listed.
        let listed = cpus();
        let Some(&first) = listed.first() else { return };
        let answer = pin_current_thread(first);
        for cpu in listed {
            assert_eq!(
                pin_current_thread(cpu),
                answer,
                "the OS listed CPU {} and CPU {} and pinned only one of them",
                first.raw(),
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
