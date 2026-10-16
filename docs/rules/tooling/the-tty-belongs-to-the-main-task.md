Only the main task of a CLI program reads the terminal. A `Core\Cli` prompt reads it only when the
code's output reaches the process's own standard streams and the process has a terminal
(`crates/nvs-stdlib/src/cli.rs:1449`). In an HTTP request, inside a `Core\Out::capture` block, and in a
spawned isolate, whose output is a buffer its parent owns (`rule:security/isolate-output-is-captured`),
a prompt answers its `default` or throws `Core\Cli\NotInteractive` (`rule:tooling/a-prompt-never-blocks`).
A question written into a response body reaches nobody, and a read after it would hold the core for a
keystroke that never comes.

`Core\Cli::arguments()` is an empty list in a request (`rule:core-classes/cli-arguments`). A served
request was not started with any words, so empty is the true answer, and a library that reads its own
arguments to pick a default keeps working when a handler reaches it.

**Writing is not claiming the terminal.** `echo` and `Core\Cli::write` write to whatever sink their
context has: the response body under a request, the terminal sink everywhere else, and a buffer its
parent owns in a spawned isolate (`rule:tooling/echo-always-has-a-sink`). Neither throws for the
context it runs in, and a live region renders nothing when its stream is not a terminal
(`rule:tooling/in-place-output-is-a-scoped-live-region`).

**Declined:**
- **Every `Core\Cli` member throws outside the main task.** A library that writes with
  `Core\Cli::write` or reads `Core\Cli::arguments()` would then fail when a handler calls it, for
  output that has a sink anyway. Only a read from the terminal needs a person, so only the prompts
  check.
