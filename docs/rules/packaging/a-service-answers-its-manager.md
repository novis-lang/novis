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
(`rule:packaging/the-windows-binary-says-what-it-is`). The event-log source is registered at install
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
