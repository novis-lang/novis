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
    status: done
    context: [docs/rules/packaging/resolution-takes-the-highest-minimum.md, docs/rules/packaging/the-lockfile-holds-every-digest.md]
    extra:
      context:
        rules: [packaging/resolution-takes-the-highest-minimum, packaging/the-lockfile-holds-every-digest, packaging/a-vendor-resolves-from-one-registry, packaging/registry-hosting-is-configuration]
  - id: 7
    title: "a root git dependency is cloned inside fixed limits and packed, and one below the root is refused"
    status: done
    context: [crates/nvs-config/src/capability.rs, crates/nvs-stdlib/src/http/transport.rs, crates/nvs-cli/src/package/resolve.rs, docs/rules/packaging/a-git-dependency-is-root-only.md]
    extra:
      context:
        rules: [packaging/a-git-dependency-is-root-only, packaging/a-package-is-its-digest, security/net-address-policy]
  - id: 4
    title: "`nvs pkg fetch` and `nvs pkg vendor` write the tree, `vendor/packages.nvs` and a grant line per package"
    status: done
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

**Stage 3's tests are unit tests, and stage 4 adds the binary's.** No command resolves or locks
until `nvs pkg fetch` exists, so the index, resolution and lock cases run in `index.rs`,
`resolve.rs` and `lock.rs` against the fixture registry `src/package/fixture.rs` packs at test time.
`--locked` is `Lock::check(graph, true)`; stage 4 puts the flag on `fetch` and `vendor` and shows a
missing entry and an edited digest through the binary in `package_client.rs`. Why: simplicity, and
no test-only subcommand. A version visited but not selected is read for its dependencies whatever
its state, as minimal version selection walks every version asked for; only the selected
version's state is judged. A retracted selection is allowed exactly when the lock already has that
version. Index and root files use `deny_unknown_fields` with `"schema": 1`, so a new key is a schema
bump the client names.

**A git dependency is read from its commit's tree, and only the archive's paths are written.**
A `file:` remote is opened in place through `gix::open_opts` with isolated options, never
through gix's `file://` transport, which runs `git upload-pack`; so nothing of the repository's
or the user's configuration is read and no program runs. An `https:` remote is fetched into a
bare repository in a scratch folder, and both then take the same walk: `package.toml` from the
commit's root, and then the root files, `src/` and the `files` list written out and packed through
`pack::file_set`. A symlink or submodule at one of those paths is refused. The size limit counts
the bytes of every object read, and the time limit is five minutes (`git::LIMITS`), which fetches
512 MiB at about 2 MiB/s. The archive is kept in `Registries` by slug (`Registries::cloned`), the
scratch folder is removed on every way out, and the lock's `source` is `git+<url>`. The pinned
version is the one the commit's `package.toml` declares, and it stays selected whatever a registry
package asks for. Why: security first, because a hostile repository's other paths are never written
and its hooks and filters never run. Simplicity, because one walk serves both remotes. Memory: one
archive per git dependency while the client runs. The `file:` cases are unit tests in `git.rs` and
`resolve.rs`, as stage 3's are, and so are the loopback cases, until a command clones; stage 4
shows a clone through the binary in `tests/package_git.rs`.

**The clone's address check is the operator's written exception alone** (stage 7). `git::Outbound`
refuses every address `nvs_config::capability::denied_by_default` names, except what the
application's written `[capabilities.net] internal` key names. It does not take a CLI run's
default loopback exception, so a repository URL on loopback or a private network is refused until
the operator writes the line the fault prints. Stage 4 builds it from the tree's `[capabilities]`
and passes it through `Registries::reaching` (`resolve.rs:148`). Every request is sent by
`nvs_stdlib::tool_stream`, which judges each resolved address, and each redirect hop's, before
anything connects, and a name that resolves to one refused address among others is refused whole.
A `401` is an ordinary failed status, so `gix` starts no credential helper. Why: security first,
because a package command reaches the network on a URL somebody else wrote. Memory: one `POST`
body, a few KiB, while it is sent.

