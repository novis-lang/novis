Asks for a password at the terminal, without showing what the person types.

`Core\Cli::secret` writes the question and reads one line back. The terminal is told not to show the
characters while they are typed, so they are never on the screen at all. Nothing is left behind in
the scrollback buffer or in a screen recording.

The answer is a `secret tainted string`. `secret` means the value cannot be printed, logged or put
in an error message. A program that tries does not compile. `tainted` means a person typed it, so
check it before you use it. `Core\Password::hash` and `Core\Password::verify` take a secret value as
it is. `Core\Secret::reveal` returns the plain text where a program really needs it, and it takes a
written reason.

There is no `default`. Where nobody can type, a nightly job for example, the call throws
`Core\Cli\NotInteractive` at once. A password nobody typed is not a password.

**The examples below** ask for a password, ask for a new one twice, and check a password at a login
prompt.
