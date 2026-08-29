# ADR 0113 — A qualified name is absolute, and the leading `\` does not parse

- **Status:** Accepted
- **Date:** 2026-08-29
- **Scope:** how a name written in source — a class, interface, enum, `type` alias or attribute
  reference — resolves to a fully-qualified name, and the spelling of the `namespace` declaration
  itself. Not in scope: *what* may be imported and under which short name
  ([0015](0015-no-name-aliasing.md)), the `Core` prefix's reservation
  ([0011](0011-functions-and-constants-are-class-members.md) § 2), how a name's case is compared
  ([0062](0062-case-sensitivity-is-a-compiler-property.md) § 1), or which file a name is loaded from
  ([0061](0061-compile-time-autoload-and-program-discovery.md)).
- **Depends on:** [0011](0011-functions-and-constants-are-class-members.md) — removing free functions
  and constants is what leaves classes as the only kind of name to resolve, and so what makes PHP's
  fallback-to-global rule removable rather than load-bearing.

> **In short:** a name containing a `\` is **absolute**, read from the root, everywhere it can be
> written. A name containing no `\` is a **short name**, resolved through the file's imports and then
> its own namespace, and nowhere else. A **leading `\` does not parse** — `\App\Models\User`,
> `use \App\Models\User;` and `namespace \App;` are each a diagnostic naming the spelling without it.
> This removes PHP's rule that a qualified name is relative to the current namespace, which is the
> only construct that changes meaning: inside `namespace App;`, `Models\User` named `App\Models\User`
> in PHP and is refused here, in favour of writing `App\Models\User` or importing it. One rule decides
> every name, in one function, with no position where the same two spellings mean different things.

## Context

- PHP gives the leading `\` two unrelated jobs depending on where the name sits, and a third position
  where it is illegal. In a `use` clause the path is already absolute, so `use \A\B;` and `use A\B;`
  are the same import and the `\` is pure noise. Anywhere else the two spellings resolve differently —
  `\A\B` from the root, `A\B` through the imports and then the current namespace. And `namespace \A;`
  is a parse error. The same token is redundant, load-bearing, and forbidden, in three positions a
  reader moves between constantly, so there is no habit to form and no local reading that settles it.
- The relative half is also PHP's most-reported namespace footgun in its own right: inside
  `namespace App;`, `new App\Models\User()` silently means `App\App\Models\User`, and the error names
  a class nobody wrote.
- Simplicity (priority 4): this is two resolution rules and a token, bought for zero semantic gain —
  exactly the shape [0015](0015-no-name-aliasing.md) removed for import renaming, the group form
  `use A\{B, C};` for imports, [0050](0050-list-destructuring-spelling-rejected.md) for destructuring
  and [0049](0049-single-open-tag-and-single-exit-keyword.md) for the open tag.
- Security (priority 1), and why this is decided now rather than later:
  [0112](0112-authority-is-keyed-on-the-enclosing-namespace.md) keys a capability grant on the
  namespace enclosing the code that asks. That turns "which namespace does this name sit in" from a
  readability question into an authority question, so a resolution rule with two branches is a grant
  lookup with two branches. The rule has to be settled underneath 0112, not beside it.
- The cost of deciding it now is measurably near zero and grows monotonically. At the time of writing,
  25 of 1188 fixtures declare a namespace, all of them flat (`namespace App;`, `namespace App\Sub;`);
  **none uses a relative qualified name and none writes a leading `\` anywhere**. The parity program's
  six goals — the framework, five drivers, the package system — are what will produce namespaced code
  in volume, and every line of it is written against whichever rule is in place when it is written.
- PHP's fallback-to-global for an unqualified name exists because a function or constant call had to
  find a built-in from inside a namespace. [ADR 0011](0011-functions-and-constants-are-class-members.md)
  removed free functions and constants entirely, so nothing is left that the fallback was for.

## Decision

**A name with a separator in it is absolute. A name without one is short, and resolves through imports
then the enclosing namespace. A leading `\` is refused rather than accepted-and-ignored.**

### 1. The two forms, and the whole rule

Given a name as written, and the namespace and import set active where it was written:

- **Contains a `\`** → it is the fully-qualified name, read from the root. The enclosing namespace and
  the import set are not consulted at all.
- **Contains no `\`** → look it up in the import set; failing that, in the enclosing namespace. If
  neither has it, the name does not resolve, and there is no third place to look.

```php
namespace App;

use App\Models\User;      // unchanged — a `use` path was always absolute
use Throwable;            // how a root-level name is reached (§ 2)

