# ADR 0077 — Routes are compiled, not registered, and the router stops at matching

- **Status:** Accepted
- **Date:** 2026-08-24
- **Scope:** the `#[Route]` attribute, the compile-time route table built from it, `Core\Router::match` and
  `::url`, the path-pattern grammar, which failures are compile errors, and how a matched segment's
  qualifier and type are resolved. Not in scope: dispatch, a controller convention, a middleware pipeline
  and any rule mapping a return value to a response — § 4 says why none of those is here.
- **Amends:** [0071](0071-derived-codecs.md) § 1 — `Core\Route` joins the closed, `Core`-owned list of
  **compiler-recognized** attributes; nothing else about it or about
  [ADR 0046](0046-attributes-shape-literal-metadata.md) changes.
  [0061](0061-compile-time-autoload-and-program-discovery.md) § 3 — the program-wide scan it built for
  `implementing<T>()` gains a second caller, with the identical opt-in rule and the identical cache
  consequence (§ 5 of that ADR).
  [0019](0019-reflection-and-ast-parsing-are-core-features.md) — its *Consequences* named "an
  attribute-driven router" as the userland thing reflection exists to serve; it is now a compiler pass, and
  that sentence is corrected there.
  [0051](0051-standard-library-tiers.md) § 3 — the Core roster gains `Core\Router`.
  [0024](0024-taint-tracking-for-injection-sinks.md) — no new sink and no new launderer rule; § 3 below is
  that ADR's existing § 2 conversion-laundering applied somewhere new.
  [docs/spec/01-core-library.md](../spec/01-core-library.md) § 13 — a `Core\Router` row.
  [docs/implementation-plan.md](../implementation-plan.md) — M4S gains the table-building pass, M7 the
  matcher.
- **Amended by:** 0082, 0085, 0096, 0097, 0102, 0110, 0146

> **In short:** `#[Route(path: "/users/{id}", method: Http\Method::Get, name: "user.show")]` on a method is
> read **while compiling**, through the program enumeration
> [ADR 0061](0061-compile-time-autoload-and-program-discovery.md) already built, into a route table baked
> into the compiled unit. `Core\Router::match` walks it and `Core\Router::url` reverses it. **A duplicate
> route, a `{param}` with no matching method parameter, and a `url()` naming a route that does not exist are
> all compile errors** — the three routing bugs every framework discovers at runtime. Two decisions do most
> of the work. **It stops at matching**: no dispatch, no controller convention, no return-value-to-response
> rule, so a framework can use all of this, half of it, or none. And **a matched segment's type comes from
> the method's own parameter**, so `{id}` on `show(uint $id)` is converted during matching — which means a
> non-numeric segment simply **does not match** (a 404, not a 500) and, because
> [ADR 0024](0024-taint-tracking-for-injection-sinks.md) § 2 already launders a checked conversion, `$id`
> arrives **unqualified** while a `string $slug` arrives `tainted`. A program with no `#[Route]` builds no
> table, runs no scan, and pays nothing.

## Context

- Routing is the one thing every web application has and no two of them spell alike. In PHP it is a runtime
  registry: a `routes.php` calling `$router->get(...)` hundreds of times, or a Symfony/Laravel attribute
  scan that reads every class through reflection on every cold boot and caches the result into a generated
  file that goes stale. The generated-cache step exists precisely because the work is compile-time work
  being done at runtime.
- The failures are consistent across every framework and every one of them is discovered late: two routes
  claiming the same path and method (the second silently wins, or the first does, depending on registration
  order), a `{id}` placeholder with no corresponding controller argument (a `null` at request time), and a
  reverse-URL call naming a route that was renamed (a broken link in an email, found by a user).
- **Novis already has the two mechanisms this needs.**
  [ADR 0046](0046-attributes-shape-literal-metadata.md) gives structured, compile-time-constant metadata on
  a declaration with no attribute class. [ADR 0061](0061-compile-time-autoload-and-program-discovery.md)
  § 3 gives a program-wide enumeration for exactly the question static resolution cannot answer — *what
  exists that nothing names* — with its opt-in rule and its cache-invalidation consequence already argued.
  A route table is that enumeration filtered by an attribute.
