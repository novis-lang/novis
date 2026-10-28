---
looper: 1
title: "`nvs` resolves, fetches and locks a program's packages"
summary: "package.toml and package.lock are read and written, nvs pkg add, fetch, update, outdated and vendor work against a registry on disk, resolution takes the highest minimum, every digest is locked and checked, a fetched package is an autoload line and a grant line, a root-only git dependency and a vendor pinned to one registry work, a pending, unlisted, retracted or withheld version is handled as its rule says, nvs pkg pack is deterministic and carries only a package's source and the files it lists within the archive limits, an external download is pinned by URL, SHA-256 and size and fetched without running anything, the commands have their feature proofs, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/package/mod.rs]
stages:
  - id: 1
    title: "`package.toml` is read, and a package's name is its namespace"
    status: done
    context: [crates/nvs-cli/src/package/manifest.rs, crates/nvs-config/src/package.rs, docs/rules/packaging/package-toml-is-a-tool-input.md, docs/rules/packaging/a-package-name-is-vendor-slash-name.md, docs/rules/packaging/a-vendor-resolves-from-one-registry.md]
    extra:
      context:
        rules: [packaging/package-toml-is-a-tool-input, packaging/a-package-name-is-vendor-slash-name, packaging/a-vendor-resolves-from-one-registry, core-api/reserved-namespace]
  - id: 2
    title: "`nvs pkg pack` builds a deterministic archive"
    status: done
    context: [crates/nvs-cli/src/package/pack.rs, docs/rules/packaging/a-package-is-its-digest.md]
    extra:
      context:
        rules: [packaging/a-package-is-its-digest, packaging/the-registry-builds-the-archive-from-git, packaging/somebody-elses-specification-is-a-crate]
  - id: 3
    title: the index is read, resolution takes the highest minimum, and the lock holds every digest
    status: open
    context: [docs/rules/packaging/resolution-takes-the-highest-minimum.md, docs/rules/packaging/the-lockfile-holds-every-digest.md]
    extra:
      context:
        rules: [packaging/resolution-takes-the-highest-minimum, packaging/the-lockfile-holds-every-digest, packaging/a-vendor-resolves-from-one-registry, packaging/registry-hosting-is-configuration]
  - id: 7
    title: "a root git dependency is cloned inside fixed limits and packed, and one below the root is refused"
    status: open
    context: [crates/nvs-config/src/capability.rs, crates/nvs-stdlib/src/http/transport.rs, crates/nvs-cli/src/package/resolve.rs, docs/rules/packaging/a-git-dependency-is-root-only.md]
    extra:
      context:
        rules: [packaging/a-git-dependency-is-root-only, packaging/a-package-is-its-digest, security/net-address-policy]
  - id: 4
    title: "`nvs pkg fetch` and `nvs pkg vendor` write the tree, `vendor/packages.nvs` and a grant line per package"
    status: open
    context: [crates/nvs-hir/src/autoload.rs, crates/nvs-config/src/grants.rs, docs/rules/packaging/a-fetched-package-is-an-autoload-line.md, docs/rules/packaging/nothing-runs-before-the-program.md, docs/rules/security/package-authority-is-granted-one-line-at-a-time.md, docs/rules/packaging/package-toml-is-a-tool-input.md]
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
  - id: 9
    title: "an external download is fetched, checked and placed, and `nvs pkg download` and `nvs pkg provide` work"
    status: open
    context: [docs/rules/packaging/an-external-file-is-pinned-by-url-sha256-and-size.md, crates/nvs-config/src/capability.rs, crates/nvs-stdlib/src/http/transport.rs]
    extra:
      context:
        rules: [packaging/an-external-file-is-pinned-by-url-sha256-and-size, packaging/a-package-carries-only-its-source-and-what-it-lists, packaging/nothing-runs-before-the-program, security/net-address-policy, programs/relative-paths-resolve-from-their-file, security/package-authority-is-granted-one-line-at-a-time, security/path-scope-canonicalise-then-prefix]
  - id: 6
    title: "`package.toml`, `nvs pkg add` and `nvs pkg fetch` have their feature proofs"
    status: open
    context: [docs/reference/tools/35-packages.md]
  - id: 8
    title: the other client commands have their feature proofs, and the rules are shipped
    status: open
    context: [docs/reference/tools/35-packages.md]
  - id: 10
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: client-proofs
    stage: 8
    name: the client's commands owe no feature proof
    run: [bun, nv, proofs, --gate, --only, "tools:packages/package-toml", "tools:packages/nvs-pkg-add", "tools:packages/nvs-pkg-fetch-and-nvs-pkg-vendor", "tools:packages/nvs-pkg-update-and-nvs-pkg-outdated", "tools:packages/package-lock-and-locked", "tools:packages/nvs-pkg-pack", "tools:packages/nvs-pkg-download-and-nvs-pkg-provide"]
  - id: owners-closes
    stage: 8
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
comes later (goal `registry-service`) and writes the same static index this goal reads. Goal
`namespace-grants` builds the grant table, so every fetched package arrives with a grant line, and the
compiler refuses what it was not granted.

