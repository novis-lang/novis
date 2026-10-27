A hosted server answers its service manager with the operations it already has, rather than a shim
reporting what it can see from outside:

| Control | What the service does |
|---|---|
| stop (`SERVICE_CONTROL_STOP`, `systemctl stop`) | reports `STOP_PENDING` with a checkpoint that advances while requests drain, then `STOPPED` — a machine restart drains in-flight requests instead of killing them |
| `SERVICE_CONTROL_PRESHUTDOWN` | requested at install, because plain `SHUTDOWN` allows roughly five seconds and a drain needs more |

No control from either manager starts a reload. The server reloads by itself when a configuration
file is saved (`rule:config/the-config-is-an-immutable-snapshot`), and tells a systemd unit
`RELOADING=1` and then `READY=1` around it. The SCM is never told the service accepts `PARAMCHANGE`,
and a systemd unit has no reload command.

Failure actions are set at install — `--restart on-failure` by default, with a reset period — beside
delayed auto-start (`--start`), dependencies (`--depends-on`, for a database that must come up first)
and a description.

**Output.** A service has no console, so under the SCM the process's stdout and stderr are redirected
into the Windows event log, one record per line, the way the journal takes a unit's stderr under
systemd: a boot warning, a refused configuration's diagnostic and `Core\Log` written to `stderr` all
land where an administrator looks first, with nothing to configure. Beside them go a small, fixed set
of lifecycle records — starting, started, stopped, failed to start. Every record's text is its one insertion
string, and `nvs.exe` carries the message table that renders it
(`rule:packaging/the-windows-binary-says-what-it-is`). Without the table, Event Viewer heads each record
with "the description for event id … cannot be found" and `Get-WinEvent` shows an empty `Message`.

The dispatcher replaces the standard output and standard error handles with anonymous pipes before the
server runs, and a thread per pipe writes each non-blank line as one record under the service's
source: information for stdout, a warning for stderr. `std` asks the platform for the standard handle
on every write, so the redirect holds for the process's life with no code above it knowing. A pipe
that cannot be made leaves the handle as it was: the records are how a failure is read, and a start
refused for want of them would be a failure with no way to read it. The reader threads are not joined,
so a line written in the last instant before exit can be lost with the process; the lifecycle records
are written directly and are not affected.

A `[log] target` of `stderr` is a working destination for a hosted service: every `Core\Log` record
becomes an event-log record, and its volume is the operator's choice, as it is under the journal.

- **Memory:** two pipes and two threads for the life of a hosted process, and one event-log write per
  line the server prints. Nothing is spent per request unless the operator sends `Core\Log` to
  stderr.

**`nvs service status` reads the last run back when the service is not running**, so a service that
would not start says why where the operator asked. After the manager's state, which on Windows
carries a stopped service's exit code, it prints the run's last 20 lines under `Last run:`. On
Windows they are the warning and error records the service's source wrote since its last *starting*
record, read newest first with `ReadEventLogW` until that record. The run starts at that record and
not after the previous run's end, because a line from an unjoined reader thread can land after its
own run's end record. On Linux they are `journalctl _SYSTEMD_INVOCATION_ID=<id> -o cat --no-pager
-n 20`, for the id `systemctl show -p InvocationID --value` keeps after the run, both by argv with no
shell (`rule:core-classes/process-is-argv-only`). That is every line of the run: systemd stores a
unit's stdout and stderr at one priority, so the journal cannot tell a warning apart. An account that
may not read the journal gets no lines, and a log that cannot be read is one line saying so; the state
is still the answer. A running service prints its state alone. A log written before the *starting*
record existed has no run to find, and prints nothing.

The event-log source is registered at install
and removed at uninstall, and `uninstall` leaves nothing behind: no registry key, no source, no unit
file, no granted ACL.

An arbitrary executable is not a Windows service: the SCM needs the process itself to call
`StartServiceCtrlDispatcher` soon after it starts and to answer controls for the rest of its life. A
shim answers on the hosted process's behalf and can only say that the process exited. The drain is
the service's own, so only the service can report it.

**Declined:**
- **Reading the last run with `EvtQuery`.** It renders each record as XML or through a render
  context to reach the event id and the text. `ReadEventLogW` gives both in the record itself and
  walks newest first, so the read stops at the run's start.
- **Ending the last run at the previous run's end record.** The stdout and stderr readers are not
  joined, so a run's last line can land after its own end record and would read as a run of its own.
- **Documenting NSSM or WinSW and shipping nothing.** A shim observes the process from outside, so a
  stop is a kill rather than a drain, and a Windows deployment would begin by downloading an unrelated
  binary.
- **A `--log-file` path stored under the service's `Parameters` key, with the dispatcher redirecting
  to that file.** It would add a second exception to storing nothing but the argv
  (`rule:packaging/the-argv-lives-in-imagepath`), and the platform already answers the question: an
  administrator looks in the event log first, and a file nobody was told about is a second place.
