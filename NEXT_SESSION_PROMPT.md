# Next session prompt

Continue MWL. M1 (front end) is done and closed out — see git history if you need the detail; it's
not repeated here per CLAUDE.md's "state a fact once" rule.

**Last session was a documentation-only detour, not `mwl-hir` work: [ADR 0031](docs/adr/0031-callable-is-the-only-closure-type.md)
was written and accepted.** No crate code changed. Read `CLAUDE.md` first (it routes to the one file
you need per topic), then run `sh .claude/brief.sh` for the live status slice.

**What ADR 0031 decided**, in one paragraph since the ADR itself is the one home for the reasoning:
PHP's anonymous-function surface collapses to one literal, `fn`, with or without a block body
(`fn($x) => $x + 1` and `fn($x) => { ...; return $x; }` both parse) and an optional self-name for
recursion (`fn factorial($n) => ... factorial($n - 1) ...`, visible only inside its own body); there is
no `use` clause of any kind, capture is always implicit/by-value/minimal, and there is no by-reference
capture — sharing mutable state between two independent closures is an ordinary object in user code, not
a language feature. `Closure` is retired as a type name; `callable` is the only surviving spelling.
`Closure::fromCallable`, `call_user_func`, `call_user_func_array` are all dropped.

**This creates real, not-yet-done implementation work, called out in the ADR's own *Verification*
section** — none of it is done yet, and none of it blocks M2 item 5 below, but it needs picking up before
M1/M2 can be called complete against this ADR:

- **`crates/mwl-syntax`** (M1 retrofit): `ClosureExpr`/`ClosureUse`/`ArrowFnExpr` in `src/ast.rs` currently
  implement PHP's *old* two-literal, `use`-clause surface — that's what M1 shipped before this ADR existed.
  These need collapsing into one AST shape for the single `fn` literal (optional block body, optional
  self-name token, no `use` field at all), and `parser.rs` needs the old `function(...) {...}` /
  `function(...) use (...) {...}` productions replaced with diagnostics naming `fn` as the replacement
  (ADR 0031 § 6 has the exact diagnostic wording for each rejected spelling).
- **`crates/mwl-hir`** (once reached): a self-named `fn` literal's name must resolve only inside its own
  body and nowhere else — likely a small, local scope-table entry in whichever resolver ends up walking
  closure bodies, not a symbol-table entry, since it's explicitly not a declaration
  ([ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md) — this is why it doesn't reopen
  that ADR's rule).
- **`mwl-types`** (not started yet, see below): `callable` replaces every place a `Closure` type atom would
  have been checked; this is a pure rename with no behavior change, since ADR 0007 already made the type
  opaque.

Given M1 is otherwise closed, the parser retrofit above is arguably the more urgent of the two open
threads this session could pick up — it's the one place written ADR and shipped code have now diverged.

**What M2's plan paragraph still needs, not yet started** (unchanged from before this session, since no
`mwl-hir` code was touched):

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
ADR 0028, ADR 0029, ADR 0030 and now ADR 0031, IR snapshot tests, the `Comparable` refusal cases from ADR
0013. None of that corpus exists yet since most of it depends on `mwl-types`/`mwl-ir`; `mwl-hir`'s own
unit tests (in `resolve.rs`, `symbol.rs`, `qname.rs`, `hierarchy.rs`, `members.rs`, `aliases.rs`, and
`requires.rs`) are the right home for name-resolution-only cases in the meantime — keep adding to them as
each new piece above lands, rather than retrofitting a separate corpus later. ADR 0029/0030's corpus is
the one exception: it can start as soon as its `mwl-syntax` check exists, no need to wait for `mwl-types`.