## Standing decisions

**A package is named by its namespace.** A package's root namespace has exactly two segments,
`Vendor\Package`. The name on the command line and in a URL is derived from it: lowercase, with `\`
replaced by `/` (`acme/http`). Nobody types a name into a form. `Core\` and `Novis\` are refused for
every account but the project's own, and a package may not take the name of a built-in component.

**Each vendor resolves from exactly one registry.** `package.toml` maps a vendor to a registry URL,
and every other vendor resolves from `https://registry.novis-lang.org`. The client never asks a second
registry for a name the first one lacks. A registry URL is `https:` or `file:`; `file:` is how the
tests and an offline mirror read a directory.

**The client defines the static index, and the service writes it.** The registry's URL serves one root
file, `registry.json`, which lists the file hosts in order; the index is one JSON file per package and
one archive per digest, under each file host (`rule:packaging/registry-hosting-is-configuration`). A
root file with no file host means the files are under the registry's URL itself, as a `file:` registry
has them. The client reads from the first file host that answers, tries the next on a network failure
or a digest mismatch, and keeps the last root file it read under `<data>/registry/`, so it still finds
the file hosts when the registry's URL does not answer. Why: the files can then move to a CDN, or gain
a second host, by a change to the root file and no `nvs` release. Goal `registry-log` signs the root
file and the index files and bounds their age. Each version carries its digest, its
state (pending with the time it becomes selectable, released, unlisted, retracted with reason and
fixed-in version, or withheld), its dependencies, its requested capabilities, its repository URL and
commit, and the package's owners. The schema has a version field. Stage 3 writes the schema into
`rule:packaging/a-package-is-its-digest`'s fragment, and goal `registry-service` writes to it. The
tests' fixture registry is built at test time from package directories through `pack.rs`, so a
committed digest never goes stale.

**The archive is deterministic.** It is an uncompressed `ustar` tar of the files
`rule:packaging/a-package-carries-only-its-source-and-what-it-lists` names (`src/**/*.nvs`,
`package.toml`, the root license and readme files, and the `files` list), within its 16 MiB and
5,000-file limits and its path rules, sorted by path bytes, with every time, owner and mode field
fixed, and no link or special file.
Its BLAKE3 digest (`Cargo.toml:425`) is the package's identity. A transport may compress it. Stage 2
writes it through the `tar` crate (a new workspace row, default features off), with every header field
set by the code, because a wire format is a crate's
(`rule:packaging/somebody-elses-specification-is-a-crate`). It states this in
`rule:packaging/a-package-is-its-digest`, and
the registry builds the same bytes from the same commit because it runs the same code
(`rule:packaging/the-registry-builds-the-archive-from-git`).

**A git clone runs inside fixed limits.** A root git dependency is cloned over HTTPS at an exact
commit alone, with no history, inside the 512 MiB size limit and the time limit
`rule:packaging/the-registry-builds-the-archive-from-git` sets, and its host's address is checked
against the outbound table
(`crates/nvs-config/src/capability.rs:263`), so an internal host is refused. The clone uses `gix` with
its default features off and no C transport, recorded in its `Cargo.toml` row with `cargo deny` green.
Its connection is the one `nvs` resolved and checked: if `gix` takes no pinned connector, stage 7
implements `gix`'s client transport over the stdlib's HTTP client
(`crates/nvs-stdlib/src/http/transport.rs`), which already pins and checks. A `file:` URL is accepted
for a git dependency as it is for a registry, because it reaches only the user's own disk.

