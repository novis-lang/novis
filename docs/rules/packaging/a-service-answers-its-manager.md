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
of lifecycle records — started, stopped, failed to start. Every record's text is its one insertion
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

The event-log source is registered at install
and removed at uninstall, and `uninstall` leaves nothing behind: no registry key, no source, no unit
file, no granted ACL.

An arbitrary executable is not a Windows service: the SCM needs the process itself to call
`StartServiceCtrlDispatcher` soon after it starts and to answer controls for the rest of its life. A
shim answers on the hosted process's behalf and can only say that the process exited. The drain is
the service's own, so only the service can report it.

**Declined:**
- **Documenting NSSM or WinSW and shipping nothing.** A shim observes the process from outside, so a
  stop is a kill rather than a drain, and a Windows deployment would begin by downloading an unrelated
  binary.
- **A `--log-file` path stored under the service's `Parameters` key, with the dispatcher redirecting
  to that file.** It would add a second exception to storing nothing but the argv
  (`rule:packaging/the-argv-lives-in-imagepath`), and the platform already answers the question: an
  administrator looks in the event log first, and a file nobody was told about is a second place.
