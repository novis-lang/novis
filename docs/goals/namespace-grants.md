---
looper: 1
title: A namespace holds only what the application grants it
summary: "The application's grant table is keyed on namespace prefixes, every `Core` call site is checked while compiling against the grants of the namespace that encloses it, a required capability that is not granted is a compile error and an optional one a guarded throw, a package file outside its package's namespace and a package path that escapes its package are compile errors, the grant table has its feature proofs, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-config/src/capability.rs, crates/nvs-types/src/capability.rs, crates/nvs-runtime/src/capability.rs]
stages:
  - id: 1
    title: the compiler knows which capability each `Core` member needs
    status: done
    context: [crates/nvs-stdlib/src/cap.rs, crates/nvs-stdlib/tests/capability.rs, docs/rules/security/authority-is-the-enclosing-namespace.md]
    extra:
      context:
        rules: [security/authority-is-the-enclosing-namespace, security/capability-check-at-the-door, testing/capability-completeness-test]
  - id: 2
    title: the grant table is read, keyed on namespace prefixes, and the longest prefix wins
    status: done
    context: [crates/nvs-config/src/cache.rs, crates/nvs-config/src/resolve.rs, docs/rules/security/grants-are-keyed-on-a-namespace.md, docs/rules/security/longest-prefix-wins.md, docs/rules/packaging/package-toml-is-a-tool-input.md, docs/rules/config/the-extension-set-is-in-every-unit-key.md]
    extra:
      context:
        rules: [security/grants-are-keyed-on-a-namespace, security/longest-prefix-wins, security/capability-roster-is-closed, packaging/package-toml-is-a-tool-input, config/the-extension-set-is-in-every-unit-key]
  - id: 3
    title: every `Core` call site is checked against the grants of its enclosing namespace
    status: done
    context: [crates/nvs-types/src/expr/calls.rs, crates/nvs-diagnostics/src/code/config.rs, docs/rules/security/authority-is-the-enclosing-namespace.md, docs/rules/security/package-authority-is-granted-one-line-at-a-time.md]
    extra:
      context:
        rules: [security/authority-is-the-enclosing-namespace, security/package-authority-is-granted-one-line-at-a-time, security/no-runtime-grant]
  - id: 5
    title: a package file stays under its package's namespace, and a package path cannot escape the package
    status: open
    context: [crates/nvs-hir/src/requires.rs, crates/nvs-hir/src/autoload.rs, docs/rules/packaging/a-package-file-stays-under-its-declared-prefix.md, docs/rules/packaging/a-package-cannot-reach-its-host.md]
    extra:
      context:
        rules: [packaging/a-package-file-stays-under-its-declared-prefix, packaging/a-package-cannot-reach-its-host, security/path-scope-canonicalise-then-prefix]
  - id: 4
    title: an optional capability compiles behind a guard, and `Core\Cap::has` answers for its namespace
    status: open
    context: [crates/nvs-stdlib/src/cap.rs, crates/nvs-types/src/expr/calls.rs, docs/rules/security/optional-capability-degrades.md]
    extra:
      context:
        rules: [security/optional-capability-degrades, security/no-runtime-grant]
  - id: 6
    title: the grant table has its feature proofs, and its rules are shipped
    status: open
    context: [docs/reference/tools/35-packages.md, docs/reference/README.md]
  - id: 7
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: grants-proofs
    stage: 6
    name: the grant table owes no feature proof
    run: [bun, nv, proofs, --gate, --only, "tools:packages/grants-what-each-namespace-may-do"]
  - id: owners-closes
    stage: 6
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, namespace-grants]
    reach: true
    floor: false
---

# A namespace holds only what the application grants it

## Why

Today every line of a program runs with the whole capability set the operator granted. A package, a
pasted file or a git submodule can open a socket or read a file as easily as the application can.
This goal makes authority a property of the namespace that encloses the code: the application grants
each namespace what it may do, one line at a time, and the compiler refuses a `Core` call that the
namespace was not granted. It comes first among the packaging goals because it protects code no
package manager fetched, and because every later goal (the client writes grant lines, the registry
shows requested capabilities) builds on the table it adds.

## Standing decisions

**A package's name is its root namespace.** A package is `Vendor\Package`, exactly two segments, and
every file in it declares that namespace or one under it. So for a fetched package the grant line and
the package are the same thing. Nothing else about a package's namespaces or class names is checked.

