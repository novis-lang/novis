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
already the default *and* the bound; the revalidation rate cap
(`rule:config/opcache-revalidation-is-system-class`); and **anything with no directive**. A mode
never gates a *behaviour* — no dev toolbar, no source-context injection, no watcher. A feature that
should differ between modes gets a directive first and a row second, in that order.
