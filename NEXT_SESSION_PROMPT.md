# Next session prompt

Continue MWL. M1 (front end) is done and closed out — see git history if you need the detail; it's
not repeated here per CLAUDE.md's "state a fact once" rule.

**Last session added no code.** It settled [ADR 0027](docs/adr/0027-callable-is-closures-and-invokables-only.md):
`callable` accepts only a `Closure` or an object declaring `__invoke` — PHP's string/array callable
spellings (`"strlen"`, `"Class::method"`, `[$obj, 'method']`) are rejected, since first-class callable
syntax (already in the M1 grammar) already gives a statically resolvable reference. It's referenced from
`CLAUDE.md`'s routing table and ground rules, and from the plan's M2 paragraph/Verify line — but it's a
checker-side rule (`mwl-types`), so it doesn't change what the next code slice below is.

**M2 — HIR, types, IR — in progress.** Read `CLAUDE.md` first (it routes to the one file you need per
topic), then run `sh .claude/brief.sh` for the live status slice, then read the plan's M2 paragraph in
`docs/implementation-plan.md` in full, then read `crates/mwl-hir/src/lib.rs`'s module docs — both now
carry the same up-to-date breakdown of what this milestone's `mwl-hir` crate covers and what's left.

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
  class/interface/enum atom is refused under the new `E_TYPE_ALIAS_ALIASES_CLASS` (`E0307`) code, added
  to `mwl-diagnostics` this session.
- `Core`-namespace reservation for *declarations* (`namespace Core;` and anything nested under it) was
  already enforced by `mwl-syntax`'s parser back in M1 — `mwl-hir` didn't need to redo it.

**What M2's plan paragraph still needs, not yet started:**

1. The class hierarchy graph: `extends`/`implements` resolved to real `Symbol`s, and trait flattening
   via `use Trait, ...` inside a class/trait body — conflicts resolved by `insteadof` alone, per
   [ADR 0015](docs/adr/0015-no-name-aliasing.md) § 3 (the `as`-rename/`as`-visibility forms are already
   rejected at parse time; only `insteadof` reaches `mwl-hir`). This is the natural next slice — it
   builds directly on the `SymbolTable` this session added, and everything after it in the list below
   depends on having a real class graph to walk.
2. Every callable/constant resolving as a class member with no bare-name fallback
   ([ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md)) — needs the class graph
   from item 1 to know what a class's members even are.
3. Substituting a resolved `type` alias into the types that reference it (the transparency half of
   [ADR 0015](docs/adr/0015-no-name-aliasing.md) § 5 — this session only collected the declarations and
   rejected the bare-class case; the substitution itself, and cycle detection
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

`mwl-types` (the type checker) and `mwl-ir` (CFG/SSA lowering) still haven't started — the plan's
Architecture diagram has them building on top of `mwl-hir`'s resolved names, not in parallel with it.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs
before it can be called done — one file per diagnostic across ADR 0007, ADR 0022, and ADR 0024, IR
snapshot tests, the `Comparable` refusal cases from ADR 0013. None of that corpus exists yet since it
depends on `mwl-types`/`mwl-ir`; `mwl-hir`'s own unit tests (in `resolve.rs`, `symbol.rs`, `qname.rs`)
are the right home for name-resolution-only cases in the meantime — keep adding to them as each new
piece above lands, rather than retrofitting a separate corpus later.
