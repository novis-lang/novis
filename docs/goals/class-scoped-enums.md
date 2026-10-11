---
looper: 1
title: An enum is declared in the class that uses it
summary: "An `enum` becomes a member of a class or interface body, the way a `type` alias already is (`rule:types/class-scoped-alias`)."
context: [crates/nvs-syntax/src/parser/decl.rs, crates/nvs-syntax/src/parser/ty.rs, crates/nvs-syntax/src/ast.rs, crates/nvs-hir/src/aliases.rs, crates/nvs-hir/src/resolve.rs, crates/nvs-types/src/enums.rs, crates/nvs-types/src/lower.rs, crates/nvs-types/src/expr/members.rs]
stages:
  - id: 2
    title: the fragments
    status: done
    extra:
      context:
        rules: [types/grammar, types/enum-case-type, enums/reflection, ide/the-stub-tree-is-where-core-is-declared, statements/nothing-gets-a-second-name, classes/a-type-is-declared-at-file-scope]
        shapes: [A rule fragment]
  - id: 3
    title: the parser
    status: done
    extra:
      context:
        playbook: ["Writing Novis > a new exprkind variant", "Tooling > a node added to nvs_syntax::walk"]
        rules: [enums/class-scoped-enum, types/grammar]
        shapes: [A diagnostic]
  - id: 4
    title: names and the checker
    status: done
    extra:
      context:
        rules: [enums/class-scoped-enum, types/enum-case-type, statements/nothing-gets-a-second-name]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 5
    title: "lowering, the runtime name, reflection and `nvs meta`"
    status: done
    extra:
      context:
        playbook: ["Tooling > nvs-ir cannot name"]
        rules: [enums/class-scoped-enum, enums/reflection, enums/representation, routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name]
        shapes: ["A `.nvst` test case"]
  - id: 6
    title: the editor and the formatter
    status: done
    extra:
      context:
        playbook: ["Writing a test case > a go-to-definition", "Tooling > a node added to nvs_syntax::walk"]
        rules: [enums/class-scoped-enum, tooling/fmt-novis-constructs, tooling/fmt-never-reorders-members]
        shapes: []
  - id: 7
    title: "every `Core` enum moves to the class that uses it"
    status: done
    extra:
      context:
        playbook: ["Writing Novis > the enum a core member returns", "Writing Novis > widening a member's declared type"]
        rules: [enums/an-enum-lives-in-the-class-that-uses-it, core-api/no-mode-strings, core-api/reference-card]
        shapes: ["A `.nvst` test case"]
  - id: 8
    title: "the built-in components' enums move too"
    status: done
    extra:
      context:
        rules: [enums/an-enum-lives-in-the-class-that-uses-it]
        shapes: []
  - id: 9
    title: "the class body is where a program's enum goes"
    status: done
    extra:
      context:
        rules: [enums/an-enum-lives-in-the-class-that-uses-it]
        shapes: []
  - id: 10
    title: the feature proofs
    status: done
    extra:
      context:
        rules: [testing/feature-proofs, testing/one-slice-is-one-feature]
        shapes: []
