# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — see git history for
detail on how M1 and M2's earlier items landed; it's not repeated here per CLAUDE.md's "state a
fact once" rule.

**Last session finished M2 item 5**, the last item on `mwl-hir`'s five-item name-resolution list
([ADR 0014](docs/adr/0014-property-observer.md) § 5's property-access rule): `$this->name` now
must resolve to something declared on the enclosing class or reached through the `ClassGraph`
(`extends`/`implements`/trait-use), the same way a `Class::member` reference already does via M2
item 2's `MemberTable`. Concretely: `ClassMembers` (in
[`crates/mwl-hir/src/members.rs`](crates/mwl-hir/src/members.rs)) grew a `props: FxHashSet<String>`
field alongside its existing `static_props`, `MemberKind` grew a `Prop` variant, and
`ExprKind::PropertyAccess` is now checked when its receiver is literally `$this` and its member
name is a literal identifier — an undeclared property is `E_UNDEFINED_PROPERTY` (`E0313`, newly
added to `crates/mwl-diagnostics`). `crates/mwl-hir/src/lib.rs`'s module docs dropped the old
numbered "what this slice of M2 covers" framing (all five items are done now, so the per-item
tracking no longer earns its keep) in favor of a short "Known gaps" section pointing at each
module's own docs. `docs/implementation-plan.md`'s M2 status paragraph and *Verify* corpus list
both got a matching update (a new "Plus ADR 0014's own entry" clause). Full workspace `cargo test`,
`cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` are all green — read
`CLAUDE.md` first (it routes to the one file you need per topic), then run `sh .claude/brief.sh` for
the live status slice.

**Known gap left behind, by design:** a property access on any receiver other than `$this` — a
typed local, a chained call result, `self::factory()`'s return, an explicit `new Foo()` — is not
checked at all yet. It needs `mwl-types`' static types to know which class's properties apply to a
given expression, which hasn't started (see below). This is documented in `members.rs`'s module
docs, not tracked as a TODO anywhere else.

**What M2's plan paragraph needed from `mwl-hir` is now entirely done** (all five items: class
hierarchy graph, member resolution, type alias substitution, `require`'s static resolution,
property-access resolution). `mwl-hir`'s five listed responsibilities are complete; nothing further
is planned for that crate until `mwl-types` needs to consume one of its tables (the `AliasTable` in
particular has no consumer yet — see below).

**Two independent threads open next, pick either:**

1. **`mwl-types` (the type checker) — the bigger, not-yet-started piece of M2.** The plan's
   Architecture diagram has it building on top of `mwl-hir`'s resolved names, so it's unblocked now
   that `mwl-hir` is feature-complete for M2. It inherits, from earlier ADRs already Accepted but
   not yet implemented anywhere:
   - ADR 0007's full type table: declared-type recording/enforcement, definite-assignment
     checking, flow-sensitive union narrowing, array element-type checks at every write and
     nesting depth, the arithmetic result-type table (including refusing `int + uint`), interned
     type descriptors. **No inference engine, no `Unknown` type** — every binding's type is
     declared, so this is a checker, not a solver.
   - ADR 0022's definite-property-initialization rule, extended into constructor bodies (every
     constructor must assign every declared property on every path).
   - ADR 0013's `Comparable` requirement for `<`/`>`/`<=`/`>=`/`<=>` on two objects — refused
     unless both sides are provably the same class implementing it, no property-walk fallback.
   - ADR 0024 §§ 2-3's `tainted` qualifier propagation (concatenation/interpolation poison) and
     laundering (a checked `as` conversion or a named `Core` function), plus the sink refusal.
   - ADR 0027's `callable`-only rule: a bare string, `"Class::method"`, or `[$obj, 'method']` are
     all refused where `callable` is declared, naming first-class-callable syntax as the fix; no
     `__invoke` exists, so `$obj(...)` is refused for any non-`callable` `$obj` regardless of its
     class's methods.
   - ADR 0028's two checker-side rules: an object used at an implicit string-conversion site
     needs a provable `Stringable` implementation; `unset()` on any declared object property is
     refused outright.
   - ADR 0031's `callable`-not-`Closure` rename — a pure rename since ADR 0007 already made the
     type opaque, no behavior change.
   - `AliasTable`'s first real consumer: substituting a `type` alias wherever a declared type
     (property, parameter, return type) is looked at. This is also the natural place for
     `require`'s `mixed`-typed expression value (ADR 0021 § 3) to get its `as`-conversion-required
     treatment, and for widening `requires.rs`'s literal-path detection into real constant folding
     if that turns out to be worth doing.
   This is a large milestone slice — worth scoping into its own sub-steps (e.g. start with the
   type table and definite-assignment, since everything else in the list above depends on having
   a type to check against) rather than attempting all of it in one session.

2. **The ADR 0029/0030 casing check, in `crates/mwl-syntax`** — smaller, self-contained, no
   dependency on `mwl-types` or anything else above. A diagnostic pass over the AST nodes M1
   already produces: class/interface/trait/enum/enum-case/namespace-segment/method/property/
   parameter/local/const declarations, and `FnExpr`'s self-name too (it's an ordinary
   local-binding-shaped identifier). Remember: property/parameter/local reject *any* leading
   underscore, no exception ([ADR 0030](docs/adr/0030-no-leading-underscores-constructor-spelling.md)),
   and a method literally named `__construct` gets its own targeted diagnostic naming
   `constructor` as the fix, distinct from the generic mis-casing message. Good filler work, or a
   fine place to start if `mwl-types`' scope needs more thinking time first.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone
needs before it can be called done — one file per diagnostic across ADR 0007, ADR 0013, ADR 0014,
ADR 0022, ADR 0024, ADR 0027, ADR 0028, ADR 0029, ADR 0030 and ADR 0031, plus IR snapshot tests.
Almost none of that corpus exists yet since most of it depends on `mwl-types`/`mwl-ir`, neither of
which has started; `mwl-hir`'s own unit tests (in `resolve.rs`, `symbol.rs`, `qname.rs`,
`hierarchy.rs`, `members.rs`, `aliases.rs`, and `requires.rs`) are the right home for
name-resolution-only cases in the meantime, and are now complete for everything `mwl-hir` itself
checks. ADR 0029/0030's corpus is the one exception that can start as soon as its `mwl-syntax`
check exists, no need to wait for `mwl-types`. `crates/mwl-syntax`'s own unit tests already cover
ADR 0031's parser-level rejections, so that ADR's M1 slice of the corpus is effectively already in
place too.
