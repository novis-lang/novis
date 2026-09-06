The PHP-shaped syntax is an on-ramp. What may be said about it is bounded.

**Permitted:** that Novis is familiar to a PHP developer, that `<?nvs` and inline HTML work the way
they expect, and that `nvs convert` mechanically rewrites much of an application's own code.

**Forbidden in any document, error message or landing page:** "PHP compatible", "drop-in", "runs your
PHP", "migrate your Laravel app", or any phrasing a reader could reasonably take as a promise that
existing packages, frameworks or code run. Existing PHP does not run unconverted, and this is a rule
about how the project speaks, not only a fact it knows.

`nvs convert` is a porting aid for an application's own code, never a migration guarantee, and its
documentation leads with what it cannot do: it cannot turn a facade into a declared method, a trait
into interface delegation, or an active-record model into a definitely-initialized class. Its default
mode emits only rewrites a differential case proves identical and comments out the rest, its runnable
mode annotates every unproven rewrite at its own site, and `--check` publishes the share — a measured
number standing where a claim would otherwise be.
