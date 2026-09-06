The privileged half is compiled into the binary, and every entry is placed by a standard-library tier
test rather than by convenience:

| Member | Placed by | Notes |
|---|---|---|
| `Core\Router::match` / `::url` | privilege | the compile-time route table |
| `Core\Session` | privilege, sink | state keyed across requests, and the cookie is a sink; a session may not be backed by a lossy local tier |
| `Core\Validate` | sink | **the launderer** — every rule converting `tainted` input into a checked value lives here, and this is the one entry that could not be a package under any circumstances |
| `Core\Password` | privilege, sink | Argon2id hash and constant-time verify over a `secret`, a crypto primitive rather than an application-security protocol |
| `Core\Queue` | privilege | enqueue, claim, retry, dead-letter |
| `Core\Socket` / `Core\Sse` | privilege, waits | persistent connections as isolates |
| `Core\Mail` | waits | **transport only** — an SMTP endpoint the operator names in root-owned configuration under a `mail.send` capability; composition belongs to the package half |
| `Core\Storage` | waits | local-filesystem object storage over the existing `fs.*` capabilities; remote backends are a package or an extension, never `Core` |
| `Core\Cldr::pluralCategory` | data nothing else carries | the cardinal plural categories and, beside them, the ordinal forms a place takes, over a **closed** roster of languages — one outside it is refused rather than given another language's rules (`rule:errors/ambiguous-input-refused`) |

`Core\Cache`, `Core\RateLimit`, `Core\Metrics`, `Core\Task`, `Core\Db` and `Core\Test` are already
`Core`, and the framework consumes them unchanged.

Because only a `Core` member may remove a qualifier, **every validator is `Core` by construction**: a
third-party framework can be written over this public surface, but it can compute over `tainted` data
and never declare it safe. A member added to this half later carries the test that placed it; one
arriving with no test is a bug in the roster.
