# ADR 0082 — Novis ships the batteries: a first-party framework, split by ADR 0051's existing tests

- **Status:** Accepted
- **Date:** 2026-08-24
- **Scope:** that Novis ships a batteries-included web framework at all; the rule that decides which half of it
  is compiled into the binary and which half is a package; the roster of each half; `nvs new`; and what the
  framework deliberately refuses to be. Not in scope: the signature of any member, which the milestone that
  builds it designs against [0063](0063-core-api-conventions.md); the package mechanism itself
  ([0081](0081-packages-are-digests-resolution-is-a-maximum.md)); and schema migration *semantics*, which
  § 7 records as a deliberate open gap.
- **Amends:** [0051](0051-standard-library-tiers.md) § 3 — the Core roster gains the members § 2 lists, each
  placed by that ADR's own six tests rather than by a new rule; § 5's "only Tier 0 may claim the `Core`
  prefix" is untouched and is why the framework's other half is named `Web`.
  [0077](0077-compile-time-routing.md) § 4 — that ADR deliberately stopped at matching and left dispatch,
  controllers and middleware to "a framework"; § 3 below is that framework, so the sentence now names a
  file rather than a hypothetical.
  [0011](0011-functions-and-constants-are-class-members.md) — `Web` joins `Core` as a namespace whose
  members are all class members; no rule changes, the roster does.
  [docs/implementation-plan.md](../implementation-plan.md) — a milestone for each half.
- **Amended by:** none.

> **In short:** Novis ships a working web framework, because "a good language and an empty registry" is the
> position every language that lost this fight occupied ([0080](0080-the-audience-nvs-is-built-for.md) § 4).
> It is split in two, and **no new placement rule is invented to do it**:
> [ADR 0051](0051-standard-library-tiers.md)'s six tests already answer where each piece goes. Anything
> needing runtime privilege, laundering a qualifier, or waiting on the outside world is **`Core`, in the
> binary** — session, validation, the job queue, persistent connections, password hashing, mail transport.
> Everything above that is **`nvs/web`, a first-party package** under
> [0081](0081-packages-are-digests-resolution-is-a-maximum.md) providing the `Web` namespace — controllers,
> middleware, auth flows, mail composition, i18n, storage, pagination, migrations, scaffolding. The
> framework has **no ORM and no runtime service container**: data access is
> [0067](0067-core-db.md)'s prepared statements plus [0071](0071-derived-codecs.md)'s derived codecs, which
> is a data mapper, and wiring is constructor injection resolved *while compiling* through
> [0061](0061-compile-time-autoload-and-program-discovery.md)'s enumeration. `nvs new` produces an
> application that runs, serves a route, talks to a database and has a passing test, with no package fetched
> beyond `nvs/web` itself. **The view layer is the language** — inline HTML is already the template engine,
> so the framework ships no second one.

## Context

- **Nobody chooses a language for a web application; they choose a framework.** Laravel, Rails, Django,
  Spring and Next are the units of decision, and the language underneath is a consequence. A language that
  arrives without one is asking a team to build the framework as their first project, which is a cost no
  team accepts for a runtime they have not yet trusted.
- **The languages that lost this fight all had good type systems.** Hack, Crystal and a dozen others shipped
  a language and waited for an ecosystem that never came. The two that won from a standing start did not:
  Elixir had Phoenix, Go had `net/http` and a standard library broad enough that the framework question was
  optional. [ADR 0080](0080-the-audience-nvs-is-built-for.md) § 4 records this as the explicit answer to
  "why not Hack", and this ADR is that answer's implementation.
- **Novis has been building a framework already, without saying so.** Compile-time routing
  ([0077](0077-compile-time-routing.md)), one database API with closure transactions
  ([0067](0067-core-db.md)), codecs derived from declared properties ([0071](0071-derived-codecs.md)),
  structured concurrency ([0072](0072-core-task-structured-concurrency.md)), scheduled work as config
  ([0073](0073-scheduled-work-is-config.md)), rate limiting ([0075](0075-core-ratelimit.md)), safe HTTP
  defaults ([0074](0074-http-defaults-safe-and-finite.md)), observability ([0076](0076-observability-export.md)),
  a testing capability ([0079](0079-testing-is-a-language-feature.md)) and application-layer security
  protocols ([0060](0060-application-security-protocols.md)) are framework primitives compiled into the
  language. What was missing was the layer that assembles them and the statement that assembling them is
  the project's job.
