# ADR 0066 — `expr as ?T` converts without throwing, yielding `null` on failure

- **Status:** Accepted
- **Date:** 2026-08-23
- **Scope:** the meaning of the conversion operator when its target type is nullable; which conversions
  admit that form and which refuse it; the removal of `Core\Validate`'s three numeric predicates. Not in
  scope: the throwing form's behaviour, unchanged in every particular, and class downcasting, which stays
  `instanceof`.
- **Amends:** [0007](0007-explicit-type-system.md) — § 2's conversion table gains a nullable-target form of
  every row it already defines: where a row throws, `as ?T` yields `null` instead. The rows themselves are
  untouched. [0047](0047-literal-and-enum-case-types.md) — its checked conversion into a literal or
  enum-case type gains the same non-throwing form.
- **Amended by:** none.
- **Relates to:** 0010, 0022, 0024, 0033, 0035, 0063

> **In short:** `$s as ?int` yields the converted value where `$s as int` would succeed and `null` where it
> would throw, so a failed conversion becomes a value you test rather than control flow you catch. **The
> syntax already parses** — `as` takes a type and `?int` is one; ADR 0007 § 2's table simply had no nullable
> row, so this defines an unspecified corner rather than adding surface. A `null` operand yields `null`.
> The form is available for exactly the conversions § 2 and ADR 0047 already define, is **refused where the
> conversion cannot fail**, and remains a compile error where no conversion exists at all. It never
> launders `tainted` or `secret`. `Core\Validate::isInteger`/`isFloat`/`isBoolean` are removed, being the
> same predicate spelled twice.

## Context

- Converting untrusted input is the single most common thing a web program does, and PHP's answer —
  `(int)$x` yielding `0` for `"abc"` and `12` for `"12abc"` — is [ADR 0007](0007-explicit-type-system.md)
  § 7's divergence 4, rejected because it silently accepts garbage. MWL's replacement throws, which is
  right when invalid input *is* an error and wrong when it is an ordinary expected outcome: an absent page
  number, an optional sort order, a filter that may not be filled in.
- [ADR 0063](0063-core-api-conventions.md) R4 makes failure throw and R5 **bans** `try…`, `…OrNull`,
  `…Safe` and `…Ex` as member names, so no `Core` member may offer the non-throwing form. The only path
  left was check-then-convert — `Core\Validate::isInteger($s) && $s as int` — which **parses the same bytes
  twice** on the request path and requires the two definitions to agree forever. That hazard was closed by
  hand in [spec § 13](../spec/01-core-library.md), by defining the predicate as "true exactly when `as int`
  succeeds". Closing it by construction is better than closing it by prose.
- [ADR 0022](0022-definite-property-initialization.md) § 1 already settles that `?T` is MWL's one spelling
  for a value that may legitimately be absent. Nothing new is being introduced here; an existing type is
  being produced by an existing operator.
- The shape is well-tested elsewhere. Swift spells it `as?` (`as` / `as?` / `as!` is its full three-way
  split), Kotlin spells it `as?`, and C# gives `as` exactly these semantics while reserving throwing to the
  `(T)x` cast. Rust and Go reach the same place through `Option`/`Result` and comma-ok, which MWL has no
  sum type for.

## Decision

### 1. `as ?T` yields `?T`, and `null` is a failed conversion

`expr as ?T` produces the value that `expr as T` would produce where that succeeds, and `null` where it
would throw. Every other property of the conversion — what counts as success, the whole-string rule for
`string → int`, the range checks — is ADR 0007 § 2's, unchanged. This ADR adds no notion of validity of its
own; it only changes what happens to a failure.

**One family is defined the other way round.** For the closed **parse roster** in *3* — `Core\Uri` and
`Core\Uuid` — `as ?T` is defined directly as *"that type's `parse`, and `null` where it throws"*, and **no
`as T` form is implied or added**. The roster's types are classes, so ADR 0007 § 2's conversion table has
no row for them and adding one would make `$s as Uri` a second spelling of `Core\Uri::parse($s)`, which R17
forbids. Defining the nullable form directly keeps `parse` as the single implementation and the named
constructor, and gives the roster the same non-throwing spelling every scalar already has.

```
var $id   = Core\Request::query('id') as ?uint;      // ?uint — null if absent, or not a uint
var $page = Core\Request::query('page') as ?uint ?? 1;
var $mode = Core\Request::query('mode') as ?SortMode ?? SortMode::Newest;
```

### 2. A `null` operand yields `null`

`as ?T` accepts a nullable operand, and `null` converts to `null` rather than being a diagnostic. This is
deliberate and it is the ergonomic point of the feature: `Core\Request::query('page')` is `?tainted string`,
so requiring a non-null operand would put a null check ahead of the single most common use.

