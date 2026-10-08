`Core\IO::temporaryDir()` is the whole temporary-file surface: it hands out an **owned directory**
and the program names files inside it. There is no `temporaryFile`, because a program needing one
temporary file needs somewhere to put the second.

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
uncaught throw, and a request that died mid-flight.

**The sweep never throws and never alters a response.** A path already gone is the goal state reached
early. A deletion the OS refuses is one log line, and the directory waits for the next sweep. An
operator may set `[debug] keep_temporary` to keep everything *visibly* — each kept path is logged —
and there is no in-language setter, because a program that can exempt its own files can be made to
hoard them. The program's own `remove` and `removeDir` are unchanged and still throw: a deliberate
action's failure is the program's to hear about.
