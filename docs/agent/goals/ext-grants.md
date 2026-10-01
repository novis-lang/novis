---
milestone: M9
position: last
---
# Loop goal 193 — a guest reaches only the files and hosts all three parties grant

ADR 0246 § 7 gives a WebAssembly guest an empty WASI and two kinds of I/O, each granted only where the
operator, the extension's manifest and the calling code all agree. Goal `ext-host` loads a component,
runs it per request and keeps it inside the request's budget, and goal `ext-compiler` lets a program
call it. This goal gives the guest its WASI imports and its grants, and proves that nothing else gets
through:

```toml
[[extension]]
path   = "geo.nvsx"
sha256 = "…"
grants = { read = ["data/geo/"], write = [], connect = ["tiles.example.com"] }
```

1. **An empty WASI.** A guest built by a toolchain that links WASI for its own libc loads and runs.
   Its standard output and standard error go to the request log at `debug`, its wall clock is the
   request's (a test's fixed clock reaches it), its random bytes come from `Core\Random`'s generator,
   and it has no environment, no arguments, no directories and no network. `exit` traps.
2. **The grant.** The `[[extension]]` entry's `grants`, the manifest's request and the caller's own
   effective set are intersected by one function, and nothing else decides what a guest holds.
3. **Files.** Each root in the intersection is a `wasi:filesystem` preopen, read-only for `read` and
   read-write for `write`. A path outside it fails, whichever of the three parties left it out.
4. **Outbound HTTP.** `wasi:http`'s outgoing handler is `Core\Http\Client`: the `net.connect` grant,
   the address policy, TLS trust and the outbound proxy apply exactly as they do to Novis code.
5. **The adversarial suite.** Attacks a hostile guest makes, and the reloads M9's verification lists,
   each run end to end against the binary.

## Why here

The grant needs every piece the three goals in front of it build: goal `ext-host`'s loader, instance
and call bridge (`crates/nvs-ext`), its manifest parser, and goal `ext-compiler`'s call from a
program, without which no attack can be written as a `.nvs` file. It sits in front of goal
`ext-tooling` because `nvs ext inspect` prints the I/O a manifest requests, which this goal defines,
and `nvs ext test` runs a guest under the WASI this goal links. It carries `position: last` because
every goal it lands behind is pinned.

What is on disk as this goal is written: wasmtime 48 is a workspace dependency (`Cargo.toml:308`), and
neither `wasmtime-wasi` nor `wasmtime-wasi-http` is in `Cargo.lock`. `Extension` has `path` and
`sha256` and nothing else (`crates/nvs-config/src/tree.rs:419`), so `grants` is refused as an unknown
key today. The one place a capability is answered for a context is
`crates/nvs-runtime/src/capability.rs:210` (`granted`), which asks the isolate's narrowing
(`crates/nvs-runtime/src/ctx/isolate.rs:688`) and then the snapshot's `[capabilities]` block
(`crates/nvs-config/src/capability.rs:636`). An outbound call is judged in
`crates/nvs-stdlib/src/http.rs:3834` (`exchanged`): `approved` (:3242) asks `net.connect` and pins the
address under the policy, `judge_trust` (:2738) asks for TLS trust, and `proxy_of` (:4347) reads the
proxy. Both rules this goal implements say **Not on disk**:
`rule:security/extension-grants-are-an-intersection` and `rule:packaging/a-guest-has-no-ambient-authority`.

## Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that makes it untrue:

- `docs/rules/security/extension-grants-are-an-intersection.md` and
  `docs/rules/packaging/a-guest-has-no-ambient-authority.md` — each loses its **Not on disk**
  paragraph, and its record gains `guardedBy` naming this goal's tests, in the session that makes the
  whole rule true (Stage 5 for both).
- The module doc of `crates/nvs-ext/src/lib.rs` (written by goal `ext-host`) — wherever it says a
  guest has no WASI or no I/O.
- `docs/reference/tools/20-config.md` — the table of blocks at line 63 names `[[extension]]`'s paths;
  it gains `grants`. Stage 5 adds the section that states the grant.

`docs/ground-rules.md`, the `docs/rules/*.md` chapters, `docs/novis.md` and `website/` are generated
and are regenerated, never edited.

## Stage 1 — the floor

