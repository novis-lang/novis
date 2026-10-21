**The mode is a shorthand. It changes only the default of directives that are each individually
settable anyway.** The complete list, and it is complete:

| Directive | Class | `production` | `development` |
|---|---|---|---|
| `[debug] inline` | `RuntimeTighten` | `false` | `true` |
| `[log] format` | `Runtime` | `"json"` | `"text"` |
| `[log] level` | `Runtime` | `Info` | `Debug` |
| `[http.errors] detail` | `Runtime` | `"generic"` | `"full"` |
| `[log] access` | `Runtime` | `false` | `true` |

Three properties follow. **Every row is spellable on its own**, so `mode = "development"` beside
`[log] format = "json"` is legal and means what it reads like — the mode supplies a default, the
explicit line overrides it. **The resolved value of every directive is printable**, so *what exactly
does development mode change?* has a complete, mechanical answer. **No row is `System`-class**,
which is what makes a runtime flip coherent: everything the table governs is something a request
could already have set for itself. The table has one home in the tree, and a test asserts its key
set — a sixth row cannot appear silently.

What a mode deliberately does not govern: `[debug] mode`'s probe bits, whose configured value is
already the default *and* the bound, so a mode that widened it would turn probes on for every
request rather than make them available, spending request-path latency nobody asked for; the
revalidation rate cap (`rule:config/opcache-revalidation-is-system-class`); loopback in server
work, a capability no request could set for itself and so a startup row
(`rule:config/a-startup-default-is-never-flipped`); and **anything with no directive**. A mode never gates a *behaviour* — no dev toolbar, no source-context injection, no
watcher. A feature that should differ between modes gets a directive first and a row second, in that
order.

**Declined:**
- **A mode with behaviours of its own** beyond the table: a dev toolbar, source maps in traces, a
  filesystem watcher. It is the most capable shape, but every later feature would have to decide
  whether it is mode-gated, different authors would decide it inconsistently, and the answer to
  *what does development mode change?* would stop being a list.
