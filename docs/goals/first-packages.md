---
looper: 1
title: "`Novis\\Rest` is the first package, and a program's version comes from `package.toml`"
summary: "nvs build --openapi writes package.toml's version as info.version, a Windows bundle carries the program's own version information and icon, Novis\\Rest gives OAuth grants, discovery and DPoP as a package in this repository that passes nvs pkg publish --dry-run, the gap records this goal owns are closed, and the checks below pass."
context: [crates/nvs-cli/src/package/manifest.rs]
stages:
  - id: 1
    title: "`package.toml`'s version is the API document's `info.version`"
    status: done
    context: [crates/nvs-cli/src/openapi.rs, crates/nvs-cli/tests/openapi.rs]
    extra:
      context:
        rules: [packaging/package-toml-is-a-tool-input, routing/api-document-is-a-deterministic-build-artifact]
  - id: 2
    title: a Windows bundle carries the program's own version information and icon
    status: done
    context: [crates/nvs-cli/src/bundle.rs, crates/nvs-cli/src/bundle/resource.rs, crates/nvs-cli/build/winres.rs, crates/nvs-cli/tests/windows_resource.rs, docs/rules/packaging/the-windows-binary-says-what-it-is.md]
    extra:
      context:
        rules: [packaging/the-windows-binary-says-what-it-is, packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host, packaging/package-toml-is-a-tool-input, packaging/a-c-dependency-answers-two-questions]
  - id: 3
    title: "`Novis\\Rest` gives OAuth grants, discovery and DPoP"
    status: done
    context: [packages/rest/package.toml, crates/nvs-stdlib/src/jwt.rs, crates/nvs-stdlib/src/session.rs, docs/rules/core-api/tier-placement.md]
    extra:
      context:
        rules: [core-api/tier-placement, packaging/a-package-name-is-vendor-slash-name, security/jws-issued-subset]
  - id: 4
    title: "`Novis\\Rest` has its tests and its proofs, and passes `nvs pkg publish --dry-run`"
    status: done
    context: [packages/rest/package.toml, tools/nv/proofs/roster.ts, docs/reference/tools/35-packages.md]
    extra:
      context:
        rules: [packaging/the-registry-builds-the-archive-from-git, packaging/a-package-is-its-digest]
  - id: 5
    title: the goal is verified once, and every failure is fixed
    status: done
    gate: true
checks:
  - id: rest-tests
    stage: 4
    name: Novis\Rest's own tests pass
    run: [cargo, run, -q, --bin, nvs, --, test, packages/rest]
  - id: rest-dry-run
    stage: 4
    name: Novis\Rest packs as the registry would build it
    run: [cargo, run, -q, --bin, nvs, --, pkg, publish, --dry-run]
    cwd: packages/rest
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, first-packages]
    reach: true
    floor: false
---

# `Novis\Rest` is the first package, and a program's version comes from `package.toml`

## Why

`package.toml` is the first place anything in the tree says what a program is published as. Two
outputs have been waiting for that: the generated API document carries a fixed `0.0.0` as its
`info.version` (`crates/nvs-cli/src/openapi.rs:79`), and an `nvs build --compile` bundle on Windows
carries Novis's own icon and version information because it is a copy of the host. This goal reads
both from the manifest. It also writes the first package the registry carries, `Novis\Rest`, the REST
and OAuth client that `rule:core-api/tier-placement` keeps out of `Core`. Everything it needs from
`Core` has landed.

## Standing decisions

**`nvs build --openapi` reads the `package.toml` the CLI finds** by walking up from the working
directory, as every package command does. With no manifest, `info.version` stays `0.0.0`. The rest of
the document does not change.

**A bundle's version information is the program's.** `FileVersion` and `ProductVersion` are the
manifest's `version`, `ProductName` and `FileDescription` its package name, and the icon is the file
the manifest's `[bundle]` table names as `icon`. With no manifest, the bundle keeps the host's resource
as today. The resource is rewritten in the copy by an existing crate that edits a PE file's resources,
chosen under `rule:packaging/a-c-dependency-answers-two-questions`. If none passes, the rewrite is
written here and argued in `rule:packaging/the-windows-binary-says-what-it-is`.

**The PE-resource crate: `editpe` if it passes, else the tree's own layouts.** An `Explore` subagent
checks `editpe` and any other pure-Rust crate that writes PE resources against `deny.toml` and the two
questions. The crate must run on every build host, so a crate that calls the Windows
`UpdateResource` API is out. If none passes, the default is to build the new resource with the layouts
`crates/nvs-cli/build/winres.rs` already writes by hand, and to rewrite the copy's `.rsrc` section with
the `object` crate the tree already has (`Cargo.toml:112`). Why: security and simplicity favour no new
supply chain for four fixed layouts the tree already writes and tests.