None is carried: after goal `goal-closeout` the suites, the `.nvst` trees and `nv verify` are the
safety net. Every check of goals `ext-host` and `ext-compiler` stays green as a test of the tree, and a
session that turns one red repairs it in the same slice.

## Stage 2 — the empty WASI, the keystone

**Does:** Links WASI 0.2 with an empty context, so a guest built with a libc loads and runs and holds
no authority.

One file set: `crates/nvs-ext/src/wasi.rs` (new), `crates/nvs-ext/src/lib.rs`, `crates/nvs-ext/Cargo.toml`
and the root `Cargo.toml:308`, `crates/nvs-stdlib/src/time.rs:3537`, `crates/nvs-stdlib/src/random.rs:927`,
and `crates/nvs-config/src/directive.rs:261` only if the settings block is still owed.

- **The dependency.** `wasmtime-wasi` at wasmtime 48's version, beside `Cargo.toml:308`'s `wasmtime`.
  `wasmtime-wasi-http` waits for Stage 5. `rule:packaging/a-guest-has-no-ambient-authority`.
- **The linker is the list.** `crates/nvs-ext/src/wasi.rs` links each WASI interface the world
  `nvs:ext@1.0.0` names, one by one, and never a crate's link-everything function. A test reads the
  linker's definitions and fails naming any interface the world does not list. Until this stage,
  goal `ext-host`'s load check refuses every WASI import; it now admits the ones linked here, read
  from the same list, so the linker and the check cannot disagree. The refusal of an import outside
  the world stays goal `ext-host`'s and is not written again.
- **What goal `ext-host` left to the linker.** If `nvs:ext/log`, `nvs:ext/settings` and the
  `[ext.<name>]` block (ADR 0246 § 9) are not linked yet, they are linked here first: `log` writes to
  the calling request's log with the extension's name as the channel, `settings` reads the block from
  the request's snapshot, and the block is a `System`/`Reload` block whose unknown key is refused at
  boot. The tests below guard them whichever goal linked them.
- **An empty context.** No environment, no arguments, no preopened directory and no socket. Stdin is
  empty. Stdout and stderr are written to the calling request's log at `debug`, with the extension's
  name as the channel, the way `nvs:ext/log` writes (`crates/nvs-stdlib/src/log.rs:1`).
- **The request's clock and generator.** The wall clock is `crates/nvs-stdlib/src/time.rs:3537`'s
  `wall_clock`, so a `#[Test(at: …)]` fixed clock reaches the guest. Random bytes, the insecure ones
  included, are drawn through `crates/nvs-stdlib/src/random.rs:927`'s `draw`, so a seeded test makes a
  guest's bytes reproducible. The monotonic clock is the host's.
- **`exit` traps**, as `rule:packaging/a-guest-crash-throws` says a trap does: the program sees
  `ExtensionError`, and the next call gets a fresh instance.
- **A libc-shaped guest.** A WAT component that imports what a `wasm32-wasip2` C or Rust libc links
  at start-up — environment, arguments, preopens, stdio streams, clocks, random — loads, runs, prints,
  reads the time and finds nothing it could use. That is the "any language" claim, made without a
  wasm toolchain in `cargo test`.
- **Pinned by** the Stage 2 checks, in `crates/nvs-ext/tests/wasi.rs`.

## Stage 3 — the grant

**Does:** Adds `grants` to `[[extension]]` and computes a guest's effective set as the intersection of
the entry, the manifest and the caller.

Two file sets, in this order. The configuration: `crates/nvs-config/src/tree.rs:419`,
`crates/nvs-config/src/capability.rs:836` (`anchor`) and `:805` (`canonicalize`),
`crates/nvs-config/tests/tree.rs:44`, `crates/nvs-config/src/cache.rs:636`, goal `ext-host`'s
`crates/nvs-config/src/extension.rs`. The intersection: `crates/nvs-ext/src/grants.rs` (new), goal
`ext-host`'s `crates/nvs-ext/src/manifest.rs`, `crates/nvs-runtime/src/capability.rs:210`,
`crates/nvs-runtime/src/ctx/isolate.rs:688`.

- **The key.** `Extension` gains `grants: { read, write, connect }`, each a list of strings, unknown
  keys refused with the file and line as every `deny_unknown_fields` block is
  (`crates/nvs-config/src/tree.rs:419`). An entry with no `grants` holds no I/O.
  `rule:security/extension-grants-are-an-intersection`.