checks:
  - id: class-scoped-enums-the-record-creates-its-fragments
    stage: 2
    name: "rulebook: Stage 2 created the member's rule and the placement rule"
    run: [bun, nv, peek, "rule:enums/class-scoped-enum", "rule:enums/an-enum-lives-in-the-class-that-uses-it"]
    output_ordered: [docs/rules/enums/class-scoped-enum.md, docs/rules/enums/an-enum-lives-in-the-class-that-uses-it.md]
  - id: nvs-syntax-an-enum-is-a-class-member
    stage: 3
    name: "nvs-syntax (an enum parses in a class and an interface body, a type and a value reach a case through two `::`, and a class in a class body or an enum in an enum body is one diagnostic)"
    run: [cargo, test, --lib, "parser::"]
    tests: [an_enum_is_a_member_of_a_class_body, an_enum_is_a_member_of_an_interface_body, a_modifier_in_front_of_a_member_enum_is_refused, a_class_or_interface_in_a_class_body_is_one_diagnostic, an_enum_in_an_enum_body_is_one_diagnostic, a_type_reaches_a_case_of_a_class_scoped_enum, an_expression_chains_two_double_colons_to_a_case, a_member_enum_is_walked_with_its_cases]
  - id: class-scoped-enums-the-checker-resolves-both-spellings
    stage: 4
    name: "an owned enum is reached from outside and inside its class, types every binding site, and is refused where the rule refuses it"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/enum/a-class-scoped-enum-is-reached-from-outside-and-inside-its-class.nvst, --case, tests/conformance/enum/a-class-scoped-enum-types-a-property-a-parameter-and-a-match.nvst, --case, tests/conformance/enum/a-class-scoped-enum-case-is-a-narrowed-type.nvst, --case, tests/conformance/enum/an-interface-owns-an-enum-too.nvst, --case, tests/conformance/enum/a-class-scoped-enum-and-a-class-of-the-same-path-are-two-types.nvst, --case, tests/conformance/reject/a-class-scoped-enum-takes-no-modifier.nvst, --case, tests/conformance/reject/a-class-scoped-enum-is-not-inherited.nvst, --case, tests/conformance/reject/a-class-scoped-enum-does-not-share-a-name-with-a-constant-or-an-alias.nvst, --case, tests/conformance/reject/a-class-scoped-enum-is-not-imported-by-use.nvst, --case, tests/conformance/reject/an-enum-is-not-declared-in-an-enum-body.nvst, --case, tests/conformance/reject/a-class-is-not-declared-in-a-class-body.nvst]
    tests: [tests/conformance/enum/a-class-scoped-enum-is-reached-from-outside-and-inside-its-class.nvst, tests/conformance/enum/a-class-scoped-enum-types-a-property-a-parameter-and-a-match.nvst, tests/conformance/enum/a-class-scoped-enum-case-is-a-narrowed-type.nvst, tests/conformance/enum/an-interface-owns-an-enum-too.nvst, tests/conformance/enum/a-class-scoped-enum-and-a-class-of-the-same-path-are-two-types.nvst, tests/conformance/reject/a-class-scoped-enum-takes-no-modifier.nvst, tests/conformance/reject/a-class-scoped-enum-is-not-inherited.nvst, tests/conformance/reject/a-class-scoped-enum-does-not-share-a-name-with-a-constant-or-an-alias.nvst, tests/conformance/reject/a-class-scoped-enum-is-not-imported-by-use.nvst, tests/conformance/reject/an-enum-is-not-declared-in-an-enum-body.nvst, tests/conformance/reject/a-class-is-not-declared-in-a-class-body.nvst]
  - id: class-scoped-enums-the-name-a-program-sees
    stage: 5
    name: "an owned enum reports its owner in its name, casts to and from its integer, and types a route capture"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/enum/a-class-scoped-enum-names-itself-with-its-owner.nvst, --case, tests/conformance/enum/a-class-scoped-enum-casts-from-an-integer-and-back.nvst, --case, tests/conformance/core/router-a-capture-takes-a-class-scoped-enum.nvst]
    tests: [tests/conformance/enum/a-class-scoped-enum-names-itself-with-its-owner.nvst, tests/conformance/enum/a-class-scoped-enum-casts-from-an-integer-and-back.nvst, tests/conformance/core/router-a-capture-takes-a-class-scoped-enum.nvst]
  - id: nvs-cli-meta-lists-an-owned-enum-under-its-owner
    stage: 5
    name: "nvs-cli (`nvs meta --json` lists an owned enum under its owner, with its cases)"
    run: [cargo, test, --test, meta]
    tests: [a_class_scoped_enum_is_listed_under_its_owner]
  - id: class-scoped-enums-the-editor-knows-an-owned-enum
    stage: 6
    name: "the language server outlines, completes, defines and hovers an owned enum from both spellings"
    run: [bun, nv, suite, tests/lsp/, --case, tests/lsp/symbols/a-class-scoped-enum-nests-under-its-class.lspt, --case, tests/lsp/completion/after-a-class-name-its-enum-is-offered.lspt, --case, tests/lsp/completion/after-a-class-scoped-enum-its-cases-are-offered.lspt, --case, tests/lsp/completion/inside-its-owner-a-class-scoped-enum-completes-bare.lspt, --case, tests/lsp/definition/a-class-scoped-enum-case-jumps-to-its-declaration.lspt, --case, tests/lsp/definition/a-bare-class-scoped-enum-jumps-to-its-declaration.lspt, --case, tests/lsp/hover/a-class-scoped-enum-shows-its-owner.lspt]
    tests: [tests/lsp/symbols/a-class-scoped-enum-nests-under-its-class.lspt, tests/lsp/completion/after-a-class-name-its-enum-is-offered.lspt, tests/lsp/completion/after-a-class-scoped-enum-its-cases-are-offered.lspt, tests/lsp/completion/inside-its-owner-a-class-scoped-enum-completes-bare.lspt, tests/lsp/definition/a-class-scoped-enum-case-jumps-to-its-declaration.lspt, tests/lsp/definition/a-bare-class-scoped-enum-jumps-to-its-declaration.lspt, tests/lsp/hover/a-class-scoped-enum-shows-its-owner.lspt]
  - id: nvs-fmt-an-owned-enum-is-laid-out
    stage: 6
    name: nvs-fmt (an owned enum is laid out as a file-scope enum and never moved)
    run: [cargo, test, --test, novis_constructs]
    tests: [a_class_scoped_enum_is_laid_out_like_a_file_scope_one]
  - id: nvs-stdlib-every-core-enum-lives-with-its-class
    stage: 7
    name: "nvs-stdlib (every `Core` enum one class uses is owned by it, every owned enum is used by its owner, and no owned enum repeats its owner's word)"
    run: [cargo, test, --test, enum_owners]
    tests: [every_core_enum_one_class_uses_is_owned_by_it, every_owned_core_enum_is_used_by_its_owner, no_owned_core_enum_repeats_its_owners_word]
  - id: nvs-lsp-a-core-enum-is-written-inside-its-class-stub
    stage: 7
    name: "nvs-lsp (the stub tree writes an owned `Core` enum inside its class's stub, and no file of its own)"
    run: [cargo, test, --lib, "stubs::"]
    tests: [an_owned_core_enum_is_written_inside_its_class_stub]
  - id: class-scoped-enums-a-core-enum-is-reached-through-its-class
    stage: 7
    name: "a `Core` enum is reached through its class, and its old name does not resolve"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/a-core-enum-is-reached-through-its-class.nvst, --case, tests/conformance/reject/a-core-enum-is-not-reached-by-its-old-name.nvst]
    tests: [tests/conformance/core/a-core-enum-is-reached-through-its-class.nvst, tests/conformance/reject/a-core-enum-is-not-reached-by-its-old-name.nvst]
  - id: nvs-ext-a-component-enum-lives-with-its-class
    stage: 8
    name: "nvs-ext (a manifest enum names its owner, one class's enum is owned by it, and the owner is a class the manifest declares)"
    run: [cargo, test, --test, manifest]
    tests: [a_manifest_enum_is_keyed_under_its_owner, every_component_enum_one_class_uses_is_owned_by_it, an_enum_owner_the_manifest_does_not_declare_is_refused]
  - id: class-scoped-enums-a-component-enum-is-reached-through-its-class
    stage: 8
    name: "a built-in component's enum is reached through its class"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/ext/a-component-enum-is-reached-through-its-class.nvst]
    tests: [tests/conformance/ext/a-component-enum-is-reached-through-its-class.nvst]
  - id: dossier-the-enum-examples-run
    stage: 9
    name: "proofs: every `lang:enums` feature owes nothing, and its examples and attacks pass after the move to class bodies"
    run: [bun, nv, proofs, --verify, --only, "lang:enums/a-case-is-its-integer", "lang:enums/a-union-of-cases-is-a-narrower-type", "lang:enums/an-enum-as-a-type", "lang:enums/comparing-cases", "lang:enums/core-enums", "lang:enums/declaring-an-enum", "lang:enums/from-an-integer-back-to-a-case", "lang:enums/match-and-switch-over-an-enum", "lang:enums/what-an-enum-does-not-have"]
    output_ordered: [nothing owed in 9 named feature(s), "proofs examples:", 0 failed, "proofs hostile:", 0 failed]
  - id: dossier-the-enum-feature-owes-nothing
    stage: 10
    name: "proofs: `lang:enums/an-enum-inside-a-class` owes nothing, and its examples and attack pass"
    run: [bun, nv, proofs, --verify, --only, "lang:enums/an-enum-inside-a-class"]
    output_ordered: [nothing owed in 1 named feature(s), "proofs examples:", 0 failed, "proofs hostile:", 0 failed]
  - id: dossier-the-enum-programs-carry-plain-comments
    stage: 10
    name: "proofs: the enum examples, attacks and benches carry comments inside the plain-comment bounds"
    run: [bun, nv, proofs, --comments, docs/examples/lang/enums, website/snippets/syntax/enums]
    output_ordered: [each inside the bounds]
  - id: dossier-the-whole-roster-owes-nothing-with-class-scoped-enums
    stage: 10
    name: "proofs: the whole roster owes nothing with every enum at its new place"
    run: [bun, nv, proofs, --gate]
    output_ordered: [nothing owed in the whole roster]