- **The interaction that made this dangerous is already settled.**
  [ADR 0046](0046-attributes-shape-literal-metadata.md) § 4 makes attribute *retrieval* structural, so a
  naive route scan would also match a third-party framework's own `#[Route(path:, method:)]` literals and
  double-register routes it does not own. [ADR 0071](0071-derived-codecs.md) § 1 answered it generally: a
  **compiler-recognized** attribute is matched **nominally**, against a closed `Core`-owned list. This ADR
  only has to add a name to that list.
- **The boundary is the hard part, and it was settled with the user.** A router that dispatches has opinions
  about controllers, about what a handler returns, about middleware, about dependency injection — and those
  opinions are what a framework *is*. Stopping at matching means a framework can build its own dispatch on
  top, ignore this entirely, or use `url()` alone.

## Decision

### 1. `#[Route]` on a method, matched nominally

```php
type Core\Route = {path: string, method: Core\Http\Method, name?: string};
```

```php
use Core\{Route, Http};

class UserController {
    #[Route(path: "/users/{id}", method: Http\Method::Get, name: "user.show")]
    public function show(uint $id): Response { … }

    #[Route(path: "/health", method: Http\Method::Get)]
    #[Route(path: "/health", method: Http\Method::Head)]
    public function health(): Response { … }
}
```

An ordinary [ADR 0046](0046-attributes-shape-literal-metadata.md) `type` alias and an ordinary attachment —
nothing about the mechanism is new. What is new is one entry on
[ADR 0071](0071-derived-codecs.md) § 1's closed list: the compiler acts on the attribute only when its name
**resolves** to `Core\Route`, so a userland `type Route = {...};` is not it however it is spelled, and a
framework carrying its own `Route`-shaped literal is not it either. That list's roster lives in
[ADR 0071](0071-derived-codecs.md) § 1 and is not restated here — a count kept in two places is a count that
goes stale ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 9).

- **`method` is an enum case** ([ADR 0063](0063-core-api-conventions.md) R11), not a string, and an enum
  case is one of the three things an attribute payload may contain
  ([ADR 0046](0046-attributes-shape-literal-metadata.md) § 2). `Core\Http\Method` is the same enum
  `Core\Request::method` returns.
- **The attribute is repeatable** ([ADR 0046](0046-attributes-shape-literal-metadata.md) § 3), which is how
  one method serves two verbs. No `methods: array<Method>` field, no union — the existing rule covers it.
- **Methods only.** No class-level prefix attribute: a prefix is a framework opinion, it interacts badly
  with inheritance, and it makes a route's path unreadable at the line that declares it.
- **`name` is optional and never derived.** A route without one is matchable but not reverse-generatable.
  Deriving a default from the class and method name would make `url()` break silently on a rename, which is
  one of the three failures this ADR exists to turn into compile errors.
- The annotated method may be `static` or an instance method, and may take parameters the path does not
  name — a `Request`, a service, anything. The router ignores them; they are the framework's business.

### 2. The path grammar

A path is a literal `string`, and it is validated during checking:

- **`{name}`** captures one whole segment.
- **`{name?}`** captures one whole segment **or none**, is permitted **only in the last position**, at most
  once, and never in the same path as a `{name...}`. **Its method parameter must have a default**, which is
  what makes the absent case well-typed rather than nullable by accident; a `{name?}` bound to a parameter
  without one is a compile error naming both. `/posts/` does **not** match `/posts/{page?}` — an empty final
  segment is not an absent one, which is
  [ADR 0095](0095-ambiguous-input-is-refused-never-repaired.md)'s never-repair rule and the same reading
  [0097](0097-development-server-and-proxied-origin.md) § 7 gives the trailing slash
  ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 4).
- **`{name...}`** captures every remaining segment as one `tainted string`, is permitted **only in the last
  position**, and at most once.
