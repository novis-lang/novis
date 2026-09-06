A hosted server answers its service manager with the operations it already has, rather than a shim
reporting what it can see from outside:

| Control | What the service does |
|---|---|
| stop (`SERVICE_CONTROL_STOP`, `systemctl stop`) | reports `STOP_PENDING` with a checkpoint that advances while requests drain, then `STOPPED` — a machine restart drains in-flight requests instead of killing them |
| `SERVICE_CONTROL_PARAMCHANGE`, `systemctl reload` | performs the configuration reload in-process; the keys it could not apply are written to the event log **by name** (`rule:config/a-reload-names-what-it-could-not-apply`) |
| `SERVICE_CONTROL_PRESHUTDOWN` | requested at install, because plain `SHUTDOWN` allows roughly five seconds and a drain needs more |

Failure actions are set at install — `--restart on-failure` by default, with a reset period — beside
delayed auto-start (`--start`), dependencies (`--depends-on`, for a database that must come up first)
and a description.

**Output.** With no console handle the process's stderr goes nowhere, so `nvs service run` binds
diagnostics and `Core\Log` to the destination the installer insisted on, and additionally writes a
small, fixed set of lifecycle records — started, stopped, failed to start, reload applied — to the
Windows event log, the first place an administrator looks. The event-log source is registered at
install and removed at uninstall, and `uninstall` leaves nothing behind: no registry key, no source, no
unit file, no granted ACL.