- **The placement question looks like it needs a new rule and does not.**
  [ADR 0051](0051-standard-library-tiers.md) § 2's six ordered tests were written for stdlib candidates and
  answer this one unchanged — which is the strongest evidence available that they were the right tests.
  Inventing a second procedure for "framework things" would create exactly the kind of duplicated authority
  AGENTS.md's one-home rule exists to prevent.
- **The two halves have genuinely different clock speeds.** A language's surface must be stable for years; a
  framework's opinions need to move. Putting the whole framework in the binary would lock its cadence to the
  runtime's, and putting all of it in a package would push privileged operations across a boundary they
  cannot cross. The split is not a compromise between those — it is the line
  [0051](0051-standard-library-tiers.md) already draws.

## Decision

### 1. The rule: ADR 0051's six tests, applied unchanged

A framework capability is placed by asking [0051](0051-standard-library-tiers.md) § 2's questions in order.
In practice three of the six decide almost everything here:

- **Test 1 — does it need runtime privilege?** State that outlives a request, the request lifecycle, the
  compiler's own tables. Sessions, the job queue, a persistent connection, the route table. → **Core.**
- **Test 2 — is it an injection sink or a launderer?** [ADR 0024](0024-taint-tracking-for-injection-sinks.md)
  § 3 permits only a `Core` member to remove a qualifier, so **every validator is Core by construction** —
  a third-party package can compute over `tainted` data but can never declare it safe.
- **Test 3 — does it wait on the outside world?** Mail transport, storage. → **Native**, capability-gated,
  with the operator naming the endpoint in root-owned config exactly as [0067](0067-core-db.md) does.

Everything that survives all six — pure composition over privileged primitives — is `nvs/web`.

### 2. The privileged half: `Core`, in the binary

| Member | Placed by | Notes |
|---|---|---|
| `Core\Router::match` / `::url` | 1 | Already built ([0077](0077-compile-time-routing.md)) |
| `Core\Session` | 1, 2 | State keyed across requests, cookie is a sink; store rules are [0059](0059-cross-request-state-is-explicit.md)'s, which already forbids backing a session with its lossy local tier |
| `Core\Validate` | 2 | **The launderer.** Every rule that converts `tainted` input into a checked value lives here; this is the one entry that could not be a package under any circumstances |
| `Core\Password` | 1, 2 | Argon2id hash and constant-time verify over a `secret` ([0033](0033-secret-qualifier-for-confidential-values.md)); a `Core\Crypto` primitive, **not** an addition to [0060](0060-application-security-protocols.md)'s closed protocol roster |
| `Core\Queue` | 1 | Enqueue, claim, retry, dead-letter ([0084](0084-durable-background-jobs.md)) |
| `Core\Socket` / `Core\Sse` | 1, 3 | Persistent connections as isolates ([0083](0083-persistent-connections-are-isolates.md)) |
| `Core\Mail` | 3 | **Transport only** — an SMTP endpoint named in root-owned `nvs.toml` under a `mail.send` capability, the shape [0067](0067-core-db.md) established for outbound endpoints. Composition is § 3's |
| `Core\Storage` | 3 | Local-filesystem object storage over [0051](0051-standard-library-tiers.md)'s existing `fs.*` capabilities. Remote backends (S3 and friends) are a package or an extension, never Core |
| `Core\Cldr::pluralCategory` | 4 | One member over CLDR's **cardinal plural rules**, carried in `nvs_stdlib::cldr` beside the date pattern grammar that module already held, so § 3's message formatting need not ship a second copy of them. The carried languages are a closed roster and one outside it is refused rather than given another language's rules ([0095](0095-ambiguous-input-is-refused-never-repaired.md)); that module's own doc is the home of both decisions |

