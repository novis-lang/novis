---
milestone: M9
position: last
---
# Loop goal 191 — an extension loads under its pin, runs once per request on its request's core, and stays inside that request's budget

ADR 0246 §§ 3–6 decided what loading checks, how a call runs and what a failure looks like. This goal
builds that as a new crate, `crates/nvs-ext`, driven through its Rust API with Novis values: the
compiler path that lets a program call it is goal `ext-compiler`'s, and the grants are goal
`ext-grants`'s.

1. **A pin is required.** An `[[extension]]` entry needs `path` and a 64-hex `sha256`, refused at boot
   and reload with its file and line. Loading then reads the file, checks the digest, validates the
   component, parses both custom sections, checks the manifest against the exports, and refuses an
   import outside the world, a `Core\` or `Novis\` class, a duplicate class and a version outside `1.x`.
2. **A packer.** A library function turns a wasm module or component, a manifest and source files
   into a `.nvsx`. The tests here use it; `nvs ext build` and the built-in components' build script
   use it later.
3. **One instance per extension per request, on its core, under its budget.** The call is
   `call_async` polled by the request's coroutine; an epoch tick yields or traps at the request's CPU
   deadline; linear memory growth is charged to the request.
4. **Failures.** An `err` throws its mapped class, a trap throws the new `ExtensionError` and the next
   call gets a fresh instance, and a limit is a resource-limit `FATAL` that reaches `onLimit`.
5. **Reload and the module cache.** Reload verifies every entry and refuses the swap whole; a compiled
   component is cached in the artifact cache under its digest and the environment.

## Why here

**Behind goal `ext-design`**, whose Stage 2 proved the async bridge in `benches/abi-probe` and whose
Stage 3 wrote the `nvs:ext@1.0.0` world. Stage 4 here is that spike moved into a crate the server
links, and Stage 2's load checks read that world's import list. If goal `ext-design` wrote a record
under its slot (b), that record decides Stage 4's call model and this file's Stage 4 is read through
it.

**In front of goal `ext-compiler`**, which registers a loaded manifest's class in the signature table
and emits a trampoline into this crate's call path: both need a loaded set and a call that works. **In
front of goal `ext-grants`**, which links WASI into the instances this goal creates; until then a
component that imports WASI does not load, which is the right failure for an empty grant.

It carries `position: last` because every goal it sits behind is pinned.

## Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that lands the
behaviour and not before:

- The *Not on disk* paragraphs of `rule:packaging/extension-loading-is-root-controlled` and
  `rule:packaging/an-nvsx-is-one-file-carrying-its-manifest` (Stage 2);
  `rule:packaging/an-extension-is-a-sandboxed-wasm-component`,
  `rule:packaging/a-guest-call-yields-on-its-core` and
  `rule:packaging/a-guest-runs-under-the-requests-budget` (Stage 4);
  `rule:packaging/a-guest-crash-throws` (Stage 5).
- `crates/nvs-config/src/tree.rs:419`'s doc comments on `Extension`, whose fields become required
  (Stage 2).
- `crates/nvs-config/src/default.toml:1006`'s `[[extension]]` block: it says "a native extension", its
  example path is `image.nvsx`, which is now a built-in component, and it does not say the pin is
  required. Its key lines keep their `# example` endings (Stage 2).
- `docs/reference/tools/20-config.md:63` and `:375`, where they name `[[extension]]`, if a stage
  changes what they say.
- `docs/reference/lang/70-errors.md:5` and `:20`, which list the classes under `RuntimeError`; they
  gain `ExtensionError` (Stage 5).

`docs/ground-rules.md`, the `docs/rules/*.md` chapters and `website/` are generated and are
regenerated, never edited.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. After goal `goal-closeout` that is nothing: the
suites, the `.nvst` trees and `nv verify` are the safety net. Goal `ext-design`'s `wasm-probe` tests
keep passing. A configuration with no `[[extension]]` entry boots, compiles and serves exactly as
before, and a request that calls no extension pays nothing.

## Stage 2 — the pin and the load refusals

**Does:** Makes `path` and a 64-hex `sha256` required on every `[[extension]]` entry, and loads a
`.nvsx` with every check 0246 §§ 3–4 lists.

Two file sets, in this order. The configuration: `crates/nvs-config/src/tree.rs`, a new
`crates/nvs-config/src/extension.rs`, `crates/nvs-config/src/resolve.rs`,
`crates/nvs-config/tests/tree.rs`, `crates/nvs-config/src/default.toml`. The loader: a new
`crates/nvs-ext` (`Cargo.toml`, `src/lib.rs`, `src/load.rs`, `src/manifest.rs`, `tests/load.rs`).

