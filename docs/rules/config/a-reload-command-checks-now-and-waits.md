`nvs reload` makes a running `nvs serve` match the disk now, for its configuration and its code
alike, and returns when the server does. It is one command for both, and it never empties a cache:
units are keyed by content (`rule:config/an-edit-reaches-the-next-request-without-a-restart`), so
stale code is not possible, only a delay, and emptying the table would recompile every program in
front of live traffic.

**What a reload does, in order.** The configuration check runs at once, without the extra quiet
check a timed pass waits for, and publishes or refuses exactly as a timed pass does
(`rule:config/the-config-is-an-immutable-snapshot`). Then every loaded program is checked at once,
with `[opcache] settle` skipped: the caller says the writes are done. The re-check after a compile
stays, so a file that moved during the compile still discards it and the compile is retried. The
reply is sent when every compile the reload started has ended. A program no request has loaded is
not compiled; its first request compiles it, as it always does.

**The endpoint.** Each `nvs serve` process opens one endpoint under `<data>/run/`, a folder beside
`<data>/cache/` made private by the same code that makes the data folder private. Its name is
`serve-<pid>-<start time>`, so a restarted process never reuses one.

- **Unix:** a Unix socket at that path. The peer's uid must be the serving account's or `0`, read
  with `SO_PEERCRED` on Linux and `getpeereid` on macOS, and any other peer is closed unanswered.
- **Windows:** a named pipe with a random 128-bit name, written into a file at that path. The pipe's
  DACL admits the serving account, `Administrators` and `SYSTEM`, and it is created with
  `PIPE_REJECT_REMOTE_CLIENTS` and `FILE_FLAG_FIRST_PIPE_INSTANCE`.
- **No TCP and no HTTP.** A request cannot reach the endpoint: the data folder is outside every
  grant a program can hold.

The endpoint is removed in the drain. A command that finds the endpoint of a process that is no
longer running skips it and removes it.

**The wire is one line each way.** The command sends `reload 1\n`. The server reads at most 64
bytes and closes any other request without a reply. It answers with one JSON line: the
configuration outcome (`applied`, `refused` or `unchanged`), the restart keys, the swapped program
count, and every refusal with its file, line and message. Only the serving account or an
administrator can read the reply, and both can read the log already, so it carries the error text
the `reload` event leaves out. The 64-byte bound is a constant, not a setting.

**Two reloads at once share one check.** A reply only ever comes from a check that began after the
reload arrived, because a check that began earlier may have read the files before the caller's
writes. So a reload that arrives while a check runs, a timed pass or another reload, waits for it to
end, and then one more check runs. Every reload that arrived during the running check shares that
next check and its reply, so a burst of reloads costs at most two checks.

**The command.** `nvs reload [--config <path>]... [--data <dir>] [--json]` resolves the data folder
as `nvs serve` does and reaches every endpoint in it, so several servers that share one data folder
each get the reload, and the command reports each. It exits `0` when every server applied
everything, `1` when any configuration file or program was refused, and `2` when no server
answered. The requests that reach a refused program fail
(`rule:config/a-broken-edit-fails-the-requests-that-resolve-it`). `--json` prints one document, in
the shape `nvs cache --json` prints. A server that stops during a reload is reported as stopped.

**Only `nvs serve` has an endpoint.** `nvs run` and `nvs test` start a new process that compiles
from disk, so they need no reload, and a long `nvs run` worker is not reached.

**A developer or an agent that relies on `nvs reload` alone sets a long interval**, such as
`revalidate_freq = "1h"` (`rule:config/opcache-revalidation-is-system-class`). A forgotten reload
still goes live within that interval.

**Each program a check changes is one log record**, whether a timed pass or `nvs reload` ran the
check (`rule:config/a-reload-names-what-it-could-not-apply`). A program that was already broken is
not logged again, though every later reload reports it in its reply. Each check that swapped or broke a program,
and every `nvs reload`, is one reload outcome for `on = "reload"` entries
(`rule:config/an-entry-runs-on-a-server-event`), and its `failed` count is every loaded program
that does not compile now, so a program that stays broken keeps the outcome `refused`.

- **Memory:** per process, one endpoint and one blocked accept, on a thread the server already has
  or one small thread of its own, and one 64-byte read buffer per connection while it is read. A
  reload holds one reply until it is written. Nothing on the request path and nothing per request.
- **Latency:** none on the request path. A reload compiles the programs a timed pass would have
  compiled, sooner, on the same compile pool.

**Declined:**
- **A cache flush.** Stale code is not possible under content keys, and a flush recompiles every
  program in front of live traffic.
- **A TCP or HTTP endpoint.** A port is reachable by whatever else runs on the host, and an HTTP
  route would be one a request could name. A private file in the data folder is reachable only by
  the accounts that can already read the log.
- **`SIGHUP`.** It has no Windows equivalent and carries no reply, so the caller cannot learn whether
  the reload applied (`rule:config/the-config-is-an-immutable-snapshot`).
- **A systemd `ExecReload` or a Windows service control.** The command would then differ per
  platform, and the manager learns nothing it can report
  (`rule:packaging/a-service-answers-its-manager`).
- **A `--timeout`.** Every compile is bounded by its own limits, and a server that stops during a
  reload is reported as stopped.
- **A `never` value for `revalidate_freq` or `validate`.** A forgotten reload would then pin the old
  code until a restart; a long interval gives the same quiet with a bound
  (`rule:config/opcache-revalidation-is-system-class`).
