# ADR 0019 — Reflection and AST/source parsing are first-class `Core` features, not aftermarket extensions

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** whether a running MWL program can introspect its own compiled program (classes, methods,
  properties, constants, attributes) and whether it can parse MWL/PHP source text into a structured AST at
  runtime; the shape both take as `Core` domain classes; what each is and is not allowed to do
- **Relates to:** [0007](0007-explicit-type-system.md) (the AST is typed data, not `array<mixed>`),
  [0011](0011-functions-and-constants-are-class-members.md) (both land as `Core` domain classes, following
  its domain-class shape and its illustrative-roster precedent), [0013](0013-comparable-interface.md) and
  [0014](0014-property-observer.md) (reflective member access runs the same checks ordinary access does —
  see § 2), [0010](0010-enums-are-a-value-type.md) (`::cases()` is already a narrow, enum-specific piece of
  this; § 4 explains why that does not conflict), [0006](0006-isolated-script-execution.md) (neither feature
  crosses an isolation boundary or grants ambient authority, so neither needs a capability grant)

> **In short:** PHP ships reflection (`ReflectionClass` and friends) as a built-in extension, but has no
> in-language AST facility at all — `token_get_all()` returns a flat token list, not a tree, and a real AST
> needs a userland library (`nikic/php-parser`) or a PECL-only extension (`ast`) most installs don't even
> have. MWL requires both to be first-class, built into `Core`, with no extension to install: **`Core\Reflect`**
> gives read-only structural introspection over a program's own classes, interfaces, enums,
> functions, properties, constants, attributes and parameters; **`Core\Ast`** exposes the exact lexer and
> parser `mwl-syntax` already uses to compile a file, so any `.mwl` program can parse a string or a file into
> a typed AST value at runtime, not just at `mwl`-toolchain time. Two invariants keep both safe under the
> priority order without becoming PHP's reflection, which doubles as a privilege-escalation tool via
> `setAccessible(true)`: a reflective call or property access runs through the *same* visibility and hook
> checks ordinary code would face at that call site (§ 2), and a parsed AST is inert typed data with no path
> back into execution — `eval` does not exist and stays rejected, so returning a tree is not a second `eval`
> in disguise (§ 3).

## Context

- Two gaps: reflection is native and mature in PHP; AST/source parsing is not — `token_get_all()` gives
  tokens, not a tree, and a real tree needs a third-party grammar (`nikic/php-parser`, or the PECL-only `ast`
  extension) that can drift from the engine's own parsing rules.
- Requirement: MWL closes both gaps as core-language features, not "reflection ships, AST parsing is
  somebody's extension."
- Correctness (priority 2): `Core\Ast` wraps `mwl-syntax`'s existing single parser rather than adding a
  second grammar to keep in sync — the same "one implementation, not two" shape
  [ADR 0015](0015-no-name-aliasing.md)/[ADR 0017](0017-hot-reload-without-restart.md) already chose.
- Simplicity (priority 4): every domain is meant to have one obvious `Core` home
  ([ADR 0011](0011-functions-and-constants-are-class-members.md)); leaving AST parsing out would send every
  framework author back to a userland parser, reproducing PHP's own fragmentation.
- Security (priority 1) is what this decision could spend if left unexamined: PHP's reflection bypasses
  visibility via `setAccessible(true)`, and a runtime-reachable parser is new attacker-reachable surface —
  both closed in *Decision* rather than inherited by default.

## Decision

**`Core\Reflect` and `Core\Ast` are built-in `Core` domain classes, present in every MWL program with no
extension to install, following [ADR 0011](0011-functions-and-constants-are-class-members.md)'s domain-class
shape.** The exact class roster in each namespace is stdlib design due at M8, the same way
[ADR 0011](0011-functions-and-constants-are-class-members.md) left its own roster to M2/M8 — what this ADR
fixes is the shape and the two invariants below, not the final member list.

### 1. `Core\Reflect`: read-only structural introspection

