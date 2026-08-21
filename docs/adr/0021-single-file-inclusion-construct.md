# ADR 0021 — `require` is the only same-frame file-inclusion construct

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** PHP's four same-frame inclusion keywords (`include`, `include_once`, `require`,
  `require_once`); the corresponding AST shape and diagnostics
- **Amends:** [0006](0006-isolated-script-execution.md) — every mention of "`include`/`require`" as the
  same-frame contrast to `spawn script` now names `require` alone; the isolation table and *Alternatives
  rejected* wording are updated, with no change to that ADR's actual decision. [`docs/spec/00-overview.md`
  § 2](../spec/00-overview.md) is rewritten to match — that document owns the surviving grammar, this ADR
  owns why the other three are gone.
- **Relates to:** [0004](0004-memory-for-simplicity.md) (simplicity bought by refusing a second name for
  one behaviour, the same trade [0015](0015-no-name-aliasing.md) already made), [0007](0007-explicit-type-system.md)
  (the one place this decision touches the type system — see *Decision § 3*)

> **In short:** MWL keeps exactly one same-frame inclusion keyword, spelled `require`, with its plain PHP
> meaning unchanged — throws if the file cannot be found or fails to parse, and splices the target into the
> calling frame every time control reaches it, with no automatic once-only guard. `include`,
> `include_once`, and `require_once` are all rejected at parse time, each with a diagnostic naming `require`
> as the replacement. Nothing new was invented for this: PHP's own plain `require` already spells exactly
> the behaviour this ADR wants kept, so the "pragmatic superset" absorbs the common case verbatim and only
> the redundant spellings are cut.

## Context

PHP ships four keywords for the same operation — splice a file into the calling frame — differing on two
independent axes:

| axis | `include` | `require` |
|---|---|---|
| missing/broken file | emits a warning, expression evaluates to `false`, execution continues | throws a fatal error |
| repeat guard | none — `_once` suffix adds it | none — `_once` suffix adds it |

Four spellings for two axes that could each vary independently is exactly the kind of surface [ADR
0015](0015-no-name-aliasing.md) and [ADR 0011](0011-functions-and-constants-are-class-members.md) already
argue against elsewhere in this language: more than one name for a reader to hold in their head for a
single underlying behaviour, bought for no semantic gain. Nothing about MWL's own design forces keeping
all four — this is a place where the pragmatic-superset promise (accept PHP syntax that isn't actively
misleading) and the simplicity priority (§ 4 of [CLAUDE.md](../../CLAUDE.md)) point the same way: collapse
to one.

**Why "warn and continue" cannot be the kept behaviour.** Every other ambient-continuation path in PHP this
project has looked at so far has been closed, not kept: an undeclared property is a hard error, not a
fallback ([ADR 0014](0014-property-observer.md)); a superglobal has no host-populated fallback at all
([ADR 0012](0012-no-superglobals.md)); comparing two objects with no `Comparable` implementation is a
diagnostic, not a property walk ([ADR 0013](0013-comparable-interface.md)). `include`'s failure mode —
swallow the error, hand back `false`, let the script keep running with whatever that implies for code that
assumed the file loaded — is the same shape of problem: a silent degrade that priority 1 (security) and
priority 2 (correctness) both weigh against, and priority 5 (memory) never gets a vote in. `require`'s
failure mode — throw — is already how every other MWL failure surfaces
([ADR 0002](0002-error-propagation.md)), so keeping it is not a new decision, only not un-deciding the one
already made everywhere else.

**Why the `_once` axis does not need to survive either.** PHP's `_once` guard exists to protect against a
file — almost always one declaring a class or function — being spliced into the same frame twice, which
would otherwise be a redeclaration fatal. In MWL, declarations are named class members resolved by
namespace ([ADR 0011](0011-functions-and-constants-are-class-members.md)), and M2's per-path compiled-unit
resolution ([ADR 0017](0017-hot-reload-without-restart.md) names the same per-path cache mechanism) is the
thing that will make a class reachable by name, not by however many times its declaring file happened to be
spliced in by a caller. The redeclaration problem `_once` guards against is a symptom of PHP's
textual-inclusion-as-module-system, which MWL is not adopting for declarations; a `require` used for what
it is actually still needed for — procedural code, a template partial rendered from a loop — must run every
time control reaches it, which is what plain `require` (no suffix) already does. Baking "run once" into the
kept construct's default would silently break exactly that loop case, and there is no default that serves
both without a suffix — so the suffix goes, not the default.

## Decision

**Exactly one construct survives: `require 'path.mwl';`, an expression, sharing the calling frame
completely (no isolation — contrast [`spawn script`](0006-isolated-script-execution.md)), throwing on a
missing or unparseable target, and executing every time control reaches it. `include`, `include_once`, and
`require_once` are rejected at parse time.**

### 1. Why `require`, not a new keyword

PHP's plain `require` already means exactly this — throw, no repeat guard, same frame — so no new keyword
was invented. This is the same reasoning that keeps `<?php` as a second spelling of `<?mwl` and `list(...)`
alongside `[...]` destructuring: reuse a PHP spelling verbatim when its existing meaning is exactly the one
MWL wants, and spend the "pragmatic superset" budget on that instead of on novelty. The three rejected
spellings are not rejected for being PHP-shaped; they are rejected because each names a behaviour (warn-and-
continue, or an implicit repeat guard) this project has already decided against having anywhere in the
language.

