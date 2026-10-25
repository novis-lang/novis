`Core\IO::temporaryDir()` is the whole temporary-file surface: it hands out an **owned directory**
and the program names files inside it. There is no `temporaryFile`, because a program needing one
temporary file needs somewhere to put the second.

Creating the directory asks `fs.write` of the directory itself. With no `fs.write` written, server
work creates one and then writes and reads only the directories its own context made, so one
request cannot reach another's directory under the shared root; a CLI run writes anywhere. A
written `fs.write` replaces that default, and must then cover the root.

Every directory is created under one root the runtime owns — `[io] temp_root`, else `tmp/` in the
data folder, which Novis creates private to its account. Exclusive ownership of that root is the
entire safety argument for the sweeps: the runtime never deletes anything it did not create, because
nothing else writes there. Sweeping a shared `/tmp`, with anyone's symlinks and anyone's names, is the
classic TOCTOU surface this forbids, and nothing in Novis reads the system temporary directory. With
neither root — the key unset and no usable data folder — `temporaryDir` throws, and only for a program
that calls it.

The two test runners, `nvs test` over a `.nvst` tree and `nvs lsp-test`, have no script and no grant,
and write their cases in a folder they create under the same root, with the same name. With no root
they stop with one error naming `--data`, because there is nothing left to run. Each runner deletes
its folder when the run ends, and a killed run's folder is an orphan like any other.

The runtime keeps a per-script list of the paths it handed out and deletes each surviving entry when
the script ends — after the exit queue on a CLI ending, after the after-response work on a request,
and off the request path, so a response never waits on a deletion. It covers normal end, `exit`, an
uncaught throw, and a request that died mid-flight, because the worker survives that request. A
`FATAL` on the CLI ends the process without that sweep, and its directory is an orphan for
`rule:core-classes/temporary-dir-orphan-sweep`. Each directory is named `nvs-<pid>-<nonce>`, and
the pid is what that sweep reads. The pid is written at ten digits, so every name has one length
and a counted run allocates the same bytes for its paths in every process.

These deletions are work a correct program owed anyway, so the default case costs nothing new, and
the work moves off the request path. What a manual rule never covered is the failure case: an
uncaught throw skips cleanup code, and code that must run whatever the ending cannot be every entry
file's `try/finally`.

**The sweep never throws and never alters a response.** A path already gone is the goal state reached
early. A deletion the OS refuses is one log line, and the directory waits for the next sweep. An
operator may set `[debug] keep_temporary` to keep everything *visibly* — each kept path is logged —
and there is no in-language setter, because a program that can exempt its own files can be made to
hoard them. The program's own `remove` and `removeDir` are unchanged and still throw: a deliberate
action's failure is the program's to hear about. Unifying the two in either direction gives either a
throwing sweep or a silent `remove`.

`[debug] keep_temporary` is reloadable, so an operator can turn it on around one problematic request
and off again. Kept directories stay in the owned root, and the operator's cleanup command is how a
debugging session's keepings are cleared.

**Memory:** one path per directory a script created, request-local and charged to the request's
memory limit, and released when the script ends. A script that never calls `temporaryDir` holds
nothing. On disk, the rare orphan of a hard-killed script waits on a machine with no server until
the operator runs the cleanup command.

**Declined:**
- **Sweeping on the next `temporaryDir` call.** A program that never calls again never sweeps, so
  leftovers grow with the scripts run, which is a leak. It also bills one request for another's
  deletions, on the request path.
- **Wall-clock lifetimes, such as "delete after ten minutes".** They delete files a long-running
  process still holds (`rule:core-classes/temporary-dir-orphan-sweep`).
- **A per-call `persist:` option.** One forgotten argument ships the leak to production, and a
  temporary that outlives its script is cross-request state
  (`rule:concurrency/cross-request-state-is-explicit`). The operator-only key covers the one honest
  use, debugging, with no code change.
