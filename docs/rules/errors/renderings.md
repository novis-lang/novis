A record is rendered as **plaintext**, **HTML** or **JSON**, and the sink already in force picks
which. **There is no format argument on any producer** — not on `dump`, not on `write`, not on
`render` — which is what keeps five producers from each growing a `$format` parameter and three
renderings from becoming fifteen.

| Sink in force | Rendering | Carrier |
|---|---|---|
| the terminal | plaintext, indented, coloured iff the terminal has colour | `Cli\Text` |
| an HTTP request writing an HTML body | a collapsible, typed, class-aware block | `Core\Html\Markup` |
| an HTTP request writing a JSON body | JSON | `mixed` |
| the log target | `[log] format`: `"json"` or `"text"` | bytes |

**Zero new carrier types.** Every one already existed, which is what lets a capture around a dump
return something that re-emits correctly instead of being escaped twice.

Colour is the only thing that varies with a tty, and the plaintext rendering reads it from the one
resolved terminal profile (`rule:tooling/the-terminal-profile-resolves-once`); structure,
substitution and redaction never vary. `[log] format` has two values rather than three because HTML
is not a log *target* rendering — a web-facing log viewer is an HTTP response, and reaches the HTML
rendering through the response sink like everything else, so a framework's log viewer gets it with no
second implementation.

The HTML block carries its own style, inline and once per response, under the response's CSP nonce:
the default headers block a linked asset, and a debug block the project's own security defaults stop
from rendering would be no use.

A rendering is not a grammar. A grammar is a string the program writes that something parses
(`rule:security/every-grammar-is-a-sink`); a rendering is output the runtime produces, so the count
of four is unchanged.

**Declined:**
- **A format argument on each producer**, such as `dump($x, "html")`. The call site is the one place
  that does not know where the output goes: the same handler runs under `nvs serve`, under a test and
  inside a spawned isolate, and the sink table (`rule:tooling/echo-always-has-a-sink`) already answers
  it in all three.
- **Choosing the rendering from the `Content-Type` header at dump time.** The body is typed
  (`rule:security/response-body-is-one-typed-member`), so the rendering is a compile-time fact, and
  reading a header would put a run-time decision in its place.