- **Roots resolve against their file.** A relative `read` or `write` root resolves against the
  directory of the file that wrote the entry, by the pass that already does it for `[capabilities]`
  (`crates/nvs-config/src/capability.rs:836`), and is canonicalised once when the snapshot is built
  (`crates/nvs-config/src/capability.rs:805`, `rule:security/path-scope-canonicalise-then-prefix`).
  An empty root grants nothing, as it does there.
- **`grants` is not in `env_hash`.** A grant changes what a guest may reach at run time and nothing a
  compiled unit calls, so `crates/nvs-config/src/cache.rs:636`'s `extension_set_hash` keeps reading the
  pins alone. A test pins it.
- **Validated with the pin.** A malformed `grants` is refused beside the pin's refusals, in goal
  `ext-host`'s `crates/nvs-config/src/extension.rs` (`validate`), naming the file and the line.
- **The manifest's request**, in goal `ext-host`'s manifest model (`crates/nvs-ext/src/manifest.rs`).
  Where that model has no shape for it yet, this one: `read` and `write` are booleans (an author
  cannot know the operator's folders), and `connect` is a list of host patterns or `"any"`.
- **The intersection, in one function** in `crates/nvs-ext/src/grants.rs`: the entry's roots and hosts,
  kept where the manifest requests that kind and the caller holds it. The caller's set is what
  `crates/nvs-runtime/src/capability.rs:210`'s `granted` answers for `fs.read`, `fs.write` and
  `net.connect` on that context: its own snapshot, narrowed by an isolate's `grants:`
  (`crates/nvs-runtime/src/ctx/isolate.rs:688`). A per-namespace grant table is not on disk; if it
  lands before this stage, the caller's set is read from it, and nothing else changes. Stage 4's
  preopens and Stage 5's handler read only what this function returns.
- **Pinned by** the Stage 3 checks: the configuration's in `crates/nvs-config/tests/tree.rs` (whose
  `"abc"` pin at line 44 goal `ext-host` already fixed), the intersection's in
  `crates/nvs-ext/tests/grants.rs`.

## Stage 4 — files

**Does:** Opens each root of the intersection as a WASI preopen, so a guest reads and writes only
there.

One file set: `crates/nvs-ext/src/wasi.rs`, `crates/nvs-ext/src/grants.rs`,
`crates/nvs-runtime/src/capability.rs:1028` (`canonicalize`), `crates/nvs-ext/tests/files.rs` (new).

- **Preopens per instance.** When an instance is created, each `read` root of the intersection is
  preopened read-only and each `write` root read-write, under its canonical absolute path. The set is
  computed from the creating context's own snapshot, so a reload reaches the next request and never a
  running one (`rule:security/capability-question-is-grant-and-scope`).
- **A guest path stays under its root.** A lookup is relative to an opened directory handle, so `..`
  past the root and a symlink planted under it that points outside both fail. That is wasmtime-wasi's
  containment, not a second canonicalise-then-prefix; the tests prove both escapes fail. If either
  escape succeeds, the filesystem host is wrapped so every open asks
  `crates/nvs-runtime/src/capability.rs:1028`'s check first, and the record of why goes in
  `crates/nvs-ext/src/wasi.rs`'s module doc.
- **One test per party.** A read outside the intersection fails when the entry leaves the root out,
  when the manifest requests no reads, and when the caller does not hold the root. A read inside all
  three succeeds, and a write to a read-only root fails.
- **Rules.** `rule:security/extension-grants-are-an-intersection`,
  `rule:security/path-scope-canonicalise-then-prefix`. The **Not on disk** paragraph of
  `rule:packaging/a-guest-has-no-ambient-authority` goes when Stage 5 lands, not here.
- **Pinned by** the Stage 4 checks.

## Stage 5 — outbound HTTP

**Does:** Implements `wasi:http`'s outgoing handler with `Core\Http\Client`, so every outbound rule
for Novis code holds for a guest.

One file set: `Cargo.toml:308`, `crates/nvs-ext/src/wasi.rs`, `crates/nvs-stdlib/src/http.rs`
(`exchanged` at line 3834, `approved` at 3242, `judge_trust` at 2738, `proxy_of` at 4347),
`crates/nvs-stdlib/src/http/transport.rs:1239`, `crates/nvs-ext/tests/http.rs` (new).

