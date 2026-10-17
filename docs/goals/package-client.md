---
looper: 1
title: "`nvs` resolves, fetches and locks a program's packages"
summary: "package.toml and package.lock are read and written, nvs pkg add, fetch, update, outdated and vendor work against a registry on disk, resolution takes the highest minimum, every digest is locked and checked, a fetched package is an autoload line and a grant line, a root-only git dependency and a vendor pinned to one registry work, a pending, unlisted, retracted or withheld version is handled as its rule says, nvs pkg pack is deterministic, the commands have their feature proofs, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/package/mod.rs]
stages:
  - id: 1
    title: "`package.toml` is read, and a package's name is its namespace"
    status: open
    context: [crates/nvs-config/src/value.rs, docs/rules/packaging/package-toml-is-a-tool-input.md, docs/rules/packaging/a-package-name-is-vendor-slash-name.md, docs/rules/packaging/a-vendor-resolves-from-one-registry.md]
    extra:
      context:
        rules: [packaging/package-toml-is-a-tool-input, packaging/a-package-name-is-vendor-slash-name, packaging/a-vendor-resolves-from-one-registry, core-api/reserved-namespace]
  - id: 2
    title: "`nvs pkg pack` builds a deterministic archive, and a git dependency is fetched and packed"
    status: open
    context: [crates/nvs-config/src/capability.rs, crates/nvs-stdlib/src/http/transport.rs, docs/rules/packaging/a-package-is-its-digest.md, docs/rules/packaging/a-git-dependency-is-root-only.md]
    extra:
      context:
        rules: [packaging/a-package-is-its-digest, packaging/a-git-dependency-is-root-only, packaging/the-registry-builds-the-archive-from-git]
  - id: 3
    title: the index is read, resolution takes the highest minimum, and the lock holds every digest
    status: open
    context: [docs/rules/packaging/resolution-takes-the-highest-minimum.md, docs/rules/packaging/the-lockfile-holds-every-digest.md]
    extra:
      context:
        rules: [packaging/resolution-takes-the-highest-minimum, packaging/the-lockfile-holds-every-digest, packaging/a-vendor-resolves-from-one-registry]
  - id: 4
    title: "`nvs pkg fetch` and `nvs pkg vendor` write the tree, `vendor/packages.nvs` and a grant line per package"
    status: open
    context: [crates/nvs-hir/src/autoload.rs, docs/rules/packaging/a-fetched-package-is-an-autoload-line.md, docs/rules/packaging/nothing-runs-before-the-program.md, docs/rules/security/package-authority-is-granted-one-line-at-a-time.md, docs/rules/packaging/package-toml-is-a-tool-input.md]
    extra:
      context:
        rules: [packaging/a-fetched-package-is-an-autoload-line, packaging/nothing-runs-before-the-program, security/package-authority-is-granted-one-line-at-a-time, security/grants-are-keyed-on-a-namespace, packaging/package-toml-is-a-tool-input, config/the-extension-set-is-in-every-unit-key]
  - id: 5
    title: "`nvs pkg add`, `nvs pkg update` and `nvs pkg outdated` select only what is released and print what a change brings"
    status: open
    context: [docs/rules/packaging/a-release-waits-seven-days-unless-an-owner-confirms.md, docs/rules/packaging/a-known-bad-version-is-retracted-not-deleted.md, docs/rules/packaging/an-owner-may-unlist.md, docs/rules/packaging/the-operator-only-withholds.md]
    extra:
      context:
        rules: [packaging/a-release-waits-seven-days-unless-an-owner-confirms, packaging/resolution-takes-the-highest-minimum, packaging/an-owner-may-unlist, packaging/the-operator-only-withholds, security/package-authority-is-granted-one-line-at-a-time]
  - id: 6
    title: the client has its feature proofs, and its rules are shipped
    status: open
    context: [docs/reference/tools/35-packages.md]
checks:
  - id: client-proofs
    stage: 6
    name: the client's commands owe no feature proof
    run: [bun, nv, proofs, --gate, --only, "tools:packages/package-toml", "tools:packages/nvs-pkg-add", "tools:packages/nvs-pkg-fetch-and-nvs-pkg-vendor", "tools:packages/nvs-pkg-update-and-nvs-pkg-outdated", "tools:packages/package-lock-and-locked", "tools:packages/nvs-pkg-pack"]
  - id: owners-closes
    stage: 6
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, package-client]
    reach: true
    floor: false
---

# `nvs` resolves, fetches and locks a program's packages

## Why

A program cannot use somebody else's code today except by copying it. This goal builds the client half
of the package system: the manifest, the lock, resolution, fetching and vendoring, all as `nvs`
subcommands, and all tested against a registry that is a directory on disk. The registry service
comes later (goal `registry-service`) and writes the same static index this goal reads. The grant
table from goal `namespace-grants` is already in place, so every fetched package arrives with a grant
line, and the compiler refuses what it was not granted.

