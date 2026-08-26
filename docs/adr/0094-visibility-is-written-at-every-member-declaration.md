# ADR 0094 — Visibility is written at every member declaration; there is no implicit `public`

- **Status:** Accepted
- **Date:** 2026-08-25
- **Scope:** whether a class-body member may omit `public`/`protected`/`private`, and what an omission
  means. Covers every member declaration slot — property, class constant and method, in a `class`,
  `interface` or anonymous-class body — plus PHP 8.4's asymmetric `(set)` form. An `enum` body has no
  member slot at all ([ADR 0010](0010-enums-are-a-value-type.md) § 3, `E0220`), so nothing here reaches
  one. Does **not** cover
  what each level *means* at an access site (who may read a `private` property), which is
  [ADR 0007](0007-explicit-type-system.md)'s checker debt owned by `mwl-types`; nor modifier *order*
  ([ADR 0039](0039-canonical-code-formatting.md) § 1); nor casing ([ADR 0029](0029-identifier-casing-is-checked.md));
  nor property-hook semantics ([ADR 0014](0014-property-observer.md)).

> **In short:** every member declaration in a class, interface or anonymous-class body writes exactly
> one of `public`, `protected` or `private`. **There is no default, because there is nothing to default** —
> an omission is `E_MISSING_VISIBILITY`, a hard compile error with no suppression, exactly as a missing type
> is under [ADR 0007](0007-explicit-type-system.md). PHP's implicit `public` (`function f()`, `const X = 1`,
> `var $x`, `static $x`, `readonly int $x`) does not survive; neither does MWL's own looser property grammar,
> which parses a bare `int $x;` today. A **parameter is not a member**: an unmodified constructor parameter
> stays a plain parameter, because visibility is what promotes one to a property, and that stays the marker.
> The asymmetric form is written as a **pair** — `public private(set) string $name;`, never a bare
> `private(set)` with its read side inferred. `mwl fmt` never inserts the keyword; `mwl convert` does, as an
> E-tier rewrite, because PHP's omission provably means `public`.

## Context

- **PHP's answer is uniform and invisible.** Omission means `public` in every slot that permits it — a
  method, a class constant, `var $x`, and any property carrying only `static`/`readonly`. The default is
  legible nowhere in the source; you have to already know the rule to read the file.
- **MWL has no answer at all today.** `Parser::parse_modifiers` takes every modifier "in any combination and
  any order — which modifiers make sense in which position is a later check, not a grammar rule", and that
  later check was never written: `E_BAD_MODIFIER` (E0106) is *defined and never emitted* anywhere in the
  tree. `MethodMember`/`ConstMember` document their modifiers as "Visibility, **if written**". So a
  visibility-less member parses, lowers, and no layer ever assigns it a meaning.
- **MWL's property grammar is looser than PHP's**, by accident rather than decision. PHP requires at least
  one modifier on a property declaration, so `class A { int $x; }` is a parse error there. Here the class-body
  parser reaches its property arm on `can_start_type()`, so `int $x;` parses. The one place PHP was strict,
  MWL currently is not.
- **This trade is already made for types.** ADR 0007's table marks the same slots "PHP syntax, now
  mandatory" for the *type*. A member that must spell `uint` but may leave its visibility to a rule nobody
  wrote is an inconsistency, not a smaller rule.
- **The default is a security default, not a style one.** An inferred `public` is the mechanism by which an
  internal helper becomes public API by omission — priority 1 and 2 in AGENTS.md's ordering, not priority 4.
  The author who forgot the keyword is exactly the author who did not decide.
- **Two accepted ADRs already assume the level was written.** [ADR 0019](0019-reflection-and-ast-parsing-are-core-features.md) § 2
  exposes a member's declared visibility through `Core\Reflect`, and
  [ADR 0028](0028-closing-the-remaining-magic-methods.md) annotates every property in a debug dump with it.
  With a default, both report a level no one typed.