- **The dependency.** `wasmtime-wasi-http` at wasmtime 48's version, linking the outgoing handler and
  the types it needs, and nothing else from `wasi:http`.
- **One HTTP stack.** The handler's send is routed to the path `crates/nvs-stdlib/src/http.rs:3834`'s
  `exchanged` takes: `approved` asks `net.connect` against the intersection's hosts and pins the
  address under `rule:security/net-address-policy`, `judge_trust` applies
  `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`, and `proxy_of` applies
  `rule:http-server/an-outbound-proxy-is-operator-configured`. The handler's own default sender is never
  called. Where `nvs-ext` cannot reach `nvs-stdlib`'s private functions, `nvs-stdlib` exposes one entry
  point and the handler calls it; the judging is never copied.
- **It waits as a guest waits.** A pending response returns a pending future and the guest suspends,
  so the core runs its other tasks (ADR 0246 § 5).
- **One test per party** for an ungranted host, and one each for a private address, the proxy and an
  untrusted certificate. A granted call succeeds.
- **Rules.** `rule:packaging/a-guest-has-no-ambient-authority` and
  `rule:security/extension-grants-are-an-intersection` are now whole: both lose **Not on disk**, and
  each record's `guardedBy` names the Stage 2 to 5 tests.
- **Pinned by** the Stage 5 checks.

## Stage 6 — the adversarial suite and the feature proofs

**Does:** Adds the attacks a hostile guest makes and the reload tests M9 lists, and the feature proofs
of the grant.