## Standing decisions

**A package is named by its namespace.** A package's root namespace has exactly two segments,
`Vendor\Package`. The name on the command line and in a URL is derived from it: lowercase, with `\`
replaced by `/` (`acme/http`). Nobody types a name into a form. `Core\` and `Novis\` are refused for
every account but the project's own, and a package may not take the name of a built-in component.

**Each vendor resolves from exactly one registry.** `package.toml` maps a vendor to a registry URL,
and every other vendor resolves from `https://registry.novis-lang.org`. The client never asks a second
registry for a name the first one lacks. A registry URL is `https:` or `file:`; `file:` is how the
tests and an offline mirror read a directory.

**The client defines the static index, and the service writes it.** The index is one JSON file per
package and one archive per digest, under the registry's URL. Each version carries its digest, its
state (pending with the time it becomes selectable, released, unlisted, retracted with reason and
fixed-in version, or withheld), its dependencies, its requested capabilities, its repository URL and
commit, and the package's owners. The schema has a version field. Stage 3 writes the schema into
`rule:packaging/a-package-is-its-digest`'s fragment, and goal `registry-service` writes to it.

**The archive is deterministic.** It is an uncompressed `ustar` tar of the package directory's regular
files, sorted by path bytes, with every time, owner and mode field fixed, and no link or special file.
Its BLAKE3 digest is the package's identity. A transport may compress it. Stage 2 states this in
`rule:packaging/a-package-is-its-digest`, and the registry builds the same bytes from the same commit
because it runs the same code (`rule:packaging/the-registry-builds-the-archive-from-git`).

**A git clone runs inside fixed limits.** A root git dependency is cloned over HTTPS at an exact
commit, with a size and a time limit, and its host's address is checked against the outbound table
(`crates/nvs-config/src/capability.rs:262`), so an internal host is refused. The clone uses `gix`,
and its connection goes through that check.

**Resolution takes the highest minimum across majors too.** A graph is never unsolvable. A break that
matters is a compile error naming the package, the member and both versions, because the whole
program is type-checked. `nvs pkg add` and `nvs pkg update` print when a selection crosses a major and which
package asked for the older one. A `0.0.x` dependency is an ordinary minimum.

**The grant lines go into `[grants]` in the application's own `package.toml`**
(`rule:packaging/package-toml-is-a-tool-input`). `nvs pkg add` and `nvs pkg fetch` write one line for every
package in the graph, an empty one included, and a person edits and reviews them. The compiler reads
that table and nothing else of the file, and only the table folds into every compiled unit's cache key
(`rule:config/the-extension-set-is-in-every-unit-key`); goal `namespace-grants` built that reader. A
line a person already edited is never rewritten.

**The lock records what the change report compares.** Beside name, version, source and digest, each
entry records the package's requested capabilities, its repository and commit, and its owners, so
`nvs pkg update` can print what changed since the lock.

**Trade-offs.** No program changes at run time: a fetched package is ordinary source behind an
`autoload` line. Nothing runs on the request path. Memory: none per request; the client holds one
resolved graph while it runs. Security gains a digest on every byte, one registry per vendor and a
grant line per package. Users gain six subcommands, each one a verb they expect from other languages.

## Stages

### 1. `package.toml` is read, and a package's name is its namespace

**Does:** the manifest parses with unknown keys refused, and the package's name, version,
dependencies, registries and requested capabilities are read from it.

