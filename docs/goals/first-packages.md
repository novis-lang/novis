---
looper: 1
title: "`Novis\\Rest` is the first package, and a program's version comes from `package.toml`"
summary: "nvs build --openapi writes package.toml's version as info.version, a Windows bundle carries the program's own version information and icon, Novis\\Rest gives OAuth grants, discovery and DPoP as a package in this repository that passes nvs pkg publish --dry-run, the gap records this goal owns are closed, and the checks below pass."
context: [docs/plan/m15.md, crates/nvs-cli/src/package/manifest.rs]
stages:
  - id: 1
    title: "`package.toml`'s version is the API document's `info.version`"
    status: open
    context: [crates/nvs-cli/src/openapi.rs, crates/nvs-cli/tests/openapi.rs, data/gaps/nvs-cli/info-version.json]
    extra:
      context:
        rules: [packaging/package-toml-is-a-tool-input, routing/api-document-is-a-deterministic-build-artifact]
  - id: 2
    title: a Windows bundle carries the program's own version information and icon
    status: open
    context: [crates/nvs-cli/src/bundle.rs, crates/nvs-cli/build/winres.rs, data/gaps/nvs-cli/a-bundle-carries-the-hosts-icon-and-version.json, docs/rules/packaging/the-windows-binary-says-what-it-is.md]
    extra:
      context:
        rules: [packaging/the-windows-binary-says-what-it-is, packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host, packaging/package-toml-is-a-tool-input]
  - id: 3
    title: "`Novis\\Rest` gives OAuth grants, discovery and DPoP"
    status: open
    context: [packages/rest/package.toml, crates/nvs-stdlib/src/jwt.rs, crates/nvs-stdlib/src/session.rs, docs/rules/core-api/tier-placement.md]
    extra:
      context:
        rules: [core-api/tier-placement, packaging/a-package-name-is-vendor-slash-name, security/jws-issued-subset]
  - id: 4
    title: "`Novis\\Rest` has its tests and its proofs, and passes `nvs pkg publish --dry-run`"
    status: open
    context: [packages/rest/package.toml, docs/reference/tools/35-packages.md]
    extra:
      context:
        rules: [packaging/the-registry-builds-the-archive-from-git, packaging/a-package-is-its-digest]
checks:
  - id: rest-tests
    stage: 4
    name: Novis\Rest's own tests pass
    run: [cargo, run, -q, --bin, nvs, --, test, packages/rest]
  - id: rest-dry-run
    stage: 4
    name: Novis\Rest packs as the registry would build it
    run: [cargo, run, -q, --bin, nvs, --, publish, --dry-run]
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

**`Novis\Rest` lives in this repository under `packages/rest/`**, so CI tests it with the toolchain it
targets. It is published by the project account from that directory, which `nvs pkg publish` sends with the
commit (goal `registry-service`). It is about seven types over OAuth grants (client credentials,
authorization code with PKCE, refresh), discovery and DPoP. A user's refresh token goes in the session
through `Core\Session::setSecret`, because a cache tier may evict it. Encrypted ID tokens and
generating a client from an OpenAPI document are out of scope.

**Its proofs are what `bun nv proofs` lists for it.** If the roster has no entry kind for a package's
class, stage 4 adds one for `packages/`, in the shape of a built-in component's.

**Trade-offs.** A program gains a real version in its API document and on Windows a bundle that names
itself. Nothing runs on the request path, and memory is unchanged. `Novis\Rest` costs its users
nothing they do not install, and keeps `Core` small. Simplicity: one manifest says what a program is,
for every output that asks.

## Stages

### 1. `package.toml`'s version is the API document's `info.version`

**Does:** the generated document's `info.version` is the manifest's version.

- [ ] Read the manifest's `version` into the emitter, and close the gap record. `crates/nvs-cli/src/openapi.rs:79`, `data/gaps/nvs-cli/info-version.json:1`
- [ ] Tests with and without a manifest. `crates/nvs-cli/tests/openapi.rs:1`

### 2. a Windows bundle carries the program's own version information and icon

**Does:** a bundle built from a program with a manifest shows its own name, version and icon in
Explorer and in a process list.

- [ ] The crate chosen and recorded, and the resource rewritten in the copy. `crates/nvs-cli/src/bundle.rs:332`
- [ ] A test that reads the bundle's resource back, beside the host's own test, and the gap record closed. `crates/nvs-cli/tests/windows_resource.rs:1`, `data/gaps/nvs-cli/a-bundle-carries-the-hosts-icon-and-version.json:1`

### 3. `Novis\Rest` gives OAuth grants, discovery and DPoP

**Does:** the package's classes request tokens, discover an issuer's endpoints, and sign DPoP proofs.

- [ ] The manifest, with its root namespace `Novis\Rest` and its required capabilities. `packages/rest/package.toml:1`
- [ ] The grants, discovery and DPoP over `Core\Jwt`, the HTTP client and the cache (`rule:core-api/tier-placement`). `packages/rest/src/Client.nvs:1`, `crates/nvs-stdlib/src/jwt.rs:263`
- [ ] The refresh token stored with `Core\Session::setSecret`. `packages/rest/src/Session.nvs:1`, `crates/nvs-stdlib/src/session.rs:1`

### 4. `Novis\Rest` has its tests and its proofs, and passes `nvs pkg publish --dry-run`

**Does:** the package is tested against a local issuer, has its proofs, and packs as the registry
would build it.

- [ ] Tests against a local OAuth issuer on a loopback listener. `packages/rest/tests/ClientTest.nvs:1`
- [ ] Its proofs, adding the roster kind if one is missing. `tools/nv/proofs/roster.ts:84`
- [ ] `nvs pkg publish --dry-run` prints its digest, and `bun nv owners --closes first-packages` passes. `packages/rest/package.toml:1`

## Handoff

### State

The goal is new. No stage is started. It needs goals `package-client` and `registry-service`
reached; stages 1 and 2 need only the manifest reader.

### Next

Stage 1 and stage 2, on the manifest's two readers outside the package commands:
- `info.version` from the manifest (`rule:packaging/package-toml-is-a-tool-input`). `crates/nvs-cli/src/openapi.rs:79`
- the bundle's resource from the manifest (`rule:packaging/the-windows-binary-says-what-it-is`). `crates/nvs-cli/src/bundle.rs:332`

### Backlog

- Publishing `Novis\Rest` to the live registry is the project account's step, done by a person once the registry is deployed (`apps/registry/README.md`).
