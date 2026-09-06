```
Core\Debug::dump(mixed ...$values): void
Core\Debug::render(mixed $value): <the carrier of the sink in force>
```

| Context | Where a dump lands |
|---|---|
| a CLI program, a scheduled script, a job worker, a test | **stderr**, plaintext |
| an HTTP request writing HTML, `[debug] inline` false | one record at `Log\Level::Debug` |
| an HTTP request writing HTML, `[debug] inline` true | a collapsible block appended to the body, **and** the log record |
| an HTTP request writing **JSON** | the log record only — **never inline** |

CLI dumps go to stderr, not stdout, so piping and redirection keep working while debugging.

**A JSON body is never modified, in either mode.** Injecting a `debug` key would make the served
shape disagree with the published contract in exactly the environment where clients are written
against it — a strict validator would then pass in production and fail in development, the worst
direction for a bug to point. So a development-mode API response is byte-identical in shape to its
production counterpart.

`[debug] inline` can only be tightened at run time: a request may turn its own inline output off and
can never turn it on, so the run mode's default is the only thing that can enable it.

**This is the security half of the rule.** The most-exploited information disclosure in PHP is not a
bug in `var_dump`; it is that `var_dump` writes to output, so a forgotten call and a production
deployment are enough. Here the forgotten call writes a log line, and the spelling that would put it
in a response does not exist outside a mode whose ceiling is closed by default.