- **[ADR 0043](0043-interface-default-methods-and-delegation-replace-traits.md) made this load-bearing inside
  an interface.** A `public` interface method with a body is a *default method*; a `private` one is an
  internal helper visible only to that interface's own bodies. The slot where PHP could argue "only one
  level is legal, so writing it is noise" no longer exists here.

## Decision

Every declaration below writes exactly one visibility keyword. There is no configuration, no per-project
override, and no suppression annotation — the same terms as ADR 0029.

| Declaration | Required | Example |
|---|---|---|
| Property, instance or `static` | yes | `private static int $calls = 0;` |
| Property with `readonly`/`lateinit` | yes — those are not visibility | `public readonly uint $id;` |
| Class constant | yes | `public const int MAX = 10;` |
| Method, instance or `static` | yes | `protected function normalize(string $s): string` |
| Method in an `interface` body | yes — `public` and `private` differ (ADR 0043) | `public function encode(): string;` |
| Member of an anonymous class body | yes | `new class { public int $n = 1; };` |
| Asymmetric property | yes, **as a pair** (§ 3) | `public private(set) string $name;` |
| Constructor parameter, unmodified | **no** — it is not a member (§ 2) | `function constructor(int $n)` |
| Anything in an `enum` body | **no** — an enum declares only cases (`E0220`) | `case Active;` |
| Property hook (`get`/`set`) | **no** — no visibility slot exists (ADR 0014) | `get => $this->a . $this->b;` |

### 1. An omission is a hard error, with no default to fall back on

A member declaration carrying no visibility keyword reports **`E_MISSING_VISIBILITY` (E0122)**, primary span
on the declaration, with a fix hint offering `public` — the level the same code has in PHP, so accepting the
fix on ported code preserves behaviour. It is an error and not a lint: there is no warning tier in this
repository, and a rule whose whole value is "the author decided" is worth nothing if the author can skip it.

The question "what is the default" therefore has no answer, and that is the point. Nothing in the compiler,
the reflection surface or a reader's head has to hold one.

### 2. A parameter is not a member, and promotion stays the marker

Visibility on a constructor parameter is not decoration — it is the promotion syntax. `function constructor(int $n)`
declares a parameter; `function constructor(public int $n)` declares a property. Requiring a keyword on every
parameter would delete the distinction, so the rule is scoped to **members**: a parameter with no visibility
is a plain parameter, complete and correct, and § 1 does not fire on it. A promoted parameter — one that
*does* carry a visibility — is a member and is already written.

### 3. The asymmetric form is written as a pair

PHP 8.4 lets `private(set) string $name;` stand alone, inferring a `public` read side. That is the same
implicit `public` this ADR removes, wearing a different spelling, so it is refused: the read visibility is
written too — `public private(set) string $name;`. A bare `(set)` form reports the same E0122, whose message
names the pair rather than a bare keyword.

This is the one place MWL's grammar is *stricter* than PHP's rather than merely less permissive, and it is
deliberate: leaving it would preserve exactly one slot where a member's read visibility is a rule you have
to know instead of a word you can see.

### 4. `var`, and the PHP property shapes that no longer parse

`var` is [ADR 0037](0037-var-local-type-inference.md)'s local-inference keyword, so PHP's `var $x;` property
form is doubly dead. Because it is the shape a porting author actually types, a class body's `var` reports
**E0122 naming the visibility** — "write `public int $x;`" — rather than falling through to the
local-declaration grammar and reporting something about statements. The same holds for the bare `int $x;`
and `static int $x;` that MWL's own grammar accepts today.

### 5. `mwl fmt` never inserts it; `mwl convert` does

[ADR 0039](0039-canonical-code-formatting.md)'s formatter orders modifiers and does not supply a missing one.
A formatter that inserted `public` would make a file's *meaning* depend on whether a tool had been run over
it, and would restore the implicit default through the back door for anyone who formats on save.

[ADR 0089](0089-convert-is-one-rule-table-with-two-modes.md)'s converter is the opposite case and carries the
insertion as an **E-tier** row: PHP's omission provably means `public`, so writing it is a behaviour-identical
rewrite, discharged by a differential case like any other E branch. Porting a PHP file therefore costs the
author nothing here.