`Core\Cache` ([0059](0059-cross-request-state-is-explicit.md)), `Core\RateLimit`
([0075](0075-core-ratelimit.md)), `Core\Metrics` ([0076](0076-observability-export.md)), `Core\Task`
([0072](0072-core-task-structured-concurrency.md)), `Core\Db` ([0067](0067-core-db.md)) and
`Core\Test` ([0079](0079-testing-is-a-language-feature.md)) are already Core and the framework consumes
them unchanged.

### 3. The opinionated half: `nvs/web`, providing the `Web` namespace

A first-party package under [0081](0081-packages-are-digests-resolution-is-a-maximum.md), fetched, locked,
logged and granted exactly like any other. It holds what an application needs and a language should not fix:

| Area | What it provides |
|---|---|
| `Web\Controller`, `Web\Middleware` | Dispatch and a middleware pipeline over [0077](0077-compile-time-routing.md)'s table — the layer that ADR deliberately did not build. The dispatch `switch` is **generated while compiling** from the route table, so an application never writes one and receives typed, laundered parameters directly ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 9) |
| `Web\Response` | `view`, `json`, `redirect`, `file`, `stream` constructors over [0074](0074-http-defaults-safe-and-finite.md)'s defaults |
| `Web\Auth` | Login and logout flows, remember-me, password reset, policies and role checks over `Core\Session`, `Core\Password` and [0060](0060-application-security-protocols.md)'s CSRF and signed cookies. It is **the** enforcer of `#[Access]`: the server checks CSRF and nothing else, so interpretation happens here or in the application's own dispatch, never in both ([0096](0096-a-route-without-a-declared-access-decision-does-not-compile.md) § 2) |
| `Web\Validation` | Named rule sets, form binding and error presentation over `Core\Validate` — the ergonomics, never the laundering |
| `Web\Mail` | Message composition, templating, attachments, and queued sending over `Core\Mail` and `Core\Queue` |
| `Web\I18n` | Catalogs, locale negotiation, message formatting and plurals over `Core\Str::format` and `Core\Cldr::pluralCategory` |
| `Web\Storage` | A backend-agnostic file API over `Core\Storage`, with upload handling and signed temporary URLs |
| `Web\Pagination`, `Web\Filter` | Cursor and offset pagination, and query shaping over [0067](0067-core-db.md) |
| `Web\Migration` | Authoring, ordering and the CLI surface for schema changes — semantics per § 7 |
| `Web\Job` | The ergonomic layer over `Core\Queue`: a job is a class, retries and backoff are declared |
| `Web\Api` | Serving and versioning the document [0085](0085-openapi-is-generated-from-the-route-table.md) generates |
| Scaffolding | The templates `nvs new` writes |

**There is no view layer, because the language is one.** Inline HTML with `<?nvs`/`<?=` is already the
template engine, [ADR 0024](0024-taint-tracking-for-injection-sinks.md)'s HTML sink already auto-escapes by
default, and a second templating language would be a second spelling of one job — the rule
[0051](0051-standard-library-tiers.md) test 6 applies to libraries. `Web\Response::view` renders a `.nvs`
file and nothing more.

### 4. What the framework refuses to be

- **No ORM.** Data access is [0067](0067-core-db.md)'s prepared statements plus
  [0071](0071-derived-codecs.md)'s `#[Db\Derive]` codecs — a data mapper, where a row becomes a declared
  class and nothing is lazy. Active record is not merely absent, it is unrepresentable: it needs `__get`
  ([0014](0014-property-observer.md) forbids the fallback), properties that are not definitely initialized
  ([0022](0022-definite-property-initialization.md) forbids that), and a shape decided at run time
  ([0007](0007-explicit-type-system.md) forbids that). Building one would mean fighting three accepted
  ADRs, and the result would be worse than what the language already gives.
- **No runtime service container, and no facades.** Wiring is constructor injection resolved **while
  compiling**: [0061](0061-compile-time-autoload-and-program-discovery.md) § 3's enumeration finds the one
  implementation of an interface, and a missing or ambiguous binding is a compile error rather than a
  `ContainerException` in production. A container that resolves a class by string name is the mechanism
  behind facades, and [0014](0014-property-observer.md) and [0027](0027-callable-is-closures-only.md)
  already removed everything it would need.
