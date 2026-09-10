Every **output** sink refuses a `secret` value, and no neutralisation bypasses one: HTML and response
output refuse outright with no auto-escape, terminal output refuses with no carrier bypass, a log
field refuses, a serialiser refuses anywhere in the value it walks — including where the encode is
inside the member, as an enqueued job's payload is — a `Throwable` message requires the
plain type, an attribute payload refuses, and a debug dump renders a fixed placeholder instead of the
value. Escaping fully neutralises injection and does **nothing** for confidentiality — an escaped
credential is still a leaked credential, just HTML-safe.

"Terminal" understates the reach, which is the argument for the row: a scheduled script, a job worker,
a test method and a spawned isolate all write through that sink, so it covers a CI log, a job log and
a test report — which is where a credential in practice leaks.

**Three positions are deliberately not on the list**: bound database parameters, a process argv, and
an outbound request's headers and body. A credential legitimately needs to reach a driver, a
subprocess or an outbound call, and refusing there would make the qualifier unusable for its own
purpose. This closes *accidental* exposure, not *intentional, narrow* use. An attribute payload is the
one sink with no way out at all, being folded into the program's own metadata; the fix is to carry the
name of where to read the secret from.
