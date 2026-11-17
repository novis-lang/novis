---
looper: 1
title: a package says where a class or method may be used
summary: "Reached when a [[restrict]] table in package.toml limits where a Core or user declaration may be used, checked while compiling, only ever narrowing, shown in the editor at the use with its reason and fix, and the feature has its proofs and pages."
context: [crates/nvs-config/src/grants.rs, crates/nvs-config/src/package.rs, crates/nvs-types/src/package.rs, docs/rules/packaging/package-toml-is-a-tool-input.md, docs/rules/packaging/a-package-file-stays-under-its-declared-prefix.md]
stages:
  - id: 1
    title: the table is read from the application's file and each package's, and a broken table is an error where it is written
    status: done
    context: [crates/nvs-cli/src/package/manifest.rs, crates/nvs-config/src/cache.rs, crates/nvs-diagnostics/src/code/config.rs, crates/nvs-diagnostics/src/code/mod.rs, docs/rules/security/grants-are-keyed-on-a-namespace.md, docs/rules/security/longest-prefix-wins.md]
  - id: 2
    title: a use outside the allowed places does not compile
    status: done
    context: [crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/capability.rs, crates/nvs-types/src/signatures.rs, docs/rules/packaging/a-components-engine-class-is-internal.md, docs/rules/packaging/a-restriction-limits-where-a-declaration-is-used.md, tests/conformance/cap/grants-a-namespace-holds-only-what-it-is-granted.nvst, crates/nvs-cli/tests/package_grants.rs]
  - id: 3
    title: the replacement is checked where it is written and becomes the fix, and a reflective act is held to the same table
    status: done
    context: [crates/nvs-types/src/deprecated.rs, crates/nvs-types/src/deprecated/template.rs, crates/nvs-types/src/deprecated/fix.rs, crates/nvs-stdlib/src/reflect.rs, docs/rules/attributes/a-deprecation-names-its-replacement-as-code.md, docs/rules/security/reflection-enforces-visibility.md, docs/rules/packaging/a-restriction-limits-where-a-declaration-is-used.md]
  - id: 4
    title: the editor shows the restriction at the use, on hover and in completion
    status: done
    context: [crates/nvs-lsp/src/diagnostics.rs, crates/nvs-lsp/src/actions.rs, crates/nvs-lsp/src/hover.rs, crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/render.rs, tests/lsp/hover/a-route-hover-in-an-autoloaded-file-lists-its-programs-grants.lspt, docs/reference/tools/40-editor.md]
  - id: 5
    title: nvs pkg api lists a package's restrictions and nvs pkg diff calls a new one a break
    status: open
    context: [crates/nvs-cli/src/package/api.rs, crates/nvs-cli/src/package/diff.rs, crates/nvs-cli/tests/package_api.rs, docs/rules/packaging/only-a-major-may-break-the-api.md]
  - id: 6
    title: the feature's proofs and its pages
    status: open
    context: [docs/reference/tools/35-packages.md, docs/examples/tools/packages/grants-what-each-namespace-may-do/about.md, website/src/content/docs/guides/how-to-use/modules.mdx, docs/agent/prose.md, tests/hostile/tools/packages/grants-what-each-namespace-may-do/01-borrow-the-application-namespace.nvs, benches/members/tools/packages/grants-what-each-namespace-may-do.nvs]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: table-read
    stage: 1
    name: "a [[restrict]] table is read from the application's package.toml and from a package's own, a broken entry is an error at its line, and an edited restriction is a cache miss while a program with none keeps its key"
    run: [cargo, test, --test, restrict, --, a_restrict_table_is_read_from_the_application_file, a_restrict_table_is_read_from_a_package_manifest, a_target_that_is_not_a_declaration_name_is_an_error, a_core_target_that_names_nothing_is_an_error, a_package_allow_outside_its_own_name_is_an_error, an_unknown_restrict_field_is_an_error, an_edited_restriction_changes_the_cache_key, a_program_with_no_restriction_keeps_its_cache_key]
    timeout: 1800
    tests:
      - a_restrict_table_is_read_from_the_application_file
      - a_restrict_table_is_read_from_a_package_manifest
      - a_target_that_is_not_a_declaration_name_is_an_error
      - a_core_target_that_names_nothing_is_an_error
      - a_package_allow_outside_its_own_name_is_an_error
      - an_unknown_restrict_field_is_an_error
      - an_edited_restriction_changes_the_cache_key
      - a_program_with_no_restriction_keeps_its_cache_key
  - id: use-refused
    stage: 2
    name: "a use outside every allowed place is an error naming the restriction, a use inside one compiles, every kind of use is held, an override is held with the method it overrides, the application cannot lift a package's restriction, and a program with none compiles as before"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/cap/restrict-a-call-outside-the-allowed-namespace-does-not-compile.nvst, --case, tests/conformance/cap/restrict-a-call-inside-the-allowed-namespace-compiles.nvst, --case, tests/conformance/cap/restrict-new-a-property-and-a-constant-are-uses.nvst, --case, tests/conformance/cap/restrict-a-type-and-instanceof-are-not-uses.nvst, --case, tests/conformance/cap/restrict-a-method-reference-is-a-use.nvst, --case, tests/conformance/cap/restrict-an-override-is-restricted-with-its-method.nvst, --case, tests/conformance/cap/restrict-a-restriction-on-an-override-is-an-error.nvst, --case, tests/conformance/cap/restrict-an-empty-allow-list-allows-no-place.nvst, --case, tests/conformance/cap/restrict-the-application-cannot-lift-a-package-restriction.nvst, --case, tests/conformance/cap/restrict-a-program-with-no-restriction-compiles-as-before.nvst]
    timeout: 1800
    tests:
      - tests/conformance/cap/restrict-a-call-outside-the-allowed-namespace-does-not-compile.nvst
      - tests/conformance/cap/restrict-a-call-inside-the-allowed-namespace-compiles.nvst
      - tests/conformance/cap/restrict-new-a-property-and-a-constant-are-uses.nvst
      - tests/conformance/cap/restrict-a-type-and-instanceof-are-not-uses.nvst
      - tests/conformance/cap/restrict-a-method-reference-is-a-use.nvst
      - tests/conformance/cap/restrict-an-override-is-restricted-with-its-method.nvst
      - tests/conformance/cap/restrict-a-restriction-on-an-override-is-an-error.nvst
      - tests/conformance/cap/restrict-an-empty-allow-list-allows-no-place.nvst
      - tests/conformance/cap/restrict-the-application-cannot-lift-a-package-restriction.nvst
      - tests/conformance/cap/restrict-a-program-with-no-restriction-compiles-as-before.nvst
  - id: layouts
    stage: 2
    name: "a framework folder with a named manifest restricts every module whose entry loads it, a module file that declares the framework's namespace is still refused, the framework's own tests read the same manifest as their application file, and an unloaded package restricts nothing"
    run: [cargo, test, --test, package_restrict, --, a_framework_folder_with_a_manifest_restricts_every_module_that_loads_it, a_module_file_that_declares_the_framework_namespace_is_still_refused, the_frameworks_own_tests_read_its_manifest_as_the_application_file, a_restriction_of_a_package_the_program_never_loads_restricts_nothing, every_restriction_on_a_declaration_must_hold, the_error_names_the_restricting_file_and_line]
    timeout: 1800
    tests:
      - a_framework_folder_with_a_manifest_restricts_every_module_that_loads_it
      - a_module_file_that_declares_the_framework_namespace_is_still_refused
      - the_frameworks_own_tests_read_its_manifest_as_the_application_file
      - a_restriction_of_a_package_the_program_never_loads_restricts_nothing
      - every_restriction_on_a_declaration_must_hold
      - the_error_names_the_restricting_file_and_line
  - id: replacement-and-reflection
    stage: 3
    name: "a replacement that does not compile, has the wrong type or uses the restricted declaration is an error in package.toml, the fix fills it in at the use, and a reflective act outside the allowed places throws"
    run: [cargo, test, --test, restrict_replace, --, a_replacement_that_does_not_compile_is_an_error_in_package_toml, a_replacement_of_the_wrong_type_is_an_error, a_replacement_that_uses_the_restricted_declaration_is_an_error, the_fix_fills_in_the_replacement_at_the_use]
    timeout: 1800
    tests:
      - a_replacement_that_does_not_compile_is_an_error_in_package_toml
      - a_replacement_of_the_wrong_type_is_an_error
      - a_replacement_that_uses_the_restricted_declaration_is_an_error
      - the_fix_fills_in_the_replacement_at_the_use
  - id: reflective-act
    stage: 3
    name: "a reflective call of a restricted method outside the allowed places throws, and inside them runs"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/cap/restrict-a-reflective-call-outside-the-allowed-places-throws.nvst, --case, tests/conformance/cap/restrict-a-reflective-call-inside-the-allowed-places-runs.nvst]
    timeout: 1800
    tests:
      - tests/conformance/cap/restrict-a-reflective-call-outside-the-allowed-places-throws.nvst
      - tests/conformance/cap/restrict-a-reflective-call-inside-the-allowed-places-runs.nvst
  - id: editor
    stage: 4
    name: "the editor shows a restricted use as an error linked to its package.toml line, offers the replacement as a fix, shows where a method is allowed on hover, sorts a restricted completion last with its reason, and re-checks open files when package.toml changes"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/diagnostics/a-restricted-call-links-to-its-restriction.lspt, --case, tests/lsp/actions/a-restricted-call-is-offered-its-replacement.lspt, --case, tests/lsp/hover/a-restricted-method-shows-where-it-is-allowed.lspt, --case, tests/lsp/completion/a-restricted-method-sorts-last-with-its-reason.lspt, --case, tests/lsp/diagnostics/an-edited-restriction-re-checks-the-open-files.lspt]
    timeout: 1800
    tests:
      - tests/lsp/diagnostics/a-restricted-call-links-to-its-restriction.lspt
      - tests/lsp/actions/a-restricted-call-is-offered-its-replacement.lspt
      - tests/lsp/hover/a-restricted-method-shows-where-it-is-allowed.lspt
      - tests/lsp/completion/a-restricted-method-sorts-last-with-its-reason.lspt
      - tests/lsp/diagnostics/an-edited-restriction-re-checks-the-open-files.lspt
  - id: api-diff
    stage: 5
    name: "nvs pkg api lists a package's restrictions, and nvs pkg diff calls a new restriction or one that allows fewer places a break, and a dropped one additive"
    run: [cargo, test, --test, package_api, --, the_api_document_lists_the_packages_restrictions, a_new_restriction_is_a_break, a_restriction_allowing_fewer_places_is_a_break, a_dropped_restriction_is_additive]
    timeout: 1800
    tests:
      - the_api_document_lists_the_packages_restrictions
      - a_new_restriction_is_a_break
      - a_restriction_allowing_fewer_places_is_a_break
      - a_dropped_restriction_is_additive
  - id: feature-proofs
    stage: 6
    name: "the restrictions feature owes no proof"
    run: [bun, nv, proofs, --id, "tools:packages/restrictions-where-a-class-or-method-may-be-used"]
    timeout: 900