**The bundle's resource: the in-tree rewrite, in a new section, and `[bundle]` added here.** No
PE-resource crate is in the lock, so the default above holds: `crates/nvs-cli/src/bundle/resource.rs`
reads the host's tree with `object`, builds the icon and version with `build/winres.rs` (included by
`#[path]`), and appends a new `.rsrc` section rather than resizing the old one, since `.reloc` follows
it. Goal `package-client` was reached without a `[bundle]` table, so this goal added it to
`package/manifest.rs`, allowed in an application's file too; there is no duplicate. With a manifest
but no name the bundle is named after its file stem, and with no version it is `0.0.0`, as
`info.version` is.

**`Novis\Rest` lives in this repository under `packages/rest/`**, so CI tests it with the toolchain it
targets. It is published by the project account from that directory, which `nvs pkg publish` sends with the
commit (goal `registry-service`). It is about seven types over OAuth grants (client credentials,
authorization code with PKCE, refresh), discovery and DPoP: `Client`, `Token`, `ClientCredentials`,
`AuthorizationCode`, `Discovery`, `DPoP` and `Session`. A user's refresh token goes in the session
through `Core\Session::setSecret`, because a cache tier may evict it. Encrypted ID tokens and
generating a client from an OpenAPI document are out of scope.

**Its proofs are what `bun nv proofs` lists for it.** If the roster has no entry kind for a package's
class, stage 4 adds one for `packages/`, in the shape of a built-in component's.

**A package's roster unit is its class, not each method.** The roster kind `package` has one entry
per type under `packages/<dir>/src/`, at `package/<Namespace-Class>` in the shared example, attack
and bench trees (`package/Novis-Rest-Client`), anchored at the declaration. It owes what a `Core`
member does without the Rust test: two tests, three examples, one attack, one bench, `about.md`, and
a comment above the class as its help. Its tests are the package's `#[Test]` methods with a
`// covers:` marker, since `packages/` is a marker root. Why: per method would be about thirty
features and 180 files for a package that is not in the binary, whose page is per class; simplicity
favours the class. `tools/nv/proofs/roster.ts`'s header is the home.

**A package's reference is its class pages, not a heading in `35-packages.md`.** Each `#` heading
of that chapter is a tool feature that owes its own proofs, and `Novis\Rest`'s seven class pages
already say what it does, so the chapter gains no `Novis\Rest` heading. Why: simplicity, since one
home per fact.

**Lanes.** Stage 1 (`openapi.rs`) and stage 2 (`bundle.rs`, `build/winres.rs`) touch disjoint files
and can run in either order or as side goals. Stage 3 creates `packages/rest/` alone. Stage 4 writes
`tools/nv/proofs/roster.ts` and `docs/reference/tools/35-packages.md`, where every package goal adds a
heading, so it merges serially with them. If stage 2 picks a crate, it edits `Cargo.toml`, which any
goal adding a dependency also edits.

**Gates.** Stage 1 needs `version` read: `crates/nvs-cli/src/package/manifest.rs` exists (goal
`api-compat` creates it reading `version` alone). Stage 2 also needs the name and the `[bundle]` table:
goal `package-client`'s stage 1 is `done` in `docs/goals/package-client.md`, or that file is gone; until
then, it reads `[bundle] icon` itself and the handoff names the duplicate. Stage 3 needs goal `package-client` reached:
`docs/goals/package-client.md` is gone. Stage 4's dry run needs `crates/nvs-cli/src/package/publish.rs`
(goal `registry-service` stage 3). A session whose stage is gated takes the earliest open stage whose
gate holds, and ends `BLOCKED` naming the missing file when none does.

**Trade-offs.** A program gains a real version in its API document and on Windows a bundle that names
itself. Nothing runs on the request path, and memory is unchanged. `Novis\Rest` costs its users
nothing they do not install, and keeps `Core` small. Simplicity: one manifest says what a program is,
for every output that asks.

**The emitter's two other gaps: scopes are a bound, a shared template is an error.** An OAuth2
requirement keeps an empty scope list, because access is the handler's and its gate's, and a scope
on `#[Api]` would be a claim the server never checks. Two routes that write one 3.1 template (one
verb on one template, or two templates that differ only in capture names) make `nvs build --openapi`
fail naming both, and write nothing. The front end does not refuse the pair, since the router serves
it. Both are stated in `crates/nvs-cli/src/openapi.rs`'s *Bounds*. `Novis\Rest` is a client and
reads no server's scopes.

