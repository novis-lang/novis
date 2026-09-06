The tty belongs to the main task of a CLI program. In a request context and inside a spawned isolate,
every `Core\Cli` member throws: two tasks interleaving escape sequences on one terminal produce output
no one can reason about, and there is no locking scheme that makes it coherent.

**`echo` is not one of those members, and does not throw.** It binds to whatever sink its context has
— the response body under a request, the terminal sink everywhere else, including a scheduled script,
a job worker, a test and a spawned isolate's buffer (`rule:tooling/echo-always-has-a-sink`). What
throws is claiming the *terminal*; writing text never does. An isolate's `echo` reaches a buffer its
parent owns, never a tty (`rule:security/isolate-output-is-captured`), which is why the two rules are
consistent rather than in tension.
