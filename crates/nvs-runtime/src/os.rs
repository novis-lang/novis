//! The host's own facts: the process's identity, the cores it may run on, the
//! memory the operating system says it holds, and the load the kernel reports.
//!
//! `Core\Os` is the surface these answer — `crates/nvs-stdlib/src/os.rs` — and
//! this is the half of it that has to say `libc` or `windows-sys`. The split is
//! `rule:security/capability-check-at-the-door`'s
//! shape and not an arrangement of convenience: ADR 0118 § 2 forbids
//! `nvs-stdlib` from reaching the operating system itself, `src/terminal.rs`
//! made the same move for the terminal profile, and
//! `nvs_stdlib_reaches_the_os_only_through_the_gate` is what holds it shut.
//!
//! # Nothing here asks a capability
//!
//! A pid, a hostname, a core count, this process's own resident set and the
//! kernel's load average are facts about the process doing the asking: a
//! program learns nothing about anything it does not already inhabit, so there
//! is no grant to check. `Core\Os` therefore carries no row in
//! `nvs_stdlib::registry::CAPABILITIES` at all, which is `Core\Path`'s position
//! rather than `Core\RateLimit::shed`'s — a `None` row declares that a member
//! of a *door* reaches nothing, and declaring one here would make a class that
//! is not a door look like one.
//!
//! # Every answer is optional, and none of it is invented
//!
//! Each function answers an [`Option`], and a `None` becomes a throw one layer
//! up rather than a plausible zero. Windows keeps no load average, so that arm
//! is `None` by construction rather than by failure, and the member reporting
//! it names the platform.

/// This process's identifier, as the operating system numbers it.
///
/// The one fact here `std` already answers, so it reaches no platform call and
/// cannot fail.
#[must_use]
pub fn pid() -> u32 {
    std::process::id()
}

/// How many cores this process may actually run on — the number the server
/// fans its workers out over, not the machine's physical core count.
///
/// [`std::thread::available_parallelism`] reads the affinity mask and the
/// container's quota where either narrows the machine, which is why it is the
/// right question for a program sizing a pool: a program in a two-CPU cgroup on
/// a 96-core host is entitled to two.
#[must_use]
pub fn cpu_count() -> Option<u32> {
    std::thread::available_parallelism()
        .ok()
        .and_then(|count| u32::try_from(count.get()).ok())
}

/// This host's own name, or `None` where the operating system would not answer
/// or answered something that is not UTF-8.
///
/// `_POSIX_HOST_NAME_MAX` is 255, and the buffer carries one byte more so that
/// a name of exactly that length still ends in the terminator this reads up to.
#[cfg(unix)]
#[must_use]
pub fn hostname() -> Option<String> {
    let mut name = [0_u8; 256];
    #[expect(
        unsafe_code,
        reason = "`gethostname` is the only way to ask a Unix host for its own name; `std` has \
                  no equivalent. The out-parameter is a stack buffer this call owns exclusively \
                  and whose length is passed with it, so the kernel writes at most 256 bytes \
                  into a live allocation, and nothing here outlives the call."
    )]
    let answered = unsafe { libc::gethostname(name.as_mut_ptr().cast(), name.len()) } == 0;
    if !answered {
        return None;
    }
    // A name that filled the buffer without a terminator is a name this cannot
    // read the end of, and a truncated hostname is worse than no answer.
    let end = name.iter().position(|byte| *byte == 0)?;
    String::from_utf8(name[..end].to_vec()).ok()
}

/// See the `unix` arm. `ComputerNameDnsHostname` is the host half of the DNS
/// name — the same thing `gethostname` answers — and deliberately not
/// `ComputerNamePhysicalDnsFullyQualified`, which appends a domain a Unix
/// caller would not have been given.
#[cfg(windows)]
#[must_use]
pub fn hostname() -> Option<String> {
    use windows_sys::Win32::System::SystemInformation::{
        ComputerNameDnsHostname, GetComputerNameExW,
    };

    let mut name = [0_u16; 256];
    let mut len = u32::try_from(name.len()).ok()?;
    #[expect(
        unsafe_code,
        reason = "`GetComputerNameExW` is the Windows spelling of `gethostname`. The buffer is a \
                  stack array this call owns exclusively and the length is passed in and written \
                  back, so the call writes at most 256 wide characters into a live allocation and \
                  fails rather than overrunning when the name is longer."
    )]
    let answered =
        unsafe { GetComputerNameExW(ComputerNameDnsHostname, name.as_mut_ptr(), &raw mut len) }
            != 0;
    if !answered {
        return None;
    }
    // The call writes back the length it used, excluding the terminator.
    let end = usize::try_from(len).ok()?;
    Some(String::from_utf16_lossy(name.get(..end)?))
}

