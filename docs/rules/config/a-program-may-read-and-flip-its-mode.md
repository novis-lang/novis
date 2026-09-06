```
Core\Env::mode(): Env\Mode                                  // read
Core\Config::set("mode.default", "development"): bool       // flip
```

**Reading it is first-class.** Applications legitimately need it — seed data, a null mail transport,
a development-only route — and a language that denies the honest accessor gets the dishonest one:
people read a governed directive as a proxy for the mode. `Core\Env::mode()` returns the typed enum;
`Core\Config::get("mode.default")` returns the same fact as a string, the way it does for every
directive. `mode.default` is the one key the mode is read and written at; the bare `mode` names the
table and is not a second spelling.

**Flipping it is the ordinary `Runtime` mechanism with no new spelling.** The set is bounded by
`[mode] ceiling`, a `System` directive stating the most permissive mode any code on the host may
select; when unset it equals the mode the server started in, so a production host that wrote nothing
refuses every flip without its operator knowing the feature exists, and one host serving mixed
applications is one root-owned line. A refused flip returns `false` and moves nothing
(`rule:config/a-refused-set-returns-false`); a name that is not one of the two modes is outside every
ceiling.

**Every flip is request-local** (`rule:config/a-runtime-set-is-request-local`), which is what makes
allowing it safe at all. An accepted flip re-derives the five mode rows into the request's overlay,
**except any the request has already set explicitly** — without the re-derivation the flip does
nothing; without the exception it stomps a deliberate choice made three lines earlier. For a mixed
host the `[[app]]` block is the primary answer and the in-code flip the escape hatch.
