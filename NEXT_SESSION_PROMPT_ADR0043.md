# Next session prompt — implement ADR 0043 (traits removed)

**This is a separate handoff file, not `NEXT_SESSION_PROMPT.md`.** That file is currently owned by an
active autonomous coordinator loop (see `AGENT_COORDINATOR_PROMPT.md`) working the `mwl-ir` milestone —
overwriting it from this session would race with that loop's subagent and likely get silently discarded.
Fold this file's content into `NEXT_SESSION_PROMPT.md` once that loop is paused or between its turns, or
run this as its own manually-launched session.

## What landed

[ADR 0043](docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md) (committed, docs-only):
`trait`, class-body `use Trait, ...;`, and `insteadof` are removed from the language entirely. In their
place: an `interface` method may carry a `public` (default, inherited/overridable) or `private`
(internal-helper) body, and one entry in a class's `implements` list may carry a `by $field;` suffix that
delegates every method that interface requires to an ordinary property. One conflict rule covers both: a
method reachable from more than one default/delegated source with no class override is
`E_INTERFACE_MEMBER_CONFLICT` — there is no `insteadof`. Full grammar, semantics, and the `mwl convert`
migration path (four PHP-trait shapes, two fully mechanical) are in the ADR; read it in full before starting,
not just this summary.

This amends [ADR 0015](docs/adr/0015-no-name-aliasing.md) (§ 3 withdrawn) and
[ADR 0022](docs/adr/0022-definite-property-initialization.md) (drops the "trait-contributed property" special
case). `docs/implementation-plan.md`'s M1/M2/M4/M11 sections and `CLAUDE.md`'s routing table/ground rules
were updated to match. Incidental "trait" mentions in ADRs 0013/0014/0019/0028/0029/0039 were cleaned up too.

## What's still code, not docs — the actual task for this session

The ADR's own *Consequences* and *Verification* sections name the exact scope; this is a restatement, not a
substitute for reading them.

**`crates/mwl-syntax` (parser/AST):**
- Remove `TraitDecl`, `UseTraitMember`, `TraitAdaptation`/`TraitAdaptationKind`, `TraitMethodRef` from
  `ast.rs`.
- Remove the parser productions for `trait Name { ... }` and class-body `use Trait, ...;` (with or without
  an adaptation block), replacing them with a parse-time diagnostic `E_TRAIT_NOT_SUPPORTED` naming the
  replacement (ADR 0043 § 7) — same shape as `E0225`'s legacy-cast rejection in ADR 0034.
- Add grammar: an interface method declaration may have a body (currently interface methods are
  signature-only); a `public`/`private` modifier controls default-vs-helper per ADR 0043 §§ 2-3.
- Add grammar: one entry in a class's `implements` list may be followed by `by $field;` (ADR 0043 § 4).
- Update `casing.rs`/`token.rs` wherever they special-cased trait declarations.

**`crates/mwl-hir` (name resolution):**
- Remove `SymbolKind::Trait` (`symbol.rs`), the `TraitDecl` match arms in `resolve.rs`/`members.rs`/
  `requires.rs`, and `hierarchy.rs`'s entire trait-use/`insteadof` machinery (`trait_refs`, `trait_methods`,
  `check_trait_conflicts`, `E_TRAIT_METHOD_CONFLICT`, and the associated tests).
- Add: resolution of a `public`/`private` interface method body: a private one is visible only from other
  method bodies declared on the *same* interface (`E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE` otherwise); `$this`
  inside any interface method body (default or private) resolves only against that interface's own declared
  members plus whatever it `extends` — not the concrete implementing class.
- Add: `by $field` resolution — `$field`'s declared type must satisfy the delegated interface
  (`E_DELEGATE_TYPE_MISMATCH` otherwise); the compiler-synthesized forwarding methods this produces.
- Add: `E_INTERFACE_MEMBER_CONFLICT` — one rule covering both a default-vs-default collision across two
  implemented interfaces and a default-vs-delegate or delegate-vs-delegate collision, whenever the class
  itself does not declare an overriding method.
- Add: the `InterfaceName::method()` qualified-call form (ADR 0043 § 5) — reuses the existing
  `parent::`/`self::`/`static::`/(formerly `TraitName::`) qualified-call shape, now meaningful for an
  interface name since interfaces can have method bodies at all.

**Verification to add**, mirroring the corpus-fixture style every other ADR in this repo uses: one fixture
per diagnostic named above, plus the positive cases (a default method inherited with no override, an
overridden default, a working `by`-delegation, a private helper called from within its own interface, a
private helper call rejected from outside it).

Do **not** attempt `mwl convert`'s migration rewrites (ADR 0043 § 6) — that's M11, not yet started, and has
its own already-designed mechanical/flagged/TODO split in the ADR; nothing to build until M11 begins.

## Before finishing

Run `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`. Commit in
small, focused commits per `CLAUDE.md`. Given the size of this change (removing an already-implemented
subsystem and building two new ones), consider splitting into at least two commits: one removing the
trait-specific code paths (with the parse-time rejection landing in the same commit, so nothing is left
half-parsing), one adding default/private methods, and one adding `by`-delegation — whatever split keeps
each commit's `cargo test` green.

Once done, fold this file's "what's left" (if anything remains) into whichever session-handoff file this
project is using by then, and delete this file (`NEXT_SESSION_PROMPT_ADR0043.md`) once its content has been
merged in — it is a one-time handoff, not a permanent second prompt file living alongside
`NEXT_SESSION_PROMPT.md`.