- **No plugin auto-discovery beyond what exists.** `Core\Program::implementing<T>()` is the one enumeration,
  with the cache consequence [0061](0061-compile-time-autoload-and-program-discovery.md) § 5 already
  argued. There is no scan of `vendor/` for service providers.
- **No configuration cache, no route cache, no autoload dump.** Every one of those exists in PHP frameworks
  to move compile-time work out of the request path, and every one of them is a thing Novis already does
  while compiling.
- **No second way to do anything the language does.** The framework may not ship its own collections, its
  own date type, its own string helpers or its own error hierarchy.

### 5. `nvs new`

`nvs new <name>` writes an application that **runs before it is edited**: a `package.toml` depending on
`nvs/web` alone, the generated `vendor/packages.nvs` ([0081](0081-packages-are-digests-resolution-is-a-maximum.md) § 7),
one `#[Route]` controller returning a view, one `#[Route]` returning JSON from a derived codec, a session-backed
login, a `Core\Db` connection reading from a sample schema, an `nvs.toml.example` an operator would edit,
and one `#[Test]` that passes ([0079](0079-testing-is-a-language-feature.md)).

Two constraints on the template, both binding:

- **It demonstrates a qualifier doing its job.** [ADR 0080](0080-the-audience-nvs-is-built-for.md)'s
  *Verification* requires the first code a new user reads to show `tainted` input being laundered by
  `Core\Validate` before it reaches a sink — because that is what Novis is, and a scaffold that leads with
  routing looks like every other framework.
- **It fetches nothing but `nvs/web`.** A scaffold whose first act is to resolve thirty transitive packages
  teaches the habit [0081](0081-packages-are-digests-resolution-is-a-maximum.md) § 4 exists to discourage.

### 6. Versioning

`nvs/web` is a package and lives under every rule in
[0081](0081-packages-are-digests-resolution-is-a-maximum.md), including the one that makes a breaking
release a new package name. Its cadence should be deliberately slow, and
[ADR 0068](0068-dependency-currency-and-the-version-contract.md)'s absorb-don't-forward discipline applies
to it as strictly as to the runtime's own dependencies. The `Core` half versions with the binary and is
bound by every compatibility rule `Core` already carries.

### 7. The one open gap: migration semantics

`Web\Migration` is listed in § 3 as a place in the roster, and **its semantics are deliberately not decided
here**: ordering and dependency between migrations, transactional DDL where the backend supports it and what
happens on the backends that do not, locking so two instances of a fleet cannot run the same migration twice,
what "reversible" means and whether a down-migration exists at all, and how any of it is safe against a live
multi-tenant database. Every one of those interacts with [0024](0024-taint-tracking-for-injection-sinks.md)
(DDL is a sink), [0067](0067-core-db.md) and root-owned configuration, and none is obvious.

**This is a known gap, not an oversight**, and it is the next thing a future session should decide before
`Web\Migration` is built. Until it is decided, nothing in `nvs/web` may ship a migration runner.

## Consequences

- **The project now owns a framework forever.** That is a permanent maintenance obligation with its own
  compatibility surface, its own security reports and its own release cadence — the largest ongoing cost
  this project has taken on, and it is taken on deliberately because
  [0080](0080-the-audience-nvs-is-built-for.md) § 4 says the alternative is the failure mode.
- **The `Core` API surface grows**, which [0051](0051-standard-library-tiers.md) identifies as the expensive
  cost (priority 4) rather than binary size. Each entry in § 2 is justified by a test that would have placed
  it there anyway; none is there because it is convenient.
- **A third-party framework is still possible and still second-class.** Everything `nvs/web` uses is public
  `Core` API, so a competitor can be written — but it cannot launder a qualifier, which means it cannot
  replace `Core\Validate`. That asymmetry is a direct consequence of
  [0024](0024-taint-tracking-for-injection-sinks.md) § 3 and is accepted.
- **The framework is the registry's heaviest user**, which is the point: a package system its own maintainers
  do not depend on does not stay good. Every rough edge in
  [0081](0081-packages-are-digests-resolution-is-a-maximum.md) is felt first-party, first.
