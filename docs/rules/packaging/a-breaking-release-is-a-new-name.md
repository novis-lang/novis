A package that breaks its API **publishes under a new name** — conventionally the old name with a
numeric suffix, `acme/http` → `acme/http2` — carrying `supersedes = "acme/http"` so that
`nvs outdated` and `nvs audit` can point at it. The two coexist in one graph without conflict,
because they are two packages.

This is what keeps `rule:packaging/resolution-takes-the-highest-minimum` sound: a minimum is never a
ceiling, so a higher version of one name must always be acceptable, and a version that is not
acceptable is therefore not a version of that name. It is the real cost of having no solver, and it
is paid by the publisher rather than by every consumer. The absorb-don't-forward discipline
(`rule:packaging/a-dependency-break-is-absorbed-never-forwarded`) — Novis's own rule for its Rust
dependencies — is what makes a rename rare enough to live with.

The rule is watched rather than enforced: if library authors route around it by shipping breaks
under the same name, either an API-diff check at publish or a ceiling mechanism becomes necessary.
