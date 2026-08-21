# Next session prompt

Continue MWL. M1 (front end) is done and closed out — see git history if you need the detail; it's
not repeated here per CLAUDE.md's "state a fact once" rule.

**Last session built M2 item 1, the class hierarchy graph, in `crates/mwl-hir`.** Read `CLAUDE.md`
first (it routes to the one file you need per topic), then run `sh .claude/brief.sh` for the live
status slice, then read the plan's M2 paragraph in `docs/implementation-plan.md` in full, then read
`crates/mwl-hir/src/lib.rs`'s module docs and `crates/mwl-hir/src/hierarchy.rs`'s module docs — all
three carry the same up-to-date breakdown of what's built and what's left.

**What exists now**, on top of the name-resolution slice from before (namespace/`use` scoping, the
symbol table, `type`-alias declaration collection — all unchanged, still in `qname.rs`/`symbol.rs`/
`resolve.rs`):

- `hierarchy.rs` — `HierarchyResolver`/`ClassGraph`/`ClassLinks`. Two-pass shape, same as `Resolver`
  itself: `collect_links` walks a file's declarations recording each one's raw `extends`/
  `implements`/trait-use references plus the namespace and `use` imports active at that point;
  `resolve` checks every reference against the already-built `SymbolTable` in a second pass, so a
  forward reference to a not-yet-declared parent (or a `use`-imported one) resolves the same way an
  import already does. Diagnoses: a wrong-kind or undeclared parent (`E_UNDEFINED_CLASS`, `E0303` —
  a class `extends`ing an interface, or naming something that isn't declared, is this code, not a
  new one); a circular `extends`/trait-use chain (`E_CIRCULAR_INHERITANCE`, `E0305`, already reserved
  for this, not new); and a trait method-name collision across a class/trait's used traits with no
  `insteadof` naming a winner (`E_TRAIT_METHOD_CONFLICT`, `E0308`, newly added this session, follows
  the E03xx name-resolution family). `Core\...` targets are trusted rather than checked, same pattern
  as `use` imports. `resolve_file` now populates `Module.graph` automatically. 13 new unit tests in
  `hierarchy.rs` (32 total in the crate now), `cargo test`/`clippy -D warnings`/`fmt --check` all
  clean across the whole workspace.
- **Known gap, called out in `hierarchy.rs`'s own module docs:** a trait pulling in another trait's
  methods is not flattened recursively — only a trait's own *directly*-declared methods are checked
  for a name collision against traits used alongside it. `trait A { use B; function hello(){} }` — a
  collision between `A`'s own `hello` and something `B` (or something `B` itself pulls in) declares
  isn't caught yet. Revisit before M2 closes if nested trait composition needs the same check; the
  plan's M2 paragraph and this milestone's *Verify* line don't currently call out a nested-trait test
  case, so it may turn out not to matter before M3 — check there aren't new requirements before
  spending time on it.

**What M2's plan paragraph still needs, not yet started:**

1. ~~The class hierarchy graph~~ — done, see above.
2. Every callable/constant resolving as a class member with no bare-name fallback
   ([ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md)) — the class graph from
   item 1 now exists, so this can build on `ClassGraph::get(qname).extends`/`.traits` to walk
   ancestors when resolving `parent::foo()`/an inherited member reference. This is the natural next
   slice — everything else left in this list is independent of it, but this is the one the plan's
   ordering puts right after item 1.
3. Substituting a resolved `type` alias into the types that reference it (the transparency half of
   [ADR 0015](docs/adr/0015-no-name-aliasing.md) § 5 — only the declarations are collected and the
   bare-class case rejected so far; the substitution itself, and cycle detection
   `type A = B; type B = A;`, are still open). Do this one wherever `TypeAtom::Name` is resolved
   against the symbol table, since it's the same lookup either way. Note `hierarchy.rs`'s cycle-
   detection code (`detect_cycles`/`visit` — a stack-based DFS over `extends`/trait-use edges) is a
   reasonable template for the `type`-alias cycle check; it's a different edge set but the same
   shape.
4. `require`'s static resolution with a dynamic fallback
   ([ADR 0021](docs/adr/0021-single-file-inclusion-construct.md)) — this is also where
   `Resolver::collect_declarations`/`resolve_imports`'s multi-file design (already shaped for it, and
   now `HierarchyResolver::collect_links`/`resolve` mirror the same shape) gets exercised for the
   first time. Keep "does this path resolve statically" structurally separate from "fall back to a
   dynamic lookup" per [ADR 0025](docs/adr/0025-wasm-browser-target.md)'s note that the browser
   target forbids the dynamic fallback outright rather than merely deprioritising it.
5. The property-access resolution rule ([ADR 0014](docs/adr/0014-property-observer.md)) — refusing
   access to anything not declared on the class or an ancestor/trait, no `__get`/`__set` fallback.
   Now unblocked by the class graph (item 1): walk `ClassGraph::get(qname).extends`/`.traits`
   (transitively) to collect every ancestor/trait's declared properties, same traversal shape as
   `hierarchy.rs`'s own DFS.

**Also open, self-contained, no dependency on the above:** implement the ADR 0029/0030 casing check in
`crates/mwl-syntax` — a diagnostic pass over the AST nodes M1 already produces (class/interface/trait/
enum/enum-case/namespace-segment/method/property/parameter/local/const declarations), no `mwl-hir`
involvement needed. Remember: property/parameter/local now reject *any* leading underscore (no exception),
and a method literally named `__construct` gets its own targeted diagnostic naming `constructor` as the
fix, distinct from the generic mis-casing message. Good filler work between the numbered items above, or a
fine place to start the session if item 2 needs more thinking time first.

`mwl-types` (the type checker) and `mwl-ir` (CFG/SSA lowering) still haven't started — the plan's
Architecture diagram has them building on top of `mwl-hir`'s resolved names, not in parallel with it.
When `mwl-types` does start, it inherits two fresh checker-side rules from ADR 0028: refuse an object used
at an implicit string-conversion site unless its static type provably implements `Stringable`, and refuse
`unset()` on any declared object property outright.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs
before it can be called done — one file per diagnostic across ADR 0007, ADR 0022, ADR 0024, ADR 0027,
ADR 0028, ADR 0029 and ADR 0030, IR snapshot tests, the `Comparable` refusal cases from ADR 0013. None of
that corpus exists yet since most of it depends on `mwl-types`/`mwl-ir`; `mwl-hir`'s own unit tests (in
`resolve.rs`, `symbol.rs`, `qname.rs`, and now `hierarchy.rs`) are the right home for name-resolution-only
cases in the meantime — keep adding to them as each new piece above lands, rather than retrofitting a
separate corpus later. ADR 0029/0030's corpus is the one exception: it can start as soon as its
`mwl-syntax` check exists, no need to wait for `mwl-types`.