**The grant table is `[grants]` in the application's own `package.toml`**
(`rule:packaging/package-toml-is-a-tool-input`). A person edits and reviews its lines; goal
`package-client` writes one per fetched package. The compiler reads that table and nothing else of
the file, so name resolution still never reads `package.toml`. The compiler finds the file by
walking up from the entry file's absolute path, the way `git` finds `.git`, and never from the
working directory: a run started in another directory would read no table, and every package would
hold the application's authority (security over the `git` habit). Only the grant table, not
the whole file, folds into every compiled unit's cache key, the way the extension set does
(`rule:config/the-extension-set-is-in-every-unit-key`, `crates/nvs-config/src/cache.rs:168`). The
operator's `nvs.toml` can only narrow it: the grant a call site holds is the intersection of the two
(`rule:security/grants-are-keyed-on-a-namespace`). The reader is a new module,
`crates/nvs-config/src/grants.rs`, beside `capability.rs`.

**The capability a `Core` member needs is recorded once, beside the member, in
`nvs_stdlib::registry::CAPABILITIES`** (`crates/nvs-stdlib/src/registry.rs:1809`). That table already
exists, with its completeness tests (`crates/nvs-stdlib/tests/capability.rs:262`, `:290`), so stage 1
adds no field: it adds the lookup the checker reads, and a test that every door calling
`nvs_runtime::capability::require` or `require_as` names the capability its member's row records. A
new `Core` class that opens a door adds its rows there as it always has: goal `core-web-push`'s
`send` is `net.connect`, and goal `core-passkey`'s class reaches nothing and has no row. The runtime
check stays exactly as it is: the compile-time check is the namespace's grant, and the door still
checks the operator's ceiling and the scope (`rule:security/no-runtime-grant`).

**A member that can ask several capabilities has a row for each, and a call needs all of them.**
`Core\IO::open` asks `fs.read` or `fs.write` by its mode, and `Core\Db::open` asks `db.open` for a
server or `fs.read` for a SQLite file. The checker sees the call, not the argument. Needing only one
of them would let a namespace granted `fs.write` alone read through `ReadWrite`, so it needs every
row (security over usability: a package that only opens a server still needs `fs.read` written for
`Core\Db::open`). `nvs_stdlib::cap::needs` returns an iterator over the rows, a `None` row stays a
member's only row, and `every_door_names_the_capability_its_member_declares` holds each door call
whose member is a literal or a `const` to the rows. Stage 3 may narrow a call by an argument the
checker already knows, such as a literal `FileMode`, and records that if it does.

**A file belongs to a package when it was reached through an autoload root inside a directory that
holds a `package.toml`.** That manifest gives the package's name and its required and optional
capabilities, read by a second new module, `crates/nvs-config/src/package.rs`, which reads those keys
and nothing else; goal `package-client`'s manifest reader calls it for them. This needs no package
manager, so the tests build the layout by hand and this goal does not wait for goal
`package-client`. Hand-copied code with no manifest gets the grant check and no file check.

