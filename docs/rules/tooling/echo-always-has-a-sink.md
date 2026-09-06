| Context | `echo` writes to | Carrier |
|---|---|---|
| an HTTP request | the response body | `Core\Html\Markup`, auto-escaping (`rule:core-classes/html-auto-escape`) |
| a CLI program — a `#[Command]` method or a script's main task | stdout | `Cli\Text`, substituting (`rule:tooling/terminal-output-is-a-sink`) |
| a `spawn script` isolate | its own output buffer, or the parent's stream under `output: 'inherit'` | the parent's carrier (`rule:security/isolate-output-is-captured`) |
| a scheduled script, a job worker, a `#[Test]` method | that run's captured output | `Cli\Text` |

**The terminal sink is the default; the HTML sink is attached by an HTTP request and by nothing
else.** That is the fail-closed direction, for the terminal sink's own reason: its substitution is
uniform rather than tty-dependent precisely because a CI log is written to a pipe and read by a human
later, which is exactly what a scheduled run's and a job worker's output is. A context with no
attached sink does not exist, so `echo` never has an undefined meaning — and where the meaning had to
be chosen, it was chosen to neutralize. Routing a sinkless context's `echo` to `Core\Log` instead was
rejected: it silently reshapes free text into the structured writer.

The same table selects a rendering (`rule:errors/renderings`): the sink in force decides not only
where a log record, a dump, a trace or a diagnostic goes but whether it is drawn as plaintext, JSON or
HTML, so no call site names a format. It also gives `Core\Out::capture` its answer
(`rule:security/capture-answers-the-carrier`), and it is why a JSON body is `Response::json` rather
than an `echo` the HTML sink would escape into corruption
(`rule:security/response-body-is-one-typed-member`).
