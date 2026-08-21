# Next session prompt

Continue MWL. M1 (front end) is done and closed out — see git history if you need the detail; it's
not repeated here per CLAUDE.md's "state a fact once" rule.

**Last session was a documentation cleanup pass, no code.** All 29 ADRs, `docs/adr/README.md`,
`docs/implementation-plan.md`, and `docs/spec/00-overview.md` were audited for redundancy and staleness:

- Every ADR's `## Context` / `## Investigation` / `## Alternatives rejected` sections were compressed into
  short bullet lists (docs/adr/README.md already told readers those sections are skippable unless they
  intend to overturn a decision — the prose length was pure padding). Metadata blocks, `## Decision`, and
  `## Consequences` are untouched, byte-for-byte, in every file. Net: ~7360 → 6640 lines across `docs/adr/`.
- Surveyed for ADR merge candidates first, before touching anything. Found none worth acting on: the two
  most tightly-coupled pairs (0029+0030, 0027+0028) turned out to be this project's amend-don't-rewrite
  convention working as intended, not fragmentation to fix. **0029's `## Decision` section still states its
  original, now-partially-superseded rules on purpose** — its `Amended by` metadata line pointing at 0030 is
  the correct fix, not a bug to reconcile. Don't "helpfully" merge them or rewrite 0029's Decision text.
- `docs/adr/README.md`, `docs/implementation-plan.md`, and `docs/spec/00-overview.md` were read in full and
  found already free of the kind of restated/duplicated content this pass targets — no changes made there.
- Added `DOC_CLEANUP_PROMPT.md` at the repo root: the reusable prompt (scope, merge policy, trim depth,
  procedure) for re-running this same kind of pass later without re-deciding the rules from scratch. Use it
  next time the ADR count has grown enough to warrant another look, rather than re-litigating from zero.

**M2 — HIR, types, IR — in progress, unchanged from before.** Read `CLAUDE.md` first (it routes to
the one file you need per topic), then run `sh .claude/brief.sh` for the live status slice, then read
the plan's M2 paragraph in `docs/implementation-plan.md` in full, then read `crates/mwl-hir/src/lib.rs`'s
module docs — both carry the same up-to-date breakdown of what this milestone's `mwl-hir` crate covers
and what's left. (The ADRs those docs link to are now shorter to read than before, but say the same thing.)

**What exists now.** `crates/mwl-hir` has name resolution's first slice, committed and tested (22 unit
tests, `cargo test`/`clippy -D warnings`/`fmt --check` all clean across the whole workspace):

- `qname.rs` — `QName`, a fully-qualified backslash-separated name (`App\Models\User`, `Core\Str`).
  Known gap, called out in its own docs: comparisons are case-sensitive; PHP's aren't. Revisit before
  M2 closes.
- `symbol.rs` — `SymbolTable`, keyed by `QName`: one entry per declared class/interface/trait/enum/
  `type`-alias. `declare()` keeps the first declaration and hands back the earlier `Symbol` on a
  collision, for `E_DUPLICATE_DECLARATION`.
- `resolve.rs` — `Resolver`/`resolve_file`: walks a parsed file's top-level `Stmt`s, tracks the current
  namespace (`namespace Name;` statement form changes it and resets the imported-short-name set for
  the rest of the sequence, matching PHP; `namespace Name { ... }` block form is scoped and doesn't
  leak), collects every type-level declaration into the `SymbolTable`, records every `use` import, and
  resolves each import against the table in a second pass (so a `use` may legally name a declaration
  that appears later in the file) — `Core\...` targets are trusted rather than checked, since
  `mwl-stdlib` doesn't exist yet. Also enforces
  [ADR 0015](docs/adr/0015-no-name-aliasing.md) § 6: a `type` alias that is nothing but one bare
  class/interface/enum atom is refused under the `E_TYPE_ALIAS_ALIASES_CLASS` (`E0307`) code.
- `Core`-namespace reservation for *declarations* (`namespace Core;` and anything nested under it) was
  already enforced by `mwl-syntax`'s parser back in M1 — `mwl-hir` didn't need to redo it.