/// The bytes of this **process's** resident set, at its high-water mark.
///
/// [ADR 0148](/docs/decisions/0148.md) § 12 names the reading: `getrusage`'s
/// `ru_maxrss`, and the peak working set on Windows, which is the same
/// quantity. It is the process's and never a request's — a request's held
/// bytes are `Core\Budget`'s, off the counter `crate::budget` keeps — and it is
/// monotone, so a caller watching it is watching how far this process has ever
/// grown rather than what it is holding this second.
#[cfg(unix)]
#[must_use]
pub fn resident_bytes() -> Option<u64> {
    #[expect(
        unsafe_code,
        reason = "`getrusage` is the portable Unix answer for a process's own resource use and \
                  `std` has no spelling of it. The out-parameter is a stack `rusage` this call \
                  owns exclusively, the struct is plain data with no invalid bit pattern, and \
                  nothing here outlives the call."
    )]
    let usage = unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        (libc::getrusage(libc::RUSAGE_SELF, &raw mut usage) == 0).then_some(usage)
    }?;
    let maxrss = u64::try_from(usage.ru_maxrss).ok()?;
    // Darwin counts `ru_maxrss` in bytes; Linux and the BSDs count it in
    // kibibytes, which is the one place this reading is not portable by itself.
    if cfg!(any(target_os = "macos", target_os = "ios")) {
        Some(maxrss)
    } else {
        maxrss.checked_mul(1024)
    }
}

/// See the `unix` arm. `PeakWorkingSetSize` is the resident high-water mark
/// Windows keeps, and `K32GetProcessMemoryInfo` is the `kernel32` entry point
/// for it, so this links no second import library.
#[cfg(windows)]
#[must_use]
pub fn resident_bytes() -> Option<u64> {
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS>()).ok()?,
        ..PROCESS_MEMORY_COUNTERS::default()
    };
    let size = counters.cb;
    #[expect(
        unsafe_code,
        reason = "the two calls are the Windows answer for a process's own memory. \
                  `GetCurrentProcess` is a pseudo-handle needing no close, and the \
                  out-parameter is a stack `PROCESS_MEMORY_COUNTERS` this call owns \
                  exclusively whose size is passed with it, so the call writes a fixed-size \
                  struct into a live, correctly-sized allocation."
    )]
    let answered =
        unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &raw mut counters, size) } != 0;
    if !answered {
        return None;
    }
    u64::try_from(counters.PeakWorkingSetSize).ok()
}

/// The kernel's load average over one, five and fifteen minutes, or `None`
/// where the platform keeps none.
///
/// The three figures are runnable-process counts and not percentages, so a
/// number above the core count is a queue and not an error.
#[cfg(unix)]
#[must_use]
pub fn load_average() -> Option<[f64; 3]> {
    let mut average = [0.0_f64; 3];
    #[expect(
        unsafe_code,
        reason = "`getloadavg` is the only spelling of this on Unix. The out-parameter is a \
                  stack array of exactly the three elements the count argument asks for, owned \
                  exclusively by this call, and the return value is how many it filled."
    )]
    let filled = unsafe { libc::getloadavg(average.as_mut_ptr(), 3) };
    (filled == 3).then_some(average)
}

/// See the `unix` arm. **Windows keeps no load average**: there is no counter
/// behind this to read, and the processor-time percentage that gets suggested
/// in its place answers a different question — how busy the cores were, not how
/// many processes were waiting for one. So this answers nothing and the member
/// above it throws, naming the platform.
#[cfg(windows)]
#[must_use]
pub fn load_average() -> Option<[f64; 3]> {
    None
}

#[cfg(test)]
mod tests {
    /// Every fact this module answers is answered on the platform the tests run
    /// on, except the load average on Windows, which has nothing to answer
    /// from.
    ///
    /// Asserted as *shapes* rather than values, because a pid, a hostname and a
    /// resident set are all different on every run: what could go wrong here is
    /// a call whose arguments are wrong, and that shows up as a `None` or a
    /// zero rather than as a wrong-looking number.
    #[test]
    fn the_host_answers_every_fact_this_platform_keeps() {
        assert!(super::pid() > 0, "a process has a pid");
        assert!(
            super::cpu_count().is_some_and(|count| count > 0),
            "a process runs on at least one core"
        );
        assert!(
            super::hostname().is_some_and(|name| !name.is_empty()),
            "a host has a name"
        );
        assert!(
            super::resident_bytes().is_some_and(|bytes| bytes > 0),
            "a running process holds memory"
        );
        assert_eq!(
            super::load_average().is_some(),
            cfg!(unix),
            "the load average is the one fact Windows keeps none of"
        );
    }
}