- **Teams arriving from Laravel or Rails will look for the ORM and the container and not find them.** § 4 is
  the answer and it must be in the documentation's first screen rather than discovered. The honest framing:
  the things those frameworks use reflection and magic to provide, Novis provides while compiling — and it
  cannot provide them any other way without giving up the guarantees it exists for.
- **`nvs new` becomes a load-bearing artifact.** It is the first experience, so it is held to the same
  standard as the compiler: it is tested on all three platforms, and a change that breaks it is a
  regression.

## Alternatives rejected

- **Primitives only — router, session, validation in Core, everything else left to the ecosystem.** Keeps
  the API surface minimal, which [0051](0051-standard-library-tiers.md) rightly treats as the expensive
  resource. Rejected because it is precisely the position that leaves a good language beside an empty
  registry, and because the ecosystem that would fill it does not exist and has no reason to appear first.
- **The entire framework in the binary, under a `Web` namespace.** One download, one version, best possible
  first run — the Go and OTP model. Rejected because a framework's opinions must move faster than a
  language's surface, and because it would make every framework fix a runtime release.
- **The entire framework as a package, including the privileged parts.** Smallest binary, fastest iteration,
  strongest dogfooding. Rejected because sessions, validation, queues and persistent connections fail
  [0051](0051-standard-library-tiers.md)'s tests 1, 2 and 3 — a package cannot launder a qualifier or hold
  state across requests, so the attempt would end with the privileged pieces in Core anyway plus a worse
  boundary.
- **Ship an ORM.** The single biggest pull for teams arriving from Laravel and Rails, and the most requested
  thing that will be missing. Rejected on § 4's grounds: active record requires three accepted ADRs to be
  weakened, and a data mapper over derived codecs is both representable today and a better fit for a language
  where a row's shape is declared. This will be re-litigated by users; the answer is here.
- **Ship a template engine.** Rejected as a second spelling of a job the language already does, with
  auto-escaping already correct at the sink.
- **Adopt a placement rule specific to framework components.** Rejected as a second authority over the same
  question; [0051](0051-standard-library-tiers.md)'s tests answered every case here without amendment,
  which is the argument for not having a second set.

## Revisiting

- **If the framework becomes the thing users talk about and the language incidental**, that is Rails'
  history, and [0080](0080-the-audience-nvs-is-built-for.md)'s *Revisiting* already names the naming and
  positioning consequence. `Web` would then likely want a real product name.
- **If the `Core`/`nvs/web` boundary is crossed repeatedly** — a package member that keeps needing a
  privileged escape hatch — the split is in the wrong place and
  [0051](0051-standard-library-tiers.md) § 2 should be applied to that member again rather than an escape
  hatch being added.
- **If a credible third-party framework appears**, the laundering asymmetry in *Consequences* becomes a
  fairness question worth re-arguing — though not at the price of
  [0024](0024-taint-tracking-for-injection-sinks.md) § 3.

## Verification

- **The split holds:** every member in § 2 has a test number in its row, and a reviewer applying
  [0051](0051-standard-library-tiers.md) § 2 to it independently reaches the same placement. A member added
  later with no test number is a bug in this table.
- **`nvs new` runs:** the scaffolded application compiles, serves its two routes, authenticates a session,
  reads a row, and its `#[Test]` passes — on Windows, Linux and macOS, asserted in CI as a first-class job
  rather than as an example.
- **The scaffold shows a qualifier:** the generated code contains a `tainted` value reaching
  `Core\Validate` before a sink, and deleting the validation call makes the scaffold fail to compile — the
  assertion that the demonstration is real rather than decorative.
- **The scaffold's graph is one package:** `package.lock` for a fresh `nvs new` names `nvs/web` and nothing
  else.
- **No laundering escapes Core:** a fixture in which `nvs/web` attempts to return an unqualified `string`
  derived from a `tainted` one fails to compile, proving the framework is subject to
  [0024](0024-taint-tracking-for-injection-sinks.md) § 3 like any other package.
- **No runtime container:** a missing interface implementation is a compile error naming the interface and
  the constructor parameter, not a runtime failure — the assertion that § 4's wiring claim is true.
- **`Web\Migration` ships nothing** until § 7's gap is closed by an ADR; a migration runner appearing in the
  package without one is a review failure.