new User();               // short: found in the imports
new App\Models\User();    // absolute — the same name from any namespace
new Models\User();        // refused (§ 5) — write App\Models\User, or import it
```

The last line is the only construct whose meaning changes. In PHP it named `App\Models\User`; here it
names `Models\User`, which does not exist, and the diagnostic says so in those terms.

### 2. There is no fallback to the root namespace

A short name resolves through the imports and the enclosing namespace, and stops. A name declared at
the root — a user class in the global namespace, or the reserved exception tree (`Throwable`,
`LogicError`, `TimeoutError`, and their siblings in `QName::is_reserved_global_class`) — is reached
from inside a namespace by importing it:

```php
namespace App;
use Throwable;

try { ... } catch (Throwable $e) { ... }
```

A single-segment `use` is not a special form; it is the ordinary absolute path to a name whose path
happens to be one segment long. The reserved tree gets no auto-import, for the same reason
[ADR 0011](0011-functions-and-constants-are-class-members.md) § 2 gives `Core` none: a name that
resolves without appearing anywhere in the file is a name a reader cannot trace, and one closed
roster's worth of exception would be a second resolution rule to hold alongside § 1's.

Code at the root namespace is unaffected — its enclosing namespace *is* the root, so it reaches those
names as short names already, which is why the existing corpus needs no import added.

### 3. The leading `\` does not parse

It is refused in all three positions, rather than stripped:

```php
\App\Models\User::find(1);   // refused
use \App\Models\User;        // refused
namespace \App;              // refused (PHP rejects this one too; Novis accepted it)
```

Refusing rather than repairing follows [0095](0095-ambiguous-input-is-refused-never-repaired.md)'s
reading and [0029](0029-identifier-casing-is-checked.md)'s: a spelling the compiler silently accepts is
a spelling that spreads. Since § 1 leaves the `\` no work to do, accepting it would restore precisely
the two-spellings-one-meaning state this ADR removes, only with the confusion moved from resolution
into style.

The formatter ([0039](0039-canonical-code-formatting.md)) does not strip it either — a construct the
parser rejects never reaches the formatter.

### 4. What does not change

- **`use` paths.** Already absolute, already conventionally written without the `\`. Every `use` in the
  corpus, and in every ADR's examples, is valid unchanged.
- **A short name for a class in the same namespace.** `namespace App\Models; ... new User();` resolves
  to `App\Models\User` exactly as before — the enclosing-namespace step in § 1 is PHP's, kept.
- **`Core`.** `Core\Str::length($s)` is a qualified name, absolute under § 1, and resolves identically
  from any namespace — which is what [ADR 0011](0011-functions-and-constants-are-class-members.md) § 2's
  examples already assume and already spell without a leading `\`.
- **Case comparison, autoload, and the `Core` reservation** are untouched; each is its own ADR's rule.

### 5. Diagnostics

Each names the replacement, in the style [0011](0011-functions-and-constants-are-class-members.md) § 4
sets. Two new codes, in the bands their phase owns:

- **`E0240`** (rejected PHP constructs, the band [0015](0015-no-name-aliasing.md) § 2's `E0212`
  sits in) — a leading `\` in any of § 3's three positions → *a name is already absolute; write
  `App\Models\User` without the leading `\`*.
- **`E0322`** (name resolution) — a qualified name that does not resolve, **but would have resolved
  relative to the enclosing namespace** → *`Models\User` does not exist; a qualified name is absolute
  here. Did you mean `App\Models\User`?* This is the migration diagnostic: it fires exactly on the one
  construct § 1 changes, and names the answer rather than leaving a generic unresolved-name error to be
  puzzled over. A qualified name that resolves neither way stays the ordinary unresolved-name error.

### 6. One function owns the rule

The rule is [`nvs_hir::resolve_ref`](../../crates/nvs-hir/src/hierarchy.rs) and nowhere else. Every
resolver in `nvs-hir` and `nvs-types` — `extends`/`implements`, a `type` alias body, an attribute name,
a static call target, a `require` target, a route or command target — already funnels through it, so
§ 1 is a change to that function's body and to none of its callers. Keep it that way: a second place
that decides what a name means is the state this ADR exists to leave.

## Consequences

**Positive**

- One rule decides every name, and it is decidable from the name alone — whether a `\` is present —
  without knowing which of three positions the name sits in. The question this ADR answers ("do I need
  the leading `\` here?") stops being askable, because the token stops existing.
- PHP's `App\App\Models\User` footgun is structurally unreachable, not merely diagnosed.
- [0112](0112-authority-is-keyed-on-the-enclosing-namespace.md)'s grant lookup resolves a name one way,
  so an authority decision cannot differ from a reader's expectation about where a name pointed.
- `resolve_ref` loses its leading-`\` branch and its fallback chain; the short-name path is a two-step
  lookup with an error at the end.

**Negative**

- **A structural break from PHP**, registered in [divergences.md](divergences.md) alongside
  [0010](0010-enums-are-a-value-type.md), [0011](0011-functions-and-constants-are-class-members.md),
  [0012](0012-no-superglobals.md) and [0015](0015-no-name-aliasing.md): PHP source using a relative
  qualified name (`Models\User` inside `namespace App;`) does not convert unconverted.
- **A namespaced file that catches now writes one more import line** (`use Throwable;`) than the same
  file in PHP. This is § 2's deliberate price for having no third lookup step.
- Reaching into a sub-namespace is more typing: `App\Models\User` or a `use`, where PHP allowed
  `Models\User`. This is the same trade [0015](0015-no-name-aliasing.md) *Consequences* made for import
  renaming — strictly more to type, strictly less to keep track of.

Unlike most divergences, this one is **mechanically lossless for `nvs convert`**: the converter holds
the enclosing namespace and the import set at every name, which is exactly what resolving the PHP form
requires, so it rewrites each relative qualified name to its absolute spelling with no human decision
and no behaviour change. It is a tier **D** rule for
[0089](0089-convert-is-one-rule-table-with-two-modes.md), not a tier that needs review — the converted
program resolves every name to the class PHP resolved it to.

## Alternatives rejected

- **Keep PHP's rules exactly.** The status quo, and what `resolve_ref` implements today. Rejected: it
  is priority 4 spent for nothing, and after [0112](0112-authority-is-keyed-on-the-enclosing-namespace.md)
  it is priority 4 spent against priority 1.
- **Accept the leading `\` and strip it, or let the formatter strip it.** Rejected in § 3: this is the
  "two spellings, one meaning" state itself, relocated into style, and every prior spelling decision in
  this project refused the same offer.
- **Keep relative qualified names, and only ban the leading `\`.** The smaller change, and the worse
  one: it leaves the confusing half (a name that means different things in different namespaces) and
  removes the half that let a reader escape it. Strictly worse than either endpoint.
- **Keep the leading `\` as the *only* way to write an absolute name, banning the bare qualified form.**
  The mirror-image rule, and internally consistent. Rejected because it makes every `use` path and
  every `Core\Str` reference in the corpus and in every ADR's examples wrong, to reach a state no more
  decidable than this one.
- **Fall back to the root namespace for an unresolved short name.** Rejected in § 2: it restores a
  two-things-could-match ambiguity for exactly the names § 1 just made unambiguous, and PHP only needed
  it for the free functions [0011](0011-functions-and-constants-are-class-members.md) removed.
- **Auto-import the reserved exception tree.** Rejected in § 2: a closed roster is still a second
  resolution rule, and it contradicts [0011](0011-functions-and-constants-are-class-members.md) § 2's
  no-auto-import reading of `Core`.
- **Move the exception tree under `Core`** so no root-level name exists at all. A larger change, whose
  cost is every `catch` clause in the corpus, to remove a case § 2's one import line already covers.
  Not this ADR's decision — if it is ever made, it is made for its own reasons.

## Revisiting

- **If a root-level name turns out to be reached often enough that `use Throwable;` reads as
  boilerplate**, the question to reopen is *moving the tree under `Core`*, not adding the fallback or
  the auto-import — those are § 2's rejected options and the reasons do not expire.
- **A relative-reference form with its own token** (some explicit "from here" marker, rather than the
  bare qualified name PHP overloads) if deeply nested namespaces make absolute paths genuinely painful
  in real code. Nothing observed yet asks for it, and adding it later breaks nothing this ADR decides.

## Verification

- **M1:** `\App\Models\User`, `use \App\Models\User;` and `namespace \App;` each report `E0240` naming
  the spelling without the `\`; the `Backslash` arm is gone from `parse_namespace_decl`'s name predicate
  and from `parse_name`'s leading `eat`.
- **M2:** `resolve_ref` consults the namespace and imports for a single-segment name only, and returns a
  multi-segment name unchanged; `Models\User` inside `namespace App;` reports `E0322` naming
  `App\Models\User`; a short name found in neither the imports nor the enclosing namespace reports the
  ordinary unresolved-name error with no root-namespace attempt; a fixture catching `Throwable` from
  inside a namespace resolves it through an explicit `use` and fails without one.
- **M11:** `nvs convert` rewrites a relative qualified name to its absolute spelling over a corpus
  containing one, and strips a leading `\`, both without a `TODO` — the tier **D** claim in
  *Consequences*.
