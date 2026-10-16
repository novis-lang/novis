Every **output** sink refuses a `secret` value, and no neutralisation bypasses one: HTML and response
output refuse outright with no auto-escape, terminal output refuses with no carrier bypass, a log
field refuses, a serialiser refuses anywhere in the value it walks — including where the encode is
inside the member, as an enqueued job's payload is — a `Throwable` message requires the
plain type, an attribute payload refuses, and a debug dump renders a fixed placeholder instead of the
value. Escaping fully neutralises injection and does **nothing** for confidentiality — an escaped
credential is still a leaked credential, just HTML-safe.

"Terminal" understates the reach, which is the argument for the row: a scheduled script, a job worker,
a test method and a spawned isolate all write through that sink, so it covers a CI log, a job log and
a test report — which is where a credential in practice leaks. A terminal is the secret shown to a
person, which is disclosure, not use; a command whose purpose is to print a token writes
`Core\Secret::reveal()` with its reason on that one line. It also agrees with the way in:
`Core\Cli::secret()` reads a password with terminal echo disabled
(`rule:tooling/a-prompt-is-a-core-member`).

The refusal costs the cheapest debugging tool there is: `echo $x` does not work for exactly the
values hardest to reason about. A dump still shows a `secret`-typed property as a placeholder, so its
presence is visible and its bytes are not, and `Core\Secret::reveal()` with a reason is one line when
the bytes are what is being debugged.

The log sink is the opposite default from `tainted`, which a log takes freely
(`rule:security/log-is-not-a-sink`). `Core\Log::write`'s `fields` bag is `array<string, mixed>` on
purpose, so no parameter type can refuse a `secret` inside it; the checker reads the call site's
written argument expressions instead. A `Throwable` message requires the plain type because a
credential in an exception message ends up in a stack trace or an error page. The dump placeholder is
a built-in rule keyed on the declared type, applied the same way to every class, and no class can hook
or override it.

**Three positions are deliberately not on the list**: bound database parameters, a process argv, and
an outbound request's headers and body. A credential legitimately needs to reach a driver, a
subprocess or an outbound call, and refusing there would make the qualifier unusable for its own
purpose. This closes *accidental* exposure, not *intentional, narrow* use. An attribute payload is the
one sink with no way out at all, being folded into the program's own metadata; the fix is to carry the
name of where to read the secret from.

**Declined:**
- **A terminal refusal that depends on whether the output is a tty.** Output piped and read by a person
  later is the common case (`rule:tooling/terminal-output-is-a-sink`).
- **Letting `echo` write a `secret` for a command that prints a token.** `echo` reaches every captured
  output (`rule:tooling/echo-always-has-a-sink`), which is what the refusal is for. If such commands turn
  out to be common, the answer is a narrow, named `Core\Cli` member, never a wider `echo`.