**The clone's tests never reach the network.** The address refusal, TLS and the time limit run against
loopback listeners built from `rcgen` and `rustls`, as `nvs-cli`'s tests already do
(`crates/nvs-cli/Cargo.toml:131`), with loopback allowed only by the operator exception the test
writes. The exact commit, the content and the size limit run against a `file:` remote of a repository
the test builds with `gix`. Why: security first, so the production path has no test-only escape;
loopback TLS exercises the same pinned connection a real host gets.

**An external download goes through the stdlib's HTTP client.** `crates/nvs-cli/src/package/download.rs`
reads a `[[download]]` entry, fetches it through the same pinned and checked connection as the clone
(`crates/nvs-stdlib/src/http/transport.rs`), checks each redirect hop against the outbound table
(`crates/nvs-config/src/capability.rs:263`), streams to a temporary file in the SHA-256 cache, and
renames it into place once size and hash match (`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`).
SHA-256 comes from the workspace's `sha2` row (`Cargo.toml:649`). Its tests run against loopback TLS listeners as the clone's do.
A package reads its listed or downloaded file through a relative string literal
(`rule:programs/relative-paths-resolve-from-their-file`) with no grant line: code under a package's
namespace may read exactly its own archive and download files, checked by canonical path
(`rule:packaging/a-package-carries-only-its-source-and-what-it-lists`). Stage 9 adds that check
beside the grant lookup goal `namespace-grants` builds (`crates/nvs-config/src/grants.rs`), reading
each package's file set from where `nvs pkg fetch` records it, and names that place in the rule's
**Not on disk** line when it ships.

**Resolution takes the highest minimum across majors too.** A graph is never unsolvable. A break that
matters is a compile error naming the package, the member and both versions, because the whole
program is type-checked. `nvs pkg add` and `nvs pkg update` print when a selection crosses a major and which
package asked for the older one. A `0.0.x` dependency is an ordinary minimum.

**The grant lines go into `[grants]` in the application's own `package.toml`**
(`rule:packaging/package-toml-is-a-tool-input`). `nvs pkg add` and `nvs pkg fetch` write one line for every
package in the graph, an empty one included, and a person edits and reviews them. The compiler reads
that table and nothing else of the file, and only the table folds into every compiled unit's cache key
(`rule:config/the-extension-set-is-in-every-unit-key`). A line a person already edited is never
rewritten.

**The manifest reader grows goal `api-compat`'s.** That goal creates `crates/nvs-cli/src/package/mod.rs`
with the `nvs pkg` group and `package/manifest.rs` reading `version` alone. Stage 1 grows the same file
into the full reader with unknown keys refused, and reads a package's name and its capability split
through `nvs_config::package` (goal `namespace-grants`), so the compiler and the client read those keys
with one parser.

**The lock records what the change report compares.** Beside name, version, source and digest, each
entry records the package's requested capabilities, its repository and commit, and its owners, so
`nvs pkg update` can print what changed since the lock.

**Gates on other goals.** `api-compat`: `crates/nvs-cli/src/package/mod.rs` exists with `Command::Pkg`
in `crates/nvs-cli/src/main.rs`; if not, stage 1 creates both as that goal's standing decision says.
`namespace-grants`: `crates/nvs-config/src/grants.rs` and `crates/nvs-config/src/package.rs` exist and
`data/rules/security/grants-are-keyed-on-a-namespace.json` says `shipped`. Stage 4 waits on it; until
it holds, the session takes stages 1, 2, 3 and 7. If only `package.rs` is missing, stage 1 reads the
name and the capability split itself, and the handoff names the duplicate.