- Everything else is a literal segment, compared byte for byte and case-sensitively
  ([ADR 0062](0062-case-sensitivity-is-a-compiler-property.md)).

`{name}` rather than `:name`, for three reasons and not one: `:` is a legal character inside a URL path
segment (`/a:b` is a valid path), so `:name` needs an escape rule the braced form does not; the braced form
is what OpenAPI, Laravel, Symfony, axum and ASP.NET all use, so it is the spelling a reader arrives with;
and it leaves room for an inline constraint later (`{id:uint}`) with no ambiguity, should one ever be wanted.

**Precedence is structural, not declaration-ordered**: within one method, a literal segment beats a
`{name}` capture, which beats a `{name?}`, which beats a `{name...}` catch-all. So `/users/new` and
`/users/{id}` coexist with no ordering rule to remember, and moving a declaration between files cannot change
which route wins. That is the radix-trie rule `matchit` implements and it is the reason a route table is
order-independent at all. A `{name?}` is one node marked terminal, so it costs a static hit's walk rather
than a second one.

### 3. A parameter's type comes from the method, and that is what launders it

Every `{name}` in a path must correspond to a parameter of the same name on the annotated method
([ADR 0029](0029-identifier-casing-is-checked.md)'s casing rules apply, so the comparison is exact).
**A `{name}` with no such parameter is a compile error** naming both. The reverse is fine: a method
parameter the path does not name is simply not the router's.

The parameter's declared type is what the segment is converted to during matching, and the conversion must
be one of: `string`, `uint`, `int`, `decimal`, `Core\Uuid`, an enum
([ADR 0010](0010-enums-are-a-value-type.md), matched on its case names), a **union of `string` or `int`
literal types**, or a **subset of an enum's cases** ([ADR 0047](0047-literal-and-enum-case-types.md) — so
`show("en"|"de"|"fr" $lang)` narrows a capture to a closed set with no grammar of its own, and
`/xx/…` simply does not match). Anything else is a compile error at the parameter. **A regex constraint is
not among them and never will be**: an application-authored pattern over the request path runs before any
rate limiting, which makes catastrophic backtracking an unauthenticated denial of service, and
[0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 5 records the
three ecosystems that shipped it and the CVEs they got. Two consequences fall out, and both are the good
kind:

- **A failed conversion is not a match.** `/users/abc` against `show(uint $id)` does not match that route;
  matching continues, and if nothing else matches the result is a `404`. This is the correct behaviour and
  it is what every framework has to write by hand as a `\d+` requirement on the placeholder.
- **A converted parameter arrives unqualified.** [ADR 0024](0024-taint-tracking-for-injection-sinks.md) § 2
  already establishes that a checked conversion launders — the value provably has the shape its type claims
  — so `uint $id` is a plain `uint`, while `string $slug` and every `{name...}` capture stay
  `tainted string` because nothing about them was checked. No new sink, no new launderer, no new rule; this
  is the existing one arriving somewhere useful.

**Two routes with the same `method` and the same path *shape* are a duplicate-route compile error**, naming
both sites, regardless of their parameter types. A type narrows matching at run time; it never disambiguates
two declarations, because a rule under which it did would make matching depend on declaration order after
all. **Duplicate `name` values are a compile error the same way, with one exception**: repeated `#[Route]`
attributes on **one method** may share a `name` when they also share a `path`, which is how a route
answering several verbs stays one thing to `url()` and to
[0076](0076-observability-export.md) § 1's `route` label
([0110](0110-one-methods-repeated-routes-share-a-name-when-they-share-a-path.md) § 1). Two methods sharing
a name, and one method whose repetitions differ in `path`, remain errors.

### 4. The API, and where it stops

```php
Core\Request::route(): ?Router\Match;                                   // the match the server made
Core\Router::match(Http\Method $method, tainted string $path): ?Router\Match;
Core\Router::methodsFor(tainted string $path): array<Http\Method>;      // [] ⇒ 404, else 405 + Allow:
Core\Router::url(string $name, array<string, mixed> $params): string;         // launder (URL path)
Core\Router::urlAbsolute(string $name, array<string, mixed> $params): string; // launder (URL)
Core\Router::urlSigned(string $name, array<string, mixed> $params,
                       {keys: array<secret bytes>, until: ?Time\Instant}): string;   // launder (URL path)
Core\Router::signedRoute(array<secret bytes> $keys): Router\Match;            // or throws
```

```php
Core\Router\Match — readonly name: ?string, params: {…}, method: Http\Method, access: {…}
```

- **The server matches once, before the handler, and `Core\Request::route()` is that match**
  ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 1). It is
  what [0096](0096-a-route-without-a-declared-access-decision-does-not-compile.md) § 4's CSRF check and
  [0076](0076-observability-export.md) § 1's `route` label read, so a request makes one match rather than
  two. `match` remains for matching some *other* path. **Matching is still not dispatching** — the refusal
  list below is unchanged.
