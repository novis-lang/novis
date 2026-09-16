---
milestone: post-parity
---
# Loop goal 61 — a shape a class owns is named inside it

A `type` alias can be declared as a member of a class, an interface or an enum, and the name it
declares is reachable as `Owner::Name` from anywhere and as a bare `Name` inside the owner's own body.
It is the same alias the file-scope form declares — transparent, compile-time only, refused for a bare
class, gone before codegen — with a second place to declare it, so a shape that belongs to one class no
longer has to sit at file scope beside the imports. The file-scope form stays exactly as it is.

## Why here

Directly after goal `unowned-closures` and in front of goal `gap-zero`, because it is the last
language-surface change the user has asked for before the terminal gate, and `gap-zero` refuses any
module-doc gap that is not owed to a future milestone: a feature that lands half-built after it would
have nowhere to record its remainder. It needs nothing the closure goals build. What it needs already
exists — the alias table (`crates/nvs-hir/src/aliases.rs`), `Owner::Name` in type position
(`crates/nvs-syntax/src/parser/ty.rs:538`, `crates/nvs-types/src/lower.rs:@lower_member_type`), and
the class-member parser (`crates/nvs-syntax/src/parser/decl.rs:@parse_class_member_with_attrs`).

## Stage 0 — the catch-up

Two sentences on disk become wrong the moment stage 2 lands, and the slice that lands it rewrites them:

- `docs/rules/types/type-alias.md:1-2` says an alias is declared "never inside a class". Stage 5's
  record amends that fragment; the code must not land against a rule that still says the opposite for
  more than the one group that lands both.
- `crates/nvs-types/src/lower.rs:333-338` records a known gap in `lower_member_type`: the name left of
  `::` is not resolved through the alias table first, so `type M = Mode; M::Read` resolves nothing.
  Stage 2 rewrites that function and closes the gap in the same slice, because the function's new job
  is exactly deciding what the left-hand name is.

## Stage 1 — the floor

Goal `unowned-closures`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: the member, its key, and the two spellings

One file set: `crates/nvs-syntax/src/parser/decl.rs`, `crates/nvs-syntax/src/ast.rs`,
`crates/nvs-hir/src/aliases.rs`, `crates/nvs-hir/src/resolve.rs`, `crates/nvs-types/src/lower.rs`.

- **The member.** `ClassMemberKind` gains a `TypeAlias(TypeAliasDecl)` arm
  (`crates/nvs-syntax/src/ast.rs:1631`), parsed where a class body dispatches on `const` and
  `function` (`crates/nvs-syntax/src/parser/decl.rs:709-724`) by the same routine the file-scope form
  uses (`decl.rs:@parse_type_alias_decl`). It is accepted in a class, an interface and an enum body,
  and a `///` run above it attaches as it does to any member.
- **The key.** The alias table is keyed by the owner's `QName` plus the member name — never by a
  synthetic namespace path, because `Ns\Order\Meta` is also the spelling of a class `Meta` in namespace
  `Ns\Order`, and the two must not share a key. The collector (`aliases.rs:@collect_in`) walks class
  bodies for the member; the cycle check and `E_TYPE_ALIAS_CYCLE` apply to it unchanged, and
  `check_alias_is_not_a_bare_class` (`resolve.rs:255`) runs on it unchanged, so `type Id = SomeClass;`
  is `E0307` inside a body exactly as it is outside one.
- **The two spellings.** `Owner::Name` in type position is `lower_member_type`'s third meaning
  (`lower.rs:339`), tried first: an alias declared by the resolved owner expands and re-lowers, then
  the enum case, then the constant fold, in that order. A bare `Name` inside the owner's body resolves
  against the owner's own aliases before the namespace's (`lower.rs:@resolve_name_type`, through
  `Ctx::current_class`). These are the only two spellings — see *Standing decisions*.
- **The left of `::` resolves through the alias table first**, closing the stage 0 gap: a file-scope or
  class-scoped alias whose expansion is a single name atom stands in for that name before the `::` is
  read, and any other expansion left of `::` is `E_UNKNOWN_MEMBER` with a help naming what the alias
  expands to.
- **Proof**: a parser test that the member round-trips, an `nvs-hir` test that the table holds the
  owner-keyed entry and refuses a cycle through it, and an `nvs-types` test that a value bound through
  `Order::Meta`, through a bare `Meta` inside `Order`, and through the same shape written at file scope
  is one type.

## Stage 3 — the refusals

Same file set as stage 2 plus `crates/nvs-diagnostics/src/lib.rs`.

- **No modifier.** `public type`, `private type`, `static type` and every other modifier before `type`
  in a body is one new diagnostic, naming the rule: a class-scoped alias has no visibility because it
  has no runtime existence, and is reachable wherever its owner is.