**The manifest's keys are fixed, and every other key is refused** (stage 1). `[package]` holds
`name`, `version` and `files`; `[dependencies]` maps a package's namespace, as `[grants]` keys it, to
a minimum version string or to `{ git, rev }` with a full lowercase commit hash; `[registries]` maps a
vendor to `{ url, key }`; then `[capabilities]`, `[grants]` and `[[download]]`. A file with no
`[package] name` is an application's and may not hold `version`, `files`, `[capabilities]` or
`[[download]]`. One spelling per value, so a dependency is never a table with a `version` key. Why:
simplicity, and a key nobody wrote a reader for is never silently ignored. A dependency key is the
namespace, not the slug, so the file names a package one way in `[dependencies]` and `[grants]`.
The verifier `key` is read and not required yet; goal `registry-log` requires it for a registry other
than the default.

**The client refuses `Core` and the built-in components' namespaces, and reads any other `Novis\`
name** (stage 1). Only the registry knows the publishing account, and the project's own account
publishes `Novis\Rest`, so the account check for `Novis\` is goal `registry-service`'s at publish.
The built-in namespaces are derived from `nvs_ext::builtin`'s `*_CLASS` constants, so a new
component is refused with no second list.

**Trade-offs.** No program changes at run time: a fetched package is ordinary source behind an
`autoload` line. Nothing runs on the request path. Memory: none per request; the client holds one
resolved graph while it runs. Security gains a digest on every byte, one registry per vendor and a
grant line per package. Users gain six subcommands, each one a verb they expect from other languages.
Maintainers gain `nvs pkg download`, and users `nvs pkg provide` for a file whose URLs died. An
archive carries only what runs, within 16 MiB, so a fetch is small; a large file is downloaded once
into a cache shared across projects, and a third-party host it comes from sees the installer's
address, which `nvs pkg add` prints.
`gix` is a large dependency, bought because a git dependency needs a clone whose connection `nvs`
checks.

**Lanes.** Stage 1 comes first. After it, stage 2 (`pack.rs`) and stage 3 (`index.rs`, `resolve.rs`,
`lock.rs`) touch disjoint files and run in either order. Stage 7 shares `resolve.rs` with stage 3 and
runs after it. Stage 4 runs after 3 and 7 and behind the `namespace-grants` gate, stage 5 after 4
(`resolve.rs`), stage 9 after 5 (`fetch.rs`, `report.rs`, `lock.rs`), then 6 and 8. The goal is not a side goal. It shares the `Command` enum in
`crates/nvs-cli/src/main.rs:372` with goals `api-compat` and `registry-log`, the workspace `Cargo.toml`
(`tar`, `gix`), and `docs/reference/tools/35-packages.md` with goals `namespace-grants` and `api-compat`; those
merge one after another.

## Stages

### 1. `package.toml` is read, and a package's name is its namespace

**Does:** the manifest parses with unknown keys refused, and the package's name, version,
dependencies, registries and requested capabilities are read from it.

- [x] The reader, found by walking up from the working directory, never by the runtime; the compiler's only part of the file is `[grants]` (`rule:packaging/package-toml-is-a-tool-input`). `crates/nvs-cli/src/package/manifest.rs:1`, `crates/nvs-config/src/package.rs:1`
- [x] A two-segment root namespace, the derived name, one name per case-folded namespace, and `Core\`, `Novis\` and built-in names refused (`rule:packaging/a-package-name-is-vendor-slash-name`, `rule:core-api/reserved-namespace`). `crates/nvs-cli/src/package/manifest.rs:1`
- [x] The vendor to registry table and its default (`rule:packaging/a-vendor-resolves-from-one-registry`). `crates/nvs-cli/src/package/manifest.rs:1`
- [x] Tests: each refusal and the derived name. `crates/nvs-cli/tests/package_client.rs:1`

### 2. `nvs pkg pack` builds a deterministic archive

**Does:** a package directory becomes the same archive bytes and digest on every platform.

- [x] The `tar` row in the workspace `Cargo.toml`, with `cargo deny` green (`rule:packaging/somebody-elses-specification-is-a-crate`). `Cargo.toml:425`
- [x] The archive and its BLAKE3 digest as the standing decision says, with an `nvs.toml` anywhere in it refused (`rule:packaging/a-package-is-its-digest`). `crates/nvs-cli/src/package/pack.rs:1`, `Cargo.toml:425`
- [x] The file set: `src/**/*.nvs`, `package.toml`, the root license and readme files and the `files` list, with a missing license, a missing listed file, a listed `.nvs` and a listed default file refused (`rule:packaging/a-package-carries-only-its-source-and-what-it-lists`). `crates/nvs-cli/src/package/pack.rs:1`
- [x] The 16 MiB and 5,000-file limits as format constants, and the path rules: NFC, 160 bytes, case-only twins, Windows reserved names and characters, a trailing dot or space. `crates/nvs-cli/src/package/pack.rs:1`
- [x] `nvs pkg pack` in the group, printing every file with its size and the total. `crates/nvs-cli/src/package/mod.rs:1`
- [x] `rule:packaging/a-package-is-its-digest`'s fragment states the archive format. `docs/rules/packaging/a-package-is-its-digest.md:1`
- [x] Tests: a pinned digest for a fixture directory, the same bytes after its files are touched and reordered, a `tests/` file and a `docs/` markdown file left out, a symlink and an `nvs.toml` refused, each file-set refusal, each path rule, and one byte over each limit refused. `crates/nvs-cli/tests/package_client.rs:1`

### 3. the index is read, resolution takes the highest minimum, and the lock holds every digest

**Does:** the client reads a fixture registry on disk, resolves the graph and writes `package.lock`.

- [ ] The index reader, its schema written into `rule:packaging/a-package-is-its-digest`'s fragment, and a fixture registry built at test time. `crates/nvs-cli/src/package/index.rs:1`, `crates/nvs-cli/tests/fixtures/registry/README.md:1`
- [ ] The root file `registry.json`: its file hosts tried in order, the next one on a network failure or a digest mismatch, and the last root file kept under `<data>/registry/` (`rule:packaging/registry-hosting-is-configuration`). `crates/nvs-cli/src/package/index.rs:1`, `crates/nvs-config/src/data.rs:163`
- [ ] Tests: a fixture registry whose first file host is a missing directory reads from the second, and a registry URL that does not answer reads through the kept root file. `crates/nvs-cli/tests/package_client.rs:1`
- [ ] Minimal version selection across majors, with the crossing reported (`rule:packaging/resolution-takes-the-highest-minimum`). `crates/nvs-cli/src/package/resolve.rs:1`
- [ ] The lock, `--locked` that fetches nothing outside it, and a digest mismatch as a hard error naming the package (`rule:packaging/the-lockfile-holds-every-digest`). `crates/nvs-cli/src/package/lock.rs:1`
- [ ] Tests: a graph across majors, a `--locked` run with a missing entry, and an edited digest. `crates/nvs-cli/tests/package_client.rs:1`

### 7. a root git dependency is cloned inside fixed limits and packed, and one below the root is refused

**Does:** a root git dependency is cloned at its exact commit through a checked connection and packed
as stage 2 packs a directory, and a git dependency anywhere below the root is refused.

- [ ] `gix` in the workspace `Cargo.toml` with its default features off, and `cargo deny` green. `Cargo.toml:1`
- [ ] The HTTPS clone at an exact commit, inside the size and time limits and behind the outbound address check, as the standing decision says. `crates/nvs-cli/src/package/git.rs:1`, `crates/nvs-config/src/capability.rs:263`
- [ ] A git dependency below the root is a hard error naming the package that declared it (`rule:packaging/a-git-dependency-is-root-only`). `crates/nvs-cli/src/package/resolve.rs:1`
- [ ] Tests, offline as the standing decision says: an internal host refused, a slow listener stopped by the time limit, the exact commit from a `file:` remote, an over-size repository stopped, and a transitive git dependency refused. `crates/nvs-cli/tests/package_git.rs:1`

### 4. `nvs pkg fetch` and `nvs pkg vendor` write the tree, `vendor/packages.nvs` and a grant line per package

**Does:** the resolved graph lands under `vendor/` with one generated autoload file and one grant line
for every package, an empty one included.

- [ ] `vendor/packages.nvs`, generated from the lock and committed (`rule:packaging/a-fetched-package-is-an-autoload-line`). `crates/nvs-cli/src/package/fetch.rs:1`, `crates/nvs-hir/src/autoload.rs:259`
- [ ] `nvs pkg add` and `nvs pkg fetch` write a line into `[grants]` in the application's `package.toml` for every package in the graph, direct and transitive, with nothing granted until a person writes it and an edited line left as it is (`rule:security/package-authority-is-granted-one-line-at-a-time`, `rule:packaging/package-toml-is-a-tool-input`). `crates/nvs-cli/src/package/fetch.rs:1`, `crates/nvs-cli/src/package/manifest.rs:1`
- [ ] Tests: a fetched package compiles only with its grant line, an edited line survives a second fetch, and an effectful top-level statement produces no effect from `fetch`, `build` or `vendor` (`rule:packaging/nothing-runs-before-the-program`). `crates/nvs-cli/tests/package_client.rs:1`

### 5. `nvs pkg add`, `nvs pkg update` and `nvs pkg outdated` select only what is released and print what a change brings

**Does:** a pending version is fetched only by its exact version, a retracted, unlisted or withheld
one is handled as its rule says, and every change prints its authority before it lands.

- [ ] A pending version is never selected by `add` or `update`, and `nvs pkg add acme/http@1.4.0` fetches it (`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`). `crates/nvs-cli/src/package/resolve.rs:1`
- [ ] An unlisted version still resolves and fetches, a withheld one fails naming its advisory, and a retracted one is never newly selected (`rule:packaging/an-owner-may-unlist`, `rule:packaging/the-operator-only-withholds`, `rule:packaging/a-known-bad-version-is-retracted-not-deleted`). `crates/nvs-cli/src/package/resolve.rs:1`
- [ ] The output: every capability requested by the package and its subgraph, capability changes since the lock, the transitive count, owner and repository changes, and the majors crossed with who asked for the older one. `crates/nvs-cli/src/package/report.rs:1`
- [ ] Tests: one fixture registry entry per version state, and the report of an update that adds a capability. `crates/nvs-cli/tests/package_client.rs:1`

### 9. an external download is fetched, checked and placed, and `nvs pkg download` and `nvs pkg provide` work

**Does:** a `[[download]]` entry is downloaded with its package, checked against its size and hash and
placed at its path, and the two commands write an entry and fill the cache.

**Split:** after the session lands `download.rs`, one writing subagent writes the loopback tests.

- [ ] The `[[download]]` reader and its form checks, shared with the registry's build (`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`). `crates/nvs-cli/src/package/manifest.rs:1`
- [ ] The download: HTTPS only, each redirect hop checked and at most five, the read stopped one byte past the size, the hash checked before the rename, the next URL on a mismatch, and the SHA-256 cache under the ownership check. `crates/nvs-cli/src/package/download.rs:1`, `crates/nvs-cli/src/cache.rs:1258`
- [ ] `nvs pkg fetch` and `nvs pkg vendor` place each file at its path, and `nvs pkg add` and `nvs pkg update` print each file's host, size and license, the total, and a host new since the lock, which records the hosts. `crates/nvs-cli/src/package/fetch.rs:1`, `crates/nvs-cli/src/package/report.rs:1`, `crates/nvs-cli/src/package/lock.rs:1`
- [ ] `nvs pkg download <url> --to <path>` writes an entry, `nvs pkg provide <file>` fills the cache only for a matching entry, and a package reads its downloaded file through a relative path. `crates/nvs-cli/src/package/download.rs:1`
- [ ] The read check: `nvs pkg fetch` records each package's file set, and a read-only `Core` door called from a package's namespace passes for a file in that set with no grant line (`rule:packaging/a-package-carries-only-its-source-and-what-it-lists`). `crates/nvs-config/src/grants.rs:1`, `crates/nvs-cli/src/package/fetch.rs:1`
- [ ] Tests: a package reads its own listed file and its downloaded file with an empty grant line, and is refused a write to it, a read through `..` or a symlink out of the set, an unlisted file placed in its folder, and another package's file. `crates/nvs-cli/tests/package_client.rs:1`
- [ ] Tests against loopback TLS listeners: a wrong hash, a body past its size, a redirect to an internal address, an `http:` URL, a dead first URL served by the second, a provided file, and an archive that stays unpacked. `crates/nvs-cli/tests/package_download.rs:1`

### 6. `package.toml`, `nvs pkg add` and `nvs pkg fetch` have their feature proofs

**Does:** the first three features have their chapter headings, about pages, examples and attacks.

**Split:** after the headings land, one writing subagent writes each feature's about page and examples,
one feature after the other, then a second writes the attacks, each held to § *Text an end user reads*
in `AGENTS.md`.

- [ ] The headings `# package.toml`, `# nvs pkg add` and `# nvs pkg fetch and nvs pkg vendor` in the chapter. `docs/reference/tools/35-packages.md:1`
- [ ] The about pages and examples. `docs/examples/tools/packages/package-toml/about.md:1`, `docs/examples/tools/packages/nvs-pkg-add/about.md:1`, `docs/examples/tools/packages/nvs-pkg-fetch-and-nvs-pkg-vendor/about.md:1`
- [ ] The attacks: a vendor answered by a second registry, and a transitive git dependency. `tests/hostile/tools/packages/package-toml/01-a-vendor-answered-by-a-second-registry.nvs:1`, `tests/hostile/tools/packages/nvs-pkg-fetch-and-nvs-pkg-vendor/01-a-transitive-git-dependency.nvs:1`
- [ ] `bun nv proofs --id` owes nothing for `tools:packages/package-toml`, `tools:packages/nvs-pkg-add` and `tools:packages/nvs-pkg-fetch-and-nvs-pkg-vendor`. `docs/reference/tools/35-packages.md:1`

