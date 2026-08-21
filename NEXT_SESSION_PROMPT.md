# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — see git history for
detail on how earlier items landed; it's not repeated here per CLAUDE.md's "state a fact once"
rule.

**Last session closed item 1 of the prior follow-up list**: [ADR 0022](docs/adr/0022-definite-property-initialization.md)
§ 2's definite-property-initialization check. Concretely:

- [`crates/mwl-diagnostics/src/lib.rs`](crates/mwl-diagnostics/src/lib.rs) — two new codes:
  `E_UNINITIALIZED_PROPERTY` (`E0409`, a required property unassigned on some path out of a
  constructor, or a class with no constructor at all to assign it) and
  `E_MISSING_PARENT_CONSTRUCTOR_CALL` (`E0410`, a subclass constructor with a path that never calls
  `parent::constructor(...)`).
- [`crates/mwl-types/src/ty.rs`](crates/mwl-types/src/ty.rs) — `TypeInterner::is_nullable`: whether
  an interned type is `null` itself or a union with `null` as a member, i.e. whether it was written
  with a leading `?` (or expands to one through a `type` alias). This is the one thing ADR 0022 § 1
  exempts from the obligation, so it needed a real answer rather than a syntactic guess at the AST.
- [`crates/mwl-types/src/signatures.rs`](crates/mwl-types/src/signatures.rs) — `ClassSignature`
  gained `required_properties: Vec<(String, Span)>`: a class's own properties that are non-nullable,
  have no inline default, and have no hook block (a hooked property is exempted entirely rather than
  modeled — see the next bullet). `own_required_properties(qname, table, graph)` walks `qname`'s own
  `required_properties` plus every used trait's own, recursively through nested trait-use — the same
  flattening idea `resolve_property`/`resolve_method` already use, but restricted to `traits` alone:
  an *inherited* property (via `extends`/`implements`) is discharged by calling
  `parent::constructor(...)`, not by assigning it a second time, so those ancestors are deliberately
  not walked here.
- [`crates/mwl-types/src/ctor_init.rs`](crates/mwl-types/src/ctor_init.rs) — new module,
  `check_class_init`, called from `check.rs` right after a `ClassDecl`'s members are checked (never
  for an interface/trait/enum — only a class is ever instantiated through a constructor). A class
  with no constructor and a required property is refused right at that property's own declaration
  (ADR 0022 § 2's third bullet). A class with a constructor gets a second, narrower flow-analysis
  pass over its body alone — separate from `locals.rs`'s pass, since it tracks different per-path
  state (which required properties are assigned, and whether `parent::constructor(...)` has been
  called) rather than re-walking for type correctness. The walk mirrors `locals.rs`'s control-flow
  shape exactly: `if`/`else` joins by intersecting the two branches' state, `switch`/`try`'s body and
  catches conservatively contribute nothing (only `finally`/a `do`-`while` body, which always run,
  update the carried state) — same known-safe gaps as `locals.rs`'s own. Diagnostics fire at every
  point a path can leave the constructor: each explicit `return`, and the implicit one at the body's
  end if some path never returns explicitly (ADR 0022 § 2 is stated per-return, not per-body). Only
  a plain `$this->prop = ...` (never a compound operator, mirroring `expr.rs::check_assign`'s own
  rule for locals) and a lexical `parent::constructor(...)` call are recognized, and only within a
  handful of common composite expression forms (assignment, calls, binary/unary/cast/ternary,
  `instanceof`, array literals) — see the module's own docs for the exact list and why a form outside
  it produces a spurious diagnostic rather than a missed one (safe, since it can only reject a valid
  program).
- [`crates/mwl-types/src/check.rs`](crates/mwl-types/src/check.rs), `lib.rs` — wiring and doc updates
  to reflect the new pass and module.
- [`docs/implementation-plan.md`](docs/implementation-plan.md)'s M2 paragraph updated to describe
  this session's work as settled fact, in the same place the prior session's own paragraph already
  lived.

Full workspace `cargo test` (10 new tests in `mwl-types::ctor_init`, everything else still green —
62 tests total in `mwl-types` alone), `cargo clippy --all-targets -- -D warnings`, `cargo fmt
--check`, and `cargo doc -p mwl-types --no-deps` (no new *unresolved* intra-doc links — the
existing "links to a private item" warnings predate this session and resolve fine with
`--document-private-items`) are all clean. The CLI was also smoke-tested by hand: a property with no
default and no constructor produced `E0409` at the property; a subclass constructor that never calls
`parent::constructor(...)` produced `E0410` at the constructor; a correct pair (base assigns its own
property, subclass calls `parent::constructor(...)` then assigns its own) produced no diagnostics.

**Deliberately out of scope for this slice — the next thread to pick up, in the order that makes
sense to attempt them:**

1. **ADR 0013 (`Comparable`), ADR 0027 (`callable` value-shape), ADR 0028 (`Stringable`, `unset()`
   refusal), ADR 0024 §§ 2-3 (tainted propagation/laundering), ADR 0010 (enum-vs-class atom
   distinction beyond "resolves to *a* symbol" — enum-specific operations).** Each is its own
   self-contained checker-side rule layered on top of the type table, signature table, and now the
   constructor-init pass this and the prior two sessions built; none of them depend on each other, so
   they can land in any order or be split across sessions. This item was already next in line before
   this session started (this session did the ADR 0022 item ahead of it, per the previous prompt's
   explicit ordering) — still not started. Note for whichever of these lands first:
   `expr::class_of_ctx` currently always interns `Ty::Class(qname)` for `self`/`static`/`$this`, even
   when the enclosing declaration is an enum — ADR 0010's own item will need to fix that (distinguish
   via `SymbolTable`'s `SymbolKind`, the same check `lower.rs`'s `resolve_name_type` already does for
   an ordinary type position) before enum-specific operations can tell `self` apart from a class.
2. **`switch`/`try` definite-assignment precision, and `parent` as a *type* atom** (`parent $x` —
   distinct from `new parent(...)`, which resolves since two sessions ago), plus the equivalent
   precision gap `ctor_init.rs` now shares with `locals.rs` (both conservatively contribute nothing
   through `switch`/`try`'s body and catches) — all named as known gaps for a while now, all
   safe-but-imprecise today (reject a few extra valid programs rather than ever accepting an invalid
   one), worth revisiting once the higher-value items above are done rather than before.
3. **Smaller, independent polish items surfaced across the last few sessions, any of which could be
   a quick follow-up on its own:** a class constant's type (`Class::CONST` stays `mixed` regardless
   of receiver — there's no const-value type table yet); a promoted constructor-parameter property
   (`function constructor(public int $x) {}`) is recorded as neither a property nor a
   definite-assignment obligation anywhere — this mirrors a pre-existing `mwl_hir::members` gap, so
   fixing it well might mean fixing both crates together, `signatures.rs`, and `ctor_init.rs` in the
   same pass; a named or spread call argument disables *all* per-argument checking for that call
   rather than being matched positionally where possible; a class with no explicit `constructor` is
   not held to a zero-argument arity check on `new Foo(...)`, nor to the
   `parent::constructor(...)` obligation `ctor_init.rs` checks for a class that does declare one; a
   property backed by a `set` hook is exempted from ADR 0022's check entirely rather than verified
   against whether the hook's own body actually commits a value.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone
needs before it can be called done. The ADR 0007 and ADR 0022 corpus entries are now both fully
satisfied; the rest of that Verify line (ADR 0010, 0013, 0014, 0024, 0027, 0028's own corpus entries,
plus IR snapshot tests) still depends on the work items above, or on `mwl-ir`, which hasn't started.