**A grant key is written `'Vendor\Package'` or `'Vendor\Package\*'`, and the subtree form covers its
own namespace too.** A value is a list of capability names and never a scope; the scope is the
operator's. `'Acme\Http\*'` matches `Acme\Http` and every depth below it, so the one line `nvs pkg
add` writes covers a whole package, and an exact `'Acme\Http'` beside it wins for `Acme\Http` alone
because an exact key beats a subtree key at equal length. Length is counted in segments. A leading
`\` is allowed and means nothing, so writing a key with and without it is the one tie (`E0656`). A
first segment `Core` is refused in any case (`E0657`), as `nvs_hir`'s `is_core` compares it; every
other comparison is exact. The rest of a key's shape, a value that is not a list, an unknown
capability and a file that is not TOML are `E0655`.

**`nvs_config::cache::env_hash` takes the grant table as a second argument**, so no caller can key a
unit without one. Every caller passes `Grants::default()` until stage 3 reads the table where the
compiler runs, and the empty table is what a tree with no `package.toml` reads.

**A server keys a unit's grant table through its trace, and its environment digest keeps the
empty table.** `Grants::read` loads `package.toml` by its path and notes each directory's missing
one, so the file is one of the reads a unit's trace watches (`crates/nvs-cli/src/script.rs`'s
`Trace`). An edited or created table sends the content to a compile, and the front end checks every
call site again. One digest per process could not do this, because two mounted entries may sit
under two tables (correctness and security over one `rekey` path). The served program id therefore
omits the table where `nvs run`'s includes it; the code is the same either way, so only an artifact
hit between the two is lost.

**`Core\Config::set` learns its call site's authority from a prepared word that lets it raise.**
The checker files `PreparedFact::ApplicationSet` for a `set` written where no grant line matches,
and `nvs-ir` hands the member `PREPARED_APPLICATION` (`crates/nvs-stdlib/src/config.rs`). Every
other call, a method reference's included, reaches `nvs_config::Request::set_as_package`, which
lets a `[limits]` key move down from the value in force and never up. The word lets a call raise,
never forbids it, so a call that lost it fails closed (security over usability: the application's
own method reference to `set` cannot raise either).

**The diagnostics take the next free codes of the `E06xx` band**, after `E0654`
(`crates/nvs-diagnostics/src/code/config.rs:822`), and each message names the namespace, the
capability and the grant line that would fix it.

**Trade-offs.** Security gains the most: a compromised dependency can compute but cannot act unless a
human wrote its name in a grant line. Latency on the request path is unchanged, because the check is
done while compiling; the only run-time cost is one predictable branch at a call site a package
declared optional. Memory: the grant table is compile-time input, and nothing of it is held per
request. Simplicity: a program with no `[grants]` table compiles and runs exactly as before, because
an unmatched namespace holds the application's authority
(`rule:security/an-unmatched-namespace-holds-the-application`). Users gain one table to learn.

**Lanes.** Stages 1 (`nvs-stdlib`), 2 (`nvs-config/src/grants.rs`, `cache.rs`) and 5 (`nvs-hir`,
`nvs-config/src/package.rs`) touch disjoint files and run in any order. Stage 3 needs 1 and 2. Stage 4
needs 3 and 5, because its guard asks which package declared a capability optional, so stage 5 runs
before it. Stage 6 is last. The whole goal can run as a side goal beside the chain now. Stage 6 creates
`docs/reference/tools/35-packages.md`, which goals `api-compat` and `package-client` add headings to,
so those merge after it.

## Stages

### 1. the compiler knows which capability each `Core` member needs

**Does:** the checker can ask which capability a `Core` member needs, and a test proves every door
agrees with its member's row.

**Split:** first, an `Explore` subagent returns a door → member → capability table: every
`capability::require` and `require_as` call under `crates/nvs-stdlib/src/` and `crates/nvs-runtime/src/`,
as `file.rs:NN`, the member string and the `Cap` it passes.

- [x] `nvs_stdlib::cap::needs(class, member)`, read from `registry::CAPABILITIES`, as `is_capability` is read by the checker. `crates/nvs-stdlib/src/cap.rs:144`, `crates/nvs-stdlib/src/registry.rs:1809`
- [x] A test scans the sources as `nvs_stdlib_reaches_the_os_only_through_the_gate` does, and fails naming a door whose `Cap` disagrees with its member's row (`rule:security/authority-is-the-enclosing-namespace`, `rule:testing/capability-completeness-test`). `crates/nvs-stdlib/tests/capability.rs:332`, `crates/nvs-runtime/src/capability.rs:71`

### 2. the grant table is read, keyed on namespace prefixes, and the longest prefix wins

**Does:** the grant table parses, every capability in it is on the roster, and a lookup returns the
grants of the longest matching key.

- [x] The reader of `[grants]` in the application's `package.toml`, found by walking up as `git` finds `.git`, with an unknown capability an error (`rule:packaging/package-toml-is-a-tool-input`, `rule:security/grants-are-keyed-on-a-namespace`). `crates/nvs-config/src/grants.rs:1`, `crates/nvs-config/src/resolve.rs:392`
- [x] The grant table alone folds into every compiled unit's cache key, so an edited grant line is an ordinary cache miss (`rule:config/the-extension-set-is-in-every-unit-key`). `crates/nvs-config/src/cache.rs:179`
- [x] A bare key grants that namespace's own declarations, a subtree key grants every depth, the longest key wins, and two keys that match identically are an error naming both (`rule:security/longest-prefix-wins`). `crates/nvs-config/src/grants.rs:1`
- [x] A key under `Core` is an error. `crates/nvs-config/src/grants.rs:1`
- [x] Tests: each of the four items above, with the `package.toml` written by the test. `crates/nvs-config/tests/grants.rs:1`

### 3. every `Core` call site is checked against the grants of its enclosing namespace

**Does:** a call to a member that needs a capability compiles only where the enclosing namespace was
granted it, intersected with the operator's configuration.

- [x] The enclosing namespace is the call's authority: file-scope code is not exempt, an anonymous function keeps the namespace it was declared in, and a `use` import transfers nothing (`rule:security/authority-is-the-enclosing-namespace`). `crates/nvs-types/src/expr/calls.rs:506`
  - [x] the front end reads the table from the entry's path and `Checked` carries it into `env_hash`
  - [x] `Env::namespace_grants` reaches every call site, instance, union and static, before the method-reference branch
- [x] A namespace granted nothing holds nothing, and an empty grant line is how that is written (`rule:security/package-authority-is-granted-one-line-at-a-time`). `crates/nvs-types/src/capability.rs:49`
- [x] A required capability not granted is a compile error naming the namespace, the capability and the line that fixes it. `crates/nvs-diagnostics/src/code/config.rs:822`
- [x] A `Core\Config::set` call under a package's namespace may lower a limit and never raise one, while the application's own code on a CLI run may raise its memory limit (goal `defaults-that-work`, decision 11), with a conformance case for each. `crates/nvs-stdlib/src/config.rs:209`
  - [x] `nvs_config::Request::set_as_package` holds a `[limits]` key below the value in force
  - [x] the checker files `PreparedFact::ApplicationSet` where no grant line matches, and `nvs-ir` hands it to `set` as its argument 0
  - [x] a unit test and `tests/conformance/cap/grants-a-package-lowers-a-limit-and-never-raises-one.nvst` pass by name
- [x] Conformance cases: each item above, and a program with no `[grants]` table compiling as before (`rule:security/an-unmatched-namespace-holds-the-application`). `tests/conformance/cap/grants-a-namespace-holds-only-what-it-is-granted.nvst:1`

### 5. a package file stays under its package's namespace, and a package path cannot escape the package

**Does:** the compiler knows which package a file is in, refuses a file that declares a namespace
outside its package, and refuses a `require` or `autoload` path that leaves the package.

- [ ] The package's own manifest read for its name and its required and optional capabilities, and nothing else. `crates/nvs-config/src/package.rs:1`
- [ ] The file to package map, built from the autoload roots as the standing decision says. `crates/nvs-hir/src/requires.rs:1248`, `crates/nvs-hir/src/autoload.rs:259`
- [ ] A package file declaring a namespace outside `Vendor\Package` is a compile error naming the file, the namespace and the package (`rule:packaging/a-package-file-stays-under-its-declared-prefix`). `crates/nvs-hir/src/requires.rs:1248`
- [ ] A `require` or `autoload` path in a package that escapes it through `..`, an absolute path or a symlink is a compile error, by canonicalise then prefix check (`rule:packaging/a-package-cannot-reach-its-host`, `rule:security/path-scope-canonicalise-then-prefix`). `crates/nvs-hir/src/requires.rs:662`
- [ ] Tests: a hand-built package layout for each refusal, the symlink case through a fake canonicalizer as `crates/nvs-stdlib/tests/capability.rs:91` does. `crates/nvs-cli/tests/package_grants.rs:1`

### 4. an optional capability compiles behind a guard, and `Core\Cap::has` answers for its namespace

**Does:** a capability a package's manifest lists as optional compiles either way, and its call site
throws a catchable error when it runs without the grant.

- [ ] The guard is one branch, emitted only at a call site in a package that declared the capability optional (`rule:security/optional-capability-degrades`). `crates/nvs-types/src/expr/calls.rs:506`
- [ ] `Core\Cap::has` answers what its own call site holds. `crates/nvs-stdlib/src/cap.rs:201`
- [ ] Conformance cases: an optional capability compiles and throws without the grant, and `Core\Cap::has` answers `false` in that package. `tests/conformance/cap/grants-an-optional-capability-throws-where-it-is-not-granted.nvst:1`

### 6. the grant table has its feature proofs, and its rules are shipped

**Does:** the grant table is one feature with its proofs, and every rule this goal owns is `shipped`.

**Split:** after the chapter heading lands, one writing subagent writes the about page, the examples
and the bench, then a second writes the attack, each held to § *Text an end user reads* in `AGENTS.md`.

- [ ] The chapter `docs/reference/tools/35-packages.md` opens with `# Grants: what each namespace may do`, linked from the reference index. `docs/reference/tools/35-packages.md:1`, `docs/reference/README.md:1`
- [ ] The about page and the examples. `docs/examples/tools/packages/grants-what-each-namespace-may-do/about.md:1`
- [ ] The bench. `benches/members/tools/packages/grants-what-each-namespace-may-do.nvs:1`
- [ ] The attack: a package file declares the application's namespace to take its grants, and does not compile. `tests/hostile/tools/packages/grants-what-each-namespace-may-do/01-borrow-the-application-namespace.nvs:1`
- [ ] `bun nv proofs --id 'tools:packages/grants-what-each-namespace-may-do'` owes nothing. `docs/reference/tools/35-packages.md:1`
- [ ] `rule:security/authority-is-the-enclosing-namespace` is `shipped` with its cases in `guardedBy`. `data/rules/security/authority-is-the-enclosing-namespace.json:3`
- [ ] `rule:security/grants-are-keyed-on-a-namespace` is `shipped`. `data/rules/security/grants-are-keyed-on-a-namespace.json:3`
- [ ] `rule:security/longest-prefix-wins` is `shipped`. `data/rules/security/longest-prefix-wins.json:3`
- [ ] `rule:security/package-authority-is-granted-one-line-at-a-time` is `shipped`. `data/rules/security/package-authority-is-granted-one-line-at-a-time.json:3`
- [ ] `rule:security/optional-capability-degrades` is `shipped`. `data/rules/security/optional-capability-degrades.json:3`
- [ ] `rule:packaging/a-package-file-stays-under-its-declared-prefix` is `shipped`. `data/rules/packaging/a-package-file-stays-under-its-declared-prefix.json:3`
- [ ] `rule:packaging/a-package-cannot-reach-its-host` is `shipped`, and `bun nv owners --closes namespace-grants` passes. `data/rules/packaging/a-package-cannot-reach-its-host.json:3`

