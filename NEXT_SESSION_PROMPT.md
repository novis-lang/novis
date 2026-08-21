# Next session prompt

Continue MWL. M1 (front end) is done and closed out — see git history if you need the detail; it's
not repeated here per CLAUDE.md's "state a fact once" rule.

**Last session finished M2 item 3, type alias substitution, in `crates/mwl-hir`.** Read `CLAUDE.md`
first (it routes to the one file you need per topic), then run `sh .claude/brief.sh` for the live
status slice, then read the plan's M2 paragraph in `docs/implementation-plan.md` in full, then read
`crates/mwl-hir/src/lib.rs`'s module docs and `crates/mwl-hir/src/aliases.rs`'s module docs — all
three carry the same up-to-date breakdown of what's built and what's left.

**What exists now**, on top of the symbol table, class hierarchy graph and member table from before
(namespace/`use` scoping, the symbol table, `type`-alias declaration collection plus ADR 0015 § 6's
bare-class rejection, `extends`/`implements`/trait-use resolution, `Class::member` resolution — all
unchanged, still in `qname.rs`/`symbol.rs`/`resolve.rs`/`hierarchy.rs`/`members.rs`):

- `aliases.rs` — `AliasTable`/`AliasResolver`. Same two-pass shape as `HierarchyResolver`/
  `MemberResolver`: `collect_aliases` walks a file's `type` declarations, recording each one's raw
  expansion (`Type` AST, cloned) together with the namespace/`use` imports active at its declaration
  site, plus every `TypeAtom::Name` leaf's extracted source text keyed by its span (so resolution
  needs no `SourceFile` afterward — the same trick `hierarchy.rs`'s `RawRef` uses for one name,
  generalised to a whole type tree). `resolve` then substitutes every alias's expansion in one
  closing pass: a `Name` atom that resolves (via `hierarchy::resolve_ref`) to another pending alias
  is replaced by that alias's own — recursively substituted, memoized — expansion; a `Name` atom
  that resolves to a class/interface/enum, or isn't a name atom at all (a scalar, `array<...>`'s own
  shape, `?`/union/intersection structure), is left exactly as written. A cycle
  (`type A = B; type B = A;`, or longer) is diagnosed once (`E_TYPE_ALIAS_CYCLE`, `E0310`, newly
  added, next after `E0309`) and every name that took part in it still gets a real `AliasTable` entry
  expanding to `mixed`, rather than a lookup miss that would read as "not an alias at all." Wired into
  `resolve_file`, populating the new `Module.aliases` field. 9 new unit tests in `aliases.rs` (62
  total in the crate now), `cargo test`/`clippy -D warnings`/`fmt --check` all clean across the whole
  workspace.
- **Known gap, called out in `aliases.rs`'s own module docs:** the table has no consumer yet. There is
  no property/parameter/return-type walk anywhere in `mwl-hir` — that arrives with `mwl-types`, which
  is where `AliasTable::get` will actually get called against a type position. An atom that resolves
  to nothing declared at all (not a class, not an alias, not `Core`) is also not diagnosed here — a
  general "does this name resolve to something real" check belongs to the type checker, same
  narrowing `members.rs` already applies to `Class::member` references.

**What M2's plan paragraph still needs, not yet started:**

1. ~~The class hierarchy graph~~ — done (M2 item 1).
2. ~~Every callable/constant resolving as a class member~~ — done (M2 item 2).
3. ~~Type alias substitution~~ — done (M2 item 3, this session).
4. `require`'s static resolution with a dynamic fallback
   ([ADR 0021](docs/adr/0021-single-file-inclusion-construct.md)) — this is where
   `Resolver::collect_declarations`/`resolve_imports`'s multi-file design (already shaped for it, and
   `HierarchyResolver`/`MemberResolver`/`AliasResolver` all mirror the same collect-then-resolve
   shape) gets exercised for the first time. Keep "does this path resolve statically" structurally
   separate from "fall back to a dynamic lookup" per
   [ADR 0025](docs/adr/0025-wasm-browser-target.md)'s note that the browser target forbids the
   dynamic fallback outright rather than merely deprioritising it. This is the natural next slice per
   the plan's ordering — item 5 below is independent of it.
5. The property-access resolution rule ([ADR 0014](docs/adr/0014-property-observer.md)) — refusing
   access to anything not declared on the class or an ancestor/trait, no `__get`/`__set` fallback.
   Unblocked by both the class graph (item 1) and the member table (item 2): `members.rs`'s
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
fine place to start the session if item 4 or 5 needs more thinking time first.

`mwl-types` (the type checker) and `mwl-ir` (CFG/SSA lowering) still haven't started — the plan's
Architecture diagram has them building on top of `mwl-hir`'s resolved names, not in parallel with it.
When `mwl-types` does start, it inherits two fresh checker-side rules from ADR 0028: refuse an object used
at an implicit string-conversion site unless its static type provably implements `Stringable`, and refuse
`unset()` on any declared object property outright. It also becomes `AliasTable`'s first real consumer —
substituting an alias wherever a declared type (a property, a parameter, a return type) is looked at.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs
before it can be called done — one file per diagnostic across ADR 0007, ADR 0022, ADR 0024, ADR 0027,
ADR 0028, ADR 0029 and ADR 0030, IR snapshot tests, the `Comparable` refusal cases from ADR 0013. None of
that corpus exists yet since most of it depends on `mwl-types`/`mwl-ir`; `mwl-hir`'s own unit tests (in
`resolve.rs`, `symbol.rs`, `qname.rs`, `hierarchy.rs`, `members.rs`, and now `aliases.rs`) are the right
home for name-resolution-only cases in the meantime — keep adding to them as each new piece above lands,
rather than retrofitting a separate corpus later. ADR 0029/0030's corpus is the one exception: it can start
as soon as its `mwl-syntax` check exists, no need to wait for `mwl-types`.
