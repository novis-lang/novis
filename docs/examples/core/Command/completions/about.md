Writes a completion script for your program, so a shell can finish command names and options while
somebody is typing them.

The script comes from the same table the usage page does: every command your program declares with
`#[Core\Command]`, and every option each one takes. You choose the shell, and Novis writes `bash`,
`zsh`, `fish` or PowerShell, each in that shell's own syntax. Add a command to your program and the
next script has it, with nothing else to update.

A script completes two things: the command word, and the options of whichever command the first word
already named. It never completes a value, because a declared type is not a list of values.

**Good to know:** the result is plain text ending with a newline. It is something to redirect into
the shell's completion directory once, not something to print at a terminal. A program with no name
of its own throws a `LogicError`, which is every way of starting a program except from a command
line.