The cost is stated plainly: at such a site an **absent** parameter and a **malformed** one both reach the
`?? 1`. That is acceptable here and nowhere else, for two reasons. The developer wrote `?? 1`, which says
"if I cannot get a `uint`, use 1" — both states satisfy that sentence. And the case where the difference
matters is not reachable through this operator at all: a value that gates access is converted with plain
`as uint`, which throws. Where the distinction *is* wanted, testing the operand for `null` before
converting still separates them.

The alternative — refusing a nullable operand — was rejected because it does not actually prevent the
conflation. `($raw ?? '') as ?uint ?? 1` collapses the same two states, so the rule would price the
conflation rather than prevent it, at the cost of a line on the most common shape in the language.

### 3. Where the form is available, and where it is refused

| operand → target | `as ?T` | why |
|---|---|---|
| any row ADR 0007 § 2's conversion table defines | **available**; `null` where the row throws | the row already defines success and failure |
| into a literal or enum-case type ([ADR 0047](0047-literal-and-enum-case-types.md)) | **available** | that conversion is already checked and throwing; this is its non-throwing twin |
| from `mixed` | **available** — every target has a checked path from `mixed` | ADR 0007 § 6 |
| a `string` operand into a **parse-roster** type (`$s as ?Uri`, `$s as ?Uuid`) | **available**; `null` where that type's `parse` throws | *1*'s directly-defined family; the roster is closed and listed under the table |
| a conversion that **cannot fail** (`decimal as ?string`, `?int as ?int`) | **compile error**, naming `as T` | a `?T` that is never `null` is a lie in the type and forces a pointless check; R17 forbids the second spelling |
| no conversion exists at all (`array<int> as ?int`) | **compile error**, exactly as today | otherwise `as ?T` becomes a universal escape hatch that erases genuine type errors |
| a class or interface type (`$obj as ?SomeClass`) | **compile error** | `instanceof` plus ADR 0007 § 6 narrowing already answers class membership; R17 |

The distinction in the last three rows is the one to keep straight: **a conversion that exists and failed
is `null`; a conversion that does not exist is a diagnostic.** From `mixed` every conversion exists, so
`$mixed as ?int` is `null` for a value holding an array — while a statically-known `array<int>` never
compiles.

**The parse roster is `Core\Uri` and `Core\Uuid`, and nothing else.** A type joins it only by amending this
list. Membership requires a `parse` that takes **exactly one `string`** and can fail; a parse taking a
format or an options bag — `Core\Time::parse`, `Core\Csv::parse` — is a member call, not a conversion, and
stays one. The roster row does **not** reopen the class-type row below it: that row closed *class
membership*, "is this object already a `SomeClass`?", which `instanceof` answers. Turning text into a value
is a different question, and `$obj as ?SomeClass` remains the compile error it is.

**`Core\Duration` is deliberately not on the roster**, though its `parse` has the right shape.
`Duration::parse` **launders** a `tainted` config value ([spec § 4](../spec/01-core-library.md)) and *4*
below says `as ?T` is never a launderer — so the two are genuinely different operations rather than two
spellings of one, and R17 has nothing to object to. Both survive.

Each roster type therefore loses its `isValid` member, by *5*'s argument exactly: `Core\Uri::isValid($s)`
and `$s as ?Uri != null` are the same predicate. That deletion is what keeps a URI's validity question and
its parse from ever being answered by two different pieces of code — the shape behind PHP's CVE-2024-5458,
where `filter_var(FILTER_VALIDATE_URL)` accepted user-info that `parse_url` read differently.

### 4. Qualifiers are untouched

`tainted string as ?int` is `?tainted int`, and `secret` behaves the same way.
[ADR 0024](0024-taint-tracking-for-injection-sinks.md) and
[ADR 0033](0033-secret-qualifier-for-confidential-values.md) are orthogonal to the result's nullability,
and **`as ?T` is never a launderer** — laundering is only ever the narrow, sink-named `Core` functions
those ADRs name.

### 5. `Core\Validate`'s three numeric predicates are removed

`Validate::isInteger($s)` and `$s as ?int != null` are the same predicate, which R17 forbids.
`isInteger`, `isFloat` and `isBoolean` are struck from [spec § 13](../spec/01-core-library.md), along with
the sentence defining them against `as`, which this ADR makes unnecessary — one implementation now exists
because there is one operation, not because two were required to agree. `isEmail`, `isIp`, `isMac`,
`isDomain`, `isAscii` and `isPrintable` — the roster that survived a later duplicate sweep — are
unaffected: none has an `as` equivalent, because none names a type. `ctype_digit` is not among them for
the same reason `isInteger` is not: it is `$s as ?uint != null`.

### 6. A bare `?T` condition is a lint, not an error

