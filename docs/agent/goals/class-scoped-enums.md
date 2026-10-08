---
milestone: M4
position: last
---
# Loop goal — An enum is declared in the class that uses it: `Order::Status::Paid`, for programs and for every `Core` and built-in enum

An `enum` becomes a member of a class or interface body, the way a `type` alias already is
(`rule:types/class-scoped-alias`). It is reached as `Order::Status` and `Order::Status::Paid` from
anywhere, and as a bare `Status` and `Status::Paid` inside its owner. Then the whole published surface
follows one placement rule: **an enum one class uses is declared in that class, and an enum several
classes share is declared in their namespace.** Every `Core` enum, every built-in component's enum,
every example, every reference chapter and every website page is moved to that rule, because there
is no release yet and the user wants one consistent language, not a feature added on top of a
settled surface.

```nvs
namespace Shop;

final class Order {
    enum Status { Open, Paid, Cancelled }

    public Status $status = Status::Open;            // bare, inside the owner

    public function pay(): void { $this->status = Status::Paid; }
}

function label(Order::Status $s): string {          // qualified, anywhere
    return match ($s) {
        Order::Status::Open => "open",
        Order::Status::Paid => "paid",
        Order::Status::Cancelled => "cancelled",
    };
}

Core\Arr::sort($rows, {order: Core\Arr::Order::Desc});   // a `Core` enum, in its class
```

## Why here

The user asked for it on 2026-10-08 and put it directly after goal `ldap`, the live goal. It goes
in front of goals `pdf` and `spreadsheet` because both add new enums to the surface: landing the
rule first means neither of them writes an enum that this goal would then move. It touches the
front end (`nvs-syntax`, `nvs-hir`, `nvs-types`), the names `nvs-ir` and the runtime print, the
editor and formatter, the `Core` registry, the two built-in components' manifests, and the
user-facing docs. That is a wide file set, so the stages below are cut by file set and most take
one session each.

It carries `position: last` because every goal on the chain is pinned there.

## Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that lands what
makes it untrue, and not before:

- **By Stage 2's record:** `docs/adr/README.md:342` ("A `class`, `interface` or `enum` is declared
  at file scope, or not at all": it stays true for a body that *runs*, and gains the class-body
  exception); `docs/rules/types/class-scoped-alias.md` § *A name is one thing* (the order of
  readings gains the enum, and `E0304` covers an enum member);
  `docs/rules/statements/an-enum-name-is-a-type-everywhere.md` (the qualified form);
  `docs/rules/enums/reflection.md` (the name an owned enum reports);
  `docs/rules/ide/the-stub-tree-is-where-core-is-declared.md` (a `Core` enum is written inside its
  class's stub).
- **By Stage 3:** the `E0233` help text at `crates/nvs-types/src/locals.rs:1773` ("move to file
  scope") names the class body as the second place an enum may go.
- **By Stage 9:** `docs/reference/lang/50-classes.md:1307` (*Nested classes*: an enum is now the
  exception), `docs/examples/lang/classes/what-a-class-cannot-declare/about.md`,
  `docs/reference/lang/20-types.md:378-384` (class-scoped aliases gain a sibling), and every
  chapter section of `docs/reference/lang/55-enums.md`.

The search that closes the stage, run after Stage 10:
`git grep -n -E "file scope|nested (class|enum|type)" -- docs/rules docs/reference docs/examples website/src`,
read line by line. Every hit is true as it stands, rewritten, or under `docs/decisions/`.
`docs/novis.md`, `docs/ground-rules.md` and the `docs/rules/*.md` chapters are generated and are
regenerated, never edited.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. Every check goal `ldap` turned green stays
green, with its `Core\Ldap` enums at their new names after Stage 7.

## Stage 2 — the record and its fragments

**Does:** Writes the record from § *Standing decisions* and the fragments it creates and rewrites.

One file set: `docs/decisions/`, `docs/rules/enums/`, `docs/rules/types/`,
`docs/rules/statements/`, `docs/rules/ide/`, `docs/adr/README.md`.

- **The record**, this goal's one slot, written first: the declaration sites, the two spellings and
  the three-part value form, no modifiers, no inheritance, one name per owner, no import of a
  member, the name an owned enum reports, the placement rule and the full rename table for `Core`
  and the built-in components (§ *Standing decisions*), and the tradeoffs. It amends ADR 0190 (the
  reading order of `Owner::Name`) and the `docs/adr/README.md:342` paragraph.
- **The fragments.** It creates `enums/class-scoped-enum` (the member itself) and
  `enums/an-enum-lives-in-the-class-that-uses-it` (the placement rule, binding `Core`, the built-in
  components and every published example), and rewrites the five fragments Stage 0 names.
- **Pinned by** the Stage 2 check.

## Stage 3 — the parser

**Does:** Parses an enum in a class or interface body, the two-`::` type and value forms, and refuses
the declarations that stay refused with one diagnostic each.

One file set: `crates/nvs-syntax/src/parser/decl.rs`, `parser/ty.rs`, `parser/expr.rs`, `ast.rs`,
`walk.rs`, `casing.rs`, `crates/nvs-syntax/src/parser/tests/decl.rs`.

- **The member.** A new `ClassMemberKind::Enum(EnumDecl)` (`ast.rs:1767`), parsed in
  `parse_class_member_with_attrs` (`parser/decl.rs:1042`) beside the alias arm at `:1079`, reusing
  `finish_enum_decl` (`:1510`). A modifier run in front of it is `E0133`, as for an alias
  (`refuse_decoration_on_a_type_alias`, `:768`, generalised). An attribute group is allowed, as on a
  file-scope enum. Today the same input gives a ten-diagnostic cascade led by `E0101`.
- **Not in an enum body.** `parse_enum_body` (`:1587`) reaches the same member parser; an enum there
  is one `E0220` naming the class body as where it may go (the check at `:1656-1658`).
- **A class or interface in a class body** is one `E0233`, not a cascade, with a help that names file
  scope.
- **Type position.** `parse_type_atom`'s `::` arm (`parser/ty.rs:561`) takes exactly one segment
  today and gives `TypeAtom::Member(Name, Span)` (`ast.rs:203`). It takes a second segment, so
  `Order::Status::Paid` is a type (`rule:types/enum-case-type`).
- **Value position.** `A::B::C` already parses as a nested `ClassConstAccess`
  (`parser/expr.rs:1094`, `:1137-1150`); nothing changes here but a parser test pinning the shape.
- **The walkers.** `walk.rs:935` `member()` emits an `EnumDecl` node with its cases as children,
  matching the statement arm at `walk.rs:520`, so `nvs fmt` and the index see it. `casing.rs:493`
  checks the member's name and cases (`check_enum_decl`, `:382`).
- **Pinned by** the Stage 3 check.

## Stage 4 — names and the checker

**Does:** Gives an owned enum its own key, resolves both spellings in type and value position, and
reports the refusals.

One file set: `crates/nvs-hir/src/resolve.rs`, `aliases.rs`, `members.rs`, `hierarchy.rs`,
`autoload.rs`, `requires.rs`, `crates/nvs-types/src/enums.rs`, `lower.rs`, `expr/members.rs`,
`check.rs`, `deprecated.rs`, `signatures.rs`, `locals.rs`.

- **The key.** `EnumTable` (`crates/nvs-types/src/enums.rs:98`) is keyed by `QName`, and `collect`
  (`:233`) walks file scope only. An owned enum is keyed by its owner and its name, never by a
  synthetic `Ns\Order\Status`, for the reason `AliasKey::Member` gives at `aliases.rs:58-70`:
  `Ns\Order\Status` is also the name of a class `Status` in namespace `Ns\Order`. The record says
  how the key is spelled; the name a program sees is § *Standing decisions*'.
- **The symbol table** (`crates/nvs-hir/src/resolve.rs:96`, `:159-172`) does not get a file-scope
  entry for an owned enum. `check_body_aliases` (`:279`) is the model.
- **Type position.** `lower_member_type` (`crates/nvs-types/src/lower.rs:406`) reads an owned enum
  after the owner's alias (`:429`) and before a case (`:434`); the two-segment atom from Stage 3
  reads the case of an owned enum. The bare name inside the owner goes in `resolve_name_type`
  (`:891`) beside the owner-alias lookup at `:924`.
- **Value position.** `infer_class_const` (`crates/nvs-types/src/expr/members.rs:77`) and
  `is_written_class_side` (`:871`) accept the nested `ClassConstAccess` whose inner access names an
  owned enum, and nothing else nested; any other chain keeps `E_DYNAMIC_CLASS_NAME`. The bare
  `Status::Paid` inside the owner resolves in `check_member_ref` (`crates/nvs-hir/src/members.rs:1395`,
  the `resolve_ref` call at `:1427`) through the owner first.
- **The refusals.** `E0304` for an enum sharing a name with a constant or an alias of its owner
  (`check_name_collisions`, `aliases.rs:302`); `E0405` naming the declaring class for `Sub::Status`
  (`report_missing_member`, `lower.rs:607`, and `alias_owned_by_an_ancestor`, `:635`); `use
  Shop\Order::Status;` refused where it is written.
- **Every walk that reads only file-scope enums** descends into class and interface members too:
  `crates/nvs-hir/src/autoload.rs:1344`, `requires.rs:1137`, `members.rs:491`,
  `crates/nvs-types/src/check.rs:438`, `deprecated.rs:456`, `signatures.rs:1015`.
- **Pinned by** the Stage 4 check.

## Stage 5 — lowering, the runtime name, reflection and `nvs meta`

**Does:** Carries an owned enum through to the program: its cases are integers, its name is the one
a program sees, and every place that looks an enum up by name finds it.

One file set: `crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-codegen/src/lib.rs`, `crates/nvs-runtime/src/object.rs`,
`crates/nvs-stdlib/src/reflect.rs`, `crates/nvs-types/src/routes.rs`, `crates/nvs-cli/src/meta.rs`.

- **The case is its integer** with no change: `ExprInfo::EnumCase` lowers at
  `crates/nvs-ir/src/lower/expr.rs:408`.
- **The name.** `enum_shapes` writes `label: qname.to_string()` (`crates/nvs-ir/src/lower/mod.rs:1064`),
  codegen defines it (`crates/nvs-codegen/src/lib.rs:1346`) and the runtime finds it by string
  (`crates/nvs-runtime/src/object.rs:2620`). The label, the route capture's
  `EnumCapture.class` (`crates/nvs-types/src/routes.rs:1891`) and `Core\Reflect\EnumInfo::name`
  (`crates/nvs-stdlib/src/reflect.rs:2618`) all give `Shop\Order::Status`.
- **`nvs meta --json`** lists an owned enum under its owner, as `alias_cards` does for an alias
  (`crates/nvs-cli/src/meta.rs:1091`): `collect` (`:750`) and `body_json`'s `_ => {}` arm (`:886`).
- **Pinned by** the Stage 5 check.

## Stage 6 — the editor and the formatter

**Does:** Makes an owned enum a first-class citizen of the language server and `nvs fmt`.

One file set: `crates/nvs-lsp/src/symbols.rs`, `completion.rs`, `definition.rs`, `hover.rs`,
`semantic.rs`, `index.rs`, `folding.rs`, `hints.rs`, `crates/nvs-fmt/`, `tests/lsp/`,
`editors/vscode/syntaxes/nvs.tmLanguage.json`.

- **Outline** nests it under its class (`symbols.rs:198`, the alias arm at `:215`).
- **Completion** offers it after `Order::` in type and value position (`completion.rs:2653`,
  `:2730`), and its cases after `Order::Status::`; a bare `Status` inside the owner completes too.
- **Go to definition, hover and references** reach it from both spellings: `definition.rs:479`
  `declared_type` looks up `module.symbols`, which Stage 4 left without an entry, so it walks the
  owner; `type_member_at` (`:979`) chooses between alias, enum and constant; `type_name_at`
  (`:1130`) reads the owner's members for a bare name.
- **Semantic tokens, folding, hints and the index** take the member arm: `semantic.rs:697`, `:788`,
  `index.rs:1029`, `folding.rs:250`, `hints.rs:216`.
- **`nvs fmt`** lays it out exactly as a file-scope enum (`brace.rs:48`, `indent.rs:86`,
  `list.rs:91`) once Stage 3's walker node exists, and never moves it (`rule:tooling/fmt-never-reorders-members`).
- **The grammar** (`editors/vscode/syntaxes/nvs.tmLanguage.json`) colours an `enum` inside a class
  body as it does at file scope.
- **Pinned by** the Stage 6 check.

## Stage 7 — every `Core` enum moves to the class that uses it

**Does:** Applies the placement rule to the 37 enums in `crates/nvs-stdlib/src/registry.rs:3075`,
with the rename table of § *Standing decisions*, and everything that names them.

One file set per slice, by namespace: the declaring stdlib file (the `CoreEnum` const and its
`*_NAME`), its tests, its `docs/examples/core/` directories, its conformance cases and its
`docs/spec/01-core-library.md` rows. The registry change goes first and alone.

- **The registry.** `CoreEnum` (`registry.rs:3011`) gains its owner, so an owned `Core` enum is
  seeded under the same key Stage 4 gives a program's (`seed_core`,
  `crates/nvs-types/src/enums.rs:193-201`, which today only splits a string on `\`). The LSP stub
  tree writes an owned enum inside its class's stub (`crates/nvs-lsp/src/stubs.rs:128`, `:371`,
  `:569`), and `registry::core_enum` (`registry.rs:3913`) finds both spellings' targets.
- **A test that reads the source of truth**, `crates/nvs-stdlib/tests/enum_owners.rs`: every enum in
  `ENUMS` that exactly one class in `CLASSES` takes or returns is owned by that class; every enum
  owned by a class is used by it; and no owned enum repeats its owner's word in its own name. A
  shared enum stays in its namespace and the test says why for each.
- **Then the renames**, one namespace per slice: `arr`, `str`, `hash`, `compress`, `crypto`,
  `mime`, `xml`, `log`, `env`, `response`, `cli` and `command`, `reflect`, `io`, `cldr`,
  `script`, `time`, `db`, `ldap`, `queue`. Each slice renames the `CoreEnum`, every Rust string
  that quotes it (`git grep` on the old name), every `.nvs`, `.nvst` and `.md` that writes it, and
  moves its feature-proof directory to the new feature id. The proofs roster's `types:enum` group
  follows the registry, so `bun nv proofs --group types:enum` is how a slice sees what it moved.
- **Pinned by** the Stage 7 checks.

## Stage 8 — the built-in components' enums move too

**Does:** Applies the same rule to the enums `extensions/image/manifest.json:80-90` and
`extensions/intl/manifest.json:137-150` declare.

One file set: `crates/nvs-ext/src/manifest.rs`, `crates/nvs-types/src/ext_lib.rs`,
`extensions/image/`, `extensions/intl/`, their docs and tests.

- **The manifest** gains an owner per enum (`crates/nvs-ext/src/manifest.rs:228`), and
  `declared_name` (`crates/nvs-types/src/ext_lib.rs:130-134`) keys it under its owner.
- **The renames**, per the table in § *Standing decisions*. `Novis\Image\Format` and
  `Novis\Intl\Width` are shared and stay.
- **Pinned by** the Stage 8 checks.

## Stage 9 — the class body is where a program's enum goes

**Does:** Makes an owned enum the default everywhere a reader learns the language.

One file set: `docs/reference/lang/55-enums.md`, `50-classes.md`, `20-types.md`,
`docs/examples/lang/`, `website/src/content/docs/syntax/`, `website/snippets/`,
`docs/examples/`, `benches/members/`, `tests/hostile/`, `examples/`, the `nvs new` templates in
`crates/nvs-cli`, `editors/vscode` snippets.

- **Every published program** that declares an enum one class uses declares it in that class: the
  51 files under `docs/examples/`, 18 under `tests/hostile/`, 16 under `benches/members/`, 7 under
  `website/snippets/`, `examples/enums.nvs`, `examples/type-test.nvs`, the 2 in `crates/nvs-cli`
  and the 2 in `editors/vscode` (`git grep -l -E "^enum [A-Z]"`). An enum two classes share stays
  at file scope, and that is said in one line where it happens. A `.out` changes only where it
  prints an enum's name.
- **The reference chapter** `55-enums.md` opens with the class-scoped form; `# Declaring an enum`
  shows it first and the file-scope form as the shared case. `50-classes.md`'s *Nested classes*
  names the enum as the one exception.
- **The website pages** `syntax/values/enums.mdx`, `syntax/classes/declaring.mdx` and the tour
  snippet `website/snippets/guides/tour/06-enums-and-match.nvs`.
- **`tests/conformance/` is not swept.** Its cases pin enum behaviour at file scope and stay as
  they are, beside the new cases of Stages 4 and 5.
- **Pinned by** the Stage 9 checks.

## Stage 10 — the feature proofs

**Does:** Writes the feature proofs for the new language feature, and closes Stage 0.

One file set: `docs/reference/lang/55-enums.md` (its new section), `docs/examples/lang/enums/`,
`benches/members/`, `tests/hostile/`, `data/proofs/policy.json`.

- **The feature** is the section `# An enum inside a class` of `docs/reference/lang/55-enums.md`,
  which makes it `lang:enums/an-enum-inside-a-class` on the roster and its help in the binary
  (`crates/nvs-cli/src/agent.rs:186`, `:436`). It owes `about.md`, tests from Novis and Rust, three
  examples, one bench with its growth, one attack (`rule:testing/feature-proofs`).
- **The examples:** an order with its status, a command whose exit codes are its own enum, and an
  interface whose enum every implementation returns.
- **The attack:** a program declares `Shop\Order\Status` as a class and `Shop\Order::Status` as an
  enum, and tries to pass a case of one where the other is wanted. Both names resolve to their own
  declaration, and every attempt does not compile.
- **The goals behind this one** that write an enum (`pdf`, `spreadsheet`) get
  `enums/an-enum-lives-in-the-class-that-uses-it` in their record's `context.rules`.
- **Pinned by** the Stage 10 checks.

## Standing decisions

- **The user's calls, as instructions.** An enum may be declared in a class or interface body. It
  is reached as `Owner::Name` and `Owner::Name::Case` from anywhere, and as a bare `Name` inside its
  owner. **It is the default for a program's enums**: an enum one class uses is declared in it.
  **`Core` and the built-in components follow the same rule**, with renames, because there is no
  release and the user wants one consistent surface. The docs and the website change with it.
- **The rules of the member**, copied from `rule:types/class-scoped-alias` and settled with the user
  when the goal was written. No visibility modifier (`E0133`): its cases leave the class in every
  return value, so `private` would hide the name and not the values. Not inherited (`E0405` naming
  the declaring class). No `self::Status` or `static::Status` spelling (`E0135`). One name per owner:
  an enum, a constant and an alias of one owner never share a name (`E0304`). Accepted in a class and
  an interface body, never in an enum body. An enum in a method body, a closure or a block stays
  `E0233`. Nested classes and interfaces stay refused.
- **The goal writer's calls, not confirmed by the user, also standing.** **No import of a member**:
  `use Shop\Order::Status;` is refused, and a program imports the owner; a member import would give
  one type a second short name (`rule:statements/nothing-gets-a-second-name`). **The name a program
  sees** — `Core\Reflect\EnumInfo::name`, a diagnostic, `nvs meta`, a route capture's error — is
  `Shop\Order::Status`. **An attribute group** may sit in front of an owned enum, as in front of a
  file-scope one. **A rename drops the owner's word** where the old name repeated it.
- **The placement rule, mechanical:** an enum exactly one class takes, returns or holds is declared
  in that class; an enum several classes use, or none (`Core\Audience`, used only by `#[Access]`),
  stays in its namespace. `crates/nvs-stdlib/tests/enum_owners.rs` enforces it for `Core`, and a
  reviewer's judgement does not override it. If a later member makes an owned enum shared, it moves
  back to the namespace in that slice.
- **The `Core` rename table**, from the registry's own use sites on 2026-10-08:
  `Core\Order` → `Core\Arr::Order`; `Core\SetOn` → `Core\Arr::SetOn`; `Core\NormalForm` →
  `Core\Str::NormalForm`; `Core\Digest` → `Core\Hash::Digest`; `Core\Codec` →
  `Core\Compress::Codec`; `Core\Weekday` → `Core\DateTime::Weekday`; `Core\Crypto\Cipher` →
  `Core\Crypto::Cipher`; `Core\Crypto\KeyFormat` → `Core\Crypto\PublicKey::Format`;
  `Core\Mime\Type` → `Core\Mime::Type`; `Core\Xml\NodeKind` → `Core\Xml\Node::Kind`;
  `Core\Log\Level` → `Core\Log::Level`; `Core\Env\Mode` → `Core\Env::Mode`;
  `Core\Response\Redirect` → `Core\Response::Redirect`; `Core\Response\SameSite` →
  `Core\Response::SameSite`; `Core\Cli\Stream` → `Core\Cli::Stream`; `Core\Cli\ColorDepth` →
  `Core\Cli::ColorDepth`; `Core\Cli\Shell` → `Core\Command::Shell`; `Core\Reflect\TypeKind` →
  `Core\Reflect::TypeKind`; `Core\IO\FileMode` → `Core\IO::FileMode`;
  `Core\Cldr\PluralCategory` → `Core\Cldr::PluralCategory`; `Core\Script\ExitReason` →
  `Core\Script\ExitReport::Reason`; `Core\Db\Tls` → `Core\Db::Tls`; `Core\Db\Isolation` →
  `Core\Db\Connection::Isolation`; `Core\Db\ErrorKind` → `Core\Db\DbError::Kind`;
  `Core\Db\ColumnType` → `Core\Db\Column::Type`; `Core\Db\Plan\Grade` →
  `Core\Db\Plan\Step::Grade`; `Core\Ldap\Scope` → `Core\Ldap\Connection::Scope`; `Core\Ldap\Tls` →
  `Core\Ldap::Tls`; `Core\Ldap\ErrorKind` → `Core\Ldap\LdapError::Kind`; `Core\Queue\State` →
  `Core\Queue::State`. **They stay:** `Core\RoundMode` (`Math`, `Decimal`), `Core\Charset`
  (`Encoding`, `IO`), `Core\Unit` (`DateTime`, `Date`, `TimeOfDay`), `Core\Crypto\KeyKind` (four
  classes), `Core\Http\Method` (seven classes and `#[Route]`), `Core\Audience`, `Core\Db\Driver`
  (`Db`, `Db\Connection`). A session that finds a use site this table missed applies the rule, not
  the table, and corrects the table in the record.
- **The built-in components' table:** every `Novis\Image` enum but `Format` moves into `Image`
  (`Gravity`, `Fit`, `Filter`, `Axis`, `Blend`, `Align`, `Output`, `HashKind`, `PlaceholderKind`),
  and `QrLevel` → `QrCode::Level`. `Novis\Intl`: `Collator::Strength`, `Collator::CaseFirst`,
  `NumberFormat::Style`, `NumberFormat::CurrencyDisplay`, `NumberFormat::CompactDisplay`,
  `PluralRules::Kind`, `PluralRules::Category`, `DateFormat::Length`, `DateFormat::ZoneStyle`,
  `RelativeTime::Unit`, `RelativeTime::Numeric`, `ListFormat::Type`, `Locale::Service`; `Width`
  stays.
- **A name that does not parse** as a member (a keyword clash) keeps its old word with the owner's
  prefix dropped the other way, and the record says which.
- **One record slot**: one new record and no other number, checked against `docs/decisions/` right
  before it is written, because another agent may take a number first. It is Stage 2's first slice.
- **The tradeoffs**, stated here and in the record because AGENTS.md asks. Performance: none at run
  time; a case is still an inlined integer. Memory: none per request; one table entry per
  declaration at compile time. Usability: a class's enums live with it, and `OrderStatus` stops
  carrying its owner in its name; every `Core` enum a program names changes spelling once, before
  the first release. Simplicity: a second place to declare an enum and a three-part value form,
  both resolved before any code runs, and one placement rule a tool enforces.
- **No new limit**, and no compatibility spelling: an old `Core` name is gone, not deprecated.
- **Neutral names only** in every test, example and record: `Shop`, `Blog`, `example.com`.
