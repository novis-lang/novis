---
looper: 1
title: Every dependency is on its latest stable release
summary: "Every Rust crate, the Rust toolchain, every npm package, every CI action, every container base and every developer tool is on its latest stable release or carries a dated hold with an upstream reason, and `bun nv deps --check` passes."
context: [rust-toolchain.toml, Cargo.toml, docs/agent/dependency-update.md, docs/rules/packaging/the-sweep-is-fired-by-a-human.md]
stages:
  - id: 1
    title: the Rust toolchain is 1.99.1
    status: done
    context: [rust-toolchain.toml, benches/proxied/Dockerfile]
  - id: 2
    title: the unwind canary checks for an unwind entry
    status: done
    context: [benches/abi-probe/tests/unwind_unavailable.rs, docs/rules/errors/propagation.md, crates/nvs-runtime/src/abi.rs]
    extra:
      context:
        rules: [errors/propagation, errors/helper-abi]
  - id: 3
    title: "the engine moves as one, and the sandbox names its wasm proposals"
    status: done
    context: [crates/nvs-ext/src/call.rs, crates/nvs-ext/src/pack.rs, crates/nvs-codegen/src/lib.rs, crates/nvs-host/src/scheduler.rs]
    extra:
      context:
        rules: [packaging/an-extension-is-a-sandboxed-wasm-component, packaging/a-dependency-move-proves-two-things]
  - id: 4
    title: the workspace crates
    status: done
    context: [Cargo.toml, crates/nvs-stdlib/src/hash.rs, crates/nvs-stdlib/src/encoding.rs, deny.toml]
  - id: 5
    title: html5ever 0.40 with the patch carried over
    status: open
    context: [vendor/html5ever/Cargo.toml, crates/nvs-stdlib/src/html.rs]
  - id: 6
    title: the built-in components and wasi-sdk
    status: open
    context: [extensions/image/Cargo.toml, extensions/pdf/Cargo.toml, extensions/intl/Cargo.toml, extensions/spreadsheet/Cargo.toml, extensions/image/libwebp/SOURCE.json]
  - id: 7
    title: the repository tools and Bun
    status: open
    context: [package.json, tools/nv-scan/Cargo.toml, tools/nv/keys/scan.ts]
  - id: 8
    title: the editor extension and the website
    status: open
    context: [editors/vscode/package.json, editors/vscode/tsconfig.json, editors/vscode/src/extension.ts, website/package.json, website/src/components/PageFrame.astro]
  - id: 9
    title: "CI actions, runners, images and machine tools"
    status: open
    context: [.github/workflows/ci.yml, .github/workflows/release.yml, docker/Dockerfile, docs/implementation-plan.md]
  - id: 10
    title: a command says what is behind
    status: open
    context: [docs/agent/dependency-update.md, tools/nv/main.ts]
checks:
  - id: toolchain
    stage: 1
    name: the pinned toolchain is 1.99.1
    output_contains: ["channel = \"1.99.1\""]
    file: rust-toolchain.toml
  - id: unwind-entry
    stage: 2
    name: a JIT frame has no unwind entry the platform unwinder can find
    run: [cargo, test, --release, -p, nvs-abi-probe, --test, unwind_unavailable]
    tests: [a_jit_frame_has_no_unwind_entry]
  - id: engine-guards
    stage: 3
    name: the wasm boundary guards hold on the new engine
    run: [cargo, test, --release, -p, nvs-abi-probe, --features, wasm-probe]
    timeout: 1200
  - id: components-load
    stage: 3
    name: every built-in component loads and runs under the narrowed proposal set
    run: [bun, nv, verify, -p, nvs-ext]
    timeout: 1800
  - id: deny
    stage: 4
    name: "advisories, licences and bans pass on the root lock"
    run: [cargo, deny, check]
  - id: vendored-html5ever
    stage: 5
    name: the vendored html5ever is 0.40
    output_contains: ["version = \"0.40."]
    file: vendor/html5ever/Cargo.toml
  - id: editor-headless
    stage: 8
    name: the extension compiles and its headless suite passes
    run: [npm, run, "test:headless"]
    cwd: editors/vscode
    timeout: 900
  - id: editor-lint
    stage: 8
    name: the extension lints clean
    run: [npm, run, lint]
    cwd: editors/vscode
  - id: website-build
    stage: 8
    name: the website builds
    run: [npm, run, build]
    cwd: website
    timeout: 1200
  - id: deps-check
    stage: 10
    name: nothing is behind its latest stable release without a dated hold
    run: [bun, nv, deps, --check]
    reach: true
    floor: false