**`Novis\Rest`'s client authenticates with `client_secret_post`, and every server value is checked
before it is trusted.** `Core\Http\Client`'s `form` takes a `secret string`, so the secret reaches
the wire without `Core\Secret::reveal`, which `client_secret_basic`'s base64 would need. A token,
scope, error word and DPoP nonce from a reply is matched against its RFC character set, then
`Core\Taint::assertTrusted`; a token goes back in a header, so this is the header-injection check.
`DPoP` signs with a P-256 pair (`ES256`), and `Client` resends a request once when a 400 from the
token endpoint or a 401 from a resource carries a new `DPoP-Nonce`. `Token::isExpired` and the
fallback lifetime take whole seconds, since a parameter default cannot be a `Duration` (`E0451`).

## Stages

### 1. `package.toml`'s version is the API document's `info.version`

**Does:** the generated document's `info.version` is the manifest's version.

- [x] Read the manifest's `version` into the emitter (`rule:packaging/package-toml-is-a-tool-input`). `crates/nvs-cli/src/openapi.rs:79`
- [x] Tests with and without a manifest. `crates/nvs-cli/tests/openapi.rs:1`
- [x] The gap record deleted. `data/gaps/nvs-cli/info-version.json:1`
- [x] The emitter's two other gaps built, or each struck as a stated bound in `openapi.rs`'s module doc, and their records deleted: OAuth2 scopes per operation, which `Novis\Rest`'s grants read, and two routes of one verb that write one path template. `data/gaps/nvs-cli/oauth2-scopes-per-operation.json:1`, `data/gaps/nvs-cli/two-routes-that-write-one-template.json:1`

### 2. a Windows bundle carries the program's own version information and icon

**Does:** a bundle built from a program with a manifest shows its own name, version and icon in
Explorer and in a process list.

**Split:** before the session starts, an `Explore` subagent checks the PE-resource crates as the
standing decision says and returns one line per crate with its verdict.

- [x] The crate chosen, or the in-tree rewrite argued, in the rule (`rule:packaging/the-windows-binary-says-what-it-is`). `docs/rules/packaging/the-windows-binary-says-what-it-is.md:1`
- [x] The resource rewritten in the copy, from the manifest's version, name and `[bundle] icon`. `crates/nvs-cli/src/bundle.rs:615`
- [x] A test that reads the bundle's resource back, beside the host's own test, with and without a manifest. `crates/nvs-cli/tests/windows_resource.rs:1`
- [x] The gap record deleted. `data/gaps/nvs-cli/a-bundle-carries-the-hosts-icon-and-version.json:1`

### 3. `Novis\Rest` gives OAuth grants, discovery and DPoP

**Does:** the package's classes request tokens, discover an issuer's endpoints, and sign DPoP proofs.

**Split:** after the session lands the manifest and `Client`, writing subagents write the other six
types one after another: `Token`, `ClientCredentials`, `AuthorizationCode`, `Discovery`, `DPoP`,
`Session`. An `Explore` subagent first returns the `Core\Jwt`, HTTP client and `Core\Session` calls the
package uses, one line each with its anchor.

- [x] The manifest, with its root namespace `Novis\Rest` and its required capabilities. `packages/rest/package.toml:1`
- [x] `Client` and `Token`, over `Core\Jwt`, the HTTP client and the cache (`rule:core-api/tier-placement`). `packages/rest/src/Client.nvs:1`, `crates/nvs-stdlib/src/jwt.rs:263`
- [x] The grants: `ClientCredentials`, `AuthorizationCode` with PKCE, and refresh. `packages/rest/src/AuthorizationCode.nvs:1`
- [x] `Discovery` and `DPoP` (`rule:security/jws-issued-subset`). `packages/rest/src/DPoP.nvs:1`
- [x] The refresh token stored with `Core\Session::setSecret`. `packages/rest/src/Session.nvs:1`, `crates/nvs-stdlib/src/session.rs:251`

### 4. `Novis\Rest` has its tests and its proofs, and passes `nvs pkg publish --dry-run`

**Does:** the package is tested against a local issuer, has its proofs, and packs as the registry
would build it.

**Split:** after the session lands the roster kind, an `Explore` subagent runs
`bun nv proofs --id` for each class and returns the paths it owes; writing subagents then write them
one class after another.

- [x] Tests against a local OAuth issuer on a loopback listener: each grant, discovery, a DPoP proof's claims, and a refresh. `packages/rest/tests/ClientTest.nvs:1`
  - [x] `tests/bootstrap.nvs` autoloads `src/`, and `nvs test packages/rest` runs 25 tests green
  - [x] the issuer is `Core\Test::answerHttp`, so no listener is needed