---

# a package says where a class or method may be used

## Why

A framework often builds its own version of a `Core` member and needs every program on it to use that
version. The person's case: a framework signs URLs with its own signer, built on
`Core\Router::urlSigned`, and adds key rotation. A developer who calls `Core\Router::urlSigned`
directly makes a link the framework cannot verify, and nothing stops them. Today Novis has three tools
that come close and none that fits. `[grants]` limits capabilities, not members. `Config::set`
narrows settings at run time. `E0856` makes a class internal, but only a built-in component's engine
class.

This goal adds `[[restrict]]` to `package.toml`. An entry names a declaration and the places that may
use it, and every other use does not compile. It only narrows: no table, file or setting can lift it.
The editor shows the error at the use, with who restricted it, why, where it is allowed and the
replacement as a fix.

Novis can make this a guarantee where other languages only lint. Clippy's `disallowed_methods`,
deptrac and ESLint's `no-restricted-imports` all lose to a dynamic call. Novis has no `eval`, no
string callables, and reflection that is held to the call site's own checks
(`rule:security/no-runtime-grant`, `rule:security/reflection-enforces-visibility`). So a static check
over every resolved use is total, once reflection checks the same table (stage 3).

**The layout it must serve, besides a fetched package (the person's call).** A framework kept in the
same repository as the modules built on it, with no package manager involved:

```txt
modules/
  package.toml              the application's file, for every module
  Framework/
    package.toml            [package] name = 'Framework\Core', and its [[restrict]] lines
    src/Url.nvs             Framework\Core\Url
  Blog/
    public/index.nvs        autoload 'Framework\Core' from '../../Framework/src';
                            autoload 'Framework\{..}' from '../src';
    src/                    Framework\Blog\...
  Shop/  ...
```

`crates/nvs-types/src/package.rs:185` walks up from each loaded file and stops at the entry's
directory, so for an entry in `Blog/public/` the walk from `Framework/src/Url.nvs` finds
`Framework/package.toml` and places the file in the package `Framework\Core`. Each module's own
`src/` is the application's code. Every test and example of this goal that has a monorepo uses these
neutral names, never the names of the person's project.

## Standing decisions

**The table (my call, decided in this conversation; safe default if the build forces otherwise: keep
the shape, drop `replace` to the backlog).**

```toml
[[restrict]]
use     = 'Core\Router::urlSigned'
allow   = ['Framework\Core\Signing\*']
note    = "Use `Framework\Core\Url::signed`. It adds the framework's key rotation."
replace = 'Framework\Core\Url::signed($route, $params)'
```

- `use` names one declaration: a class, interface or enum (`'Ns\Name'`), a method, property or class
  constant (`'Ns\Name::member'`), a function, or a namespace (`'Ns'` for its own declarations,
  `'Ns\*'` for every depth below, as `rule:security/longest-prefix-wins` spells a subtree). It is
  required.
- `allow` lists the places that may use it. An entry matches a use when it names the use's enclosing
  namespace (exact, or a `\*` subtree), its enclosing class, or its enclosing `Class::method`. An
  empty list allows no place, so `allow = []` forbids a declaration everywhere. It is required, so an
  entry is never ambiguous about where it allows.
- `note` is Markdown shown in the error and on hover, and is text an end user reads. Optional.
- `replace` is one Novis expression, with the target's parameters and `$this` as its placeholders,
  exactly as `#[Core\Deprecated]`'s `replace` is (`rule:attributes/a-deprecation-names-its-replacement-as-code`).
  Optional, and only on a method or function target.
- Unknown fields are an error (`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`).
- Declined: a map form like `[grants]` (`'Core\Router::urlSigned' = [...]`). It has no room for
  `note` and `replace`, and the error without its reason is the thing the person asked to avoid.

**Where it is read: the application's `package.toml` and each package's own (my call).** The
application's file is the one `crates/nvs-config/src/grants.rs:160` finds. A package's file is the one
`crates/nvs-types/src/package.rs:185` places its files under, so a hand-placed folder with a named
`package.toml` counts exactly as a fetched package does
(`rule:packaging/a-package-file-stays-under-its-declared-prefix`). `rule:packaging/package-toml-is-a-tool-input`
is rewritten: the compiler reads `[grants]` and `[[restrict]]` of the application's file, and the
name, the capability split and `[[restrict]]` of a package's. Declined, each with its reason in the
new rule: an attribute on the allowed class (it holds only when that class is compiled, and lets any
pasted file claim a `Core` member); `nvs.toml` (it is the operator's and deployment-scoped); a
runtime form through `Config::set` (it costs a branch per call, fails only when the line runs, and
the editor cannot see it).

**Compile time only, and an error (my call).** The check costs nothing per request and holds no
memory per request. A restriction from a package is an error at every use outside its `allow`, never
a warning, because a warning is what the person's framework author is trying to get past. A package
that adds a restriction makes a breaking change, and stage 5 makes `nvs pkg diff` say so
(`rule:packaging/only-a-major-may-break-the-api`).

**What counts as a use.** A call (static, instance, `parent::`), a method reference
(`Name::method`), `new`, a property read or write, a class constant, an enum case, and a
reflective act on the member (stage 3). A type position, an `is` test and `catch` are not uses, so
allowed code can pass a restricted object around through typed signatures.

**Overrides and interfaces (my call).** A restriction on a method holds for every override and
implementation of it, so a call through a subclass or an interface reaches the same check. A `use`
that names a method which itself overrides or implements another one is an error when the table is
read, with the help "restrict `Parent::method` instead". The check is on the declaration the call
resolved to and its ancestors' restricted declarations, through `resolve_method`
(`crates/nvs-types/src/signatures.rs:2456`).

**A package's restriction allows only its own code (my call).** Every `allow` entry of a package's
`[[restrict]]` must lie under the package's name, or the table is an error. And a use matches it only
when the using file is inside that package by layout. So a module file that writes
`namespace Framework\Core\Signing;` is still refused, which closes the override bypass that grants
accept for capabilities. The application's own entries match by namespace alone, since the
application has no package folder.

**Every restriction holds, and nothing lifts one.** A use must pass every restriction on the
declaration, from the application's file and from every loaded package. The application cannot
remove, widen or override a package's entry. A package's entry applies to the application's code and
to every other package's code too.

**When a table entry names nothing.** A `Core` target that names no `Core` declaration is an error,
since `Core` is always known. Any other target that names no declaration of this program is warning
only in the application's own file, and silent in a package's file, where it often names code the
current program does not load.

**When the check runs.** Packages are found as their files load, so the restriction table is complete
only once the program's file set is. The check is a pass after type checking, over each resolved use,
and it looks up a hash set of restricted declarations. With no `[[restrict]]` anywhere the pass is
skipped (`AGENTS.md`'s "a mechanism costs nothing where unused"). If the typed tree does not keep a
resolved target for some use kind, the checker records those uses into a side list while it checks,
only when the program has a restriction table. The session that builds stage 2 picks which, and
records it in the rule.

**The cache key.** `Grants::digest` (`crates/nvs-config/src/grants.rs:274`) folds into every unit's
key at `crates/nvs-config/src/cache.rs:225`. The restriction tables fold in beside it, as one digest
of every table the program read, so an edited line is an ordinary cache miss. A program with no table
keeps exactly its key today.

**The codes.** `E0664` (`E_RESTRICTED_USE`) for a use outside the allowed places, `E0665`
(`E_BAD_RESTRICTION`) for a broken table entry, `E0666` (`E_BAD_RESTRICTION_REPLACE`) for a
replacement that does not compile, in `crates/nvs-diagnostics/src/code/config.rs`, and `W1031`
(`W_RESTRICTION_NAMES_NOTHING`) in `warnings.rs` for a target in the application's file that names
nothing. Each has its card.

**Where the tables live in memory (stage 1's call).** `Grants` holds a `Restrictions`
(`crates/nvs-config/src/restrict.rs`): the application file's entries from `Grants::parse`, then
each loaded package's, which `crates/nvs-cli/src/main.rs` sets with `Grants::set_restrictions` beside
`set_optional` and `set_files`. So the one value every compile already carries reaches the check pass
and the cache key, and no second parameter is threaded through. A `Target` is `Name` (namespace,
class or function: the file cannot tell which), `Subtree` or `Member`. Stage 2 resolves a `Name`
against the program. `Restrictions::check_core` takes the `Core` lookup as a closure, because
`nvs-config` cannot see `nvs_types::core_lib`; stage 2 calls it with that lookup.

**The error's text** (end-user prose, `AGENTS.md` § *Text an end user reads*): first what is not
allowed, then where it is allowed, then the note, then the source. For example: "`Core\Router::urlSigned`
cannot be used here. It is allowed only in `Framework\Core\Signing`. Use `Framework\Core\Url::signed`.
It adds the framework's key rotation. This rule is in `modules/Framework/package.toml`, line 4." The
editor carries the same text.

**What it costs.** Nothing at run time and nothing per request, apart from one table lookup per
reflective act on a member, which stage 3 adds. At compile time: one read of each `[[restrict]]`
table, one hash-set lookup per resolved use when a table exists, and none when it does not. The
language gains one table and four codes. The restriction tables are held once per compiled program.

**How a reflective act is held (stage 3's call).** The checker, not the run time, decides: for each
`Core\Reflect\ClassInfo` acting call it lists every act on a restricted member its place does not
allow, with `E0664`'s text, and the call-site constant those members already take carries the list.
The member throws a `LogicError`, the class a refused reflective `call` of a `private` member
throws; no `Core\AccessError` exists. A callable reference to such a member lists every restricted
act, since where it runs is not known. Declined: one unit-wide table matched at run time, because a
package's entry matches by file layout, which the run time cannot see, so it would be a second copy
of the checker's match. The rule records both.

**Lanes.** Stages 1 to 3 run in order: each builds on the one before and they share
`crates/nvs-types`. After stage 3, stage 4 (`crates/nvs-lsp`) and stage 5 (`crates/nvs-cli/src/package`)
touch disjoint files and may run as side goals at once. Stage 6 waits for stages 3 to 5, because its
pages quote the final error text and the editor's behaviour.

**What stops the goal.** One attempt per item, two sessions per stage. An item still red after its
attempt goes into the handoff with its cause and the stage moves on to its other items. A stage still
open after two sessions stops the loop with `BLOCKED` naming the item.

## Stages

### 1. the table is read from the application's file and each package's, and a broken table is an error where it is written

**Does:** writes the rule, reads `[[restrict]]` into one type shared by both files, checks each entry
where it is written, and folds the tables into the cache key.

**Split:** an `Explore` subagent lists every citation of `packaging/package-toml-is-a-tool-input`'s
sentence "the compiler reads its `[grants]` table and nothing else" across `docs/` and `website/`
(`path:line`, one line each). The session writes the code.

- [x] `docs/rules/packaging/a-restriction-limits-where-a-declaration-is-used.md:1`: the new rule from
  § *Standing decisions*, with its reasons, its memory line and its Declined list. One line in
  `docs/ground-rules.md`. `docs/rules/packaging/package-toml-is-a-tool-input.md:1` says the compiler
  reads `[[restrict]]` too, and each citation the subagent lists follows. `bun nv rules --check` is
  green.
- [x] `crates/nvs-config/src/restrict.rs:1`: the `Restriction` type (`use`, `allow`, `note`,
  `replace`, and the file and line it came from) and its parser, shared by both readers. The target
  and `allow` shapes are checked here: a name that is not a declaration name, a `\*` anywhere but the
  end, an unknown field.
- [x] `crates/nvs-config/src/grants.rs:192`: `Grants::parse` reads `[[restrict]]` from the
  application's file beside `[grants]`. `crates/nvs-config/src/package.rs:209`: the package `File`
  reads it from a package's file, and a package entry whose `allow` lies outside the package's name is
  `E0665`. `crates/nvs-cli/src/package/manifest.rs:280`: `Manifest` accepts the table, so `nvs pkg`
  does not refuse a file that has one.
- [x] `crates/nvs-config/src/cache.rs:225`: the restriction digest folds in beside the grants digest,
  and adds nothing to the key when no table exists.
- [x] `E0664`, `E0665`, `E0666` and the `W` code with their cards in
  `crates/nvs-diagnostics/src/code/config.rs:922`, listed in `ALL` at
  `crates/nvs-diagnostics/src/code/mod.rs:121`.
- [x] `crates/nvs-config/tests/restrict.rs:1`: the eight tests the `table-read` check names, run once
  by name.

### 2. a use outside the allowed places does not compile

**Does:** the pass of § *Standing decisions* *When the check runs*: each resolved use of a restricted
declaration is checked against every restriction on it, and `E0664` carries the text of § *The
error's text*. A `Core` target's existence and an override target are checked here, once the
signatures are known.

**Split:** once the pass lands, a writing subagent writes the ten `.nvst` cases the `use-refused`
check names, copying the shape of `tests/conformance/cap/grants-a-namespace-holds-only-what-it-is-granted.nvst`
with a `--FILE package.toml--` section. A second one, whose files are disjoint, writes the six tests
of `crates/nvs-cli/tests/package_restrict.rs`, each in a temporary tree with the monorepo layout of
§ *Why* where the test needs it. The session runs both checks once.

- [x] `crates/nvs-types/src/restrict.rs:1`: the restricted-declaration set built from the program's
  tables once the file set is complete, the use's place (namespace, class, method and package, from
  `crates/nvs-types/src/package.rs:58` `of`), and the match of § *Standing decisions*.
  - [x] `build` indexes the entries by name, member and subtree, after `build_signatures`.
  - [x] `Place` from `crate::deprecated::window`, which now takes the body's namespace.
  - [x] `allows`: the package check, the four `allow` forms and a declaration's own code.
- [x] The pass over resolved uses, after the type check. Its reader is the session's choice between
  the typed tree and a side list (§ *When the check runs*), recorded in the rule. Every use kind of
  § *What counts as a use* reaches it: calls through `crates/nvs-types/src/expr/calls.rs:534`
  `infer_static_call` and `:54` `infer_method_call`, the method reference at `:617`, `new`, a property
  and a constant.
  - [x] The typed tree: `check_uses` reads the body's expression table entries through
    `crate::deprecated::resolved`, recorded in the rule's *When it runs*.
  - [x] Tried by hand: a static call, `new`, a property, a constant, a method reference and a `Core`
    call are refused, and the allowed namespace compiles.
- [x] Overrides: a use that resolves to an override or implementation is checked against the
  restriction on the method it overrides. A `use` that names an override is `E0665` with its help.
  - [x] `holding` walks the ancestors of a method use.
  - [x] `report_override` in `build`, tried by hand.
- [x] The `Core` target check: a `use` under `Core` that names no declaration is `E0665`. The `W`
  code for an application target that names nothing.
  - [x] `check_core` with `names_something`, and `W1031` for an application entry, tried by hand.
- [x] The ten `.nvst` cases and the six Rust tests, the `use-refused` and `layouts` checks run once.
  - [x] Ten cases under `tests/conformance/cap/restrict-*.nvst`, `use-refused` green.
  - [x] `crates/nvs-cli/tests/package_restrict.rs`, `layouts` green.

### 3. the replacement is checked where it is written and becomes the fix, and a reflective act is held to the same table

**Does:** a `replace` is compiled where the table is read, through the template checker
`#[Core\Deprecated]` uses, and `E0664` carries the filled-in replacement as its `Suggestion`. A
reflective act on a restricted member checks the same table at run time.

- [x] `crates/nvs-types/src/deprecated/template.rs:551`: the template check takes its scope from a
  restriction as well as from an attribute: the target's parameters and `$this`, compiled in the
  namespace of the file the table is in (the package's name, or the application's global
  namespace). Its errors are `E0666`, pointing into `package.toml`, carrying the inner message. A
  replacement that uses the declaration it restricts is `E0666`.
  - [x] `SourceFile::code_in` and `Restriction::replace_at` place the copy at the value's bytes
  - [x] `check_restriction` infers it, `restrict::check_replacement` reports `E0666`
  - [x] `check_files` runs each job before the walk, in a scratch `Env` (`file_env`)
- [x] `crates/nvs-types/src/deprecated/fix.rs:104`: `fill` serves `E0664`'s suggestion as it serves
  `W1003`'s, with every side-effect rule unchanged. No new fix code.
  - [x] `deprecated::attach` shared by `W1003` and `E0664`, `restriction_fix` builds the template
- [x] `crates/nvs-stdlib/src/reflect.rs:2903`: a reflective act on a member checks the restriction
  table against the place of the code that called the reflection member, and throws
  `Core\AccessError` (or the error the visibility check throws there) with `E0664`'s text. The table
  is compiled into the unit. Cost: one hash lookup per reflective act, none elsewhere.
  - [x] `restrict::record_acts` lists each refused act per call site into the expression table
  - [x] the call-site constant carries it (`nvs_runtime::source::encode_site`, `restricted`)
  - [x] `call`, `get`, `set`, `constant` and `construct` throw `LogicError` through `refuse_restricted`
- [x] `crates/nvs-cli/tests/restrict_replace.rs:1`: the four tests the `replacement-and-reflection`
  check names, and the two `.nvst` cases of the `reflective-act` check, each run once.
  - [x] the four tests, run once and green (in `nvs-cli`, since they drive `nvs check --fix`)
  - [x] the two `.nvst` cases of the `reflective-act` check

### 4. the editor shows the restriction at the use, on hover and in completion

**Does:** `E0664` reaches the editor like every other diagnostic. This stage adds what the person asked
for on top: the link to the `package.toml` line, the hover, the completion and a re-check when the
file changes.

**Split:** once the code lands, a writing subagent writes the five `.lspt` cases the `editor` check
names, copying the shape of `tests/lsp/hover/a-route-hover-in-an-autoloaded-file-lists-its-programs-grants.lspt`
and `docs/agent/conventions.md` § *An `.lspt` case*. The session runs that check once.

- [x] `crates/nvs-lsp/src/diagnostics.rs:521`: `related_information` is filled from a diagnostic's
  secondary label, and `E0664` carries one at the restricting `package.toml` line. This is the first
  user of it, so `crates/nvs-lsp/src/render.rs:1` learns to write it down in an `.lspt` answer.
- [x] `crates/nvs-lsp/src/actions.rs:272`: `E0664`'s suggestion is offered as a quick fix. Expected
  to need no code, as `W1003`'s does. The `.lspt` case proves it.
- [x] `crates/nvs-lsp/src/hover.rs:514`: beside `deprecation`, a `restriction` section on a restricted
  member's hover: where it is allowed, the note, and the file that says so.
- [x] `crates/nvs-lsp/src/completion.rs:3850`: a completion item for a member that is restricted at
  the cursor's place sorts after every other item and has its reason in `detail`. It is not tagged
  deprecated, because it is not deprecated.
- [x] The language server re-checks the open files of a program when its application file or a
  loaded package's `package.toml` changes. The grants path already watches the application file
  (`crates/nvs-config/src/grants.rs:152`); a package's manifest joins it.
- [x] `docs/reference/tools/40-editor.md:191`: a section *Restricted classes and methods* after
  *The grants of a route's program*, in end-user prose.
- [x] The five `.lspt` cases, and the `editor` check run once.

### 5. nvs pkg api lists a package's restrictions and nvs pkg diff calls a new one a break

**Does:** a package's restrictions are part of its public API, so a version that adds one, or widens
one's target or narrows its `allow`, is a breaking change.

- [ ] `crates/nvs-cli/src/package/api.rs:126`: `extract` writes the package's `[[restrict]]` entries
  into the API document under a `restrict` key, each with `use` and `allow`. `note` and `replace` are
  left out, since a change to them breaks no caller.
- [ ] `crates/nvs-cli/src/package/diff.rs:1130`: a new entry, a `use` that names more, or an `allow`
  that names less is `breaking`. A dropped entry or a wider `allow` is additive. The module doc's
  classification lists both.
- [ ] `docs/rules/packaging/only-a-major-may-break-the-api.md:1`: one sentence naming a new
  restriction as a break.
- [ ] `crates/nvs-cli/tests/package_api.rs:1`: the four tests the `api-diff` check names, run once.

### 6. the feature's proofs and its pages

**Does:** the feature `tools:packages/restrictions-where-a-class-or-method-may-be-used` gets every
proof `rule:testing/feature-proofs` lists, and every page a user reads explains it. Every file here
is text an end user reads (`AGENTS.md` § *Text an end user reads*), and each `.nvs` comment passes
`bun nv proofs --comments`.

**Split:** three writing subagents, whose files are disjoint, run side by side once the session has
written the reference section: one writes the three examples and their `.out` files, one the attacks,
one the bench. Each is told the reference section's path and the error text from stage 2. The session
writes the reference section, `about.md`, `details.md` and the website page, and runs the
`feature-proofs` check once.

- [ ] First, before the pages: a call, property read or write through a `mixed` receiver is held
  too. Today `mixed $v = new App\Secret\Vault(); $v->open();` runs a restricted method with no
  error (probed by stage 3's session). The fix reuses stage 3's mechanism: an erased call or
  property access in a body of a program with a table gets a call-site constant carrying
  `restrict::record_acts`' list for its place, and `nvs_runtime::call_erased_method_from`,
  `read_erased_property_owned` and `write_erased_property` check it through
  `nvs_runtime::source::restricted`. With no table nothing is emitted. Add attack
  `08-call-through-mixed` below, and one `.nvst` case beside the two `reflective-act` cases.
- [ ] `docs/reference/tools/35-packages.md:113`: a `#` section *Restrictions: where a class or method
  may be used* after *The rules for a package's files*, which makes the feature and its help (`nvs
  agent find restrict` reaches it). The `[[restrict]]` keys join the `package.toml` key list at
  `:125`, the page's `keywords` gain `[[restrict]]`, `restrict`, `allow`, `E0664`, `E0665`, `E0666`,
  and its `src:` comment cites the new rule.
- [ ] `docs/examples/tools/packages/restrictions-where-a-class-or-method-may-be-used/about.md:1` and
  `details.md:1`: what the table does, the four keys, what counts as a use, that nothing lifts a
  restriction, and the monorepo layout as the second way to make a package.
- [ ] Three examples in the same folder, each with its `.out`, as the grants examples are laid out:
  `01-a-framework-allows-one-namespace` (a fetched package under `vendor/` restricts
  `Core\Router::urlSigned`, and its own signer uses it), `02-a-module-uses-the-framework-signer`
  (the monorepo layout of § *Why*: `modules/Framework/package.toml`, a module's entry, and the
  module calling the framework's signer), `03-an-application-forbids-a-method-everywhere` (the
  application's own file with `allow = []`, and the program that compiles without the call).
- [ ] Attacks in `tests/hostile/tools/packages/restrictions-where-a-class-or-method-may-be-used/`,
  each `// hostile: expect-refusal`: `01-declare-the-framework-namespace` (a module file writes the
  framework's namespace), `02-call-through-a-subclass` and `03-call-through-an-interface`,
  `04-take-a-method-reference`, `05-call-by-reflection` (expects the throw instead),
  `06-lift-the-restriction-in-the-application-file` (the application's file names the same `use`
  with a wider `allow`), `07-allow-the-application-from-a-package` (a package's `allow` names the
  application's namespace).
- [ ] `benches/members/tools/packages/restrictions-where-a-class-or-method-may-be-used.nvs:1`: an
  allowed call of a restricted `Core` method in a loop, against the same call with no table, which
  shows the run time is the same. `// bench: complexity constant`.
- [ ] `website/src/content/docs/guides/how-to-use/modules.mdx:43`: a section *A framework shared by
  every module* after *One entry file for each module*: the framework folder's `package.toml` makes it
  a package, and a restriction in it holds in every module. It links to the reference section.
- [ ] The `feature-proofs` check run once, and every proof it names written.

### 7. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 3 are done and their checks green; `nvs-types` `restrict.rs` holds the check. A call
through a `mixed` receiver is not held yet: stage 6's first item, a security gap stage 7 needs.

Stage 4 is done; `editor` is green, run once. The language server now reads `[[restrict]]`
(`crates/nvs-lsp/src/document.rs` `analyse_through`): `Grants::read` from the program's entry, kept
by `Grants::restrictions_alone` (no grant key, so still no `E0658` in the editor), each loaded
package's entries from `nvs_types::package::check` (its diagnostics dropped), the digest folded
into `graph_key`, and `Analysed::restrictions` and `Analysed::manifests` (canonical, so
`to_republish` finds them). `diagnostics::related` fills `relatedInformation` from a secondary
label in another file; an `E0664`'s wire message carries its first note. Hover: `hover.rs`
`restriction`. Completion: `completion.rs` `restricted_here`, `LAST` sort prefix. The server
watches `**/package.toml` (`server.rs` `MANIFESTS`, `changed_manifests`). `nvs-types` now exports
`restrict::{entries_on, allowed_text, allows_at, Site}`. The `.lspt` runner gained
`diagnostics changed=<path>` (`suite.rs` `recheck`), and the renderer writes `related …` and
`sorted last: …` lines.

Sweep reds: `env-secret-proofs` is stale service state with no log and no definition anywhere in the
tree. Nightly 43849315979 ran on 14fbea97; its `docs` and macOS reds were fixed after it
(`2934ebe1ae`, `9efc45b6fd`).

Expected red at the gate: possibly an existing `.lspt` diagnostics case whose diagnostic has a
secondary label in another file, which now renders a `related` line; re-freeze it.
`crates/nvs-lsp/tests/completion_files.rs` already expects the second watcher glob.

### Next

Stage 5 (`crates/nvs-cli/src/package`): `api.rs:126` `extract` writes the `restrict` entries,
`diff.rs:1130` classifies them, the rule sentence, and the four `api-diff` tests, run once.

### Backlog

- The erased-call gap above is stage 6's first item. `nvs_runtime::dispatch::call_erased_method`
  gets no site today, so the erased lowering needs a trailing constant for it, only where the
  checker recorded a list (`ExprTypeTable::restricted_acts`).
- `held_acts` lists an inherited member under the class that inherits it, held by the entries on
  its declaring class, as the static check holds a use. An entry on a subclass does not hold an
  inherited member's reflective act, matching `$child->inherited()` statically.

- A `replace` naming an undeclared class or member is not `E0666` (see *State*). Running
  `nvs_hir::MemberResolver::check` over the template needs it wrapped as a statement in its
  namespace, and `module.members` handed to it; stage 7 or a follow-up decides whether it pays.
  Check whether `#[Core\Deprecated]`'s `replace` has the same gap.

- The error's note names the manifest by its absolute path. The rule's example names it relative to
  the project (`modules/Framework/package.toml`). The editor's hover names it relative to the
  program's directory (`hover.rs` `relative_to`); stage 6 picks the CLI's form with the pages
  (`crates/nvs-types/src/restrict.rs` `refusal`).
- Editor gaps, each small: a `Core` member's hover (the card path) shows no restriction; completion
  does not read the method around the cursor, so an `allow` naming `Class::method` still sorts the
  item last there; `restrict::entries_on` skips an entry on a parent's method an override replaces.
- The error prints `Ns\*` for a subtree in "It is allowed only in". Stage 6 writes the final
  wording, with the pages that quote it.
- The case `restrict-a-type-and-instanceof-are-not-uses.nvst` keeps the name the `use-refused` check
  gives it, but Novis writes the test as `is`. Rename both together if the name matters.
- The new tests carry no `covers:` line, because the feature id does not exist yet. Stage 6 adds one
  to each when it creates the feature.
- The end-user pages still say the compiler reads only `[grants]` and `[capabilities]`:
  `docs/examples/tools/packages/package-toml/about.md:9` and `docs/reference/tools/35-packages.md:128`.
  Stage 6 rewrites both with the feature's pages, once a restriction does something.
- A package's file read through `nvs pkg` (`crates/nvs-cli/src/package/manifest.rs:489`) is also
  parsed by `Grants::parse`, which reads its `[[restrict]]` as an application's and skips the
  package-name check. `Package::parse` runs right before it and makes that check, so nothing is
  missed. Stage 5 may read the entries from `Package` alone.
