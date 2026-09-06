A target the platform can only run by handing it to a second command-line parser — `.bat` and `.cmd`
to `cmd.exe`, `.ps1` to `powershell.exe` — is refused before spawning, with a diagnostic naming the
extension and the reason.

The check runs on **every platform build**, not only on Windows, even though the underlying risk (a
second parser re-reading an already-quoted argument) is real only there. Behaviour that silently
diverges by platform is the failure this refusal exists to prevent, and a case that passes on the
developer's machine and refuses in production is worth more than one that does the reverse.

Unix needs no equivalent. A shebang script is launched by `execve` reading the interpreter line and
invoking it in the same kernel call that receives the original, already-split argument vector — no
second program re-parses a command line, because there never was one. The asymmetry is the honest
shape of the underlying problem.

There is no convenience for the case where a batch file really is the target: a caller spawns
`cmd.exe` explicitly, through the same argv API, and takes the quoting risk visibly rather than
through a flag that looks as safe as every other call.