---

# Every dependency is on its latest stable release

## Why

The user fired this sweep by hand, as `rule:packaging/the-sweep-is-fired-by-a-human` requires, and
asked for **all** of it: every crate, the Rust toolchain, every npm package, every CI action, every
container base and every developer tool. The tree has drifted behind on all of them. The last
engine bump, to wasmtime 49 and cranelift 0.136, was reverted because the unwind canary tripped. That
left the JIT and the sandbox one major behind. The procedure is `docs/agent/dependency-update.md`.
This goal is its work list, with the breaks the release notes name and where each one lands in this
tree.

## Standing decisions

**Prototyping regime, and no compatibility to keep.** The workspace version is `0.0.1` and Novis is
not released. `docs/agent/dependency-update.md` § *Before you start* applies: do its steps 1, 2, 4
and 7, update anything, fix what breaks and commit. There is no classification, no deprecation
cycle and no ladder. A change a program can see goes in as it is. One example is `toml` 1.x printing
a `Datetime` with no seconds (`crates/nvs-config/src/request.rs:459`).

**Latest stable means a final release.** Never an `-rc`, `-alpha`, `-beta` or `-dev`. The versions
named below were the latest when this goal was written. A stage takes whatever is latest when it is
worked, and the notes here are the starting point for reading the release notes. They do not cap the
version.

**One gate per group, not per bump.** Each bump is committed after its narrow check, and `looper
verify` runs once on the tip of the session's bumps, as `docs/agent/dependency-update.md` § *2. Move
one thing at a time* says. The user chose this, because a gate per bump made the sweep slow.

**One bump, one commit** (`rule:packaging/one-bump-one-commit`). These groups move as one commit each,
because their crates share types or pin each other exactly:
- wasmtime, wasmtime-wasi-io, wasmtime-internal-jit-icache-coherence, the five `cranelift-*` crates
  and `object`. cranelift-jit pins wasmtime's internal crates with `=`.
- the RustCrypto hashes: sha2, sha1, sha3, md-5 and hmac, with the `sha2-v11` alias removed.
- html5ever across its three `[patch]` users: the root, `fuzz/` and `extensions/pdf/`.
- wit-bindgen in the four components, the `nvs ext new` template and CI's `wit-bindgen-cli`.
- eslint with `@eslint/js`.