Illustrative shape, mirroring PHP's `Reflection*` family under the reserved namespace rather than one
grab-bag class: `Core\Reflect\ClassInfo`, `MethodInfo`, `PropertyInfo`, `ParameterInfo`, `ConstantInfo`,
`AttributeInfo`, `EnumInfo`, reachable from a value or a class name (`Core\Reflect\ClassInfo::of(User::class)`
or `Core\Reflect\ClassInfo::of($someObject)`). Covers classes, interfaces, enums (name and cases — see § 4),
methods, properties, constants, parameters and attributes — including which methods an interface declares as
`public` (part of its contract) versus `private` (an internal helper, per
[ADR 0043](0043-interface-default-methods-and-delegation-replace-traits.md)) — matching what PHP's own
extension covers today, minus traits, which do not exist.

### 2. No `setAccessible(true)` — reflective access enforces the same checks ordinary code would

**Reading metadata** (a member's existence, name, declared type, visibility, attributes, doc comment) is
always visible through `Core\Reflect`, regardless of the member's visibility — this is introspection of the
program's *shape*, not of a running value, and PHP's reflection makes the same call.

**Acting on a member** — calling a reflected method, reading or writing a reflected property, invoking a
reflected constructor — is different: it runs through *exactly* the visibility check and, where one is
declared, the `PropertyObserver` hook ([ADR 0014](0014-property-observer.md)) that ordinary code at that call
site would face. A reflective call from outside a class to one of its `private` methods fails the same way
an ordinary out-of-class call would. **There is no `setAccessible(true)` and no equivalent** — PHP's escape
hatch for reaching a private member from anywhere is rejected outright, not merely left undocumented, because
it is a structural privilege-escalation path priority 1 does not get to spend on convenience.

### 3. `Core\Ast`: a runtime door onto `mwl-syntax`'s own parser, returning inert typed data

`Core\Ast::parse(string $source): Core\Ast\Node` and `Core\Ast::parseFile(string $path): Core\Ast\Node` — the
name is illustrative, the mechanism is not: **both call directly into the same lexer and parser the compiler
itself runs**, so a construct that parses when `mwl` compiles a file parses identically when a running
program calls `Core\Ast::parse()` on the same text, and a rejected construct is rejected identically in both
places. There is no second grammar implementation anywhere in this project.

The return value is a **typed** node tree — `Core\Ast\ClassDecl`, `Core\Ast\MethodDecl`, and so on, one type
per production the same way `mwl-syntax`'s own AST is typed — never `array<mixed>` or a stringly-keyed
associative structure. [ADR 0007](0007-explicit-type-system.md) already rejects `mixed` as anything but the
one deliberately unchecked position in the language; returning the parse tree as untyped data would be
exactly the shortcut `token_get_all()` takes, reintroduced at the one place a fully-typed alternative is
easiest to give.

**A parsed tree is inert. There is no path from an AST value back into execution.** `eval` does not exist in
MWL and stays rejected outright — a string has no stable identity, no cache key, and no
capability-grantable path (see [the spec](../spec/00-overview.md) § 2, and
[ADR 0006](0006-isolated-script-execution.md) *Alternatives rejected*). `Core\Ast::parse()` does not weaken
that: it hands back a value a program can walk, print, or rewrite into a new source string to hand to a
human or a file — never a way to run what it describes. Shipping `Core\Ast` is therefore not "`eval` under a
different name"; it is the same rejection restated for a second time this project needed it stated.

### 4. Enums: `::cases()` stays the special case it already is

[ADR 0010](0010-enums-are-a-value-type.md) already gives enums one deliberately narrow reflective surface —
`::cases()` — while explicitly declining to give an enum "a class's worth of machinery," reflection included.
`Core\Reflect\EnumInfo::of(Status::class)` does not reopen that: it reports the same shape `::cases()` already
exposes (the type's name and its closed case list) as ordinary structural metadata, the same as `ClassInfo`
reports a class's shape. It grants an enum no method dispatch, no interface, no identity it didn't already
have — the two are consistent, not competing, descriptions of the same closed integer type.

### 5. Neither feature is capability-gated

Both are pure in-memory operations over a program's own compiled shape or its own supplied string — neither
touches the filesystem, the network, or another process, so neither needs an `mwl.ini` capability grant the
way `Core\IO` or process execution do ([ADR 0005](0005-config-changeability.md),
[the plan](../implementation-plan.md) M8). `Core\Ast::parse()` on a string is exactly as ambient-authority-free
as `Core\Json::decode()` on one.

## Consequences

**Positive**

- One parser, two call sites (`mwl` toolchain, running program) — a rejected construct is rejected
  identically everywhere, unlike PHP's engine-parser/userland-parser split.
- A migrated framework's DI container, ORM hydration, or attribute-driven router gets the reflection surface
  it already expects, with no new privilege-escalation primitive PHP's `setAccessible(true)` gave it.
- `Core\Ast`'s typed tree makes a source-rewriting tool (a linter, a codemod, `mwl fmt` itself) a program any
  MWL user can write, not a capability reserved for the toolchain's own Rust code.

**Negative**

- `Core\Ast::parse()` makes `mwl-syntax`'s lexer and parser **attacker-reachable from inside a live request**
  the moment a handler passes user-supplied text to it — a materially different trust boundary than "the
  deployer's own `.mwl` files on disk," and one the M1 fuzz-hardening effort (already tightened once, per
  the switch-parsing bounded-growth fix on this branch) must be held to as an ongoing runtime obligation, not
  a one-time compile-time bar.
- Reflective method calls and property access are one more code path that must reach the exact same
  visibility/hook logic ordinary call sites use, rather than a shortcut around it — implemented as a shared
  check both paths call into, not duplicated logic that could drift.
- The stdlib now carries two more domain-class families to design at M8, on top of the roster
  [ADR 0011](0011-functions-and-constants-are-class-members.md) already deferred there.

## Alternatives rejected

- **Ship reflection, defer AST parsing to a userland or extension-provided library**, matching PHP's own
  split. Rejected: precisely the asymmetry that produces grammar drift in PHP's ecosystem, at zero marginal
  cost to avoid via `Core\Ast`.
- **PHP-equivalent `setAccessible(true)`.** Rejected per § 2: a structural visibility bypass is a cost
  priority 1 doesn't get to spend for convenience, especially given the [ADR 0014](0014-property-observer.md)
  hook it would skip silently.
- **Return the AST as `array<mixed>`** (PHP's `ast` extension/`token_get_all()` shape). Rejected per § 3: the
  exact `mixed`-shaped shortcut [ADR 0007](0007-explicit-type-system.md) closes everywhere else.
- **A `Core\Ast::eval()`-style convenience.** Never seriously considered: `eval` with extra ceremony, already
  rejected by [ADR 0006](0006-isolated-script-execution.md) for reasons that apply unchanged to a tree.

## Revisiting

- **The exact `Core\Reflect`/`Core\Ast` class rosters and method signatures** are M8 stdlib design, not this
  ADR; the names above are illustrative, following the precedent
  [ADR 0011](0011-functions-and-constants-are-class-members.md) already set for its own roster.
- **How a request's CPU-time cap applies to a long-running native call inside `Core\Ast::parse()`** — whether
  it is preempted by the same safepoint mechanism JIT-compiled loops poll, or needs its own internal
  yield/budget check — is an M6/M8 mechanism question this ADR flags but does not resolve. Whatever the
  answer, the parser itself must stay resource-bounded (linear time and space in input length) regardless of
  how preemption is ultimately wired, the same bound the recent switch-parsing fix already establishes as
  the pattern for any new grammar surface.
- **Whether reflective construction (building an object via `Core\Reflect` without calling a visible
  constructor) needs its own rule beyond § 2's "same checks as ordinary code."** Likely answer is that
  constructing still goes through the constructor and its visibility check like any other call, with no
  special case — but this is worth pinning down explicitly when M8 designs the actual API rather than
  assumed here.

Verification, in the order it becomes possible:

- **M1**: no change — `mwl-syntax`'s lexer/parser is already the single implementation `Core\Ast` will wrap;
  its fuzz corpus and the switch-parsing bounded-growth fix already establish the resource-bound precedent
  § 3 and *Revisiting* rely on.
- **M8**: `Core\Reflect` and `Core\Ast` land as real domain classes. Verify: a reflective call to a `private`
  method from outside its class fails identically to the equivalent ordinary call; `Core\Ast::parse()` on a
  string accepted by `mwl ast` produces the same tree shape (round-trip/snapshot-style test against the
  existing `mwl-syntax` `insta` snapshots); `Core\Ast::parse()` on a string `mwl ast` rejects fails
  identically; fuzzing `Core\Ast::parse()` with the same corpus as the M1 lexer/parser fuzz target finds no
  panic and no unbounded growth.