extra:
  milestone: M4
  files: []
  env:
    valgrind:
      skip: [examples/fatal.nvs, examples/uncaught.nvs, examples/cache-shared-socket.nvs, examples/queue-sqlite.nvs, examples/cache-shared-tls.nvs]
    wsl:
      targetDir: /var/tmp/nvs-target-wsl
  context:
    milestones: []
    plan: [Open now, Blocking]
    playbook: ["Tooling > the end", "Tooling > one call reads", "Tooling > a commit message", "Tooling > another agent"]
    rules: [types/class-scoped-alias, enums/declaration, enums/no-class-machinery, statements/an-enum-name-is-a-type-everywhere]
    shapes: [A commit message]
---

# An enum is declared in the class that uses it

## Why

### The target

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

### Why here

The user asked for it on 2026-10-08 and put it directly after goal `ldap`, the live goal. It goes
in front of goals `pdf` and `spreadsheet` because both add new enums to the surface: landing the
rule first means neither of them writes an enum that this goal would then move. It touches the
front end (`nvs-syntax`, `nvs-hir`, `nvs-types`), the names `nvs-ir` and the runtime print, the
editor and formatter, the `Core` registry, the two built-in components' manifests, and the
user-facing docs. That is a wide file set, so the stages below are cut by file set and most take
one session each.

