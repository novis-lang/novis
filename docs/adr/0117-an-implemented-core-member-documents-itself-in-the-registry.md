# ADR 0117 — An implemented Core member documents itself in the registry

- **Status:** Accepted
- **Date:** 2026-08-29
- **Scope:** where an *implemented* `Core` member's reference documentation lives, what `nvs meta --json`
  emits, and how a consumer resolves a field both sources speak about. It does not decide named-argument
  call syntax — the registry gaining parameter names is a prerequisite this ADR shares with that future
  decision, not the decision itself — nor the website's rendering, which
  [website/README.md](../../website/README.md) owns, nor the spec's role as the member list, which
  [0011](0011-functions-and-constants-are-class-members.md) § 2 deferred and
  [0051](0051-standard-library-tiers.md) § 3 placed, nor the API shape rules, which are
  [0063](0063-core-api-conventions.md).
- **Depends on:** [0011](0011-functions-and-constants-are-class-members.md), [0051](0051-standard-library-tiers.md)

> **In short:** an implemented `Core` member's reference documentation — short description, parameter
> names and descriptions, shape-key docs, return description, thrown errors — lives in its registry
> declaration in `nvs-stdlib`, next to the code it documents, and `nvs meta --json` prints the whole
> registry as JSON. [docs/spec/01-core-library.md](../spec/01-core-library.md) remains authoritative for
> the *surface* — every member, implemented or not, and its signature as designed. Where both speak about
> an implemented member, precedence is **field-wise**: a doc field the registry carries wins, a field it
> lacks falls back to the spec — so documentation migrates member by member, with no flag day and no
> consumer ever blocked on an unfilled field. Extended prose (long descriptions, examples, tips) never
> moves into Rust; it stays in the website's pages.

## Context

Two places describe a `Core` member today: the spec, which is authoritative for every signature and is the
only source of parameter names, and the registry in `nvs-stdlib`, which carries types but no names — its
recorded gap 3. The website's reference is generated from the spec and scans the Rust source textually,
only to learn which members exist. Nothing carries a member's *documentation* in a machine-readable form
at all: descriptions live in hand-edited website pages, deeply nested and far from the code, so a
contributor changing a member's behaviour has to remember to update prose somewhere they will never
otherwise look. That is the exact staleness pattern this repository's one-home rule exists to prevent.

Two facts decide where the home should be. First, gap 3: named arguments cannot exist until the registry
knows parameter names, so names are moving into the Rust declarations regardless — a documentation scheme
that puts them anywhere else creates the duplicate the migration would then have to undo. Second, a
registry declaration is the one artifact that provably matches the shipped behaviour, because it is the
data the runtime dispatches on.

## Decision

### 1. The doc fields

A registry member declaration may carry, all optional and all inline markdown: a **short description**
(one or two sentences), per-parameter **name and description** — and for a shape-typed parameter, each
key's **type and description** — a **return description**, and a list of **thrown errors, each with a
description**. Extended prose is deliberately excluded: long-form text in Rust string literals is the
worst editing surface available, so anything beyond the reference card stays in the website's pages.

### 2. `nvs meta --json`

The `nvs` binary prints its registry as JSON on `nvs meta --json`: classes, members, and each member's
doc fields under a `doc` key —

```json
{ "classes": [ { "name": "Core\\Str", "members": [ { "name": "length",
    "doc": { "short": "…",
             "params": [ { "name": "s", "desc": "…",
                           "shape": [ { "key": "pretty", "type": "bool", "desc": "…" } ] } ],
             "return": "…",
             "errors": [ { "error": "Core\\Error\\…", "desc": "…" } ] } } ] } ] }
```

The command owns the contract; consumers ignore fields they do not know. The website's consumer is
`website/scripts/lib/meta.mjs`, and it fails soft: a toolchain without the subcommand means "no registry
docs yet", never an error.

### 3. Field-wise precedence

For an implemented member, a doc field the registry carries is authoritative; a field it lacks falls back
to the spec-derived default. For a member not yet implemented, the spec is all there is. A consumer that
finds the two disagreeing on something both state — a parameter the registry documents that the spec does
not declare, a signature drift — warns rather than silently choosing. This rule is what makes landing
incremental: the seam can ship empty, and every member documented afterwards upgrades on its own.

## Consequences

- A contributor documents a member where they implement it, and the reference follows on the next site
  sync — no second file to remember.
- The binary spends memory: static doc strings, per-process not per-request, on the order of a few
  hundred bytes per documented member — low hundreds of KB with all 297 members documented. Under
  [0004](0004-memory-for-simplicity.md)'s ordering that is the cheap side of the trade; a `docs` cargo
  feature can strip it later if a deployment ever cares.
- The spec's entries for implemented members stop being the place doc wording is edited; the spec keeps
  the surface, the types and the design rationale.
- The website may drop its textual source scan once the registry reports implementation status through
  `meta` too; until then the scan stays.

## Alternatives rejected

- **Keep the spec as the sole home.** It leaves documentation far from the code it describes, and gap 3
  will duplicate parameter names into the registry anyway — the duplicate this ADR exists to avoid.
- **Extended prose in Rust too.** Editing long markdown inside string literals, with escaping and code
  review noise, is strictly worse than editing a markdown page; the reference card is the part worth
  co-locating, the essay is not.
- **Doc comments extracted via rustdoc JSON.** The rustdoc JSON format is nightly-unstable and the
  registry members are struct literals, not items — doc comments do not attach to them.
- **A sidecar doc file in the crate.** Another home beside the declaration is the same problem at a
  shorter distance.

## Verification

When `nvs meta` lands, `nvs-cli` gets a golden test holding the JSON shape of § 2, and the website's
`npm run sync` warns on every § 3 disagreement — those warnings are the drift detector.
