# ADR 0096 — A route without a declared access decision does not compile

- **Status:** Accepted
- **Date:** 2026-08-25
- **Scope:** decides that every `#[Route]` method carries a sibling `#[Access]` attribute, that an omission
  is a compile error, and that CSRF enforcement is on by default for unsafe methods. Does **not** decide
  what an access name *means*, who evaluates it, or how a session is established — those are the M7 server's
  and `mwl/web`'s ([0082](0082-the-first-party-framework.md)'s `Web\Auth`). Does **not** add a field to
  `#[Route]`, and does **not** introduce middleware, filters, groups or a dispatch opinion of any kind
  ([0077](0077-compile-time-routing.md) § 4 stands, unamended). Does **not** address object-level
  authorisation — see *Consequences*.
- **Amends:** nothing. [0077](0077-compile-time-routing.md) is extended by a **sibling attribute**
  precisely so its § 4 does not have to be reopened.

## Context

MWL's security investment is aimed at injection. `tainted` and `secret`
([0024](0024-taint-tracking-for-injection-sinks.md), [0033](0033-secret-qualifier-for-confidential-values.md)),
the fail-closed sink default ([0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md)), prepared
statements only ([0067](0067-core-db.md)), no `include`, no stream wrappers, no `eval`
([0021](0021-single-file-inclusion-construct.md), [0052](0052-closed-doors.md)) — between them these delete
SQL injection, local file inclusion and the whole `php://filter` technique, and they make XSS hard to write
by accident.

Two 2025 figures put that work in proportion. Of vulnerabilities **discovered** in PHP applications, XSS was
34.7%, CSRF 19%, LFI 12.6%, broken access control 10.9% and SQLi 7.2%. Of attacks actually **blocked in the
field**, broken access control was **57%**, privilege escalation **20%**, LFI 10%, SQLi 5% — and XSS **1%**.

MWL structurally deletes LFI and SQLi, and aims most of its remaining machinery at the class that is 1% of
exploitation. The class that is 57% has no mechanism at all: a handler that forgets its permission check
compiles, serves, and produces no diagnostic at any stage.

[0077](0077-compile-time-routing.md) already builds the route table while compiling, and already makes a
duplicate route, an unbound placeholder and a stale `url()` name compile errors. What it deliberately does
not do is attach behaviour: *"No middleware, no filters, no groups, no route-level rate limit — each is a
dispatch opinion, and this table has no dispatch to attach them to."* That reasoning is correct and this ADR
does not disturb it.

The distinction it leaves room for: **a required declaration is not dispatch.** The table already carries a
path it does not dispatch. [0085](0085-openapi-is-generated-from-the-route-table.md) already has the
compiler validate metadata on that table — an `#[Api]` attribute contradicting the code is a compile error —
without the compiler ever calling anything.

## Decision

### 1. `#[Access]` is a required sibling of `#[Route]`

A method carrying `#[Route]` must also carry `#[Access]`. A `#[Route]` without one is a **compile error**
naming the method, and suggesting `#[Access(Public)]` for a route that is genuinely open.

```php
#[Route(method: Get, path: "/admin/users")]
#[Access(Role::Admin)]
public fn listUsers(): Response { … }

#[Route(method: Get, path: "/")]
#[Access(Public)]                       // written, because an omission is not a default
public fn home(): Response { … }
```

`#[Route]` gains **no field**. Two attributes rather than one is the whole mechanism by which
[0077](0077-compile-time-routing.md) § 4 stays true: that ADR rejected attaching *behaviour* to its
attribute, and this attaches a *declaration* to the method instead.

### 2. The compiler checks presence and resolution, and nothing else

It verifies that the attribute is there and that the name inside it resolves — an enum case, a class
constant, whatever the application declares, checked structurally like any other attribute
([0046](0046-attributes-shape-literal-metadata.md)). It never asks what the name means, never calls
anything, and has no opinion about roles, policies or sessions. Interpretation belongs to whoever dispatches:
the M7 server, or `Web\Auth` in [0082](0082-the-first-party-framework.md)'s `mwl/web`.

This is the same division the path already lives under. The compiler knows `/admin/users` is a route's path
and refuses a duplicate; it does not serve it.

### 3. An omission is an error, not a default

