# Next session prompt

Continue MWL. M1 (front end) is done, including its ADR 0031 retrofit — see git history if you need
the detail; it's not repeated here per CLAUDE.md's "state a fact once" rule.

**Last session did the `mwl-syntax` retrofit ADR 0031 called for**: `ClosureExpr`/`ClosureUse`/
`ArrowFnExpr` are gone from `crates/mwl-syntax/src/ast.rs`, replaced by one `FnExpr`/`FnBody` shape
covering `fn(...) => expr`, `fn(...) => { ... }`, and an optional self-name for recursion
(`fn factorial($n) => ...`). `function(...) {...}` and `function(...) use (...) {...}` are rejected
with a diagnostic naming `fn` (`E0222`); a `use (...)` clause of either capture mode gets its own,
more specific diagnostic distinguishing by-reference (`E0223`/`E0224`, per ADR 0031 § 6's exact
wording). `crates/mwl-hir`'s two AST walkers that pattern-matched the old shapes
(`requires.rs`, `members.rs`) are updated for the new one. Full workspace `cargo test`, `cargo clippy
--all-targets -- -D warnings`, and `cargo fmt --check` are all green. ADR 0031's own *Verification*
section and `docs/implementation-plan.md`'s status block are both updated to mark this M1 item done —
read `CLAUDE.md` first (it routes to the one file you need per topic), then run `sh .claude/brief.sh`
for the live status slice.

**What's still open from ADR 0031**, not blocking M2 item 5 below:

- **`crates/mwl-hir`**: a self-named `fn` literal's name must resolve only inside its own body and
  nowhere else — likely a small, local scope-table entry in whichever resolver ends up walking closure
  bodies, not a symbol-table entry, since it's explicitly not a declaration
  ([ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md) — this is why it doesn't
  reopen that ADR's rule). Nothing in `mwl-hir` resolves any closure body's variable references yet at
  all (that's what M2 item 5 and `mwl-types` will start doing), so this has no test corpus yet either.
- **`mwl-types`** (not started, see below): `callable` replaces every place a `Closure` type atom would
  have been checked — a pure rename, no behavior change, since ADR 0007 already made the type opaque.

**What M2's plan paragraph still needs, not yet started:**

1. ~~The class hierarchy graph~~ — done (M2 item 1).
2. ~~Every callable/constant resolving as a class member~~ — done (M2 item 2).
3. ~~Type alias substitution~~ — done (M2 item 3).
4. ~~`require`'s static resolution~~ — done (M2 item 4).
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
   can drop the numbered-item framing entirely. This is the natural next task to pick up.

**Also open, self-contained, no dependency on the above:** implement the ADR 0029/0030 casing check in
`crates/mwl-syntax` — a diagnostic pass over the AST nodes M1 already produces (class/interface/trait/
enum/enum-case/namespace-segment/method/property/parameter/local/const declarations, and now `FnExpr`'s
self-name too, since it's an ordinary local-binding-shaped identifier), no `mwl-hir` involvement
needed. Remember: property/parameter/local now reject *any* leading underscore (no exception), and a
method literally named `__construct` gets its own targeted diagnostic naming `constructor` as the fix,
distinct from the generic mis-casing message. Good filler work between item 5 and `mwl-types`, or a
fine place to start the session if item 5 needs more thinking time first.

`mwl-types` (the type checker) and `mwl-ir` (CFG/SSA lowering) still haven't started — the plan's
Architecture diagram has them building on top of `mwl-hir`'s resolved names, not in parallel with it.
When `mwl-types` does start, it inherits two fresh checker-side rules from ADR 0028 (refuse an object used
at an implicit string-conversion site unless its static type provably implements `Stringable`, and refuse
`unset()` on any declared object property outright) plus the `callable`-not-`Closure` rename from ADR
0031 above. It also becomes `AliasTable`'s first real consumer — substituting an alias wherever a
declared type (a property, a parameter, a return type) is looked at — and it's the natural place for
`require`'s `mixed`-typed expression value (ADR 0021 § 3) to get its `as`-conversion-required treatment,
and for widening `requires.rs`'s literal-path detection into real constant folding if that turns out to be
worth doing.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs
before it can be called done — one file per diagnostic across ADR 0007, ADR 0022, ADR 0024, ADR 0027,
ADR 0028, ADR 0029, ADR 0030 and ADR 0031, IR snapshot tests, the `Comparable` refusal cases from ADR
0013. None of that corpus exists yet since most of it depends on `mwl-types`/`mwl-ir`; `mwl-hir`'s own
unit tests (in `resolve.rs`, `symbol.rs`, `qname.rs`, `hierarchy.rs`, `members.rs`, `aliases.rs`, and
`requires.rs`) are the right home for name-resolution-only cases in the meantime — keep adding to them as
each new piece above lands, rather than retrofitting a separate corpus later. ADR 0029/0030's corpus is
the one exception: it can start as soon as its `mwl-syntax` check exists, no need to wait for `mwl-types`.
`crates/mwl-syntax`'s own unit tests already cover ADR 0031's parser-level rejections
(`function_closure_with_use_by_ref_is_rejected` and its siblings in `parser.rs`), so that ADR's M1 slice
of the corpus is effectively already in place too.