- **`match` returns a name and typed parameters, and nothing invocable.** No `->invoke()`, no callable, no
  class-and-method strings. Invoking would be dispatch, which is § 4's whole boundary; and a first-class
  reference to the matched method would need `callable` to carry a signature, which
  [ADR 0007](0007-explicit-type-system.md) § 3 defers. **Only the second of those two grounds would fall to
  typed `callable`, so routing is not a forcing case for that deferral** — routes do not share a signature,
  handlers share no return type, and the boundary ground survives regardless
  ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 9, which
  withdraws the claim this ADR originally made here).
- **`methodsFor` tells a missing path from a refused verb.** Empty means no route claims the path — `404`.
  Non-empty means it is claimed under other verbs — `405`, and the list is the `Allow:` header RFC 9110
  requires. It is called only after `match` has returned `null`, so a served request pays nothing, and it is
  what lets an application answer a plain `OPTIONS` that
  [0097](0097-development-server-and-proxied-origin.md) § 7 had to pass through. Where a `{name?}` makes a
  node terminal, both forms report the same verbs.
- `params` is a shape typed from the matched route's declared parameters. Because `match` may return any
  route in the table, reading a field is guarded by `name` — an ordinary discriminated read, and the reason
  a framework built on this writes one `switch` and not several.
- **`url` is a launderer for the URL-path sink** in
  [ADR 0024](0024-taint-tracking-for-injection-sinks.md) § 3's existing shape: it percent-encodes each
  substituted value, so a `tainted` parameter produces a plain `string` path that is safe *as a path* and
  for nothing else. It also **prepends the request's mount prefix**
  ([0097](0097-development-server-and-proxied-origin.md) § 3), which is what lets one compiled table serve
  the same module at `/ModuleA`, at `/ModuleB` or at `/` — and is why link generation must go through this
  member rather than concatenating a declared path. A literal `$name` that is not a declared route, and a `$params` array that does not
  cover the route's captures, are **compile errors**; a computed `$name` throws.
- **A key that is not a capture becomes a percent-encoded query string**, so a link with `?page=2` has a
  laundered spelling instead of driving the caller to concatenate. A literal key that is neither a capture
  nor one of the route's declared `#[Query]` parameters is a **compile error** in the same shape as the
  unknown-name one, so a typo cannot silently become a query parameter.
- **`urlAbsolute` prepends a configured origin, never a sniffed one** — per mount, falling back to
  `[app] origin`, `System`-class and `Reload`-able
  ([0097](0097-development-server-and-proxied-origin.md) § 3). It exists because
  [0097](0097-development-server-and-proxied-origin.md) § 6 refuses to derive an origin from `Host` or
  `X-Forwarded-Host` — that is host-header injection, and an emailed link is where it lands — so the only
  safe absolute URL is a configured one. A unit calling it under a mount that resolves no origin is a boot
  error.
