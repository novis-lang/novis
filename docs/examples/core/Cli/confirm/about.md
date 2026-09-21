Asks a yes/no question at the terminal and returns `true` or `false`.

A command line program that is about to do something it cannot undo should ask first. `Core\Cli::confirm`
writes the question, reads the answer, and returns `true` for `y` or `yes` and `false` for `n` or `no`.
Upper case works too. Anything else is not an answer, so the question is asked again. That matters for a
program that deletes on `true`: a typo never counts as a yes.

The `default` option is what the `Enter` key takes. The question shows it as a capital letter, `[Y/n]` or
`[y/N]`, so the person can see which way `Enter` goes. Where there is no terminal, a nightly job for
example, the `default` is the answer. Give none and the call throws `Core\Cli\NotInteractive` at once, so
the program never waits for an answer that cannot come.

**The examples below** ask before deleting, show what the `default` does, and walk through a deploy
script that asks before each step.