**`fetch` keeps a folder that already has the locked files, and `vendor` writes every folder again**
(stage 4). Both resolve, check the lock and take `--locked`. A package folder is
`vendor/<vendor>/<name>/`, the slug as a path, so its folder does not move when its version does.
`fetch` repacks a folder in place and skips its archive when the digest is the locked one, so a
second fetch reads no archive; `vendor` downloads and replaces every folder, for a tree a person
changed by hand. Only the folders of packages the old lock had are removed, because `vendor/` may
hold hand-vendored code. The grant line is the subtree form, `'Acme\Http\*' = []`, with what the
package requests as its comment, because a package's code is its namespace and every one below
it; a package with a key naming its namespace in either form gets no line, which is how an edited
line is never rewritten. The lines are added to the file's text, so comments stay. A git
dependency's clone is written under the data folder's `tmp/`. Why: simplicity, one pipeline for
both commands; security, every archive path is checked before it is written. Memory: one archive
at a time.

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

- [x] The index reader, its schema written into `rule:packaging/a-package-is-its-digest`'s fragment, and a fixture registry built at test time. `crates/nvs-cli/src/package/index.rs:1`, `crates/nvs-cli/tests/fixtures/registry/README.md:1`
- [x] The root file `registry.json`: its file hosts tried in order, the next one on a network failure or a digest mismatch, and the last root file kept under `<data>/registry/` (`rule:packaging/registry-hosting-is-configuration`). `crates/nvs-cli/src/package/index.rs:1`, `crates/nvs-config/src/data.rs:163`
- [x] Tests: a fixture registry whose first file host is a missing directory reads from the second, and a registry URL that does not answer reads through the kept root file. `crates/nvs-cli/src/package/index.rs:1`
- [x] Minimal version selection across majors, with the crossing reported (`rule:packaging/resolution-takes-the-highest-minimum`). `crates/nvs-cli/src/package/resolve.rs:1`
- [x] The lock, `--locked` that fetches nothing outside it, and a digest mismatch as a hard error naming the package (`rule:packaging/the-lockfile-holds-every-digest`). `crates/nvs-cli/src/package/lock.rs:1`
- [x] Tests: a graph across majors, a `--locked` run with a missing entry, and an edited digest. `crates/nvs-cli/src/package/lock.rs:1`, `crates/nvs-cli/src/package/resolve.rs:1`

### 7. a root git dependency is cloned inside fixed limits and packed, and one below the root is refused

**Does:** a root git dependency is cloned at its exact commit through a checked connection and packed
as stage 2 packs a directory, and a git dependency anywhere below the root is refused.

- [x] `gix` in the workspace `Cargo.toml` with its default features off, and `cargo deny` green. `Cargo.toml:1`
- [x] The HTTPS clone at an exact commit, inside the size and time limits and behind the outbound address check, as the standing decision says. `crates/nvs-cli/src/package/git.rs:1`, `crates/nvs-config/src/capability.rs:263`
  - [x] A `file:` remote opened in place with isolated options, the commit's tree written for the archive's paths alone inside both limits, and packed. `crates/nvs-cli/src/package/git.rs:66`
  - [x] A stdlib entry for the tool that resolves a host, refuses an internal address, and streams a reply under a byte ceiling and a deadline. `crates/nvs-stdlib/src/http.rs:4070`
  - [x] `gix_transport`'s `Http` trait over that entry, and `connect_http` into a bare repository in the scratch folder. `crates/nvs-cli/src/package/git.rs:514`
  - [x] The fetch of the one commit at depth 1, by its hash as the refspec, then the same tree walk. `crates/nvs-cli/src/package/git.rs:185`
