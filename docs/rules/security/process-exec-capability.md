Running another program is a capability, `process.exec`, asked at the door
(`rule:security/capability-check-at-the-door`). **A CLI run may run any program** when the key is not
written, as a command-line program in any other language does. **Server work runs no program**
without a written line. A written `exec` list replaces the default on both kinds of run, and `[]`
grants nothing. A request may narrow the grant further and never widen it, and calling without it
throws (`rule:security/denial-is-a-runtime-error`). The grant is asked for before the target is
looked at, so an ungranted deployment never learns whether a binary exists.

The capability is only half of it, and the other half is that **there is no shell to interpolate
into**. An executable path and an argv array, with nothing in between that parses a command line,
removes the escaping question rather than answering it — which is a stronger guarantee than any amount
of quoting. Under the sink predicate the executable path is an instruction, so the path is the sink
(`rule:security/sink-predicate`). The argv elements are no sink of their own, and they are still typed
plain `string`, so an argument list holding a `tainted string` does not compile
(`rule:core-classes/process-is-argv-only`): no shell parses an argument, but the program that receives
it does, and an attacker's value in that position can be an option such as `--output=/etc/passwd`.

A grant that names executable roots resolves the same way a spawn root does, canonicalise-then-prefix
(`rule:security/path-scope-canonicalise-then-prefix`).

**A bare name such as `git` is looked up on `PATH`.** A bare name has no separator, no drive, and is
not `.` or `..`. The compiler leaves a bare name written as the program argument as it is, and joins
every other relative literal to its file's folder (`rule:programs/relative-paths-resolve-from-their-file`).
At the door the lookup comes after the grant: a run whose grant allows no program at all is refused
before `PATH` is read. The program found is then asked about by its absolute path, like any other, so
the file the operating system starts is the file the check approved. Any other path that does not
start at a root is refused.

**A bare name in the grant is looked up on `PATH` once, when the configuration loads**, and the entry
becomes that program's absolute path. A name no folder of `PATH` answers is a load error that names
the entry. So `exec = ["git"]` in server work allows exactly the `git` the server would start. A
folder beside the configuration file is written with a separator, `"bin/"` or `"./bin"`.

**Cost:** one `stat` per folder of `PATH` per bare-name call, and the same per bare-name entry at load.

Declined:

- **A bare name resolved against the current directory.** Under `nvs serve` that directory is shared
  by every app and is a system folder under a service manager, so the name would start a different
  file depending on how the server was started.
- **A bare name looked up in server work before any grant exists.** The lookup would tell an
  ungranted deployment which programs are installed.
- **Programs open by default in server work.** A request that reaches any program reaches everything
  the server's user can run, which is the one thing a server most needs to keep closed.
