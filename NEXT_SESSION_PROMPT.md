# Next session prompt

Continue MWL. M1 (front end) is done and closed out — see git history if you need the detail; it's
not repeated here per CLAUDE.md's "state a fact once" rule.

**Last session finished M2 item 4, `require`'s static resolution, in `crates/mwl-hir`.** Read
`CLAUDE.md` first (it routes to the one file you need per topic), then run `sh .claude/brief.sh` for
the live status slice, then read the plan's M2 paragraph in `docs/implementation-plan.md` in full,
then read `crates/mwl-hir/src/lib.rs`'s module docs and `crates/mwl-hir/src/requires.rs`'s module
docs — all three carry the same up-to-date breakdown of what's built and what's left.

**What exists now**, on top of the symbol table, class hierarchy graph, member table and alias table
from before (namespace/`use` scoping, the symbol table, `type`-alias declaration collection plus ADR
0015 § 6's bare-class rejection, `extends`/`implements`/trait-use resolution, `Class::member`
resolution, `type`-alias substitution with cycle detection — all unchanged, still in
`qname.rs`/`symbol.rs`/`resolve.rs`/`hierarchy.rs`/`members.rs`/`aliases.rs`):

- `requires.rs` — [`resolve_program`]. Walks the `require` graph reachable from one entry file using
  an explicit worklist (not recursion, to keep the borrow checker happy around `SourceMap`'s mutable
  file-loading calls): each literal `'...'`/`"..."` `require` path is resolved relative to the
  requiring file's own on-disk directory, loaded and parsed the first time its canonicalized path is
  named, and fed through `Resolver::collect_declarations`/`HierarchyResolver::collect_links`/
  `MemberResolver::collect_members`/`AliasResolver::collect_aliases` — the exact multi-file design
  those four resolvers were already shaped for, per their own module docs. A missing/unloadable
  literal target is `E_REQUIRE_TARGET_NOT_FOUND` (`E0311`, newly added); a require chain leading back
  to a file already being resolved is `E_CIRCULAR_REQUIRE` (`E0312`, newly added) rather than infinite
  recursion; a file reachable by more than one path (a diamond, not a cycle) is loaded and collected
  exactly once, keyed by its canonicalized path, so it doesn't collide with itself under
  `E_DUPLICATE_DECLARATION`. A non-literal path (a variable, a concatenation, an interpolated string)
  is left completely untouched — no diagnostic, nothing collected — for the dynamic runtime fallback
  ADR 0021 names; a literal `require` inside a file with no on-disk path (every test fixture built with
  `SourceMap::add` rather than `SourceMap::load`) has no directory to resolve against and is left the
  same way. 7 new unit tests in `requires.rs` (67 total in the crate now, using real temp-directory
  fixtures since this is the first `mwl-hir` slice that needs actual files on disk), `cargo
  test`/`clippy -D warnings`/`fmt --check` all clean across the whole workspace.
- **Known gaps, called out in `requires.rs`'s own module docs:** only a plain quoted-string literal is
  recognised as statically known — heredoc/nowdoc and anything built out of one (concatenation, a
  `const`, an `as` conversion) is left dynamic even where a human could work out the value; that's a
  constant-folding problem for a later milestone, not a name-resolution one. The literal-cooking
  function handles a practical escape subset (`\\`, `\"`, `\$`, `\n`, `\r`, `\t`, `\v`, `\f`, `\e`) —
  good enough for a file path — rather than the full PHP string-escape grammar; the real cooker is
  future work once something besides this module needs it.

**What M2's plan paragraph still needs, not yet started:**

1. ~~The class hierarchy graph~~ — done (M2 item 1).
2. ~~Every callable/constant resolving as a class member~~ — done (M2 item 2).
3. ~~Type alias substitution~~ — done (M2 item 3).
4. ~~`require`'s static resolution~~ — done (M2 item 4, this session).
5. The property-access resolution rule ([ADR 0014](docs/adr/0014-property-observer.md)) — refusing
   access to anything not declared on the class or an ancestor/trait, no `__get`/`__set` fallback.
   Unblocked by both the class graph (item 1) and the member table (item 2): `members.rs`'s
   `MemberTable`/`member_declared` walk is almost the exact shape this needs — it would just need
   instance property names added to `ClassMembers` (currently only static properties are tracked,
   since item 2 only needed those) and a new walker hook at `ExprKind::PropertyAccess` (already
   structurally walked by `members.rs`'s `walk_expr` for its *sub-expressions*, but not yet checked as
   a member reference itself — extending `members.rs` in place, or a new sibling module, are both
   reasonable; decide based on how much the two diagnostics end up sharing once you're looking at the
   code). This is the last item on M2's name-resolution list — once it lands, `crates/mwl-hir`'s five
   listed responsibilities are all done and the crate's module docs' "what this slice covers" section
   can drop the numbered-item framing entirely.

**Also open, self-contained, no dependency on the above:** implement the ADR 0029/0030 casing check in
`crates/mwl-syntax` — a diagnostic pass over the AST nodes M1 already produces (class/interface/trait/
enum/enum-case/namespace-segment/method/property/parameter/local/const declarations), no `mwl-hir`
involvement needed. Remember: property/parameter/local now reject *any* leading underscore (no exception),
and a method literally named `__construct` gets its own targeted diagnostic naming `constructor` as the
fix, distinct from the generic mis-casing message. Good filler work between item 5 and `mwl-types`, or a
fine place to start the session if item 5 needs more thinking time first.

`mwl-types` (the type checker) and `mwl-ir` (CFG/SSA lowering) still haven't started — the plan's
Architecture diagram has them building on top of `mwl-hir`'s resolved names, not in parallel with it.
When `mwl-types` does start, it inherits two fresh checker-side rules from ADR 0028: refuse an object used
at an implicit string-conversion site unless its static type provably implements `Stringable`, and refuse
`unset()` on any declared object property outright. It also becomes `AliasTable`'s first real consumer —
substituting an alias wherever a declared type (a property, a parameter, a return type) is looked at — and
it's the natural place for `require`'s `mixed`-typed expression value (ADR 0021 § 3) to get its
`as`-conversion-required treatment, and for widening `requires.rs`'s literal-path detection into real
constant folding if that turns out to be worth doing.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs
before it can be called done — one file per diagnostic across ADR 0007, ADR 0022, ADR 0024, ADR 0027,
ADR 0028, ADR 0029 and ADR 0030, IR snapshot tests, the `Comparable` refusal cases from ADR 0013. None of
that corpus exists yet since most of it depends on `mwl-types`/`mwl-ir`; `mwl-hir`'s own unit tests (in
`resolve.rs`, `symbol.rs`, `qname.rs`, `hierarchy.rs`, `members.rs`, `aliases.rs`, and now `requires.rs`)
are the right home for name-resolution-only cases in the meantime — keep adding to them as each new piece
above lands, rather than retrofitting a separate corpus later. ADR 0029/0030's corpus is the one exception:
it can start as soon as its `mwl-syntax` check exists, no need to wait for `mwl-types`.