- **The entry.** `crates/nvs-config/src/tree.rs:419`'s `Extension` keeps `path` and `sha256`, gains
  the optional `memory` ceiling `rule:packaging/a-guest-runs-under-the-requests-budget` names (a size,
  written as `[limits] memory` is), and leaves `grants` to goal `ext-grants`.
  `rule:packaging/extension-loading-is-root-controlled`.
- **The refusal.** A new `crates/nvs-config/src/extension.rs` has `validate(config, origins)`, the
  shape `crates/nvs-config/src/queue.rs:81`'s is, called beside the others at
  `crates/nvs-config/src/resolve.rs:384`. An entry without `path`, without `sha256`, or with a pin that
  is not 64 hexadecimal digits is refused, naming its file and line. Codes come from the configuration
  band's next free numbers.
- **The test that used a bad pin.** `crates/nvs-config/tests/tree.rs:44` writes `sha256 = "abc"`; it
  gets a 64-hex pin, because a test that parses a refused entry pins nothing.
- **The crate.** `crates/nvs-ext` is a workspace member through the `crates/*` glob. It depends on
  wasmtime (component model, async, pooling allocator), `sha2` and `serde_json` from the workspace.
- **The manifest model never links wasmtime.** The manifest's Rust type, its JSON parser and the
  reader of the two custom sections sit where `nvs-types` can depend on them without wasmtime, because
  goal `ext-compiler`'s checker and the language server read manifests and never instantiate. Where
  that is — a module of `nvs-ext` behind no engine, or a small crate — is the session's call, written
  in the module doc.
