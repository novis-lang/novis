`Core\Process` is the one way to run another program, and it is argv-only. There is no shell-string
form anywhere in it and no flag that turns one on, so `exec`, `system`, `shell_exec`, `passthru`,
`popen`, `proc_open` and backticks all reach one of two members taking a path and an array of
arguments.

The path and every element of the argument array are plain `string`: a `tainted` value needs a
checked conversion or an explicit launderer first, exactly like any other sink. A name the program
did not choose is the whole of what a command injection is, and the array carries no nesting mark of
its own because `array<tainted string>` is simply not `array<string>`.

Running anything at all takes the deny-by-default `process.exec` capability, asked before the target
is looked at, so an ungranted program cannot even learn whether a binary exists.

What this costs is the one case where a shell genuinely was the feature — a pipeline, a glob, a
redirect. Those are written in Novis, or by spawning the shell explicitly and owning the quoting at
that call site.

The two members differ only in whether the caller waits, which is an axis worth keeping. Shell string
against argv is the axis this class removes: there is no escaping function to forget, because there is
no string position to escape into.

**Declined:**
- **One member with a `$useShell` flag.** A flag that looks like an ordinary parameter switches a call
  site between an argv and a shell string whose escaping is the caller's problem, and it invites
  passing `true` to get something working.
- **A separate, clearly named shell member** such as `runViaShell(string $command)`. It is still a
  convenience handed out for the one operation this class makes inconvenient on purpose. Spawning
  `sh -c` or `cmd.exe /c` through the argv API is one line longer and visibly the caller's own risk.