Two file sets, in this order. The programs: `docs/reference/tools/20-config.md:259` (the section
before which the new one goes), `tests/hostile/tools/config/extension-grants-what-a-component-may-reach/`,
`docs/examples/tools/config/extension-grants-what-a-component-may-reach/`, `crates/nvs-ext/tests/fixtures.rs`
(goal `ext-compiler`'s). The reload: `crates/nvs-cli/tests/live_config.rs:518`, `crates/nvs-cli/src/control.rs:495`.

- **The section.** `docs/reference/tools/20-config.md` gains `# Extension grants: what a component may
  reach`, after `# Network grants` at line 259: the `grants` key, the three parties, what an entry
  with none holds, and a fenced `nvs.toml` block. That heading is the feature
  `tools:config/extension-grants-what-a-component-may-reach` and its Help proof
  (`rule:testing/feature-proofs`).
- **The fixtures**, in the shape goal `ext-compiler` set for its conformance fixtures: component WAT
  text packed by `nvs-ext`'s packer into a committed `.nvsx` beside it, in a folder under the proof
  directory, with its pin in that directory's `nvs.toml`. Goal `ext-compiler`'s test in
  `crates/nvs-ext/tests/fixtures.rs` reads `tests/conformance/ext/fixtures/`; a second test beside it
  reads every fixture under `tests/hostile/` and `docs/examples/`, and fails when a committed file or
  the pin in its `nvs.toml` is not what the packer builds from the text.
- **The attacks**, each with the `// hostile:` contract of `tests/hostile/README.md` and one
  `nvs.toml` for the directory (`rule:testing/hostile-case-contract`):
  `01` reads files nobody granted: a path outside every root, `..` out of a granted root, a write to
  a read-only root, the configuration file itself, a path with a NUL byte. Each failure is caught.
  `02` calls hosts nobody granted: an ungranted name, `127.0.0.1`, `169.254.169.254`, `[::1]`. Each
  failure is caught. `03` is a guest that never stops, and ends at the CPU limit
  (`// hostile: ends-early <step> FATAL`). `04` is a guest that grows its memory without end, and ends
  at the memory limit. `05` is a guest that stores a value in a global and then crashes: the
  `ExtensionError` is caught, and the next call starts from a fresh instance with the value gone.
- **The reloads**, in `crates/nvs-cli/tests/live_config.rs` against a running `nvs serve`, the shape
  of `a_changed_app_origin_reaches_the_mount_rows` at line 518: an extension added by a reload is
  callable from the next request with no restart; a reload whose pin does not match its file is
  refused whole and the previous set keeps serving; a unit compiled under one extension set is not
  reused under another; a reload that narrows an entry's `grants` reaches the next request with no
  recompile. Goal `ext-host` proves the first three inside `nvs-ext`; these prove them through the
  control socket (`crates/nvs-cli/src/control.rs:495`).
- **The rest of the feature proofs.** An `about.md`, the examples `bun nv proofs --id` says the
  feature owes (one reads a file through a granted root and prints its first line), and the Rust
  tests of Stages 3 to 5 marked `// covers: tools:config/extension-grants-what-a-component-may-reach`.
- **Pinned by** the Stage 6 checks.

## Standing decisions

- **The user's calls, as instructions.** Values cross as typed WIT values by ADR 0246 § 1's table,
  and the `value` handle only for `mixed`. A guest links WASI with an empty context, and may be
  granted files (preopens) and outbound HTTP (`wasi:http` through `Core\Http\Client`) and nothing
  else. Grants are the intersection of the `[[extension]]` entry's `grants`, the manifest's request
  and the caller's own effective set. A guest call runs on its request's core as a wasmtime async call
  polled by the coroutine, yielding at every epoch tick (about 1 ms), with no compute pool. The Novis
  source half travels inside the `.nvsx` (`nvs.source`), covered by the one pin. `sha256` is required
  on every entry, and there are no signatures in M9. A trap throws `ExtensionError` (extends
  `RuntimeError`); a CPU or memory limit is a resource-limit `FATAL`. Intl covers ADR 0247 § 5's
  web-app set, without MessageFormat. `nvs ext new` ships Rust and C templates only. `Novis\Image` and
  `Novis\Intl` are built into every binary and always on. The build compiles the component crates for
  `wasm32-wasip2`, and libwebp is prebuilt by a `bun nv` tool, committed beside its source hash, and
  rebuilt and compared by CI.
- **The record writer's calls, standing but not confirmed by the user.** The manifest is JSON written
  from an `nvsx.toml`. `Novis\` is reserved. The error variant `invalid|parse|runtime` throws
  `LogicError|ParseError|RuntimeError`. One instance per extension per request, and a second task
  waits. A third-party settings block is `[ext.<name>]`. `nvs check` and the language server read
  manifests and never instantiate. A bundle embeds the `.nvsx` files the build's configuration lists.
- **This goal writer's calls, not confirmed by the user.** `grants` is not in `env_hash`. The
  manifest's request is two booleans and a host list, unless goal `ext-host`'s parser already has a
  shape. An isolate shares nothing (`rule:security/isolate-shares-nothing`), so it has its own
  instance, whose preopens come from its own narrowed set. A guest sees each root under its canonical
  absolute path. The attacks live under the new `tools:config/…` feature, because a hostile case
  belongs to a feature and a `tests/hostile/ext/` folder belongs to none, so no sweep would run it.
- **No record slot.** ADR 0246 decided all of this. A question it does not answer is settled under
  AGENTS.md's priority ordering and written in the module doc of the file that answers it.
- **A libc import outside the world is linked as a refusing stub, never granted.** If a real
  `wasm32-wasip2` libc imports a WASI interface outside the world even when nothing calls it
  (`wasi:sockets` is the likely one, not checked), the host links that interface with every function
  returning its own "access denied" error — never a trap, never an authority. The interface is named
  in the world's doc as stubbed, and an attack proves a call to it fails. Any import that is not WASI
  is still refused at load. This is the record writer's call, not the user's, and the user has not
  confirmed it.
- **An attack is never softened to pass** (`tests/hostile/README.md`). A real failure is fixed or
  recorded with a gap.
- **Tradeoffs.** Performance: a request that calls no extension pays nothing. A first call pays one
  preopen per granted root, and an HTTP call costs what `Core\Http\Client` costs plus the copy of the
  body across the boundary. Memory: one WASI context and a few directory handles per instance,
  freed with the request, so O(in-flight). Usability: the operator writes one `grants` key, and an
  author's libc works with no shim. Simplicity: one more key and one function deciding what a guest
  holds; no second HTTP stack and no second path check.
- **Test guests are component WAT compiled by the `wat` crate**, so `cargo test` needs no wasm
  toolchain. A debug cargo command never takes `-p`. A symlink test creates the real link and, where
  the host refuses, prints the reason and returns, and its assertions are proved once under WSL
  (`docs/agent/playbook/writing-a-test-case/`).
- **Every name in a test, an example and a fixture is neutral** — `Geo`, `Shop`, `example.com`.
- **Every comment in a new `.nvs` and every new `about.md` follows `AGENTS.md` § *Text an end user
  reads* at the first write**, and `bun nv proofs --comments <paths>` is run over them before the wrap.
