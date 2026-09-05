# Security Policy

Novis is a language whose selling point *is* a set of security claims — untrusted data cannot reach a
sink, secrets cannot leak, downloaded code does only what it was granted. A hole in one of those claims is
a vulnerability in the language, not a bug in the program that hit it. That is what this policy is about.

## Supported versions

| Version | Supported |
|---|---|
| `main` | yes — fixes land here |
| everything else | no |

**Novis is pre-alpha and has no release.** Nothing is stable, nothing is supported in production, and there
is no backport branch. Do not run it anywhere that matters yet.

## Reporting a vulnerability

**Report privately, through GitHub:**
[**Security → Report a vulnerability**](https://github.com/novis-lang/novis/security/advisories/new).

Do not open a public issue, a discussion, or a pull request containing a fix for an unreported flaw — the
patch discloses the bug.

Please include, as far as you have it: the affected component (crate, `Core` class, ADR), the version or
commit, a minimal `.nvs` or Rust reproducer, what you expected the boundary to do, and what it did instead.

What to expect:

- **Acknowledgement within 3 days**, an initial assessment within 10.
- We fix, then publish a GitHub advisory naming you as reporter unless you'd rather stay anonymous.
- **Coordinated disclosure, 90 days by default**, shortened once a fix is out and extended only by
  agreement.
- There is no bug bounty. This is a pre-1.0 project with no money behind it; credit is what we can offer.

Good-faith research is welcome and we will not pursue you for it. Test against your own machine: do not
access other people's data, do not run denial-of-service against third parties or public infrastructure,
and do not pivot beyond what is needed to demonstrate the issue.

## What counts as a vulnerability

The security model is one page: [docs/adr/ground-rules.md](docs/adr/ground-rules.md) § *Security and
isolation*, with each rule's ADR behind it. Anything that defeats one of these is in scope:

- **Qualifier bypass** — a `tainted` value reaching a sink unlaundered, a `secret` reaching output, a log,
  a dump, a `Throwable` or serialization, or a sink that should refuse and doesn't
  ([0024](docs/adr/0024-taint-tracking-for-injection-sinks.md),
  [0033](docs/adr/0033-secret-qualifier-for-confidential-values.md),
  [0088](docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)).
- **Request or isolate boundary escape** — one request reading, corrupting or influencing another;
  anything shared between requests but compiled code; a poisoned artifact or hot-reload cache
  ([0006](docs/adr/0006-isolated-script-execution.md),
  [0017](docs/adr/0017-hot-reload-without-restart.md),
  [0059](docs/adr/0059-cross-request-state-is-explicit.md)).
- **Extension sandbox or capability escape** — a wasm component reaching outside its grant, or a manifest
  loosening the qualifier analysis instead of tightening it
  ([0003](docs/adr/0003-extension-system.md),
  [0055](docs/adr/0055-extension-qualifier-declarations.md)).
- **Memory unsafety** — any undefined behaviour reachable from safe Novis or from safe Rust, including in
  the three audited `unsafe` modules, and any JIT miscompilation that emits unsound code.
- **Process-wide impact from one request** — a panic, resource-limit bypass or budget evasion that takes
  down the process or another request instead of dying alone
  ([0004](docs/adr/0004-memory-for-simplicity.md),
  [0020](docs/adr/0020-error-escalation-ladder.md)).
- **A closed door found open** — any route to FFI or native loading, stream wrappers and scheme dispatch,
  cross-request state, or `eval` ([0052](docs/adr/0052-closed-doors.md)).
- **Unsafe protocol or platform defaults** — request smuggling, header or cookie parsing flaws, CSRF/CORS
  defaults that fail open, session-record forgery, a shell string accepted by `Core\Process` or argv
  quoting that recombines, an outbound request that ignores address pinning
  ([0044](docs/adr/0044-core-process-argv-only-no-shell.md),
  [0058](docs/adr/0058-outbound-request-policy.md),
  [0074](docs/adr/0074-http-defaults-safe-and-finite.md),
  [0139](docs/adr/0139-a-session-is-a-record-its-store-issued.md)).
- **Supply chain** — anything that makes a build produce something other than these sources.

## What does not count

- **A program using a documented escape hatch as documented.** `Core\Html::toSource` with a written
  reason, a capability the operator granted, a launderer applied to the wrong string — that is the
  application's bug. A *missing* refusal is ours.
- **Running adversarial Novis source in-process.** An isolate is exactly as strong as the request boundary
  and no stronger; sandboxed wasm is the boundary for code you must assume hostile
  ([0006](docs/adr/0006-isolated-script-execution.md) *Consequences*,
  [0003](docs/adr/0003-extension-system.md)).
- **The built-in server exposed directly to the internet.** It is a development server and a proxied
  origin — no TLS listener, no h2c, no FastCGI
  ([0097](docs/adr/0097-development-server-and-proxied-origin.md)). Findings that require ignoring that
  are documentation questions.
- **A fault that kills one request and only that request.** That is the design; report it as a normal bug.
- **Scanner output with no demonstrated path**, missing hardening flags with no exploit, and advisories
  against a dependency that is not reachable. Open an ordinary issue instead.

## Dependencies

`cargo deny` gates advisories with **no blanket ignores** ([deny.toml](deny.toml)), Dependabot watches the
tree, and a RUSTSEC advisory or a yanked crate is the one thing allowed to interrupt planned work
([0068](docs/adr/0068-dependency-currency-and-the-version-contract.md)). If you find one we've missed and
it is reachable, report it privately as above; if it isn't reachable, an issue is fine.

## How the claims are checked

CI runs on three platforms with Miri over the front end, ASAN over the JIT-bearing crates, libFuzzer
targets, `cargo deny`, and an audit that keeps `unsafe` out of every crate but the three that are allowed
it. None of that is a proof — which is why this file exists.