**Rust 1.99.1 exactly.** 1.99.0 on x86_64 leaves junk in bits 1 to 7 of a `bool` that an
`extern "C"` Rust function returns (rust-lang/rust#163911). The JIT reads helper results such as
`ptr_eq` as `I8` (`crates/nvs-codegen/src/lib.rs:2426`), and `nvs_str_eq` returns `bool`
(`crates/nvs-runtime/src/string.rs:1528`). Under 1.99.0 a string comparison could be wrong. 1.98.x is
skipped too: its Windows fiber TLS-destructor bug was fixed only in 1.99.0, and wasmtime runs guest
calls on Windows fibers. `rust-version` moves to `1.99` with the pin.

**The canary was tripped by stack layout, not by a capability.** cranelift-jit 0.135, 0.136 and
0.137 register no unwind tables. None contains `__register_frame` or `RtlAddFunctionTable`. 0.135.6
and 0.136.2 emit byte-identical code for the canary's wrapper. On Windows x64 a PC with no function
entry is treated as a leaf and `[RSP]` is read as the return address. That stale slot decides
whether the walk happens to reach `catch_unwind`, so the probe's result depends on the test binary's
layout. The sentence in `rule:errors/propagation` stays true. The canary is rewritten to ask the
platform directly: `RtlLookupFunctionEntry` on Windows and `_Unwind_Find_FDE` on Unix return nothing
for a JIT PC. It fails when that changes. Checked returns stay, as the rule already says. The user
agreed to this.

**Turning off unused wasm proposals is part of the engine stage.** wasmtime's defaults enable GC,
exceptions, threads, tail calls, memory64, multi-memory, relaxed-simd and function references, and 49
adds wide arithmetic. Recent wasmtime advisories hit exactly the exception and GC code. The built-in
components are `panic = "abort"` wasip2 guests built with `+simd128`. The engine config names every
proposal a component may use and turns the rest off. A future wasmtime bump then cannot widen the
sandbox silently. Security outranks everything else, so a proposal is on only when a built-in
component or the `nvs ext new` template needs it, and the test that loads each component proves the
set is enough. The user agreed to this.

**base64 0.23 takes no SIMD.** Its new default feature `simd-unsafe` adds `unsafe` AVX2 and NEON
code to `Core\Encoding`'s path. Security outranks latency, so the row is
`default-features = false, features = ["std"]`. The user agreed to this.

**The user updates Bun and PHP on the machine.** The copies on the Windows `PATH` are the user's to
install. The goal moves only what the tree pins: `engines.bun` and the plan's Toolchain field. A
session that needs the new version before the user has installed it says so in the handoff and
works on another item.

**A bump blocked upstream gets a hold, not a fork.** The row gets a
`# HOLD (revisit <date one month out>): <the upstream reason>` line directly above it, in the
`Cargo.toml` or `package.json` that declares it. These were blocked when this goal was written:
- image: `harfrust` 0.14, because resvg 0.48 still needs ^0.12. `moxcms` 0.9 and `color_quant` 2,
  because `image` 0.25 still needs the old ones.
- pdf: `usvg`/`resvg` 0.48, because `krilla-svg` 0.8 needs ^0.47. That also keeps the `rustybuzz`
  advisory ignore in `deny.toml`.
- intl: ICU 2.4, if it ships, because `icu_experimental` 0.6.0 needs `~2.3`.
- root: `fallible-iterator` 0.3, because `postgres-protocol` 0.6 uses 0.2. `generic-array` 0.14.9
  and `brotli-decompressor` 6.1, which are pinned by their parents.
- editor: TypeScript 7, because `typescript-eslint` allows `<6.1`. The extension goes to 6.0.x.
- `ubuntu-22.04` runners stay. They are the glibc floor of the release binaries.

**No new limits in this goal.** hyper 1.12 adds `max_header_size`. Whether to set it is a separate
question for the user (`crates/nvs-server/src/serve.rs:2308`).

**Unused workspace rows are removed, not bumped.** `smallvec`, `indexmap`, `postgres-types`,
`proptest` and `anstream` are named by no member manifest.

**Out of the agent's reach, so the handoff names them for the user.** `.github/dependabot.yml` is
read only from the `landing` branch, so the person who pushes mirrors any change there. The local
MSVC 14.44 toolset is the last one in Visual Studio 2022, and moving to VS 2026's 14.50 is a change
to the user's machine.

**Trade-offs.** Request path: hyper 1.12 adds smuggling fixes. Rust 1.99.1 brings LLVM 23, so the
callgrind figures and the committed guest-against-native image figures may move. Battery leg E records
the change. Memory: none expected. wasmtime 50, when it ships final, adds
`Engine::release_idle_pool_memory`, which serves idle-near-zero. Simplicity: the sandbox gets an
explicit proposal list, the RustCrypto set gets one digest generation in our own code, and five unused
rows go away. Developers who use the language see `toml` 1.x's datetime text. Nothing else they can
see changes.

## Stages

### 1. the Rust toolchain is 1.99.1

**Does:** the whole tree builds, tests and lints clean on Rust 1.99.1 on all three platforms.

- [x] `channel = "1.99.1"`, and `rust-version = "1.99"`. `rust-toolchain.toml:4`, `Cargo.toml:40`
- [x] The proxied bench builds on `rust:1.99.1-slim-trixie`, with its runtime stages on trixie too, as its comment says. `benches/proxied/Dockerfile:19`
- [x] clippy's `chunks_exact_to_as_chunks` sites are rewritten to `as_chunks::<N>()`. There are about 25 sites in 15 files, all with literal sizes. `crates/nvs-db/src/pg.rs:2651`
- [x] A full `cargo clippy --all-targets` is clean. That includes a dependency macro's `semicolon_in_expressions_from_non_local_macros` warning, which `-D warnings` turns into a failure. `Cargo.toml:1085`
- [x] The plan's Toolchain field says 1.99.1. `docs/implementation-plan.md:32`

### 2. the unwind canary checks for an unwind entry

**Does:** the canary asks the platform whether a JIT PC has an unwind entry, so its result no longer depends on stack layout. `rule:errors/propagation` names how it checks.

- [x] Replace the panic probe with `a_jit_frame_has_no_unwind_entry`. It uses `RtlLookupFunctionEntry` on Windows and `_Unwind_Find_FDE` on Unix, on a PC inside a finalized JIT function, with the canary's flags (`is_pic=false`, `unwind_info=true`). The module doc is rewritten as a whole. `benches/abi-probe/tests/unwind_unavailable.rs:1`
- [x] The rule's canary paragraph says what the canary checks. The sentence about unwind tables stays. `docs/rules/errors/propagation.md:25`

### 3. the engine moves as one, and the sandbox names its wasm proposals

**Does:** the JIT and the sandbox are on the latest final wasmtime and its matching cranelift. Battery leg B is green, and the engine enables only the wasm proposals the components use.

- [x] One commit: wasmtime ×3, cranelift ×5 and `object` 0.40. With wasmtime 49 / cranelift 0.136, only manifests changed the last time this was tried. `Cargo.toml:99`, `Cargo.toml:346`, `Cargo.toml:112`
- [x] `wit-parser`, `wasmparser` and `wit-component` move to the version that wasmtime uses. At 0.258 the `ComponentEncoder` builder takes `&mut self`. At 0.261 `embed_component_metadata` gains `canonical_names`, at four sites. `crates/nvs-ext/src/pack.rs:196`
- [x] Turn the unused proposals off in the engine config, and test that each component still loads. `crates/nvs-ext/src/call.rs:722`
- [x] wasmtime 50 is only `50.0.0-rc.1`, so a `HOLD` line above `wasmtime` names it. Its two changes wait in the Backlog: the epoch callback's fuel refresh (`StoreHookState` has no `set_hostcall_fuel`) and dropping the `new_own` workaround. `Cargo.toml:346`
- [x] `corosensei` 0.3, in its own commit. `with_stack` needs `Stack: 'static`, and 0.3.1 changes forced unwinding. `crates/nvs-host/src/scheduler.rs:658`

### 4. the workspace crates

**Does:** every root workspace crate is at its latest stable or holds, the lock is refreshed, and `cargo deny check` and the attribution file are current.

- [x] `cargo update` inside the ranges. hyper 1.12 and 76 other moves add `simdutf8`, `core_detect` and `multiversion_no_op`, so the attribution file must be regenerated. `Cargo.toml:1`
- [x] The RustCrypto group moves to digest 0.11, and the alias goes. `hmac` 0.13's `EagerHash` bound changes the `mac!` macro. The comment that claims one digest generation is rewritten. `crates/nvs-stdlib/src/hash.rs:861`, `Cargo.toml:641`
- [x] base64 0.23: `InvalidLastSymbol` becomes a struct variant, and SIMD is off. `crates/nvs-stdlib/src/encoding.rs:972`
- [x] toml 1.x. `crates/nvs-config/src/request.rs:459`
- [x] criterion 0.8. Its new `alloca` C dependency is dev-only. `Cargo.toml:1055`
- [x] The five unused rows are removed. `Cargo.toml:412`
- [x] Every blocked row has its `HOLD` line. `Cargo.toml:1`

### 5. html5ever 0.40 with the patch carried over

**Does:** the vendored html5ever is 0.40 with every `// Novis:` hunk carried over and its tests passing, in the root, `fuzz/` and `extensions/pdf/`.

- [x] Re-vendor 0.40.1 and carry the hunks from 6a9030d4c over. The patch is not upstream. `finish_attribute` conflicts with upstream's atom comparison, and the tokenizer states moved into enums. `vendor/html5ever/Cargo.toml:1`
- [x] The three `[patch]` rows and `markup5ever` 0.40 move. `Cargo.toml:1180`
- [ ] Fuzz `lex` and `parse` per battery leg D. `fuzz/Cargo.toml:1`

### 6. the built-in components and wasi-sdk

**Does:** each component's own lock is current, its incompatible bumps are taken or held, and CI and libwebp build with one wasi-sdk, the latest.

- [ ] wit-bindgen 0.63 in the four components and the template, with CI's `wit-bindgen-cli` at the same version. `crates/nvs-cli/templates/ext/rust/Cargo.toml:12`, `.github/workflows/ci.yml:564`
- [ ] pdf: taffy 0.15. The alignment fields are not `Option`, `Position::Static` is now the default, and absolute insets are relative to the border box, so the layout output moves. `extensions/pdf/src/layout/flex.rs:25`
- [ ] pdf: hayro-interpret 0.8, the same as image and the root. `extensions/pdf/src/read.rs:29`
- [ ] image: ravif 0.13. `extensions/image/src/encode.rs:78`
- [ ] One wasi-sdk, the latest, for CI and libwebp. CI is at 25 and libwebp's pin at 30. Rebuild libwebp with `bun nv webp-lib`. `.github/workflows/ci.yml:535`, `extensions/image/libwebp/SOURCE.json:11`
- [ ] The `.nvsx` hashes and the committed guest figures are regenerated. `crates/nvs-ext/build.rs:183`

### 7. the repository tools and Bun

**Does:** the Bun tooling runs on the latest Bun, and `tools/nv-scan` and `fuzz/` are on current crates.

- [ ] Bun 1.4.x in `engines.bun`, with `@types/bun` to match. The user installs it on the machine first. Check `bun:ffi` `dlopen` in `proctree.ts` and `tty.ts`, and `bun test --parallel`, which now isolates. `package.json:8`
- [ ] `playwright-core` and `smol-toml`. `package.json:16`
- [ ] nv-scan's `=` pins follow the root lock, including syn 3 if the root has moved its own use. Every scanner digest moves, and that is expected. `tools/nv-scan/Cargo.toml:1`, `tools/nv/keys/scan.ts:32`
- [ ] The `fuzz/` lock is refreshed. `fuzz/Cargo.toml:1`

### 8. the editor extension and the website

**Does:** the VS Code extension and the website are on their latest dependencies, build, lint and pass their suites.

- [ ] vscode-languageclient 10. The tsconfig moves to `node16` resolution, the output channel is created with `{ log: true }`, and the client code is checked for 10.x's middleware changes. `editors/vscode/src/extension.ts:122`, `editors/vscode/tsconfig.json:1`
- [ ] TypeScript 6.0.x, with `"types": ["node", "mocha"]`. eslint 10 with `@eslint/js` 10. `editors/vscode/package.json:1`
- [ ] mocha 12, which is ESM-only, plus vsce 4, test-electron 3 and the VS Code build the host tests download. `editors/vscode/test/host/index.ts:18`, `editors/vscode/scripts/host.mjs:31`
- [ ] Starlight 0.42: the mobile menu has no `aria-expanded`, so `PageFrame` styles it from the new markup. `website/src/components/PageFrame.astro:94`
- [ ] astro latest, and `engines.node` raised to `>=22.12.0`, which Astro needs. `website/package.json:1`

### 9. CI actions, runners, images and machine tools

**Does:** every action SHA is its latest release, no job names a retired runner, the images are on the current Debian, and the plan's Toolchain field is true.

- [ ] Action SHAs move for `setup-node`, `upload-artifact`, `download-artifact`, `setup-qemu`, `setup-buildx`, `build-push` and dtolnay's nightly branch. Each `# vX` comment is rewritten with its SHA. `.github/workflows/ci.yml:1`
- [ ] Use `cc-debian13:nonroot` and `debian:trixie-slim`. Debian 12 is out of security support. `docker/Dockerfile:20`
- [ ] Use alpine 3.24.x for the Go bench. `benches/compare/go/Dockerfile:26`
- [ ] Pin `cargo-geiger` to a version. `.github/workflows/ci.yml:994`
- [ ] The user installs PHP 8.5.11 on the Windows `PATH`. The session moves the WSL copy to match, writes the Toolchain field and runs battery leg F. `docs/implementation-plan.md:32`

### 10. a command says what is behind

**Does:** `bun nv deps` lists every pin in the tree with its latest stable release, and `--check` fails on one that is behind with no `HOLD` line. The next sweep starts from its output.

- [ ] Read every pin and compare it with its registry: the Cargo workspaces, the three `package.json` files, action SHAs, Dockerfile bases, the toolchain and the wasi-sdk pins. Hold lines are read. The tool never edits a file. `tools/nv/cmd/deps.ts:1`
- [ ] `dependency-update.md` § *1. Orient* starts from `bun nv deps`. Its pin table gains the npm, Docker and wasi-sdk rows it is missing. `docs/agent/dependency-update.md:1`

## Handoff

### State

Stages 1 to 4 are done. Stage 5 has its fuzz item left.
- The root lock has six crates behind latest, and each has a `HOLD` line dated 2026-11-16 in
  `Cargo.toml`. `wit-parser`, `wasmparser` and `wit-component` 0.262 wait for wasmtime, whose
  `sha2` 0.10 also keeps `generic-array` at 0.14.7 (`Cargo.toml:372`). `brotli` 9.0.0 needs
  `brotli-decompressor ~6.0` (`Cargo.toml:948`). `postgres-protocol` 0.6.12 uses
  `fallible-iterator` 0.2 (`Cargo.toml:1010`).
- html5ever is 0.40.1 in `vendor/`, merged three ways from 0.39.0. All eleven `// Novis:` sites
  are there. Upstream now compares attribute names as `LocalName` atoms, so the duplicate-name set
  holds atoms (`vendor/html5ever/src/tokenizer/mod.rs:569`). 0.40.1 still has no cap on open elements
  and no fingerprint. `markup5ever` 0.40, `string_cache` 0.11 and `web_atoms` 0.3 follow in all
  three locks: the root, `fuzz/` and `extensions/pdf/`.
- `cargo deny check` passes, and the attribution file is regenerated.
- The nightly run 38869457440 tested `cb7021a24`, an old commit, again. Its red jobs are not this
  tree's until the user pushes. The floor debt run needs `samba-ad`, and Docker is not running on
  this machine.

### Next

**Stage 5's fuzz item, then stage 6.** Files: `fuzz/`, then `extensions/*/Cargo.toml`.

- [ ] Fuzz `lex` and `parse` per battery leg D against html5ever 0.40.1. `fuzz/Cargo.toml:1`
- [ ] wit-bindgen 0.63 in the four components and the template, with CI's `wit-bindgen-cli` at the same version. `crates/nvs-cli/templates/ext/rust/Cargo.toml:12`, `.github/workflows/ci.yml:564`
- [ ] pdf: hayro-interpret 0.8, the same as image and the root. `extensions/pdf/src/read.rs:29`

### Backlog

- The user pushes `main`, so the nightly tests the current tree (`.github/workflows/ci.yml:7`), and decides on `looper sync`: `looper lint` says `AGENTS.md`'s managed text and `.claude/settings.json` drifted.
- Proof debt: the stale perf records and the `Core\Test\Outcome`/`Summary` proofs (`bun nv proofs --owed`).
- Load flakes: `ldap` deadlines, the `live_edit` 30 s wait (`crates/nvs-cli/tests/live_edit.rs:1010`), `spreadsheet_read` and `parallel_tests_overlap_on_more_than_one_core` (`crates/nvs-cli/src/runner.rs:4100`). Linux only: `crates/nvs-cli/tests/run_command.rs:688`, and the scaling margin (`crates/nvs-cli/src/serve.rs:3957`).
- wasmtime 50.0.0 final and cranelift 0.137, held at `Cargo.toml:346`: move the epoch callback's fuel refresh out, because `StoreHookState` has no `set_hostcall_fuel` (`crates/nvs-ext/src/call.rs:1075`), and drop the `new_own` workaround and its playbook bullet (`crates/nvs-ext/src/handle.rs:176`).
- hyper 1.12's `max_header_size` is a question for the user (`crates/nvs-server/src/serve.rs`).
- The user mirrors any `.github/dependabot.yml` change on `landing` and decides about MSVC 14.50 (`docs/setup.md`).