- [x] A git dependency below the root is a hard error naming the package that declared it (`rule:packaging/a-git-dependency-is-root-only`). `crates/nvs-cli/src/package/resolve.rs:1`
- [x] Tests, offline as the standing decision says: an internal host refused, a slow listener stopped by the time limit, the exact commit from a `file:` remote, an over-size repository stopped, and a transitive git dependency refused. `crates/nvs-cli/tests/package_git.rs:1`
  - [x] The exact commit, a missing commit and the size limit against a `file:` remote, as unit tests. `crates/nvs-cli/src/package/git.rs:353`
  - [x] A root git dependency resolved, and a transitive one refused, as unit tests. `crates/nvs-cli/src/package/resolve.rs:516`
  - [x] An internal host refused and a silent listener stopped by the time limit, on loopback, as unit tests. `crates/nvs-cli/src/package/git.rs:846`
  - [x] A loopback HTTPS host serves a repository over protocol v2, and the clone fetches the commit and packs it to the digest its `file:` clone has. `crates/nvs-cli/src/package/git.rs:931`

### 4. `nvs pkg fetch` and `nvs pkg vendor` write the tree, `vendor/packages.nvs` and a grant line per package

**Does:** the resolved graph lands under `vendor/` with one generated autoload file and one grant line
for every package, an empty one included.

- [x] `vendor/packages.nvs`, generated from the lock and committed (`rule:packaging/a-fetched-package-is-an-autoload-line`). `crates/nvs-cli/src/package/fetch.rs:1`, `crates/nvs-hir/src/autoload.rs:259`
  - [x] `fetch` reads the manifest, builds `Registries` reaching the tree's `[capabilities]`, resolves, checks and writes the lock
  - [x] each archive is unpacked under `vendor/<vendor>/<name>/` through path and limit checks
  - [x] `vendor/packages.nvs` is written from the lock
- [x] `nvs pkg add` and `nvs pkg fetch` write a line into `[grants]` in the application's `package.toml` for every package in the graph, direct and transitive, with nothing granted until a person writes it and an edited line left as it is (`rule:security/package-authority-is-granted-one-line-at-a-time`, `rule:packaging/package-toml-is-a-tool-input`). `fetch::add_grants` is the writer, and stage 5's `add` calls it. `crates/nvs-cli/src/package/fetch.rs:1`, `crates/nvs-cli/src/package/manifest.rs:1`
- [x] Tests: a fetched package compiles only with its grant line, an edited line survives a second fetch, and an effectful top-level statement produces no effect from `fetch` or `vendor` (`rule:packaging/nothing-runs-before-the-program`). `nvs build`'s half is stage 6's attack. `crates/nvs-cli/tests/package_fetch.rs:1`
- [x] `--locked` on `nvs pkg fetch` and `nvs pkg vendor` through `Lock::check(graph, true)`, and the binary's tests of a locked run with a missing entry and an edited digest. `crates/nvs-cli/src/package/lock.rs:140`, `crates/nvs-cli/tests/package_fetch.rs:1`
  - [x] a git dependency through the binary. `crates/nvs-cli/tests/package_git.rs:1`

### 5. `nvs pkg add`, `nvs pkg update` and `nvs pkg outdated` select only what is released and print what a change brings

**Does:** a pending version is fetched only by its exact version, a retracted, unlisted or withheld
one is handled as its rule says, and every change prints its authority before it lands.

- [ ] A pending version is never selected by `add` or `update`, and `nvs pkg add acme/http@1.4.0` fetches it (`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`). `crates/nvs-cli/src/package/resolve.rs:1`
- [ ] An unlisted version still resolves and fetches, a withheld one fails naming its advisory, and a retracted one is never newly selected (`rule:packaging/an-owner-may-unlist`, `rule:packaging/the-operator-only-withholds`, `rule:packaging/a-known-bad-version-is-retracted-not-deleted`). `crates/nvs-cli/src/package/resolve.rs:1`
- [ ] The output: every capability requested by the package and its subgraph, capability changes since the lock, the transitive count, owner and repository changes, and the majors crossed with who asked for the older one. `crates/nvs-cli/src/package/report.rs:1`
- [ ] Tests: one fixture registry entry per version state, and the report of an update that adds a capability. `crates/nvs-cli/tests/package_fetch.rs:1`

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
- [ ] The attacks: a vendor answered by a second registry, a transitive git dependency, and a package whose top-level statement has an effect, which `nvs build` does not run. Add `// covers: tools:packages/nvs-pkg-fetch-and-nvs-pkg-vendor` to `crates/nvs-cli/tests/package_fetch.rs` once the heading exists. `tests/hostile/tools/packages/package-toml/01-a-vendor-answered-by-a-second-registry.nvs:1`, `tests/hostile/tools/packages/nvs-pkg-fetch-and-nvs-pkg-vendor/01-a-transitive-git-dependency.nvs:1`
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