- **No collision.** An alias and a constant, or an alias and an enum case, sharing a name under one
  owner is `E_DUPLICATE_DECLARATION` at the second declaration, because both are spelled `Owner::Name`
  in type position and one spelling names one thing.
- **Not inherited.** `Child::Meta` where only `Parent` declares `Meta` is `E_UNKNOWN_MEMBER`, with a
  help naming `Parent::Meta` as the spelling that resolves. An interface's alias is likewise reached
  through the interface's own name, never through an implementor's.
- **Not in a body.** `type` inside a method, a closure or a block is refused where it is written, by
  name, the way `E0233` refuses a nested class — not left to fall through as a statement error.
- **Proof**: one conformance `reject` case per refusal, each pinning the diagnostic's own text.

## Stage 4 — the tooling sees the member

One file set: `crates/nvs-fmt/src/`, `crates/nvs-cli/src/meta.rs`, `crates/nvs-lsp/src/definition.rs`,
`crates/nvs-lsp/src/symbols.rs`, `crates/nvs-lsp/src/semantic.rs`, `crates/nvs-lsp/src/completion.rs`.

- **The formatter** lays the member out exactly one way, and a formatted file formats to itself.
- **`nvs meta --json`** emits a class-scoped alias inside its owner's entry, under a `types` array, each
  card the same shape `user_alias_json` already writes (`meta.rs:803`) with the name spelled
  `Owner::Name` fully qualified.
- **The LSP**: go-to-definition on `Owner::Name` in type position lands on the member
  (`definition.rs:283`), document symbols nest it under its owner (`symbols.rs:102`), semantic tokens
  colour it as a type (`semantic.rs:677`), and completion after `Owner::` in type position offers it
  (`completion.rs:493`).
- **Proof**: a named test in each of `nvs-fmt`, `nvs-cli` and `nvs-lsp`.

## Stage 5 — the rule and the record

One file set: `docs/rules/types.json`, `docs/rules/types/`, `docs/decisions/`.

- **One new record and no other number**, claiming the next free number when it lands. Its `changes:`
  block creates `types/class-scoped-alias` and modifies `types/type-alias`. The record argues the
  three calls in *Standing decisions* — no visibility, lexical scope, no third spelling — and states
  what the feature spends: nothing per request, one table entry per declaration at compile time.
- **The fragments**: `types/class-scoped-alias` is the rule, opening on a sentence that stands alone
  in `ground-rules.md`; `types/type-alias`'s first paragraph names both declaration sites. Both list
  the stage 2–3 conformance cases in `guardedBy`, and `python tools/rules.py --render` rewrites the
  generated chapter in the same commit.
- **Proof**: `python tools/rules.py --check` is green, and the conformance cases the fragments name
  exist and pass.

## Standing decisions

- **No visibility, ever.** A class-scoped alias takes no modifier and has no access rule: it is
  reachable wherever its owner's name is. Deciding otherwise means inventing a leak rule for a name
  that does not exist at runtime. If an implementation detail seems to want one, the answer is still
  no, and the obstacle goes in the handoff.
- **Lexical scope, no inheritance.** An alias is reached as `Owner::Name` from anywhere and as a bare
  `Name` inside the owner's own body. It is not reached through a subclass, an implementor,
  `self::`, `static::` or `parent::`. `self::Name` and `static::Name` in type position are not
  spellings of it and are refused as they are today; a bare `Name` inside the body is the short form.
- **Class, interface and enum bodies alike.** One rule for every body that has a class-shaped name is
  simpler than one rule plus two refusals. Nothing about the member differs between the three.
- **The file-scope form is untouched.** Its rule, its tests and its `nvs meta` emission do not
  change; the new record modifies `types/type-alias` only to name the second site.
- **Resolution order at `Owner::Name` in type position is alias, then enum case, then constant.**
  A collision is refused at the declaration (stage 3), so a program that compiles never depends on the
  order; the order exists so the diagnostic for an unknown name can say which of the three it looked
  for.
- **Transparency is not negotiable.** `Order::Meta` and `{total: decimal, note?: string}` are one type
  in both directions. Nothing downstream of the checker — codegen, the value layout,
  `Core\Reflect::typeOf`, an isolate crossing — sees the name, exactly as `rule:types/type-alias`
  already states for the file-scope form.
- **No known gap is left behind.** Every item above lands with its test in this goal; a bound that
  turns out to be needed is written as a sentence in the rule fragment and pinned by a `reject` case,
  never recorded as a gap for a later goal. The stage 0 `lower_member_type` gap is closed here, not
  carried.
- **Not this goal**: parametric aliases (`type Rows<T> = …`), a newtype, and any change to
  `rule:types/alias-is-never-a-bare-class`.