- [x] The roster kind for a package's class, if one is missing. `tools/nv/proofs/roster.ts:35`
  - [x] kind `package` and `packageFeatures` in `roster.ts`, its policy in `collect.ts`
  - [x] `packages/` is a marker root, and a `.nvs` marker credits its `#[Test]` method
  - [x] `tools/nv/test/roster-packages.test.ts` passes, and `tsc` is clean
- [x] The proofs the roster lists for each class. `packages/rest/src/Client.nvs:1`
  - [x] one writing subagent per class wrote `about.md`, 3 examples, 1 attack and 1 bench
  - [x] a bench's copy rebases its `autoload` folders (`writeCopy` in `tools/nv/cmd/scaling.ts`)
  - [x] perf recorded, and `bun nv proofs --gate --group 'package:Novis\Rest'` owes nothing
- [x] `nvs pkg publish --dry-run` prints its digest. `packages/rest/package.toml:1`
  - [x] `LICENSE` and `README.md` next to `package.toml`, committed, since the dry run packs the commit
- [x] The grants hover follows the entry's tree and `[grants]`, and its gap record is deleted, so `bun nv owners --closes first-packages` passes. `crates/nvs-lsp/src/grants.rs:35`
  - [x] `Analysed::program` is the lender's entry, and the hover reads that entry's tree
  - [x] the hover lists `net.internal`, each extension's `grants` and `package.toml`'s `[grants]`
  - [x] the `.lspt` runner surveys its case's lenders, and a new hover case pins the text
  - [x] the rule says so, the gap record is deleted, and `bun nv owners --closes first-packages` passes

### 5. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [x] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [x] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [x] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1-4 are done. `packages/rest/` is `Novis\Rest` 0.1.0, seven classes, 27 tests green
under `nvs test packages/rest`, and every class's proofs under `package/Novis-Rest-<Class>`, so
`bun nv proofs --gate --group 'package:Novis\Rest'` owes nothing. `Session` runs its examples,
attack and bench as requests against Redis, and its tests are skipped in
`data/proofs/skip/package/Novis-Rest-Session.json`. `nvs pkg publish --dry-run` prints its digest.

The route hover's grants follow the program a file belongs to (`Analysed::program`), read from
its entry, with `net.internal`, each extension's `grants` and `package.toml`'s `[grants]`. The
`.lspt` runner now surveys lenders as the server does; all 576 cases pass, and
`bun nv owners --closes first-packages` passes.

Stage 5 is done, and the goal is reached. The gate's failures, each fixed: the attributes
chapter's `details.md` still said `info.version` is always `0.0.0` (5a5c9bbfc edited the generated
chapter), `packages/` had no row in CONTRIBUTING.md's layout listing, and `bundle.rs`'s `identity`
read as a climbing path to the test impact check. The site pages and questions for
`nvs build --openapi` and `--compile` are current, the CLI chapter's `operationId` is `Class.method`,
and `lang:attributes` perf is re-recorded. The second `bun nv affected --run` is green, and so are
`bun nv verify --doc`, `bun nv owners --closes first-packages`, `bun nv ci --check`, `rest-tests`
and `rest-dry-run`.

The first gate run saw `nvs_ext_test_runs_the_projects_tests_with_the_built_file_loaded`
(`crates/nvs-cli/tests/ext_command.rs:752`) fail once beside the other binaries and pass alone,
with no assertion text in the log. It passed in every run since. Expect it may go red again under load.

### Next

Nothing: the goal is reached. The loop retires it and goes on to `arr-key-order-and-nulls`.

### Backlog

- `nvs_ext_test_runs_the_projects_tests_with_the_built_file_loaded` failed once under verify's load
  and passed alone, and its log held no assertion text. Its second `nvs ext test` runs from the
  scratch folder's parent, which every test shares (`crates/nvs-cli/tests/ext_command.rs:745`).

- `Core\Test::answerHttp` called twice for one URL keeps the first answer, and the docs do not say
  so (`docs/examples/core/Test/answerHttp/about.md`). Several `Novis\Rest` attacks give each step its
  own URL for this reason. Either the second call replaces the answer, or the page says it does not.
- A top-level `jsonAs<tainted { count: int }>()` in a script is E0102/E0117 (an anonymous object
  read as a block), though the same shape compiles inside a method.
- Publishing `Novis\Rest` to the live registry is the project account's step, done by a person once the registry is deployed (`apps/registry/README.md`).
- From goal `registry-website`: a search row restored from a backup is checked only indirectly, since
  `BackupTest::aRestoreBringsBackEveryRelease` restores releases, not `search`. A goal that changes
  `apps/registry/src/Backup.nvs` adds a test that a restored registry still finds a package.