- **Load, in order, each refusal naming the entry** (0246 § 4): read the file; its SHA-256 against the
  pin; the component validates; `nvs.manifest` is present and parses as `{"manifest": 1, ...}`;
  `nvs.source` parses as a list of relative paths and bytes; every export the manifest names exists
  with the WIT type 0246 § 1's table gives each Novis type, and a Novis type outside the table is
  refused; every import is in the world `ext-design` wrote, and an import outside it is refused naming
  it; the class is not under `Core\` (`rule:core-api/core-means-always-present`) or `Novis\`; no other
  loaded entry declares the class; the world version is `1.x` with `x` no newer than the host's, and a
  refusal names both versions. `rule:packaging/an-nvsx-is-one-file-carrying-its-manifest`.
- **The test guests** are component WAT text compiled by the `wat` crate, with the two custom sections
  added by a test helper (a custom section is valid at a component's top level). Stage 3's packer is
  the production path and replaces the helper where it can.
- **Pinned by** the Stage 2 checks.

## Stage 3 — the packer

**Does:** Adds the library function that turns a module or component, a manifest and source files
into a `.nvsx` that loads.

One file set: `crates/nvs-ext/src/pack.rs`, `crates/nvs-ext/tests/pack.rs`, the root `Cargo.toml`.

- **The function** takes a core wasm module or a component, the manifest JSON, and the source files as
  `(relative path, bytes)`, and returns the `.nvsx` bytes. A core module is componentized with
  `wit-component` (added to `[workspace.dependencies]` at the 0.254 `Cargo.lock` holds) against the
  `extension` world and the manifest's exported interface. `rule:packaging/nvs-ext-is-the-authoring-tool`.
- **The output loads.** Every file it writes passes Stage 2's load, and the tests prove it with one
  module and one component.
- **Deterministic.** The same inputs give the same bytes, so a pin written once stays true; source
  files are written in path order.
- **A source path that leaves the project** (`..`, an absolute path) is refused by the packer. The load
  refusal for a file outside the extension's namespace is goal `ext-compiler`'s.
- **Pinned by** the Stage 3 check.

## Stage 4 — instances, the call bridge and the budget

**Does:** Runs a guest call on its request's core as an async call polled by the request's coroutine,
with one instance per extension per request, and charges the call's CPU time and memory to that
request.

One file set: `crates/nvs-ext/src/` (the engine, the per-request instances, the call),
`crates/nvs-ext/tests/call.rs`; read and touched where the seam needs it:
`crates/nvs-runtime/src/ctx/limits.rs`, `crates/nvs-runtime/src/budget.rs`,
`crates/nvs-host/src/scheduler.rs`, `crates/nvs-host/src/reactor.rs`,
`crates/nvs-host/src/watchdog.rs`. The reference is goal `ext-design`'s
`benches/abi-probe/tests/wasm_async.rs`.

- **One engine per process.** Pooling allocator, epoch interruption and async support, as
  `benches/abi-probe/src/wasm.rs:95` builds them; one epoch ticker thread advances it about once a
  millisecond. A compiled component is shared by every core. `rule:packaging/a-guest-call-yields-on-its-core`.
- **One instance per extension per request.** Created at the request's first call to that extension
  and dropped when the request ends (`rule:packaging/a-fresh-instance-per-request`). The per-request
  state hangs off the request and is dropped where a request's end is, which is
  `crates/nvs-host/src/isolate.rs:1155`'s `finish` and `nvs run`'s root task, not the connection's
  context. A request that calls no extension creates
  nothing. A second task of the same request calling the same extension waits until the first call
  returns.
- **The call.** `call_async`'s future is polled by the request's coroutine. A `Pending` poll parks the
  task as `crates/nvs-host/src/scheduler.rs:1376`'s `suspend_current` does, and the waker is the
  core's `RemoteWake` (`crates/nvs-host/src/reactor.rs:252`, `:824`).
- **CPU.** The store's epoch deadline callback reads the request's CPU limit
  (`crates/nvs-runtime/src/ctx/limits.rs:495`): past the deadline the call traps as a CPU limit;
  before it the call yields so the core's other tasks run. A guest is outside
  `crates/nvs-host/src/watchdog.rs:641`'s safepoint sweep, so the callback is its CPU check, and the
  guest's time counts toward the request's CPU time the way compiled code's does.
- **Memory.** A `ResourceLimiter` charges each linear memory growth to the request
  (`crates/nvs-runtime/src/budget.rs:383`'s `charge_memory`), and the store's limit is the least of
  what the request has left (`crates/nvs-runtime/src/ctx/limits.rs:35`, `:126`), the entry's `memory`
  and the manifest's declared maximum. A pooling-allocator memory is not allocated through the
  accounting allocator (`budget.rs:810`), so the limiter is the only place it is seen.
  `rule:packaging/a-guest-runs-under-the-requests-budget`.
- **Pinned by** the Stage 4 check, whose tests run under `nvs-host`'s real scheduler.

## Stage 5 — failures

**Does:** Makes a guest's `err` throw its mapped class, a trap throw `ExtensionError` with the next
call getting a fresh instance, and a limit end the request as a resource-limit `FATAL`.

Two file sets. The class: `crates/nvs-hir/src/errors.rs`, `crates/nvs-runtime/src/throwable.rs`,
`docs/reference/lang/70-errors.md`. The mapping: `crates/nvs-ext/src/` (the call),
`crates/nvs-ext/tests/failure.rs`, `crates/nvs-runtime/src/ctx/limits.rs`.

- **`ExtensionError`** joins `crates/nvs-hir/src/errors.rs:96`'s `TREE` under `RuntimeError` and
  `crates/nvs-runtime/src/throwable.rs:318`'s `ThrownClass`, and the reference chapter names it with
  the other classes under `RuntimeError`. It declares no property of its own; its message names the
  extension, the export and the trap. `rule:packaging/a-guest-crash-throws`, `rule:errors/throwable-hierarchy`.
- **An `err`** throws by its case: `invalid` → `LogicError`, `parse` → `ParseError`, `runtime` →
  `RuntimeError`, each with the guest's message.
- **A trap that is not a limit** (a panic, `unreachable`, an out-of-bounds access in the guest's own
  memory, invalid UTF-8 in a returned string) throws `ExtensionError`. The trapped instance is
  discarded, and the next call in the same request gets a fresh one.
- **A limit** reached inside a guest — the CPU deadline or the memory limit — ends the request with the
  same resource-limit `FATAL` the runtime raises for compiled code
  (`crates/nvs-runtime/src/ctx/limits.rs:338`'s `memory_breach` is the memory one), so it reaches
  `onLimit` (`rule:errors/on-limit`) and no `catch`.
- **Pinned by** the Stage 5 checks.

## Stage 6 — reload and the module cache

**Does:** Loads the extension set at boot and at reload, refuses a reload whole when one entry fails,
and stores compiled components in the artifact cache under their digest and the environment.

Two file sets. The cache: `crates/nvs-ext/src/` (the module cache seam),
`crates/nvs-ext/tests/cache.rs`, `crates/nvs-cli/src/cache.rs`, `crates/nvs-config/src/cache.rs`. The
process: `crates/nvs-cli/src/control.rs`, `crates/nvs-cli/src/serve.rs`,
`crates/nvs-cli/tests/live_config.rs`, `crates/nvs-cli/tests/compile_cache.rs`.

- **Boot and reload load the set.** Boot loads every entry through Stage 2 and refuses to start on a
  failure. Reload (`crates/nvs-cli/src/control.rs:602`) verifies every entry before the swap and
  refuses the whole swap if one fails, so the previous set stays live — 0078's shape.
- **The environment moves with the set.** The pins already fold into `env_hash`
  (`crates/nvs-config/src/cache.rs:636`, read at `crates/nvs-cli/src/control.rs:495`), so a unit
  compiled under one set is never reused under another. A test pins that with real files.
- **The module cache.** A compiled component is serialized into the artifact cache keyed by the
  component's digest and the environment (`rule:packaging/a-wasm-module-cache-reuses-the-artifact-cache`).
  The cache writer is `crates/nvs-cli/src/cache.rs`'s, and `nvs-ext` cannot name `nvs-cli`, so the
  crate takes a seam the CLI fills. A bad entry is a miss, never an error
  (`rule:packaging/a-bad-cache-entry-is-a-miss-never-an-error`).
- **Pinned by** the Stage 6 checks. Calling an extension from a program after a reload is goal
  `ext-grants`' adversarial suite, because it needs goal `ext-compiler`'s call path.

## Standing decisions

**The user's calls**, for all eight M9 goals. They are not re-decided here or in any session.

1. Values cross as typed WIT values by 0246 § 1's table; the `value` handle is only for `mixed`.
2. A guest links WASI with an empty context. It may be granted files (preopens) and outbound HTTP
   (`wasi:http` through `Core\Http\Client`) and nothing else.
3. Grants are the intersection of the `[[extension]]` entry's `grants`, the manifest's request, and
   the caller's own effective set.
4. A guest call runs on its request's core as a wasmtime async call polled by the coroutine, yielding
   at every epoch tick (about 1 ms). There is no compute pool.
5. The Novis source half travels inside the `.nvsx` (`nvs.source` section), covered by the one pin.
6. `sha256` is required on every entry. There are no signatures in M9; M15 owns them.
7. A trap throws `ExtensionError` (extends `RuntimeError`); a CPU or memory limit is a resource-limit
   `FATAL`.
8. Intl covers the web-app set of 0247 § 5; MessageFormat is out.
9. `nvs ext new` ships Rust and C templates only.
10. `Novis\Image` and `Novis\Intl` are built into every binary and always on; nothing turns them off.
11. The build compiles the component crates for `wasm32-wasip2`; libwebp is prebuilt by a `bun nv`
    tool and committed beside its source hash, and CI rebuilds and compares.

**The record writer's calls**, standing but not confirmed by the user: the manifest is JSON written
from an `nvsx.toml`; `Novis\` is reserved; the error variant `invalid|parse|runtime` maps to
`LogicError|ParseError|RuntimeError`; one instance per extension per request, and a second task waits;
a third-party settings block is `[ext.<name>]`; `nvs check` and the language server read manifests and
never instantiate; a bundle embeds the `.nvsx` files the build's configuration lists.

**The goal writer's calls**, not confirmed by the user: the manifest model and the section reader
never link wasmtime (Stage 2); Stage 2's tests add the custom sections with a helper until Stage 3's
packer exists; the module cache is a seam `nvs-cli` fills (Stage 6); a program calling an extension
after a reload is proven by goal `ext-grants`, not here.

**No record slot.** Everything this goal builds is decided by ADR 0246 and its rules. A question a
session meets is decided under the priority ordering and recorded in `crates/nvs-ext`'s module doc,
never `BLOCKED` and never a new record. The `[ext.<name>]` settings block and the `nvs:ext/log` and
`nvs:ext/settings` imports are linked here only if a stage needs them to load a test guest; otherwise
they are goal `ext-grants`'s, with the rest of the linker.

**Tradeoffs of this goal.** Performance: a request that calls no extension pays nothing; a call costs
about 11 ns plus the copy, a request's first call to an extension about 8–23 µs of instantiation
(0246's figures, not re-measured here), and an epoch tick one check per millisecond of guest time.
Memory: a guest's linear memory is charged to its request and freed with it, O(in-flight), under the
request's own cap; the pooling allocator reserves address space per slot, not committed memory; one
compiled component per process, shared by every core. Usability: a bad pin, a wrong digest or an
unoffered import is a boot or reload error naming the line, and a crashing extension is a `catch`
away from a `422`. Simplicity: one new crate, one new global class, one required key and one optional
one.

**House rules for every M9 goal.** A test guest in a Rust test is **component WAT text compiled by the
`wat` crate**, so no wasm toolchain is needed to run `cargo test`; only the built-in components, from
goal `ext-image` on, need `wasm32-wasip2`, which `rust-toolchain.toml` lists. Every new `.nvs` comment
and `about.md` follows `AGENTS.md` § *Text an end user reads*. Every name in a test, a fixture and a
record is neutral — `Shop`, `Blog`. A debug cargo command never takes `-p`.