- **`urlSigned` and `signedRoute` sign a route's *identity*, which a path cannot.** They exist for one
  property and not for convenience: [0097](0097-development-server-and-proxied-origin.md) § 3 lets one
  compiled table serve at `/ModuleA`, at `/ModuleB` or at `/`, so a signature over an assembled path
  stops verifying the moment a mount moves, while one over the route name and its typed parameters
  survives. `signedRoute` verifies against the match the server already made
  ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 1)
  rather than re-parsing anything, and answers that `Router\Match` or throws — **the application calls
  it, at whatever place it keeps**, because nothing here dispatches and nothing here renders a refusal.
  Everything else about signing, including `$uri->sign` for the case that is not a route at all, is
  [ADR 0146](0146-a-signature-is-over-a-payload-and-a-url-is-a-payload-core-uri.md).

**What it deliberately does not do**, because each is a framework opinion and the user confirmed the point
of stopping short:

- **No dispatch.** Nothing calls the matched method.
- **No controller convention.** The attribute attaches to any method on any class; there is no base class,
  no naming rule and no directory layout.
- **No return-value-to-response rule.** What a handler returns is between it and whoever called it.
- **No middleware, no filters, no groups, no route-level rate limit** — the last one specifically, because
  [ADR 0075](0075-core-ratelimit.md) § 4 declines to decide what a limit *does* for the same reason.
- **No `HEAD`-implies-`GET`, no automatic `OPTIONS`, no trailing-slash normalisation.** Each is a real
  convention and each is somebody's wrong default. The M7 server decides them above this table, visibly, and
  [0097](0097-development-server-and-proxied-origin.md) § 7 is where it did: `HEAD` runs as `GET` with the
  body discarded — and `Core\Request::method()` reports `Get`, so a `Get`-only table still matches — a CORS
  preflight is answered from `[http.cors]` before any handler runs, and a trailing slash is never
  normalised.

### 5. The table is built by the same scan `implementing<T>()` uses

Finding routes is the same question [ADR 0061](0061-compile-time-autoload-and-program-discovery.md) § 3
built its program enumeration for — *what exists that nothing names* — and it reuses it wholesale, including
its consequences:

- **A program containing no `Core\Router::match`/`::url` call performs no scan and builds no table**, the
  identical opt-in rule 0061 § 3 states for a discovery query. A program with no `#[Route]` anywhere pays
  nothing at all, including no pass.
- **The scan makes the compiled unit depend on directory contents**, so every directory listed joins the
  revalidation set and the sorted route list hashes into the cache key — exactly
  [ADR 0061](0061-compile-time-autoload-and-program-discovery.md) § 5, with no new dependency kind and no
  new directive.
- Routes declared in files reached by an ordinary `require` are included too; they are already in the graph.
- The table is emitted into the compiled unit as a radix trie
  ([ADR 0042](0042-on-disk-artifact-cache-format.md)), sorted by path so it is byte-deterministic across
  builds. Cost is **O(routes in compiled code)** in the artifact
  ([ADR 0004](0004-memory-for-simplicity.md)), not per request and not per object; a match is a trie walk
  that allocates nothing for a route with no captures.

### 6. `route` is also a metric label

[ADR 0076](0076-observability-export.md) § 1 labels its request series with a route's **declared name**,
never the raw request path, because a path is unbounded and a name is a closed set known at compile time.
That is the one place this table is read by something other than the application, and it is why a program
without one gets no `route` label rather than a cardinality bomb.

## Consequences

**Positive**

- **Three runtime routing bugs become compile errors** — a duplicate route, an unbound placeholder, a stale
  `url()` name. Each is currently found by a `404`, a `null`, or a user clicking a broken link in an email.
- **No route cache to warm and no cache to go stale.** Symfony and Laravel both generate a routing cache
  file precisely because the scan is compile-time work; here it is compile-time work, keyed and invalidated
  by machinery [ADR 0042](0042-on-disk-artifact-cache-format.md) and
  [ADR 0061](0061-compile-time-autoload-and-program-discovery.md) already built.
- **The `\d+` requirement everybody writes is a type.** `show(uint $id)` both narrows matching and produces
  an unqualified `uint`, from one declaration that was going to exist anyway.
- **Stopping at matching is what makes it adoptable.** A framework can build dispatch on top, keep its own
  router and use `url()` alone, or ignore all of it — and none of those choices is penalised.