### 8. the other client commands have their feature proofs, and the rules are shipped

**Does:** the last four features have their proofs, and every rule this goal owns is `shipped`.

**Split:** as stage 6: about pages and examples one feature after the other, then the attacks.

- [ ] The headings `# nvs pkg update and nvs pkg outdated`, `# package.lock and --locked`, `# nvs pkg pack` and `# nvs pkg download and nvs pkg provide`. `docs/reference/tools/35-packages.md:1`
- [ ] The about pages and examples. `docs/examples/tools/packages/nvs-pkg-update-and-nvs-pkg-outdated/about.md:1`, `docs/examples/tools/packages/package-lock-and-locked/about.md:1`, `docs/examples/tools/packages/nvs-pkg-pack/about.md:1`, `docs/examples/tools/packages/nvs-pkg-download-and-nvs-pkg-provide/about.md:1`
- [ ] The attacks: a lock whose digest was edited, and a download that redirects to an internal address. `tests/hostile/tools/packages/package-lock-and-locked/01-an-edited-digest.nvs:1`, `tests/hostile/tools/packages/nvs-pkg-download-and-nvs-pkg-provide/01-a-redirect-to-an-internal-address.nvs:1`
- [ ] `bun nv proofs --id` owes nothing for the four. `docs/reference/tools/35-packages.md:1`
- [ ] `rule:packaging/package-toml-is-a-tool-input` and `rule:packaging/a-package-name-is-vendor-slash-name` are `shipped`. `data/rules/packaging/package-toml-is-a-tool-input.json:3`, `data/rules/packaging/a-package-name-is-vendor-slash-name.json:3`
- [ ] `rule:packaging/a-vendor-resolves-from-one-registry` and `rule:packaging/a-package-is-its-digest` are `shipped`. `data/rules/packaging/a-vendor-resolves-from-one-registry.json:3`, `data/rules/packaging/a-package-is-its-digest.json:3`
- [ ] `rule:packaging/resolution-takes-the-highest-minimum` and `rule:packaging/the-lockfile-holds-every-digest` are `shipped`. `data/rules/packaging/resolution-takes-the-highest-minimum.json:3`, `data/rules/packaging/the-lockfile-holds-every-digest.json:3`
- [ ] `rule:packaging/a-git-dependency-is-root-only`, `rule:packaging/a-fetched-package-is-an-autoload-line` and `rule:packaging/nothing-runs-before-the-program` are `shipped`. `data/rules/packaging/a-git-dependency-is-root-only.json:3`, `data/rules/packaging/a-fetched-package-is-an-autoload-line.json:3`, `data/rules/packaging/nothing-runs-before-the-program.json:3`
- [ ] `rule:packaging/a-package-carries-only-its-source-and-what-it-lists` and `rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size` are `shipped`, and `bun nv owners --closes package-client` passes. `data/rules/packaging/a-package-carries-only-its-source-and-what-it-lists.json:3`, `data/rules/packaging/an-external-file-is-pinned-by-url-sha256-and-size.json:3`