### 7. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1, 2 and 3 are done. The call-site check is `reject_ungranted_call`
(`crates/nvs-types/src/capability.rs:159`), and four conformance cases pass by name under
`tests/conformance/cap/grants-*.nvst`.

The server needed no key change. `Grants::read` (`crates/nvs-config/src/grants.rs:143`) now loads
`package.toml` by its path and notes each directory's missing one, so the file is a read in every
unit's trace, and an edit or a new file recompiles the unit. The standing decision *A server keys a
unit's grant table through its trace ...* says why the environment digest keeps the empty table.
`a_package_toml_created_or_edited_beside_the_entry_recompiles_the_unit`
(`crates/nvs-cli/src/script.rs:2674`) passes by name.

`Core\Config::set` is on `PREPARED_MEMBERS`, so its helper takes three arguments. The checker's
`prepare_config_set` (`crates/nvs-types/src/capability.rs:120`) files `PreparedFact::ApplicationSet`
where no grant line matches, and every other call reaches `Request::set_as_package`
(`crates/nvs-config/src/request.rs:324`), which only lowers a `[limits]` key.
`a_package_lowers_a_limit_and_never_raises_one` and
`grants-a-package-lowers-a-limit-and-never-raises-one.nvst` pass by name.

Tests that may go red at the gate: an IR snapshot that prints a `Core\Config::set` call now shows
`prepared config.application` or `prepared none` before its arguments, and a test that counts
`SourceMap` files or `missed` paths for a compile now sees one missed `package.toml` per directory
above the entry.
The sweep's `env-secret-proofs` log on HEAD exits 0, and nightly run 40291801939 was covered before.

### Next

**Stage 5**: a package file stays under its package's namespace, and a package path cannot escape.

- [ ] `crates/nvs-config/src/package.rs`: read a package's `package.toml` for its name and its required and optional capabilities, and nothing else, beside `grants.rs`. `crates/nvs-config/src/package.rs:1`, `crates/nvs-config/src/grants.rs:143`
- [ ] The file to package map, from the autoload roots inside a directory with a `package.toml`. `crates/nvs-hir/src/requires.rs:1248`, `crates/nvs-hir/src/autoload.rs:259`
- [ ] The namespace refusal and the path-escape refusal, each with a hand-built layout test. `crates/nvs-hir/src/requires.rs:662`, `crates/nvs-cli/tests/package_grants.rs:1`

### Backlog

- The editor reads the table and shows `E0658`. Stage 6, or a later goal. `crates/nvs-lsp/src/document.rs:771`
- `nvs pkg fetch` writes the grant lines for fetched packages in goal `package-client` (`docs/goals/package-client.md`).
- The audit warning for an unmatched autoload root is goal `package-audit`'s (`docs/goals/package-audit.md`).