- **`url()` is a real launderer**, so building a link from user-supplied data has a correct spelling that a
  reviewer can see.
- **Two ADRs pay off here at once.** [ADR 0071](0071-derived-codecs.md)'s nominal-matching rule is what
  stops a framework's own `#[Route]` literals being double-registered, and it cost one sentence there
  instead of a breaking change after M9.

**Negative**

- **`match` returning a name means a `switch`.** A framework built on this writes a dispatch table mapping
  names to behaviour, which is more code than a router that invokes. This is the boundary working as
  intended, and it goes away if typed `callable` signatures ever land.
- **A compile-time table means a route cannot be added at run time.** A CMS with database-defined URLs
  cannot use this at all and must match its own way — which is fine, and is exactly the "use none of it"
  case, but it is a real limit worth stating.
- **The scan is opt-in but not free.** One `Core\Router::match` call makes the compiled unit depend on
  directory listings, with the revalidation cost
  [ADR 0061](0061-compile-time-autoload-and-program-discovery.md) § 5 bounds. Zero under
  `validate = never`, which is what production runs.
- **A cross-module route needs that module compiled.** The same consequence
  [ADR 0061](0061-compile-time-autoload-and-program-discovery.md) already records for a hard cross-module
  reference, arriving in a second place.
- **`{name}` will annoy the people who wanted `:name`**, and the reverse. § 2 gives three reasons; one of
  them is arbitrary-looking and the decision still had to be made.
- **No `HEAD`/`OPTIONS`/trailing-slash convention** means the server layer has to decide, and two
  deployments may decide differently. Deliberate, and it is a place where "no opinion" costs the user an
  opinion.
- **A second compiler-recognized attribute**, one ADR after the first. [ADR 0071](0071-derived-codecs.md)'s
  *Consequences* already named this exact risk — cheap to add is how an attribute set becomes an
  undocumented second language surface — and the closed list only works if every entry is argued.

## Alternatives rejected

- **A runtime registry** (`$router->get("/users/{id}", …)`), the PHP status quo. Rejected: it moves all three
  compile-time checks to runtime, needs a generated cache file to be fast, and the registration order
  becomes semantically load-bearing.
- **Structural attribute matching**, consistent with [ADR 0046](0046-attributes-shape-literal-metadata.md)
  § 4. Rejected by [ADR 0071](0071-derived-codecs.md) § 1 already: a framework's own `Route`-shaped literal
  would register routes Novis does not own, and the fix after M9 would be a breaking change to attribute
  retrieval.
- **`:name` placeholders.** Rejected in § 2: `:` is legal inside a path segment, so it needs an escape rule,
  and the braced form is what every neighbouring ecosystem already uses.
- **Declaration-order precedence** (first match wins), which is what a linear route list gives. Rejected: it
  makes moving a declaration between files a behaviour change, and it is precisely why framework routing
  files must be read top to bottom to be understood.
- **Parameter types disambiguating two routes at the same path** — `/{id}` on `uint` and `/{slug}` on
  `string` coexisting. Tempting, and rejected in § 3: it reintroduces order-dependence through the back door
  and makes the duplicate check partial rather than total.
- **`match` returning something invocable.** The obvious ergonomic win. Rejected on both halves of § 4: it is
  dispatch, and it needs a typed `callable` Novis does not have.
- **Deriving `name` from the class and method.** Convenient. Rejected: it makes `url()` break silently on a
  rename, which is one of the three bugs this ADR exists to catch.
- **A class-level `#[Route(path: "/admin")]` prefix.** Familiar from Symfony. Rejected in § 1: it interacts
  with inheritance, and it makes a method's real path unreadable at the line that declares it.
- **Route groups, middleware attachment, or a per-route rate limit** in the attribute. Rejected in § 4: each
  is a dispatch opinion, and this table has no dispatch to attach them to.
- **A route table in `nvs.toml`.** Rejected on lifetime, the same way
  [ADR 0073](0073-scheduled-work-is-config.md) accepted it for schedules and this rejects it for routes: a
  schedule is deployment state and a URL is source state — the code that handles a path and the path itself
  change together, in the same commit.
