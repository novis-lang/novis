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

Colour is the only thing that varies with a tty; structure, substitution and redaction never do.
`[log] format` has two values rather than three because HTML is not a log *target* rendering — a
web-facing log viewer is an HTTP response, and reaches the HTML rendering through the response sink
like everything else.
