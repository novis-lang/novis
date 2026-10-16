//! How a `Core\Process` child is started: with its three pipes and no other
//! handle of this process's, and so that it ends when this process ends,
//! however this process ends.
//!
//! [`crate::capability::exec_with`] starts every child through [`spawn`], and
//! this is the half of that door that talks to the operating system. A child
//! is a program a request chose, so what it is given is a request-isolation
//! question: a handle it inherits is a handle the request can reach.
//!
//! While this process runs, a child ends with the task that started it:
//! [`crate::HeldChild`]'s `Drop` kills and reaps a spawned one, and `run` waits
//! for its own. Neither runs when this process is killed or crashes, and what
//! ends the child then is the platform's section below.
//!
//! The child is the only process this module answers for. What the child
//! starts on its own is the child's business, and nothing here tracks it or
//! ends it.
//!
//! # Windows
//!
//! `std` calls `CreateProcessW` with `bInheritHandles = TRUE` and no handle
//! list, so the child inherits every inheritable handle this process has.
//! `std` opens its own files, sockets and pipe ends non-inheritable, and makes
//! the child's three pipe ends inheritable only under its own spawn lock. What
//! is left are the three standard handles this process was started with: a
//! parent that passed them by inheritance made them inheritable. A child that
//! inherits this process's standard output can write into the server's own
//! output, and keeps that pipe open after this process has ended, so the
//! reader on the other end does not see the end of it. `spawn` clears
//! `HANDLE_FLAG_INHERIT` on the three before every child it starts.
//! `Stdio::inherit` keeps working for any other caller, because `std`
//! duplicates the standard handle as an inheritable copy for that one spawn.
//!
//! A `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` would also cover a handle this
//! process inherited from its parent beyond the standard three. The pinned
//! stable toolchain has no way to pass one to `Command`
//! (`CommandExt::raw_attribute` is unstable), so that case is not covered.
//!
//! Every child is put into one job object that this process creates the first
//! time it starts a child, with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. The job's
//! only handle is held in a static and never closed here. The operating system
//! closes it when this process ends, however it ends, and that ends every child
//! still in the job. A child that has exited has already left it.
//!
//! The job also has `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK`, so a process a
//! child creates is not put into the job, and this process ending does not end
//! it. One job serves every child because a job only ever acts here when this
//! process ends, and at that moment one job and many end the same children.
//!
//! The child is created suspended and resumed only once it is in the job, so
//! there is no moment in which it runs outside the job. `std` closes the handle
//! of the child's first thread before `spawn` returns, and the call that keeps
//! it (`ChildExt::main_thread_handle`) is unstable, so the child is resumed
//! with `NtResumeProcess`, which takes the process handle. It is an `ntdll`
//! export that `windows-sys` does not declare, so this module declares it.
//! Finding the thread in a `CreateToolhelp32Snapshot` listing instead lists
//! every thread on the machine, which costs more than starting the process
//! does.
//!
//! # Linux and macOS
//!
//! `std` opens every descriptor close-on-exec, so a child gets its three pipes
//! and nothing else. `std` starts it with `posix_spawn`, which costs the same
//! however much memory this process has, and any thread may start one while
//! another does. A `pre_exec` hook would make `std` fork this whole process
//! instead, so this module adds none.
//!
//! Nothing in the kernel ends a child when its parent ends without a hook in
//! the child, so [`crate::child_watch`] starts a watcher: a second copy of the
//! `nvs` binary, started with the first child and connected to this process
//! by a socket. Each child is sent to it as it starts. When this process ends,
//! however it ends, the watcher reads the end of the socket and kills every
//! child that is still running. That module's doc owns the mechanism: a pidfd
//! per child on Linux, a `kqueue` exit registration on macOS.
//!
//! Two cases are not covered. A child that runs a set-user-ID program runs as
//! another user, and the watcher may have no permission to kill it. A watcher
//! that is killed itself ends nothing. This process finds it gone at the next
//! child start, writes a warning to the log and starts a new one, and the
//! children the old one tracked are no longer ended when this process is
//! killed.
//!
//! Other Unix systems start a child the same way and have no watcher.
//!
//! **What it spends:** on Windows, one job handle per `nvs` process that has
//! started a child, held until that process ends. On Linux and macOS, one
//! watcher process per `nvs` process that has started a child, one message per
//! child start, and on Linux one descriptor in the watcher per child that is
//! still running.

use std::io;
use std::process::{Child, Command};

/// Starts `command` with no handle of this process's but the pipes it names,
/// and returns the child with a warning for the log when it is not ended if
/// this process is killed.
///
/// # Errors
///
/// What the operating system said about starting the child, or on Windows about
/// putting it into the job. A child that started and could not be put into the
/// job is killed and reaped before the error comes back. The watcher never
/// makes a start fail.
#[cfg(not(windows))]
pub(crate) fn spawn(command: &mut Command) -> io::Result<(Child, Option<String>)> {
    let child = command.spawn()?;
    let warning = crate::child_watch::track(&child);
    Ok((child, warning))
}

/// See the other arm. A child that is in the job always ends with this
/// process, so there is never a warning.
///
/// # Errors
///
/// See the other arm.
#[cfg(windows)]
pub(crate) fn spawn(command: &mut Command) -> io::Result<(Child, Option<String>)> {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::CREATE_SUSPENDED;

    windows::keep_standard_handles()?;
    let job = windows::job()?;
    let mut child = command.creation_flags(CREATE_SUSPENDED).spawn()?;
    if let Err(err) = windows::contain(job, &child) {
        // The child has not run a single instruction, so ending it here
        // leaves nothing behind.
        let _ = child.kill();
        let _ = child.wait();
        return Err(err);
    }
    Ok((child, None))
}