- **Building the table from an inert `Core\Ast` walk at runtime**
  ([ADR 0019](0019-reflection-and-ast-parsing-are-core-features.md)). Possible today. Rejected: it parses
  the program again inside a request, gets none of the three compile-time checks, and puts
  `nvs-syntax` on the request path for something the compiler already knows.

## Revisiting

- **Typed `callable` signatures** ([ADR 0007](0007-explicit-type-system.md) § 3) would *not* remove § 4's
  `switch`, so routing is not a case that forces the deferral: routes do not share a signature, handlers
  share no return type, and § 4's boundary ground stands whichever way that deferral goes. The `switch` is
  generated by `Web\Controller` instead ([0082](0082-the-first-party-framework.md) § 3), and the deferral
  is owed its own ADR argued on the three cases that do force it
  ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 9).
- **An inline constraint** (`{id:uint}`) is **not** taken, and the grammar room § 2 reserved stays reserved.
  A capture's closed set is spelled as a literal-union type on the parameter (§ 3), which states the fact
  once rather than in two places that can disagree.
- **Host and scheme matching** is answered one layer down, at
  [0097](0097-development-server-and-proxied-origin.md) § 3's mount table, and `#[Route]` carries **no
  `host` field** — a hostname in the compiled table would end the relocatability that section bought, and a
  path is source state where a hostname is deployment state. `Core\Request::mount()` exposes the matched
  mount's glob captures, which is what a multi-tenant application actually needs
  ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 7).
- **A second selector for [ADR 0061](0061-compile-time-autoload-and-program-discovery.md)'s scan** — that
  ADR's *Revisiting* says a selector beyond `implementing<T>` should wait for a second real use case. This
  is that second case, and it argues for the scan's filter becoming a shared, named mechanism rather than
  two hand-written passes over the same enumeration.

## Verification

- **M4S** (the checker/IR pass, alongside `#[Json\Derive]`'s): `#[Route]` is matched nominally — a userland
  `type Route = {...};` on a method produces no entry, `#[Core\Route]` and a `use`d `#[Route]` are the same
  attribute, and a bare `#[{path: …}]` literal produces none.
- **M4S:** each of the three failures is a compile error naming both sites — two routes with the same method
  and path shape, a `{id}` with no `$id` parameter, two routes with the same `name`. Plus: a `{name...}` not
  in the last position; two catch-alls in one path; a captured parameter whose type is not one of § 3's.
- **M4S:** `Core\Router::url("user.show", {id: 7})` with a literal name folds; an unknown literal name is a
  compile error; a `$params` array missing a capture is a compile error; a computed name throws at run time.
- **M4S:** a program with no `Core\Router` call performs no scan and emits no table, asserted the same way
  [ADR 0061](0061-compile-time-autoload-and-program-discovery.md)'s "a program with no query performs no
  scan" already is.
- **M7:** `/users/new` and `/users/{id}` both declared, the literal wins, and reversing their declaration
  order changes nothing — the order-independence claim.
- **M7:** `/users/abc` against `show(uint $id)` does not match and falls through to a `404`, rather than
  matching and throwing; `/users/7` matches and `params.id` is a plain `uint`, while a `string $slug`
  capture and every `{name...}` capture are `tainted` — checked by a fixture that would not compile if the
  qualifier were wrong.
- **M7:** `Core\Router::url` percent-encodes a `tainted` parameter and returns an unqualified `string`,
  asserted against a value containing `/`, `?` and `#`.
- **M6/M7:** adding a file declaring a new route invalidates the cache with no existing source file
  modified, and a directory listing that changes without changing the route set recompiles nothing — the
  same pair of assertions [ADR 0061](0061-compile-time-autoload-and-program-discovery.md) § 5 already
  requires for a discovery query.
- **M7:** a matched route's `name` reaches [ADR 0076](0076-observability-export.md)'s `route` label, and a
  program with no route table emits the request series with no `route` label rather than with a path.