There is no implicit `Public` and no configuration that supplies one. The reasoning is
[0094](0094-visibility-is-written-at-every-member-declaration.md)'s, applied to a second kind of
declaration: **the implicit default is the one nobody chose.** A route that is public because its author
decided so and a route that is public because its author forgot are indistinguishable at every later
reading, and the second is 57% of what gets exploited.

The cost is stated plainly: every genuinely public route writes one extra line. That is the same trade
0094 accepted for visibility, and it buys the same thing — the reader of a route never has to ask whether
the absence meant anything.

### 4. CSRF is on by default for unsafe methods

The M7 server refuses a `POST`, `PUT`, `PATCH` or `DELETE` without a valid token, using the route table to
know which handler is which. [0060](0060-application-security-protocols.md) already owns generation and
constant-time verification; this decides only the default.

A route that legitimately needs no token — a webhook receiver authenticated by signature, an API
authenticated by bearer token — says so in its own declaration, named, per route. This follows
[0074](0074-http-defaults-safe-and-finite.md)'s premise that a deployment with nothing configured is
already safe.

Rejected within this section: enforcing only when a session cookie is present. It sounds tighter — CSRF can
only target cookie-authenticated requests — but it makes protection depend on what the client sent rather
than on what the code says, which is harder to reason about and harder to test.

## Consequences

- **Every route costs one more line**, including the public ones. Deliberate, per § 3.
- **`#[Route]` is unchanged**, so nothing built against [0077](0077-compile-time-routing.md) moves, and its
  § 4 needs no amendment.
- **Adding `#[Access]` to a codebase later is a migration**, which is the reason this is decided before M7
  rather than after M16: the attribute is free while the route count is zero and a sweep once it is not.
- **The route table gains one more column that [0085](0085-openapi-is-generated-from-the-route-table.md)
  can read**, so a generated OpenAPI document can carry a security scheme it did not previously know.
- **This addresses the 57%, not the 20%.** Privilege escalation in the field is mostly IDOR — "this row
  belongs to another user" — and no language-level mechanism answers it: the check needs the row. Claiming
  otherwise would be the false confidence
  [0024](0024-taint-tracking-for-injection-sinks.md) exists to avoid.
- **Runtime cost is zero.** Everything here is compile-time; the check a handler performs at run time is one
  it would have performed anyway.

## Alternatives rejected

- **An `access:` field on `#[Route]`.** One attribute reads better and cannot be half-applied. Rejected:
  it puts a field on the attribute [0077](0077-compile-time-routing.md) § 4 and its *Alternatives* both
  argued against, so that ADR would need amending rather than extending — and the line between "metadata
  the table carries" and "a dispatch opinion" gets harder to hold for whatever asks next.
- **A completeness check with no new attribute** — `mwl check` refusing a program where a route is not
  covered by an access map the framework declares. No language surface at all. Rejected: the map is a
  second place to keep in sync with the routes, which is the drift
  [0077](0077-compile-time-routing.md) built a compile-time table to delete, and the diagnostic points at
  the map rather than at the route that is missing.
- **Leave it to `Web\Auth`** ([0082](0082-the-first-party-framework.md)'s arrangement, unchanged).
  Smallest `Core`, purest route table. Rejected on the *Context* figures: it is the arrangement every PHP
  framework already has, and 57% of exploited vulnerabilities in 2025 happened underneath it.
- **A default of `#[Access(Public)]` when omitted.** Ergonomic, and it makes adoption free. Rejected in
  § 3: it is the implicit default whose absence this ADR exists to make visible.
- **CSRF tokens available but enforcement left to the program.** Rejected in § 4: an unprotected
  state-changing route would be indistinguishable from a protected one at compile time *and* at run time,
  which is the same failure this ADR closes for authorisation.

## Verification

- A `.mwlt` case per shape: a `#[Route]` with no `#[Access]` is a compile error naming the method; one with
  `#[Access(Public)]` compiles; one whose access name does not resolve is a compile error naming the name.
- A case asserting a method carrying `#[Access]` and **no** `#[Route]` is accepted — this ADR requires the
  sibling in one direction only.
- A server fixture asserting an unsafe method without a token is refused, that the named opt-out is
  honoured, and that a safe method is unaffected.
- The generated OpenAPI document for a fixture with mixed access levels, frozen, so
  [0085](0085-openapi-is-generated-from-the-route-table.md)'s emitter is pinned against this column.