- [ ] The reader, found by walking up from the working directory, never by the runtime; the compiler's only part of the file is `[grants]` (`rule:packaging/package-toml-is-a-tool-input`). `crates/nvs-cli/src/package/manifest.rs:1`
- [ ] A two-segment root namespace, the derived name, one name per case-folded namespace, and `Core\`, `Novis\` and built-in names refused (`rule:packaging/a-package-name-is-vendor-slash-name`). `crates/nvs-cli/src/package/manifest.rs:1`
- [ ] The vendor to registry table and its default (`rule:packaging/a-vendor-resolves-from-one-registry`). `crates/nvs-cli/src/package/manifest.rs:1`

### 2. `nvs pkg pack` builds a deterministic archive, and a git dependency is fetched and packed

**Does:** a package directory or a cloned commit becomes the same archive bytes and digest on every
platform.

- [ ] The archive and its BLAKE3 digest as the standing decision says, with an `nvs.toml` anywhere in it refused (`rule:packaging/a-package-is-its-digest`). `crates/nvs-cli/src/package/pack.rs:1`, `Cargo.toml:425`
- [ ] The HTTPS clone at an exact commit, inside the size and time limits and behind the outbound address check. `crates/nvs-cli/src/package/git.rs:1`, `crates/nvs-config/src/capability.rs:262`
- [ ] A git dependency below the root is a hard error naming the package that declared it (`rule:packaging/a-git-dependency-is-root-only`). `crates/nvs-cli/src/package/resolve.rs:1`

### 3. the index is read, resolution takes the highest minimum, and the lock holds every digest

**Does:** the client reads a fixture registry on disk, resolves the graph and writes `package.lock`.

- [ ] The index reader, its schema written into the rule, and a fixture registry under `crates/nvs-cli/tests/fixtures/registry/`. `crates/nvs-cli/src/package/index.rs:1`
- [ ] Minimal version selection across majors, with the crossing reported (`rule:packaging/resolution-takes-the-highest-minimum`). `crates/nvs-cli/src/package/resolve.rs:1`
- [ ] The lock, `--locked` that fetches nothing outside it, and a digest mismatch as a hard error naming the package (`rule:packaging/the-lockfile-holds-every-digest`). `crates/nvs-cli/src/package/lock.rs:1`

### 4. `nvs pkg fetch` and `nvs pkg vendor` write the tree, `vendor/packages.nvs` and a grant line per package

**Does:** the resolved graph lands under `vendor/` with one generated autoload file and one grant line
for every package, an empty one included.

- [ ] `vendor/packages.nvs`, generated from the lock and committed (`rule:packaging/a-fetched-package-is-an-autoload-line`). `crates/nvs-cli/src/package/fetch.rs:1`, `crates/nvs-hir/src/autoload.rs:259`
- [ ] `nvs pkg add` and `nvs pkg fetch` write a line into `[grants]` in the application's `package.toml` for every package in the graph, direct and transitive, with nothing granted until a person writes it and an edited line left as it is (`rule:security/package-authority-is-granted-one-line-at-a-time`, `rule:packaging/package-toml-is-a-tool-input`). `crates/nvs-cli/src/package/fetch.rs:1`, `crates/nvs-cli/src/package/manifest.rs:1`
- [ ] A test that a package with an effectful top-level statement produces no effect from `fetch`, `build` or `vendor` (`rule:packaging/nothing-runs-before-the-program`). `crates/nvs-cli/tests/package_client.rs:1`

### 5. `nvs pkg add`, `nvs pkg update` and `nvs pkg outdated` select only what is released and print what a change brings

**Does:** a pending version is fetched only by its exact version, a retracted, unlisted or withheld
one is handled as its rule says, and every change prints its authority before it lands.

- [ ] A pending version is never selected by `add` or `update`, and `nvs pkg add acme/http@1.4.0` fetches it (`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`). `crates/nvs-cli/src/package/resolve.rs:1`
- [ ] An unlisted version still resolves and fetches, a withheld one fails naming its advisory, and a retracted one is never newly selected (`rule:packaging/an-owner-may-unlist`, `rule:packaging/the-operator-only-withholds`). `crates/nvs-cli/src/package/resolve.rs:1`
- [ ] The output: every capability requested by the package and its subgraph, capability changes since the lock, the transitive count, owner and repository changes, and the majors crossed with who asked for the older one. `crates/nvs-cli/src/package/report.rs:1`

### 6. the client has its feature proofs, and its rules are shipped

**Does:** each command is a feature with its proofs, and every rule this goal owns is `shipped`.

- [ ] The headings `# package.toml`, `# nvs pkg add`, `# nvs pkg fetch and nvs pkg vendor`, `# nvs pkg update and nvs pkg outdated`, `# package.lock and --locked` and `# nvs pkg pack` in `docs/reference/tools/35-packages.md`, each owing nothing. `docs/reference/tools/35-packages.md:1`
- [ ] The attacks: a lock whose digest was edited, a transitive git dependency, and a vendor answered by a second registry. `tests/hostile/tools/packages/nvs-fetch-and-nvs-vendor/01-an-edited-digest.nvs:1`
- [ ] Every rule this goal owns is `shipped`, and `bun nv owners --closes package-client` passes. `data/rules/packaging/a-package-is-its-digest.json:1`

## Handoff

### State

The goal is new. No stage is started. The tree has no package system: nothing reads `package.toml`
or `package.lock`, and nothing digests an archive.

### Next

Stage 1, on the new `package` module of the CLI:
- the manifest reader and the derived name (`rule:packaging/package-toml-is-a-tool-input`). `crates/nvs-cli/src/package/manifest.rs:1`
- the subcommand group beside the others. `crates/nvs-cli/src/main.rs:361`

### Backlog

- Verifying each archive's inclusion in the transparency log is goal `registry-log`'s (`docs/goals/registry-log.md`).
- `nvs pkg audit` and the advisory feed are goal `package-audit`'s (`docs/goals/package-audit.md`).
