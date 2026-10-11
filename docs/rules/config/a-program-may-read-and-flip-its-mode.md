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
**except any the request has already set explicitly or the file wrote** — without the re-derivation
the flip does nothing; without the exception it stomps a deliberate choice made three lines earlier,
or an operator's line written beside the mode. For a mixed host the `[[app]]` block is the primary
answer, since it involves no application code and nothing the application can get wrong; the in-code
flip is the escape hatch for when the application knows something the operator does not.

**What opening the ceiling costs.** On a host whose ceiling is `development`, any code path that
reaches `Core\Config::set` can turn on inline dumps and detailed error responses for its own request,
so an application bug that routes user input into a configuration set becomes an
information-disclosure primitive on that host. Three things bound it: the ceiling is closed unless
an operator writes one line, the flip is request-local so nothing reaches another request, and a
detailed error page refuses a `secret` value like every other sink
(`rule:security/secret-sinks-refuse`). The mode resolves when a configuration is published, so the
request path pays nothing for a request that does not flip.

- **Memory:** an accepted flip holds one string pair in the request's overlay for `mode.default` and
  one for each of the five rows it re-derives, until the request ends. A request that never flips
  holds none.

**Declined:**
- **The mode as a `System` directive, unflippable from code.** It is the purest security answer, but
  one host serving production and in-progress applications side by side is a real deployment, and
  the ceiling keeps the whole security property for every host that has not deliberately opened it.