## Consequences

- **Security and correctness (priorities 1–2):** a member's audience is stated at the member. The failure
  mode where a helper is public because someone forgot a word is gone, and ADRs 0019 and 0028 report a level
  that was actually written.
- **Latency and memory (3, 5):** none, in either direction. The check is one pass over already-parsed
  declarations, and nothing about it survives into the IR.
- **Usability and simplicity (4):** the real cost. Every member gains up to nine characters, and every ported
  PHP class needs one edit per member that omitted it — automated by § 5's converter row, but still a diff.
  Against that, the language surface loses a rule rather than gaining one: there is no default to document,
  no asymmetry between the type (mandatory) and the visibility (inferred), and no per-slot table of which
  omissions are legal. The net for a reader is a smaller language; the net for a writer is more keystrokes.
- **Consistency with the existing corpus:** small. Four member declarations across `.mwlt`/`.mwl` fixtures
  and roughly sixty inline snippets in Rust tests omit a visibility today and are rewritten with the check.
- **This ADR does not make `private` mean anything yet.** Enforcement at the access site is separate,
  unbuilt work (`mwl-types`' gap list). Until it lands, a written `private` is an accurate declaration that
  nothing yet checks — which is strictly better than an unwritten one.

## Alternatives rejected

- **PHP's implicit `public` (the status quo).** Rejected on priority 1/2: it makes the safest-to-forget
  option the widest one. It is also the only rule in this area a reader cannot see in the source.
- **An implicit `private` default** — safer, and a real choice in Rust and in C++ classes. Rejected because
  it is *silently* different from PHP: a ported file compiles and quietly changes meaning, which is worse
  than either the PHP rule or an error. MWL diverges from PHP loudly or not at all.
- **A warning, or a lint with a suppression annotation.** Rejected on ADR 0029's precedent — this repository
  has no warning tier and no suppression mechanism, and adding one for this would be the first.
- **Let `mwl fmt` insert `public`.** Rejected in § 5: a formatter that changes meaning is not a formatter.
- **Require it on properties only, leaving methods PHP-shaped.** Rejected: half the rule costs a reader the
  whole rule, because they must still remember which half applies where.
- **Exempt interface methods**, on the pre-0043 argument that only `public` is legal there. Rejected because
  it is no longer true: `public` and `private` are two different constructs inside an interface body.
- **A fourth keyword for "package/module-visible".** Not rejected on merit — out of scope. MWL has no module
  boundary below the class, and inventing one here would be a language feature smuggled in as a syntax rule.

## Verification

- **M1, in `mwl-syntax`.** The check belongs with the other post-parse declaration checks — the walk over
  every class, interface and anonymous-class body that `check_casing` already made — so it reaches every
  call site that already runs the casing pass (`mwl-cli`, and `mwl_hir::requires` for a required file) with
  no new wiring. That walk is `check_declarations` now: it answers two questions rather than one, and
  `mwl_syntax::casing`'s own module doc says why they share a visit.
- `E_MISSING_VISIBILITY` = **E0122**, parser band, with the fix hint of § 1.
- **One report per declaration.** An `enum` body parses its members through the same class-member path and
  already rejects each one with `E0220`, so the walk skips an enum's member list rather than adding a
  second diagnostic to a declaration that is refused outright.
- Guard tests, in `crates/mwl-syntax`:
  - `a_member_without_visibility_is_a_compile_error` — one case each for a method, a property, a class
    constant, an interface method and an anonymous-class member.
  - `a_bare_set_visibility_is_a_compile_error` — § 3's `private(set)` alone.
  - `a_class_body_var_names_the_missing_visibility` — § 4's message, not the local-declaration error.
  - `a_plain_constructor_parameter_needs_no_visibility` — § 2's negative case, which is the one this rule
    could plausibly break.
- Conformance: `tests/conformance/reject/a-member-must-declare-its-visibility.mwlt`.
- The corpus rewrite lands in the same slice as the check, per the Stage 0 rule that no fixture is written
  against a rule about to change.
