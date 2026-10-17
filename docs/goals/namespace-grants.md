---
looper: 1
title: A namespace holds only what the application grants it
summary: "The application's grant table is keyed on namespace prefixes, every `Core` call site is checked while compiling against the grants of the namespace that encloses it, a required capability that is not granted is a compile error and an optional one a guarded throw, a package file outside its package's namespace and a package path that escapes its package are compile errors, the grant table has its feature proofs, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-config/src/capability.rs, crates/nvs-types/src/capability.rs, crates/nvs-runtime/src/capability.rs]
stages:
  - id: 1
    title: the compiler knows which capability each `Core` member needs
    status: open
    context: [crates/nvs-stdlib/src/cap.rs, crates/nvs-stdlib/src/registry.rs, docs/rules/security/authority-is-the-enclosing-namespace.md]
    extra:
      context:
        rules: [security/authority-is-the-enclosing-namespace, security/capability-check-at-the-door]
  - id: 2
    title: the grant table is read, keyed on namespace prefixes, and the longest prefix wins
    status: open
    context: [crates/nvs-config/src/tree.rs, crates/nvs-config/src/cache.rs, docs/rules/security/grants-are-keyed-on-a-namespace.md, docs/rules/security/longest-prefix-wins.md, docs/rules/packaging/package-toml-is-a-tool-input.md, docs/rules/config/the-extension-set-is-in-every-unit-key.md]
    extra:
      context:
        rules: [security/grants-are-keyed-on-a-namespace, security/longest-prefix-wins, security/capability-roster-is-closed, packaging/package-toml-is-a-tool-input, config/the-extension-set-is-in-every-unit-key]
  - id: 3
    title: every `Core` call site is checked against the grants of its enclosing namespace
    status: open
    context: [crates/nvs-types/src/expr/calls.rs, crates/nvs-diagnostics/src/lib.rs, docs/rules/security/authority-is-the-enclosing-namespace.md, docs/rules/security/package-authority-is-granted-one-line-at-a-time.md]
    extra:
      context:
        rules: [security/authority-is-the-enclosing-namespace, security/package-authority-is-granted-one-line-at-a-time, security/no-runtime-grant]
  - id: 4
    title: an optional capability compiles behind a guard, and `Core\Cap::has` answers for its namespace
    status: open
    context: [crates/nvs-stdlib/src/cap.rs, crates/nvs-types/src/expr/calls.rs, docs/rules/security/optional-capability-degrades.md]
    extra:
      context:
        rules: [security/optional-capability-degrades, security/no-runtime-grant]
  - id: 5
    title: a package file stays under its package's namespace, and a package path cannot escape the package
    status: open
    context: [crates/nvs-hir/src/requires.rs, crates/nvs-hir/src/autoload.rs, docs/rules/packaging/a-package-file-stays-under-its-declared-prefix.md, docs/rules/packaging/a-package-cannot-reach-its-host.md]
    extra:
      context:
        rules: [packaging/a-package-file-stays-under-its-declared-prefix, packaging/a-package-cannot-reach-its-host, security/path-scope-canonicalise-then-prefix]
  - id: 6
    title: the grant table has its feature proofs, and its rules are shipped
    status: open
    context: [docs/reference/tools/35-packages.md, docs/reference/README.md]
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
the file, so name resolution still never reads `package.toml`. `nvs check` and the editor find the
file by walking up from the working directory, the way `git` finds `.git`. Only the grant table, not
the whole file, folds into every compiled unit's cache key, the way the extension set does
(`rule:config/the-extension-set-is-in-every-unit-key`, `crates/nvs-config/src/cache.rs:168`). The
operator's `nvs.toml` can only narrow it: the grant a call site holds is the intersection of the two
(`rule:security/grants-are-keyed-on-a-namespace`).

**The longest matching key wins** (`rule:security/longest-prefix-wins`, which this goal owns). A bare
key grants that namespace's own declarations, a subtree key grants every depth, and two keys that
match identically are an error naming both.

**The capability a `Core` member needs is recorded once, beside the member.** Stage 1 adds it to the
member's registry entry, and a test enumerates every door that calls
`nvs_runtime::capability::require` and fails naming a member whose entry does not record the same
capability. The runtime check stays exactly as it is: the compile-time check is the namespace's
grant, and the door still checks the operator's ceiling and the scope (`rule:security/no-runtime-grant`).

**A file belongs to a package when it was reached through an autoload root inside a directory that
holds a `package.toml`.** That manifest gives the package's name and its required and optional
capabilities. This needs no package manager, so the tests build the layout by hand and this goal does
not wait for goal `package-client`. Hand-copied code with no manifest gets the grant check and no
file check.

**The diagnostics take the next free codes of the `E06xx` band** in `crates/nvs-diagnostics/src/lib.rs`,
and each message names the namespace, the capability and the grant line that would fix it.

**Trade-offs.** Security gains the most: a compromised dependency can compute but cannot act unless a
human wrote its name in a grant line. Latency on the request path is unchanged, because the check is
done while compiling; the only run-time cost is one predictable branch at a call site a package
declared optional. Memory: the grant table is compile-time input, and nothing of it is held per
request. Simplicity: a program with no `[grants]` table compiles and runs exactly as before, because
an unmatched namespace holds the application's authority
(`rule:security/an-unmatched-namespace-holds-the-application`). Users gain one table to learn.

## Stages

