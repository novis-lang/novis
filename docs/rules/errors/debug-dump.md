```
Core\Debug::dump(mixed ...$values): void
Core\Debug::render(mixed $value): Core\Cli\Text
```

| Context | Where a dump lands |
|---|---|
| a CLI program, a scheduled script, a job worker, a test | **stderr**, plaintext |
| an HTTP request writing HTML, `[debug] inline` false | one record at `Log::Level::Debug` |
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

**The inline block names its request.** The block an HTML body gets under `[debug] inline` starts
with one line: the number of queries the request ran, and a link to the request in the viewer,
`/requests/<trace id>/<span id>` on the address `[debug] listen` names, with the scheme and host
`nvs debug` prints its own link with. A `listen` that names no address a link can use, such as the
port `0`, leaves the count alone on the line. The line is written only when `[debug] inline` is on and `[debug] store` is on for the
request: without `store`, the viewer has no record of it. The link carries no token, so the page
leaks none. A browser that has opened the viewer once has its cookie, and any other gets the
viewer's own login.

`[debug] inline` can only be tightened at run time: a request may turn its own inline output off and
can never turn it on, so the run mode's default is the only thing that can enable it, and
`rule:http-server/the-mode-ceiling-defaults-to-the-startup-mode` bounds reaching that mode at all.

**This is the security half of the rule.** The most-exploited information disclosure from a debug
dump is not a bug in the dump; it is that the dump writes to output, so a forgotten call and a
production deployment are enough. Here the forgotten call writes a log line, and the spelling that would put it
in a response does not exist outside a mode whose ceiling is closed by default.

**Declined:**
- **A `debug` key injected into a JSON body in development.** The served shape would contradict the
  published API document (`rule:routing/api-document-is-a-deterministic-build-artifact`), for the
  reason above.
- **A response header carrying a dump id**, the shape some PHP frameworks use for a debug token. The
  server already sends the request's ids in `traceresponse`, and the viewer opens a request by them,
  so a second id would name the same thing twice.
- **A toolbar that updates after the response.** It needs a script in the program's page, which
  changes the page under test and its content security policy. The inline link is static text.
- **The viewer's token in the inline link.** The page is HTML a person may save, share or paste into
  a bug report, and the token opens every recorded request.