Under [ADR 0035](0035-truthy-boolean-context.md) both `null` and `0` are falsy, so `if ($s as ?int)` is
false for an invalid value *and* for a valid zero — reconstructing precisely the `(int)$x > 0` defect that
motivated this ADR. `mwl check` warns when a `?T` is used directly as a condition, naming `!= null`.

It is a lint rather than a diagnostic because **the hazard is not new and not specific to this operator**:
any `?int` in an `if` has always had it. Making it an error would amend ADR 0035 for every nullable value
in the language, which is a larger decision than this one and is not taken here.

## Consequences

- **M2's checker** gains the form, which is small: the result type is the target with a nullability flag,
  and ADR 0007 § 6's existing `?T` narrowing handles everything downstream. There is **no parser work** —
  the grammar already accepts a nullable type after `as`.
- **One parse instead of two** on the request path, where check-then-convert scanned the same bytes twice.
  Priority 3, and it removes a call from the hottest thing a web program does.
- **Zero memory cost.** `?T` is already representable; nothing gains a byte.
- **`Core` gets smaller by three members**, and the class of API argument that produces `from`/`tryFrom`
  pairs is closed for every type at once rather than per type.
- **`mwl convert` (M11) must not take the obvious shortcut.** PHP's `(int)$x` now has a tempting mechanical
  target in `$x as ?int ?? 0`, which would quietly restore the silent-zero behaviour ADR 0007 § 7 diverges
  from deliberately — and would not even be faithful, since `(int)"12abc"` is `12` in PHP and `null` here.
  It keeps emitting a diagnostic naming both forms and lets the author choose.

## Alternatives rejected

- **A `Core` member — `Str::toIntOrNull`, `Int::tryParse`.** Rejected by [ADR 0063](0063-core-api-conventions.md)
  R5, which bans those exact name shapes, and by arithmetic: it needs one member per target type, forever,
  while an operator covers every type including ones added later.
- **Requiring a non-nullable operand.** Keeps "missing" and "invalid" syntactically distinct. Rejected in
  § 2: it does not prevent the conflation, only prices it, and charges a line on the most common shape in
  the language to do so.
- **A `Result`/`Either` return.** Distinguishes *why* a conversion failed. Rejected: MWL has no sum type,
  and adding one for this would be a language-scale decision to avoid a `null` check.
- **Refusing `?T` in a truthy position outright.** The strongest guarantee against § 6's trap, and rejected
  there: it amends ADR 0035 for every nullable value rather than for conversions.
- **Keeping the three `Validate` predicates.** A validation class with `isEmail` but no `isInteger` reads as
  a hole. Rejected on R17: the hole is apparent rather than real, since the removed three name *types* and
  the survivors name *formats*, and only types have an `as`.

## Verification

- **M2:** checker fixtures for `"42" as ?int` typing as `?int`; a `null` operand yielding `null`;
  `decimal as ?string` and `?int as ?int` rejected naming `as T`; `array<int> as ?int` rejected as having
  no conversion; `$mixed as ?int` accepted; `$s as ?SortMode` accepted for an enum and `$s as ?"a"|"b"` for
  a literal-union type; `tainted string as ?int` typing as `?tainted int`; and a `secret` operand keeping
  `secret`.
- **M3/M4:** a runtime suite asserting `"abc" as ?int`, `"12abc" as ?int` and `"" as ?int` are each `null`
  while `"42" as ?int` is `42` — the whole-string rule of ADR 0007 § 2 reaching the nullable form unchanged
  — and that `19.99 as ?int` is `null` rather than `19`.
  **§§ 1–3's lowering has landed for the checked numeric targets.**
  `tests/conformance/lang/a-nullable-conversion-yields-null-rather-than-throwing.mwlt` is that suite, and it
  covers § 2's `null` operand and § 3's `mixed` one as well. `mwl_ir::lower::Lowering::convert_or_null`
  emits one `mwl_ir::Helper` per target — `ToIntOrNull`/`ToUintOrNull`/`ToFloatOrNull`, with no error edge,
  since the form cannot fail — and each dispatches on the operand's runtime tag, which is why § 2's and
  § 3's rows need no lowering branch of their own. Each row has exactly one implementation, shared with the
  throwing form: `mwl_runtime::helpers`' own `row` module. Still owed: the enum/literal-type target, blocked
  on the same case set ADR 0010 § 5's integer-into-an-enum row waits for, and every § 3 **refusal** —
  `mwl_types` does not yet reject a conversion that cannot fail or one that does not exist, so `mwl-ir`
  panics naming this ADR where it should have been a diagnostic.
- **M4B:** `mwl check` warns on `if ($s as ?int)` and on any `?T` condition, naming `!= null`, and does
  **not** warn on `($s as ?int) != null`.
- **M11:** `mwl convert` emits a diagnostic for PHP's `(int)$x` naming both `as int` and `as ?int ?? 0`,
  and never rewrites to either silently.