It carries `position: last` because every goal on the chain is pinned there.

### Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that lands what
makes it untrue, and not before:

- **By Stage 2:** `docs/rules/classes/a-type-is-declared-at-file-scope.md` (a `class`, `interface`
  or `enum` is declared at file scope: it stays true for a body that *runs*, and gains the
  class-body exception); `docs/rules/types/class-scoped-alias.md` § *A name is one thing* (the order of
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
read line by line. Every hit is true as it stands or rewritten.
`docs/novis.md`, `docs/ground-rules.md` and the `docs/rules/*.md` chapters are generated and are
regenerated, never edited.

### Stage 1 — the floor

Whatever the chain carries when this goal is reached. Every check goal `ldap` turned green stays
green, with its `Core\Ldap` enums at their new names after Stage 7.

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
  `Core\Compress::Codec`; `Core\Weekday` → `Core\Time\DateTime::Weekday`; `Core\Crypto\Cipher` →
  `Core\Crypto::Cipher`; `Core\Crypto\KeyFormat` → `Core\Crypto\PublicKey::Format`;
  `Core\Mime\Type` → `Core\Mime::Type`; `Core\Xml\NodeKind` → `Core\Xml\Node::Kind`;
  `Core\Log\Level` → `Core\Log::Level`; `Core\Env\Mode` → `Core\Env::Mode`;
  `Core\Response\Redirect` → `Core\Response::Redirect`; `Core\Response\SameSite` →
  `Core\Response::SameSite`; `Core\Cli\Stream` → `Core\Cli::Stream`; `Core\Cli\ColorDepth` →
  `Core\Cli::ColorDepth`; `Core\Cli\Shell` → `Core\Command::Shell`; `Core\Reflect\TypeKind` →
  `Core\Reflect::TypeKind`; `Core\IO\FileMode` → `Core\IO::FileMode`;
  `Core\Cldr\PluralCategory` → `Core\Cldr::PluralCategory`; `Core\Script\ExitReason` →
  `Core\Script\ExitReport::Reason`; `Core\Db\Tls` → `Core\Db::Tls`; `Core\Db\ErrorKind` → `Core\Db\DbError::Kind`;
  `Core\Db\ColumnType` → `Core\Db\Column::Type`; `Core\Db\Plan\Grade` →
  `Core\Db\Plan\Step::Grade`; `Core\Ldap\Scope` → `Core\Ldap\Connection::Scope`; `Core\Ldap\Tls` →
  `Core\Ldap::Tls`; `Core\Ldap\ErrorKind` → `Core\Ldap\LdapError::Kind`; `Core\Queue\State` →
  `Core\Queue::State`. **They stay:** `Core\RoundMode` (`Math`, `Decimal`), `Core\Charset`
  (`Encoding`, `IO`), `Core\Unit` (`DateTime`, `Date`, `TimeOfDay`), `Core\Crypto\KeyKind` (four
  classes), `Core\Http\Method` (seven classes and `#[Route]`), `Core\Audience`, `Core\Db\Driver`
  (`Db`, `Db\Connection`), `Core\Db\Isolation` (`Db\Connection`, `Db\Transaction`). A session that finds a use site this table missed applies the rule, not
  the table, and corrects the table in the placement rule's fragment.
- **The built-in components' table:** every `Novis\Image` enum but `Format` moves into `Image`
  (`Gravity`, `Fit`, `Filter`, `Axis`, `Blend`, `Align`, `Output`, `HashKind`, `PlaceholderKind`),
  and `QrLevel` → `QrCode::Level`. `Novis\Intl`: `Collator::Strength`, `Collator::CaseFirst`,
  `NumberFormat::Style`, `NumberFormat::CurrencyDisplay`, `NumberFormat::CompactDisplay`,
  `PluralRules::Kind`, `PluralRules::Category`, `DateFormat::Length`, `DateFormat::ZoneStyle`,
  `RelativeTime::Unit`, `RelativeTime::Numeric`, `ListFormat::Type`, `Locale::Service`; `Width`
  stays.
- **A name that does not parse** as a member (a keyword clash) keeps its old word with the owner's
  prefix dropped the other way, and the fragment says which.
- **The tradeoffs**, stated here, and in the fragments and the commit, because AGENTS.md asks.
  Performance: none at run time; a case is still an inlined integer. Memory: none per request; one
  table entry per declaration at compile time. Usability: a class's enums live with it, and
  `OrderStatus` stops carrying its owner in its name; every `Core` enum a program names changes
  spelling once, before the first release. Simplicity: a second place to declare an enum and a three-part value form,
  both resolved before any code runs, and one placement rule a tool enforces.
- **No new limit**, and no compatibility spelling: an old `Core` name is gone, not deprecated.
- **Neutral names only** in every test, example and fragment: `Shop`, `Blog`, `example.com`.

## Stages

### 2. the fragments

**Does:** Writes the fragments § *Standing decisions* creates and rewrites.

One file set: `docs/rules/enums/`, `docs/rules/types/`, `docs/rules/statements/`,
`docs/rules/ide/`, `docs/rules/classes/`.

- **The fragments**, written first: the declaration sites, the two spellings and the three-part
  value form, no modifiers, no inheritance, one name per owner, no import of a member, the name an
  owned enum reports, the placement rule and the full rename table for `Core` and the built-in
  components (§ *Standing decisions*), each with its reasons, a `**Memory:**` bullet where it
  spends memory, and a `**Declined:**` list, and the tradeoffs. It creates
  `enums/class-scoped-enum` (the member itself) and `enums/an-enum-lives-in-the-class-that-uses-it`
  (the placement rule, binding `Core`, the built-in components and every published example). It
  rewrites `rule:types/class-scoped-alias` (the reading order of `Owner::Name`),
  `rule:classes/a-type-is-declared-at-file-scope` (the class-body exception) and the other
  fragments Stage 0 names.
- **Pinned by** the Stage 2 check.

### 3. the parser

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

### 4. names and the checker

**Does:** Gives an owned enum its own key, resolves both spellings in type and value position, and
reports the refusals.

One file set: `crates/nvs-hir/src/resolve.rs`, `aliases.rs`, `members.rs`, `hierarchy.rs`,
`autoload.rs`, `requires.rs`, `crates/nvs-types/src/enums.rs`, `lower.rs`, `expr/members.rs`,
`check.rs`, `deprecated.rs`, `signatures.rs`, `locals.rs`.

- **The key.** `EnumTable` (`crates/nvs-types/src/enums.rs:98`) is keyed by `QName`, and `collect`
  (`:233`) walks file scope only. An owned enum is keyed by its owner and its name, never by a
  synthetic `Ns\Order\Status`, for the reason `AliasKey::Member` gives at `aliases.rs:58-70`:
  `Ns\Order\Status` is also the name of a class `Status` in namespace `Ns\Order`. The fragment
  `enums/class-scoped-enum` says how the key is spelled; the name a program sees is § *Standing decisions*'.
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

### 5. lowering, the runtime name, reflection and `nvs meta`

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

### 6. the editor and the formatter

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

### 7. every `Core` enum moves to the class that uses it

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

### 8. the built-in components' enums move too

**Does:** Applies the same rule to the enums `extensions/image/manifest.json:80-90` and
`extensions/intl/manifest.json:137-150` declare.

One file set: `crates/nvs-ext/src/manifest.rs`, `crates/nvs-types/src/ext_lib.rs`,
`extensions/image/`, `extensions/intl/`, their docs and tests.

- **The manifest** gains an owner per enum (`crates/nvs-ext/src/manifest.rs:228`), and
  `declared_name` (`crates/nvs-types/src/ext_lib.rs:130-134`) keys it under its owner.
- **The renames**, per the table in § *Standing decisions*. `Novis\Image\Format` and
  `Novis\Intl\Width` are shared and stay.
- **Pinned by** the Stage 8 checks.

### 9. the class body is where a program's enum goes

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

### 10. the feature proofs

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

## Handoff

### State

**Goal `class-scoped-enums`: an enum is declared in the class that uses it, `Order::Status::Paid`, for programs and for every `Core` and built-in enum. Stages 2 to 10 are done, and the goal is claimed reached.**

- `lang:enums/an-enum-inside-a-class` is on the roster from `docs/reference/lang/55-enums.md`'s `# An enum inside a class`, and `bun nv proofs --id` prints `complete.`: `about.md`, three examples, the attack, the bench with its growth, three `.nvst` cases and `crates/nvs-cli/tests/meta.rs::a_class_scoped_enum_is_listed_under_its_owner`, each with a `// covers:` line.
- The attack `tests/hostile/lang/enums/an-enum-inside-a-class/01-a-class-and-an-enum-with-the-same-name.nvs` declares the class as `Shop\Status`, not `Shop\Order\Status`: a file holds one `namespace` (`rule:statements/one-namespace-statement-per-file`), so the exact same-path pair lives in the two-file conformance case `tests/conformance/enum/a-class-scoped-enum-and-a-class-of-the-same-path-are-two-types.nvst`. Step 4 writes `Order\Status::Paid`, the path of that class.
- The renames of Stages 7 and 8 staled the perf of every member of a touched registry file. All of it is re-measured in `docs/perf/members.ndjson`, and `bun nv proofs --gate` owes nothing over 1391 features.
- `pdf` and `spreadsheet` carry `enums/an-enum-lives-in-the-class-that-uses-it` in the `context.rules` of the stage before their Stage 3.
- Left uncommitted, because this goal did not write them knowingly: `benches/members/core/IO-Metadata/{size,modifiedAt}.nvs` (iterations cut to 1024, `callgrind` added, during the perf re-record) and `benches/compare.json` and `website/src/{data/compare.json,lib/compare.ts}`, which another session's `bench-compare --record` is writing.

Both rules stay `designed`. `bun nv links` still reports `AGENTS.md:403 -> docs/decisions/README.md`, which this goal did not cause.

### Next

**None in this goal.** The acceptance sweep judges the Stage 10 checks: `dossier-the-enum-feature-owes-nothing`, `dossier-the-enum-programs-carry-plain-comments` and `dossier-the-whole-roster-owes-nothing-with-class-scoped-enums`. All three were green at the end of this session. The next goal on the chain is `nvs-code`.

### Backlog

- A bare `Status::` in **type** position inside the owner does not complete the cases. Value position works through `owned_named` (`crates/nvs-lsp/src/completion.rs`).
- `nvsx.toml`'s `[[enums]]` table has no `owner` key yet (`crates/nvs-cli/src/ext/nvsx_toml.rs:216`), so a third-party extension cannot own an enum. Owner: `rule:enums/an-enum-lives-in-the-class-that-uses-it`.
- 32 component enums are uncovered by a Syntax page (`bun nv site --stale`), and `website/src/data/core.json` still names the old component enums. Owner: the website's reference area.
- The two `IO-Metadata` bench edits above need an owner's decision: commit them with a reason, or restore them.