Stages 1, 2, 3, 7 and 4 are done. `nvs pkg fetch` and `nvs pkg vendor` (`crates/nvs-cli/src/package/fetch.rs:93`) resolve,
check and write `package.lock`, unpack each package under `vendor/<vendor>/<name>/`
(`fetch::unpack`), write `vendor/packages.nvs` (`fetch::autoloads`), and add one grant line per new
package (`fetch::add_grants`, `fetch::with_grants`). `--locked` refuses a missing lock, a missing
entry and an edited digest. The tree's `[capabilities]` reach the clone through
`config::written_capabilities` (`crates/nvs-cli/src/config.rs:865`). The `dead_code` allowances on the client modules
are gone; three fields keep one each, for serde or for stage 9's `[[download]]`.

Tests: 3 unit cases in `fetch.rs`, 3 binary cases in `tests/package_fetch.rs`, and 1 in
`tests/package_git.rs`, all passing. The rule fragments' **Not on disk** lines say what `fetch`
and `vendor` ship. No test should go red.

The sweep's reds were stale: the `env-secret-proofs` log exits 0, and Miri run 40474302703 ran on
`55b03e97a`, before the fixes `22f1e4ec4` and `e461612ce`.

### Next

**Stage 5: `nvs pkg add`, `nvs pkg update` and `nvs pkg outdated`**, in `resolve.rs`, a new
`crates/nvs-cli/src/package/report.rs`, the `Command` enum (`crates/nvs-cli/src/package/mod.rs:41`) and
`tests/package_fetch.rs`. Write the stage's checkpoints into the goal file first.

- [ ] `add <slug>[@<version>]` writes the `[dependencies]` line into the file's text as `with_grants` does (`crates/nvs-cli/src/package/fetch.rs:373`), then runs the fetch pipeline; a pending version is selected only by its exact version (`crates/nvs-cli/src/package/resolve.rs:436`).
- [ ] `update` resolves without the lock's retracted allowance and writes the lock; `outdated` lists each package's newest released version and changes nothing. `crates/nvs-cli/src/package/resolve.rs:218`
- [ ] `report.rs`: the capabilities the package and its subgraph request, the changes since the old lock (capabilities, owners, repository), the transitive count, and each crossing (`crates/nvs-cli/src/package/resolve.rs:76`).
- [ ] Tests: one fixture registry entry per version state, and an update that adds a capability, in `tests/package_fetch.rs` with its `Tree` helper (`crates/nvs-cli/tests/package_fetch.rs:16`).

### Backlog

- An `https:` file host is read through `nvs_stdlib::tool_get`, whose reply ceiling is 1 MiB, so an archive over 1 MiB fails there. Stage 9 reads archives with the ceiling at `pack::MAX_ARCHIVE_BYTES`, through the checked connection the download uses. `crates/nvs-cli/src/package/index.rs:137`, `crates/nvs-stdlib/src/http.rs:3933`
- Verifying each archive's inclusion in the transparency log is goal `registry-log`'s (`docs/goals/registry-log.md`).
- `nvs pkg audit` and the advisory feed are goal `package-audit`'s (`docs/goals/package-audit.md`).
- The editor reads the `[grants]` table: the grants hover follows a namespace's line, and the
  editor shows `E0658`, `E0660` and `E0661` as `nvs check` does. Handed over from goal
  `namespace-grants`. `data/gaps/nvs-lsp/the-grants-hover-reads-the-entrys-tree-and-every-grant.json`,
  `crates/nvs-lsp/src/document.rs:771`