**What M2's plan paragraph still needs, not yet started:**

1. The class hierarchy graph: `extends`/`implements` resolved to real `Symbol`s, and trait flattening
   via `use Trait, ...` inside a class/trait body — conflicts resolved by `insteadof` alone, per
   [ADR 0015](docs/adr/0015-no-name-aliasing.md) § 3 (the `as`-rename/`as`-visibility forms are already
   rejected at parse time; only `insteadof` reaches `mwl-hir`). This is the natural next slice — it
   builds directly on the `SymbolTable` already in place, and everything after it in the list below
   depends on having a real class graph to walk.
2. Every callable/constant resolving as a class member with no bare-name fallback
   ([ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md)) — needs the class graph
   from item 1 to know what a class's members even are. This is also where the ADR 0029/0030 casing
   check could live if it isn't already sitting in `mwl-syntax` by the time this item starts — either
   home is correct, since the check needs no resolution; don't block item 2 on it either way.
3. Substituting a resolved `type` alias into the types that reference it (the transparency half of
   [ADR 0015](docs/adr/0015-no-name-aliasing.md) § 5 — only the declarations are collected and the
   bare-class case rejected so far; the substitution itself, and cycle detection
   `type A = B; type B = A;`, are still open). Do this one wherever `TypeAtom::Name` is resolved against
   the symbol table, since it's the same lookup either way.
4. `require`'s static resolution with a dynamic fallback
   ([ADR 0021](docs/adr/0021-single-file-inclusion-construct.md)) — this is also where
   `Resolver::collect_declarations`/`resolve_imports`'s multi-file design (already shaped for it: call
   `collect_declarations` once per file into one shared `Module`, then a single closing
   `resolve_imports`) gets exercised for the first time. Keep "does this path resolve statically"
   structurally separate from "fall back to a dynamic lookup" per
   [ADR 0025](docs/adr/0025-wasm-browser-target.md)'s note that the browser target forbids the dynamic
   fallback outright rather than merely deprioritising it.
5. The property-access resolution rule ([ADR 0014](docs/adr/0014-property-observer.md)) — refusing
   access to anything not declared on the class or an ancestor/trait, no `__get`/`__set` fallback.
   Needs the class graph (item 1) to walk ancestors/traits.

**Also open, self-contained, no dependency on the above:** implement the ADR 0029/0030 casing check in
`crates/mwl-syntax` — a diagnostic pass over the AST nodes M1 already produces (class/interface/trait/
enum/enum-case/namespace-segment/method/property/parameter/local/const declarations), no `mwl-hir`
involvement needed. Remember: property/parameter/local now reject *any* leading underscore (no exception),
and a method literally named `__construct` gets its own targeted diagnostic naming `constructor` as the
fix, distinct from the generic mis-casing message. Good filler work between the numbered items above, or a
fine place to start the session if item 1 needs more thinking time first.

`mwl-types` (the type checker) and `mwl-ir` (CFG/SSA lowering) still haven't started — the plan's
Architecture diagram has them building on top of `mwl-hir`'s resolved names, not in parallel with it.
When `mwl-types` does start, it inherits two fresh checker-side rules from ADR 0028: refuse an object used
at an implicit string-conversion site unless its static type provably implements `Stringable`, and refuse
`unset()` on any declared object property outright.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs
before it can be called done — one file per diagnostic across ADR 0007, ADR 0022, ADR 0024, ADR 0027,
ADR 0028, ADR 0029 and ADR 0030, IR snapshot tests, the `Comparable` refusal cases from ADR 0013. None of
that corpus exists yet since most of it depends on `mwl-types`/`mwl-ir`; `mwl-hir`'s own unit tests (in
`resolve.rs`, `symbol.rs`, `qname.rs`) are the right home for name-resolution-only cases in the meantime —
keep adding to them as each new piece above lands, rather than retrofitting a separate corpus later. ADR
0029/0030's corpus is the one exception: it can start as soon as its `mwl-syntax` check exists, no need to
wait for `mwl-types`.