### 10. the goal is verified once, and every failure is fixed

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

Stages 1 and 2 are done; the `api-compat` and `namespace-grants` gates hold.

`crates/nvs-cli/src/package/pack.rs`: `file_set` builds the allowlist (`pack.rs:188`), `check_path`
the path rules (`pack.rs:358`), `MAX_BYTES` and `MAX_FILES` the limits (`pack.rs:37`), and `write`
streams the `ustar` bytes through BLAKE3 into any writer (`pack.rs:451`). `nvs pkg pack [dir]
[--out FILE]` (`mod.rs:86`) writes `<vendor>-<name>-<version>.tar` through a `.partial` file. A
pack refusal is a plain `error:` line, since it names a file and not a line of `package.toml`.
A symlink anywhere the set is read from is refused. The `tar` row is `Cargo.toml:479`, and
`THIRD-PARTY-LICENSES.txt` is regenerated. The digest rule's fragment states the format. The
digest and file-set rules stay `designed` until the fetch side exists; stage 8 ships them.

`mod.rs` keeps `#[allow(dead_code)]` on `manifest` until `fetch` and `add` read the rest of it.

The path rules and limits are unit tests in `pack.rs`, and the pinned digest and file-set
refusals are in `crates/nvs-cli/tests/package_client.rs`; all pass. No test should go red.