### 2. What still parses, and what does not

```php
require 'partials/header.mwl';        // kept — throws if missing, runs every time
$config = require 'config.mwl';       // kept — expression form, config.mwl ends with `return [...]`

include 'partials/header.mwl';        // rejected — "use `require`"
include_once 'lib/util.mwl';          // rejected — "use `require`"
require_once 'lib/util.mwl';          // rejected — "use `require`"
```

All four keep their token spellings in the lexer — none becomes a plain identifier — so the diagnostic for
the three rejected forms can be precise and name the replacement, the same pattern [ADR 0015 § 7](0015-no-name-aliasing.md)
and the `eval`/`extract`/`settype` rejections already use: *there is exactly one inclusion construct,
`require`; it already throws on failure and runs every time it is reached.*

### 3. The expression's type

`require`'s value — what a `return`-ing target file hands back, or `1` when it does not `return` at all —
cannot be known statically the way [ADR 0007](0007-explicit-type-system.md) otherwise requires every
expression's type to be. This is not a new problem needing a new mechanism: it is exactly the case `mixed`
exists for, ADR 0007's "one unchecked position." `$config = require 'config.mwl';` therefore requires the
same explicit `as` conversion any other `mixed`-typed boundary value needs before it can populate a typed
binding — no special-casing for `require`, and no third exception carved into the type system for it.

### 4. Diagnostics

Each rejected spelling gets the same message, following [ADR 0015 § 7](0015-no-name-aliasing.md)'s pattern
of naming the replacement directly:

- `include '...';` / `include_once '...';` / `require_once '...';` → *MWL keeps exactly one inclusion
  construct; use `require` — it already throws on a missing file and runs every time it is reached.*

## Consequences

**Positive**

- One AST shape, one keyword, one diagnostic to teach, where PHP has four of each split across two
  independent axes.
- The kept behaviour is the one already consistent with every other failure path in the language
  ([ADR 0002](0002-error-propagation.md)); no new "warn and continue" exit from the checked-return
  discipline gets added back in through this one construct.
- No redeclaration-guard mechanism needs designing at the file-inclusion layer at all — the problem it
  solved in PHP is a namespace-resolution question ([ADR 0011](0011-functions-and-constants-are-class-members.md)),
  answered once there rather than twice.

**Negative**

- **A structural break from PHP**, joining the divergence list [ADR 0007](0007-explicit-type-system.md) § 7
  already carries forward: PHP source using `include`, `include_once`, or `require_once` does not convert
  unconverted. Mechanical for `mwl convert` ([M11](../implementation-plan.md)) in the common case (rewrite
  the keyword to `require`); a script that relied on `include`'s warn-and-continue behaviour — testing the
  expression's `false` result to decide whether the file loaded — needs a human decision, since that
  control flow has no direct equivalent once a missing file always throws.
- A `mixed`-typed `require` expression assigned into a typed binding needs an explicit `as` at the call
  site, one more small piece of ceremony than PHP's `$config = require 'config.php';` — accepted as the
  same trade ADR 0007 already makes at every other boundary of unknown-until-runtime shape.

## Alternatives rejected

- **Keep `include`, dropped its `_once` suffix only.** Rejected in *Context*: `include`'s defining
  difference from `require` is the warn-and-continue failure mode, which is the one thing this ADR most
  wants gone. Keeping `include`'s name would also mislead every PHP-familiar reader into expecting exactly
  that behaviour from the one keyword that survives.
- **Keep both `require` and `require_once`, drop only the `include` family.** Considered, since it needs
  the smallest diagnostic surface. Rejected: it still leaves two names for one behaviour on the repeat-guard
  axis, and *Context* already gives the reason a repeat guard does not belong at this layer once
  declarations resolve by namespace rather than by however many times their file was spliced in.
- **Invent a new, shorter keyword** (e.g. `load`) instead of reusing `require`. Rejected: `require`'s
  meaning in PHP is already exactly the kept behaviour, so there is no mismatch a new spelling would be
  fixing, and a real `.php` corpus file already spelling `require` correctly would otherwise need
  rewriting for no behavioural reason — the opposite of what the pragmatic-superset promise is for.
- **Bake in an automatic once-guard as `require`'s only mode** (dedup by resolved path, silently). Rejected:
  it would silently break the template-partial-in-a-loop case, replacing one silent PHP behaviour
  (`include`'s warn-and-continue) with a different silent one dropped into the one construct meant to have
  none.

## Revisiting

- If M2's namespace/module resolution design turns out to still want a file-level "load this at most once"
  primitive for some case *Context* did not anticipate, that is a new, separately named construct at the
  resolution layer — not a reason to reopen `require`'s own semantics decided here.

Verification, in the order it becomes possible:

- **M1**: the parser accepts `require expr;` (and as an expression, `$x = require expr;`) building the one
  AST shape; `include`, `include_once`, and `require_once` are each rejected with a diagnostic naming
  `require` as the replacement; none of the four keywords is available as a plain identifier.
- **M2**: a `require`d file resolves through the same per-path compiled-unit cache
  [ADR 0017](0017-hot-reload-without-restart.md) already defines, statically when the path is a literal and
  by a dynamic fallback otherwise; a missing or unparseable target throws per
  [ADR 0002](0002-error-propagation.md) rather than degrading to a diagnosable-but-continuing state.
