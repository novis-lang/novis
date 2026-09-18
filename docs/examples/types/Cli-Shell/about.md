Names the shell a completion script is written for.

A completion script is what makes a shell offer your program's own command names and options when
somebody presses Tab. Each shell reads a different language for it, so a script is always written for
one shell in particular. `Core\Cli\Shell` has one case per shell Novis can write for — `Bash`, `Zsh`,
`Fish` and `Pwsh`, which covers PowerShell 7 and Windows PowerShell alike. Hand one of them to
`Core\Command::completions` and it writes the script from the commands your program already declares,
so the completions cannot drift from the program.

**Good to know:** there is no case meaning "whatever shell this is". A program cannot tell reliably,
and the script is a file somebody saves rather than something to print at a prompt, so the shell is
named every time. There is also no case without a script writer behind it: a fifth shell would mean a
fifth generator, never just a fifth name.