The sweep's reds are fixed at `HEAD`, and the sweep repeated them unchanged. The three nightly Miri jobs of run 40474302703 ran
on `55b03e97a`. `22f1e4ec4` skips the `secret` span test under Miri, `c33a8b8f7` counts the spool
wakes against files opened, and `e461612ce` and `c33a8b8f7` skip the two slow spool tests.
`env-secret-proofs` exits 0 when run, and no goal or floor names that check any more. Its log
under `.looper/loops/main/checks/` dates from 22 October, so the sweep reads a stale check list.

### Next

**Stage 3: the index, resolution and the lock**, files `crates/nvs-cli/src/package/index.rs`,
`resolve.rs`, `lock.rs`, `mod.rs`, `docs/rules/packaging/a-package-is-its-digest.md` (the index
schema), `crates/nvs-cli/tests/fixtures/registry/README.md` and `package_client.rs`. Split the
index and root file from resolution and the lock if it runs past one session.

- [ ] The index reader and its schema in the digest rule's fragment; the test fixture registry built from package directories through `pack::file_set` and `pack::write`. `crates/nvs-cli/src/package/index.rs:1`, `crates/nvs-cli/src/package/pack.rs:188`
- [ ] `registry.json`'s file hosts in order, the next on a failure or digest mismatch, and the last root file kept under `<data>/registry/`. `crates/nvs-config/src/data.rs:163`
- [ ] Minimal version selection across majors with the crossing reported. `crates/nvs-cli/src/package/resolve.rs:1`, `crates/nvs-cli/src/package/manifest.rs:236`
- [ ] The lock, `--locked`, and a digest mismatch as a hard error naming the package. `crates/nvs-cli/src/package/lock.rs:1`
- [ ] Tests as stage 3 lists them. `crates/nvs-cli/tests/package_client.rs:1`

### Backlog

- Stage 7 (the git dependency) after stage 3; it packs a clone through `pack::file_set`. `crates/nvs-cli/src/package/pack.rs:188`
- The fetch stops one byte past the largest archive `MAX_BYTES` and `MAX_FILES` allow, before the digest is checked (stage 4). `crates/nvs-cli/src/package/pack.rs:37`
- Verifying each archive's inclusion in the transparency log is goal `registry-log`'s (`docs/goals/registry-log.md`).
- `nvs pkg audit` and the advisory feed are goal `package-audit`'s (`docs/goals/package-audit.md`).
- The editor reads the `[grants]` table: the grants hover follows a namespace's line, and the
  editor shows `E0658`, `E0660` and `E0661` as `nvs check` does. Handed over from goal
  `namespace-grants`. `data/gaps/nvs-lsp/the-grants-hover-reads-the-entrys-tree-and-every-grant.json`,
  `crates/nvs-lsp/src/document.rs:771`