### 1. the compiler knows which capability each `Core` member needs

**Does:** every `Core` member that opens a door records the capability it needs, and a test proves
the record matches the door.

- [ ] Each door's capability is a field of its member's registry entry, read by the checker through `nvs_stdlib::cap` as `is_capability` is. `crates/nvs-stdlib/src/cap.rs:136`, `crates/nvs-stdlib/src/registry.rs:1`
- [ ] A test lists every `capability::require` call and fails naming a member whose entry disagrees (`rule:security/authority-is-the-enclosing-namespace`). `crates/nvs-runtime/src/capability.rs:71`

### 2. the grant table is read, keyed on namespace prefixes, and the longest prefix wins

**Does:** the grant table parses, every capability in it is on the roster, and a lookup returns the
grants of the longest matching key.

- [ ] The reader of `[grants]` in the application's `package.toml`, found by walking up as `git` finds `.git`, with an unknown capability an error (`rule:packaging/package-toml-is-a-tool-input`, `rule:security/grants-are-keyed-on-a-namespace`). `crates/nvs-config/src/capability.rs:686`, `crates/nvs-config/src/tree.rs:494`
- [ ] The grant table alone folds into every compiled unit's cache key, so an edited grant line is an ordinary cache miss (`rule:config/the-extension-set-is-in-every-unit-key`). `crates/nvs-config/src/cache.rs:168`
- [ ] A bare key grants that namespace's own declarations, a subtree key grants every depth, the longest key wins, and two keys that match identically are an error naming both (`rule:security/longest-prefix-wins`). `crates/nvs-config/src/capability.rs:686`
- [ ] A key under `Core` is an error. `crates/nvs-config/src/capability.rs:58`

### 3. every `Core` call site is checked against the grants of its enclosing namespace

**Does:** a call to a member that needs a capability compiles only where the enclosing namespace was
granted it, intersected with the operator's configuration.

- [ ] The enclosing namespace is the call's authority: file-scope code is not exempt, an anonymous function keeps the namespace it was declared in, and a `use` import transfers nothing (`rule:security/authority-is-the-enclosing-namespace`). `crates/nvs-types/src/expr/calls.rs:506`
- [ ] A namespace granted nothing holds nothing, and an empty grant line is how that is written (`rule:security/package-authority-is-granted-one-line-at-a-time`). `crates/nvs-types/src/capability.rs:49`
- [ ] A required capability not granted is a compile error naming the namespace, the capability and the line that fixes it. `crates/nvs-diagnostics/src/lib.rs:3028`

### 4. an optional capability compiles behind a guard, and `Core\Cap::has` answers for its namespace

**Does:** a capability a package's manifest lists as optional compiles either way, and its call site
throws a catchable error when it runs without the grant.

- [ ] The guard is one branch, emitted only at a call site in a package that declared the capability optional (`rule:security/optional-capability-degrades`). `crates/nvs-types/src/expr/calls.rs:506`
- [ ] `Core\Cap::has` answers what its own call site holds. `crates/nvs-stdlib/src/cap.rs:126`

### 5. a package file stays under its package's namespace, and a package path cannot escape the package

**Does:** the compiler knows which package a file is in, refuses a file that declares a namespace
outside its package, and refuses a `require` or `autoload` path that leaves the package.

- [ ] The file to package map, built from the autoload roots as the standing decision says. `crates/nvs-hir/src/requires.rs:1248`, `crates/nvs-hir/src/autoload.rs:139`
- [ ] A package file declaring a namespace outside `Vendor\Package` is a compile error naming the file, the namespace and the package (`rule:packaging/a-package-file-stays-under-its-declared-prefix`). `crates/nvs-hir/src/requires.rs:1248`
- [ ] A `require` or `autoload` path in a package that escapes it through `..`, an absolute path or a symlink is a compile error, by canonicalise then prefix check (`rule:packaging/a-package-cannot-reach-its-host`). `crates/nvs-hir/src/requires.rs:662`

### 6. the grant table has its feature proofs, and its rules are shipped

**Does:** the grant table is one feature with its proofs, and every rule this goal owns is `shipped`.

- [ ] The chapter `docs/reference/tools/35-packages.md` opens with `# Grants: what each namespace may do`, and `bun nv proofs --id 'tools:packages/grants-what-each-namespace-may-do'` owes nothing. `docs/reference/tools/35-packages.md:1`
- [ ] The attack: a package file declares the application's namespace to take its grants, and does not compile. `tests/hostile/tools/packages/grants-what-each-namespace-may-do/01-borrow-the-application-namespace.nvs:1`
- [ ] Every rule this goal owns is `shipped` with its cases in `guardedBy`, and `bun nv owners --closes namespace-grants` passes. `data/rules/security/grants-are-keyed-on-a-namespace.json:1`

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written. Nothing in the tree reads a per-namespace grant table yet.

### Next

Stage 1, on the `Core` registry and the capability helper:
- record each door's capability on its member's entry (`rule:security/authority-is-the-enclosing-namespace`). `crates/nvs-stdlib/src/cap.rs:136`
- the test that every `capability::require` call agrees with its entry. `crates/nvs-runtime/src/capability.rs:71`

### Backlog

- `nvs pkg fetch` writes the grant lines for fetched packages in goal `package-client` (`docs/goals/package-client.md`).
- The audit warning for an unmatched autoload root is goal `package-audit`'s (`docs/goals/package-audit.md`).
