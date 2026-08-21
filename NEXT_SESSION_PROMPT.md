# Next session prompt

Continue MWL. M1 (front end) is done and closed out — see git history if you need the detail; it's
not repeated here per CLAUDE.md's "state a fact once" rule.

**Last session built M2 item 2, member resolution, in `crates/mwl-hir`.** Read `CLAUDE.md` first (it
routes to the one file you need per topic), then run `sh .claude/brief.sh` for the live status slice,
then read the plan's M2 paragraph in `docs/implementation-plan.md` in full, then read
`crates/mwl-hir/src/lib.rs`'s module docs and `crates/mwl-hir/src/members.rs`'s module docs — all
three carry the same up-to-date breakdown of what's built and what's left.

**What exists now**, on top of the symbol table and class hierarchy graph from before (namespace/`use`
scoping, the symbol table, `type`-alias declaration collection, `extends`/`implements`/trait-use
resolution — all unchanged, still in `qname.rs`/`symbol.rs`/`resolve.rs`/`hierarchy.rs`):

- `members.rs` — `MemberTable`/`MemberResolver`. Same two-pass shape as `HierarchyResolver`:
  `collect_members` walks a file's declarations recording each class/interface/trait/enum's own
  directly-declared method, constant (enum cases count as constants) and static-property names into a
  `MemberTable`; `check` then walks the same file's statements a second time, descending into every
  method body, property default, constant value and parameter default, looking for a
  `self::`/`static::`/`parent::`/explicit-class-name `Class::member` reference (a static call, a class
  constant, `Class::class`, a static property) and resolving the class side the same way
  `hierarchy.rs` resolves `extends`/`implements`, then checking the member side against `MemberTable`
  plus every ancestor reached by walking `ClassGraph`'s `extends`/`implements`/`traits` edges
  transitively. Diagnoses an undeclared or wrong-kind class side (`E_UNDEFINED_CLASS`, `E0303`,
  reused) and an undeclared member (`E_UNDEFINED_MEMBER`, `E0309`, newly added this session, next in
  the E03xx name-resolution family after `E0308`). `Core\...` targets are trusted rather than checked,
  same pattern as everywhere else. `resolve_file` now runs this pass automatically and populates
  `Module.members`. 12 new unit tests in `members.rs` (44 total in the crate now), `cargo
  test`/`clippy -D warnings`/`fmt --check` all clean across the whole workspace.
- **Known gaps, called out in `members.rs`'s own module docs:**
  1. `self::`/`static::` inside a *trait*'s own method body resolves against the trait's own members
     plus whatever it itself pulls in via `use` — never against the class that ends up composing it,
     since that isn't known at the trait's declaration site. Matches PHP's own dynamic binding for
     this case; not treated as a bug.
  2. `new Foo(...)`/`new self(...)`/etc.'s target is not checked here — instantiation resolution is a
     distinct concern from a callable/constant reference and was deliberately left out of this slice's
     scope.
  3. A member's visibility (`private`/`protected`) is not checked — only whether it is declared
     anywhere in the chain. Visibility enforcement wasn't asked for by this item and needs
     `mwl-types`-level context (the accessing class) that member resolution alone doesn't carry cleanly.
  4. A dynamic class side (`$var::method()`, `(expr)::CONST`) is silently skipped — not statically
     resolvable, consistent with how this milestone treats everything it can't be sure of.

**What M2's plan paragraph still needs, not yet started:**

1. ~~The class hierarchy graph~~ — done (M2 item 1, prior session).
2. ~~Every callable/constant resolving as a class member~~ — done, see above.
3. Substituting a resolved `type` alias into the types that reference it (the transparency half of
   [ADR 0015](docs/adr/0015-no-name-aliasing.md) § 5 — only the declarations are collected and the
   bare-class case rejected so far; the substitution itself, and cycle detection
   `type A = B; type B = A;`, are still open). Do this one wherever `TypeAtom::Name` is resolved
   against the symbol table, since it's the same lookup either way. Note `hierarchy.rs`'s cycle-
   detection code (`detect_cycles`/`visit` — a stack-based DFS over `extends`/trait-use edges) is a
   reasonable template for the `type`-alias cycle check; it's a different edge set but the same
   shape. This is the natural next slice — items 4 and 5 below are independent of it, but the plan's
   ordering puts this one next.
4. `require`'s static resolution with a dynamic fallback
   ([ADR 0021](docs/adr/0021-single-file-inclusion-construct.md)) — this is also where
   `Resolver::collect_declarations`/`resolve_imports`'s multi-file design (already shaped for it, and
   `HierarchyResolver`/`MemberResolver` now mirror the same collect-then-resolve shape) gets exercised
   for the first time. Keep "does this path resolve statically" structurally separate from "fall back
   to a dynamic lookup" per [ADR 0025](docs/adr/0025-wasm-browser-target.md)'s note that the browser
   target forbids the dynamic fallback outright rather than merely deprioritising it.
5. The property-access resolution rule ([ADR 0014](docs/adr/0014-property-observer.md)) — refusing
   access to anything not declared on the class or an ancestor/trait, no `__get`/`__set` fallback.
   Now unblocked by both the class graph (item 1) and the member table (item 2): `members.rs`'s
   `MemberTable`/`member_declared` walk is almost the exact shape this needs — it would just need
   instance property names added to `ClassMembers` (currently only static properties are tracked,
   since item 2 only needed those) and a new walker hook at `ExprKind::PropertyAccess` (already
   structurally walked by `members.rs`'s `walk_expr` for its *sub-expressions*, but not yet checked as
   a member reference itself — extending `members.rs` in place, or a new sibling module, are both
   reasonable; decide based on how much the two diagnostics end up sharing once you're looking at the
   code).

**Also open, self-contained, no dependency on the above:** implement the ADR 0029/0030 casing check in
`crates/mwl-syntax` — a diagnostic pass over the AST nodes M1 already produces (class/interface/trait/
enum/enum-case/namespace-segment/method/property/parameter/local/const declarations), no `mwl-hir`
involvement needed. Remember: property/parameter/local now reject *any* leading underscore (no exception),
and a method literally named `__construct` gets its own targeted diagnostic naming `constructor` as the
fix, distinct from the generic mis-casing message. Good filler work between the numbered items above, or a
fine place to start the session if item 3 needs more thinking time first.

`mwl-types` (the type checker) and `mwl-ir` (CFG/SSA lowering) still haven't started — the plan's
Architecture diagram has them building on top of `mwl-hir`'s resolved names, not in parallel with it.
When `mwl-types` does start, it inherits two fresh checker-side rules from ADR 0028: refuse an object used
at an implicit string-conversion site unless its static type provably implements `Stringable`, and refuse
`unset()` on any declared object property outright.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs
before it can be called done — one file per diagnostic across ADR 0007, ADR 0022, ADR 0024, ADR 0027,
ADR 0028, ADR 0029 and ADR 0030, IR snapshot tests, the `Comparable` refusal cases from ADR 0013. None of
that corpus exists yet since most of it depends on `mwl-types`/`mwl-ir`; `mwl-hir`'s own unit tests (in
`resolve.rs`, `symbol.rs`, `qname.rs`, `hierarchy.rs`, and now `members.rs`) are the right home for
name-resolution-only cases in the meantime — keep adding to them as each new piece above lands, rather
than retrofitting a separate corpus later. ADR 0029/0030's corpus is the one exception: it can start as
soon as its `mwl-syntax` check exists, no need to wait for `mwl-types`.
