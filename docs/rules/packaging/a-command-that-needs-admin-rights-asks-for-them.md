**A command that needs admin rights and runs without them asks for them itself.** The list of commands
that may ask is closed, and it is one `match` in `crates/nvs-cli/src/elevate.rs`:

| Command | Asks |
|---|---|
| `nvs service install` and `nvs install-service`, `uninstall`, `start`, `stop` | before it starts; a `--dry-run` never asks, because it changes nothing |
| `nvs init` | only when the system denies the change to the folder it writes into, once the operator has agreed to that change |
| every other command, `run`, `serve` and `test` included | never, because program code would then run as an administrator |

**Asking is starting the same binary again with the same arguments.** On Windows that is
`ShellExecuteExW` with the verb `runas` and a hidden window. The first process waits and returns the
second's exit code. The second gets a hidden `--elevated-for <pid>` in front of the arguments, frees its
own console and attaches to the first process's (`AttachConsole`), then opens `CONOUT$` and `CONIN$` as
its standard handles. Its output and its prompts, the `--account` password included, are in the
terminal the command was typed in. On Unix the same arguments run under `sudo` in the same terminal.
`nvs init` adds `--yes` to them, because the operator already answered its question.

**The second process trusts nothing the first found.** It parses its arguments and runs every check
again. A process started with `--elevated-for` that still has no admin rights never asks again, so a
declined prompt cannot loop.

**Where no prompt can show, nothing waits.** Without a terminal on standard input and standard error,
with `SSH_CONNECTION`, `SSH_CLIENT`, `SSH_TTY` or `CI` set on Windows, with `--no-elevate`, or on Unix
with no `sudo` on `PATH`, the command prints `run this in a terminal started as administrator:` on
Windows or `run this as root:` on Unix, then the command, and exits `1`. A consent prompt shows on the
local desktop, so a remote session or a CI job would wait for a click nobody can make. `sudo` asks in
the terminal itself, so a remote Unix session asks. A declined prompt prints that it was declined and
exits `1`.

The console attach is the output path because it needs no channel of its own: the elevated process
writes to the console the user already sees, and nothing another account can open is created. A named
pipe with a DACL for the user's SID is the declined alternative, and stays the fallback if a terminal
is found where the attach fails. Cost: one token query per run of a listed command, and nothing for any
other command.
