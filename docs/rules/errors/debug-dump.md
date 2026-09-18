```
Core\Debug::dump(mixed ...$values): void
Core\Debug::render(mixed $value): Core\Cli\Text
```

| Context | Where a dump lands |
|---|---|
| a CLI program, a scheduled script, a job worker, a test | **stderr**, plaintext |
| an HTTP request writing HTML, `[debug] inline` false | one record at `Log\Level::Debug` |
| an HTTP request writing HTML, `[debug] inline` true | a collapsible block appended to the body, **and** the log record |
| an HTTP request writing **JSON** | the log record only — **never inline** |

CLI dumps go to stderr, not stdout, so piping and redirection keep working while debugging.

**`render` answers `Core\Cli\Text` under every sink**, and that is a bound rather than a rounding of
the table above. A member whose carrier varied would have to *declare* both of
`rule:security/capture-answers-the-carrier`'s classes, and which arm a call answers is the channel's
question rather than the program's — so a caller narrowing that union with `is`
(`rule:types/type-test`) would branch on the deployment's configuration and never on anything its own
source says, where the plaintext carrier is the one it can embed under either sink. A *dump* is still
rendered for the sink in force, which is what `rule:errors/renderings` says and what the
`[debug] inline` row spends.

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