/// The Windows calls [`spawn`] makes.
#[cfg(windows)]
mod windows {
    use std::io;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use std::process::Child;
    use std::sync::OnceLock;

    use windows_sys::Win32::Foundation::{
        GetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE, NTSTATUS,
        RtlNtStatusToDosError, SetHandleInformation,
    };
    use windows_sys::Win32::System::Console::{
        GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JobObjectExtendedLimitInformation, SetInformationJobObject,
    };

    #[expect(
        unsafe_code,
        reason = "`NtResumeProcess` is not in `windows-sys`, so it is declared here with the \
                  signature `ntdll` has exported it with since Windows XP: one process handle in, \
                  an `NTSTATUS` out. `raw-dylib` links it without an import library."
    )]
    #[link(name = "ntdll", kind = "raw-dylib")]
    unsafe extern "system" {
        /// Decrements the suspend count of every thread of `process`, which
        /// needs `PROCESS_SUSPEND_RESUME` access.
        fn NtResumeProcess(process: HANDLE) -> NTSTATUS;
    }

    /// Clears `HANDLE_FLAG_INHERIT` on this process's three standard handles,
    /// and fails if one of them is still inheritable afterwards.
    ///
    /// Run before every spawn, not once: `SetStdHandle` can replace a standard
    /// handle at any time, and the calls cost microseconds against a process
    /// start. A standard handle that is not set, and one the operating system
    /// reports no flags for, is not one a child can inherit.
    pub(super) fn keep_standard_handles() -> io::Result<()> {
        for which in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            #[expect(
                unsafe_code,
                reason = "`GetStdHandle` reads this process's own handle table and has no \
                          `std` spelling that leaves the handle open. `SetHandleInformation` \
                          and `GetHandleInformation` take that handle by value and a stack \
                          `u32` out-parameter this call owns, and neither closes or takes \
                          ownership of the handle."
            )]
            let inheritable = unsafe {
                let handle = GetStdHandle(which);
                if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                    continue;
                }
                SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0);
                let mut flags = 0_u32;
                GetHandleInformation(handle, &raw mut flags) != 0
                    && flags & HANDLE_FLAG_INHERIT != 0
            };
            if inheritable {
                return Err(io::Error::other(
                    "this process's standard handles could not be kept from the program",
                ));
            }
        }
        Ok(())
    }

    /// The job every child is put into, created the first time it is needed.
    ///
    /// Its handle lives in a static that is never dropped, so it closes only
    /// when this process ends. Two threads that both find it missing both
    /// create one; the second one's `set` fails and its job closes empty. A
    /// creation that fails is tried again by the next spawn.
    pub(super) fn job() -> io::Result<&'static OwnedHandle> {
        static JOB: OnceLock<OwnedHandle> = OnceLock::new();
        if let Some(job) = JOB.get() {
            return Ok(job);
        }
        let _ = JOB.set(new_job()?);
        JOB.get()
            .ok_or_else(|| io::Error::other("the job for child processes was not kept"))
    }

    /// A new job that ends every process in it when its last handle closes,
    /// and that a process created by one of those processes is not part of.
    fn new_job() -> io::Result<OwnedHandle> {
        #[expect(
            unsafe_code,
            reason = "`CreateJobObjectW` with no security attributes and no name creates an \
                      unnamed job whose handle is not inheritable. A non-null result is a handle \
                      this process owns and nothing else holds, so `OwnedHandle` takes it."
        )]
        let job = unsafe {
            let raw = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if raw.is_null() {
                return Err(io::Error::last_os_error());
            }
            OwnedHandle::from_raw_handle(raw)
        };
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK;
        let size = u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
            .map_err(io::Error::other)?;
        #[expect(
            unsafe_code,
            reason = "the job handle is live for this call, and the limits are a stack struct of \
                      the type the information class names, passed with its own size, which the \
                      call reads and does not keep."
        )]
        let set = unsafe {
            SetInformationJobObject(
                job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&raw const limits).cast(),
                size,
            )
        };
        if set == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(job)
    }

    /// Puts a child that was created suspended into `job`, then resumes it.
    pub(super) fn contain(job: &OwnedHandle, child: &Child) -> io::Result<()> {
        #[expect(
            unsafe_code,
            reason = "both handles are live for this call: the job is borrowed and the process \
                      handle is owned by `child`, which outlives it. The call takes neither."
        )]
        let assigned =
            unsafe { AssignProcessToJobObject(job.as_raw_handle(), child.as_raw_handle()) };
        if assigned == 0 {
            return Err(io::Error::last_os_error());
        }
        #[expect(
            unsafe_code,
            reason = "the process handle is owned by `child`, which outlives the call, and was \
                      returned by `CreateProcessW` with all access, so it has \
                      `PROCESS_SUSPEND_RESUME`. The call takes no pointer and keeps nothing. \
                      `RtlNtStatusToDosError` takes a status by value."
        )]
        let failed = unsafe {
            let status = NtResumeProcess(child.as_raw_handle());
            (status < 0).then(|| RtlNtStatusToDosError(status))
        };
        match failed {
            Some(code) => Err(io::Error::from_raw_os_error(
                i32::try_from(code).unwrap_or(i32::MAX),
            )),
            None => Ok(()),
        }
    }
}
