---
milestone: M9
position: last
---
# Loop goal 190 — the `nvs:ext@1.0.0` world is written, and a guest call is proven to park and yield inside a Novis coroutine

ADR 0246 decided how an extension crosses, runs and fails, and ADR 0247 decided that the image and
intl components are built in. Neither wrote a line of WIT, and 0246 § 5 rests on one mechanism it could
not take on trust. This goal does those two things before any loader exists:

1. **The async bridge.** In `benches/abi-probe`, behind its `wasm-probe` feature, a guest component
   call made with wasmtime's `call_async` is polled from a stackful coroutine. It parks on a pending
   host import and resumes; a second coroutine on the same thread enters wasm while the first is
   parked; an epoch tick yields to another coroutine; a CPU deadline traps a yielding guest; and after
   each of those a fresh call on the thread succeeds, so wasmtime's thread-local call chain is intact.
2. **The world text.** `wit/nvs-ext/` holds the package `nvs:ext@1.0.0`: the `types`, `log` and
   `settings` interfaces and the `extension` world, which imports them and the WASI 0.2 interfaces
   0246 § 2 allows and nothing else. A test walks every `Core` value class against it.
3. **The image interface.** `wit/image.wit` holds the `Codec` interface with 0120 § 3's eight exports,
   written only in 0246 § 1's types, and a test maps every builder member of 0120 § 2 to an export, a
   plan step or Novis source.

## Why here

**First of the eight M9 goals**, appended behind goal `php-oracle-retired`, which is last on the chain
by the user's decision; it carries `position: last` because every goal it sits behind is pinned. The
parity program, the editor and the closing goals are finished before it, so `Core` is complete and the
walk in Stage 3 reads a roster that will not move under it.

**In front of goal `ext-host`, because the bridge is the riskiest piece** (0246 *Consequences*). Goal
`ext-host` builds its instances and its call path on Stage 2's mechanism, and 0246 *Revisiting*
reopens § 5 only from this spike's evidence. Finding out in a probe costs one crate behind a feature;
finding out inside `nvs-ext` costs a loader built on the wrong model.

**The world before the compiler and the components.** Goal `ext-compiler` registers classes from
manifests whose types are this world's, and goals `ext-image` and `ext-intl` build components against
it. A shape that cannot cross is found here, in a design stage, and not in a port.

## Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that lands the
behaviour and not before:

- `rule:packaging/a-guest-call-yields-on-its-core`'s *Not on disk* paragraph ("proves epoch
  interruption on a plain thread only"), at Stage 2: the bridge is proven in `benches/abi-probe`, and
  no request calls a guest yet.
- `rule:packaging/a-value-crosses-as-its-wit-type`'s ("There is no world file") and
  `rule:packaging/an-nvsx-is-one-file-carrying-its-manifest`'s ("no world file"), at Stage 3: the world
  file exists, and nothing loads or calls through it yet.
- `docs/plan/m9.md`'s paragraph on goal `ext-design`, only if a stage lands something it states
  otherwise.

`docs/ground-rules.md`, the `docs/rules/*.md` chapters and `website/` are generated and are
regenerated, never edited.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. After goal `goal-closeout` that is nothing: the
suites, the `.nvst` trees and `nv verify` are the safety net. Every existing `wasm-probe` test in
`benches/abi-probe/tests/wasm_sandbox.rs` keeps passing; the spike adds beside them and changes none.

## Stage 2 — the async bridge, the keystone

**Does:** Proves in `benches/abi-probe` that a wasmtime async component call, polled from a stackful
coroutine, can park, let a second coroutine enter wasm, yield at an epoch tick and trap at a deadline,
with a fresh call succeeding after each.

One file set: `benches/abi-probe/Cargo.toml`, `benches/abi-probe/src/wasm.rs`, a new
`benches/abi-probe/tests/wasm_async.rs`; read only: `crates/nvs-host/src/scheduler.rs`,
`crates/nvs-host/src/reactor.rs`.

- **The feature's dependencies.** `benches/abi-probe/Cargo.toml:22`'s `wasm-probe` turns on
  wasmtime's component model and async support, and the `wat` text parser for component text, at the
  versions `Cargo.lock` already holds (wasmtime 48, `wat` 1.257). Nothing outside the feature changes,
  so a default `cargo build` still links no wasmtime. Read the feature graph with `cargo tree -e
  features` before a manifest comment states it.
- **The bridge.** A new async probe beside `benches/abi-probe/src/wasm.rs:66`'s `WasmProbe`, built the
  same way (`:95`'s pooling allocator, `:108`'s epoch interruption) plus async support, over a guest
  written as **component WAT text**. A coroutine (`corosensei`, the crate
  `crates/nvs-host/src/scheduler.rs:168` uses) polls the call's future; a `Pending` poll suspends the
  coroutine the way `crates/nvs-host/src/scheduler.rs:1376`'s `suspend_current` parks a task, and the
  waker marks it runnable the way `crates/nvs-host/src/reactor.rs:824`'s `wake_this_task` does. A
  small single-thread loop in the probe runs the coroutines; driving them with `nvs-host`'s real
  scheduler instead (a dev-dependency already) is the session's call, and is preferred when it needs
  no change to `nvs-host`. `rule:packaging/a-guest-call-yields-on-its-core`.
- **A pending host import.** The guest imports one async host function whose future the test
  completes later, so the guest suspends inside wasm with the coroutine parked around it.
- **Two coroutines in wasm on one thread.** The second coroutine starts a call on the same store's
  engine (its own store and instance) while the first is parked with wasm frames on its fiber; both
  finish and both answers are right. This is the composition 0246 § 5 says must be designed, not
  assumed.
- **The epoch tick.** A ticker thread advances the engine's epoch; the store's deadline callback yields
  (`UpdateDeadline::Yield` or its wasmtime 48 equivalent), and a counter proves another coroutine ran
  between two slices of one long guest call.
- **The CPU deadline.** The same callback, past a deadline the test sets, returns a trap; the call
  ends with that trap, the coroutine finishes, and nothing panics.
- **The chain is intact.** After each of the four cases, a fresh call on the same thread returns the
  right answer. Each test ends with that call.
- **Pinned by** the Stage 2 check. The *Not on disk* paragraph of
  `rule:packaging/a-guest-call-yields-on-its-core` is rewritten in the same commit.

## Stage 3 — the world text

**Does:** Writes the `nvs:ext@1.0.0` package and the `extension` world, and tests them against the
`Core` roster and 0246 § 2's import list.

One file set: a new `wit/nvs-ext/` folder, a new `crates/nvs-stdlib/tests/ext_world.rs`,
`crates/nvs-stdlib/Cargo.toml`, the root `Cargo.toml`; read only:
`crates/nvs-stdlib/src/registry.rs`.

- **`types`** (`wit/nvs-ext/types.wit`): `variant error { invalid(string), parse(string),
  runtime(string) }` (0246 § 6); the `value` resource and its accessor functions — kind, scalar reads,
  string and bytes bulk copy, array length and element, shape field (0246 § 2); and **one record per
  `Core` value class that may cross**, each field in 0246 § 1's types.
  `rule:packaging/a-value-crosses-as-its-wit-type`.
- **`log`** (`write(level, message)`) and **`settings`** (`get(key) -> option<setting>`, where
  `setting` is a variant over the six types 0246 § 9 allows: `bool`, `int`, `uint`, `float`, `string`
  and a list of strings).
- **The world `extension`** imports `types`, `log`, `settings` and the WASI 0.2 interfaces 0246 § 2
  allows: `wasi:cli` (environment, exit, stdin, stdout, stderr), `wasi:clocks`, `wasi:random`,
  `wasi:filesystem`, and `wasi:http`'s outgoing handler with the types it is written in. `wasi:io` is
  admitted only as the streams, poll and error interfaces those are written in. The WASI WIT is vendored
  under `wit/nvs-ext/deps/` at the 0.2 release wasmtime 48 implements.
  `rule:packaging/a-guest-has-no-ambient-authority`.
- **The test** (`crates/nvs-stdlib/tests/ext_world.rs`) parses the folder with `wit-parser`, added to
  `[workspace.dependencies]` at the 0.254 that `Cargo.lock` already holds and as a dev-dependency of
  `nvs-stdlib`; it reaches `wit/` through `nvs-repo`, as `crates/nvs-stdlib/Cargo.toml:138` already
  does for other files.
- **The `Core` walk.** A value class is a `CoreClass` with `slots`
  (`crates/nvs-stdlib/src/registry.rs:1569`), read from `CLASSES` (`registry.rs:1685`). Each one either
  has its record in `types.wit` or is on a refusal list in the test, one sentence per entry saying why it
  cannot cross. Any other class fails the test, naming the class. M8's parametric array signatures are
  reused, `uint` as `u64`.
- **The import list.** A second test checks the world imports exactly the allowed interfaces, written
  once as a constant in the test, and fails naming an extra or a missing one.
- **Pinned by** the Stage 3 check. The *Not on disk* paragraphs Stage 0 names for this stage are
  rewritten in the same commit.

## Stage 4 — the image interface against the world

**Does:** Writes the image component's `Codec` interface in the world's types, and tests that every
builder member of 0120 § 2 has a place.

One file set: a new `wit/image.wit`, `crates/nvs-stdlib/tests/ext_world.rs`; read only:
`docs/decisions/0120.md` §§ 2, 3 and 8.

- **The interface.** Package `nvs:image@1.0.0` with interface `codec` and its eight exports
  (`docs/decisions/0120.md:168`): `info`, `run`, `variants`, `compare`, `hash`, `placeholder`,
  `palette`, `qr`. Each returns `result<T, error>` with `nvs:ext/types`' `error`. Its `source`, `plan`,
  `info`, `diff` and option types use only the WIT types 0246 § 1's table gives: no `u8`, `u32` or
  `f32` on its own, `list<u8>` for bytes. A world `image` includes the `extension` world and exports
  `codec`. `rule:core-classes/image-one-entry-point-per-job`, `rule:core-classes/image-pipeline`.
- **An overlay is a pipeline, and WIT has no recursive types.** `composite` takes a whole pipeline
  (`docs/decisions/0120.md:150`). The goal writer's suggestion: a `plan` carries a flat table of
  overlay pipelines, and a `composite` step names one by its index; a nested overlay is an earlier row.
  That keeps 0120's meaning and needs no record.
- **The member map.** A test lists every member of 0120 § 2's table and § 8, each with where it runs:
  an export, a plan step, or Novis source. It checks each named export and plan step exists in the
  parsed interface, and fails naming a member with no place.
- **Record slot (a), here if needed.** `measureText`, `raw`/`fromRaw` and `hashDistance` are in 0120,
  and § 3's eight exports do not carry them. Decide each under § *Standing decisions*.
- **Pinned by** the Stage 4 check.

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

**The goal writer's calls**, not confirmed by the user: the spike's tests are a new
`benches/abi-probe/tests/wasm_async.rs`; the world's WASI dependencies are vendored under
`wit/nvs-ext/deps/`, and `wasi:io` is admitted only as what the allowed interfaces are written in; a
`Core` value class is one with `slots`; the image package is `nvs:image@1.0.0`; the overlay table in
Stage 4.

**At most two record slots**, each one new record and no other number, checked against
`docs/decisions/` right before it is written, because another agent may take a number first:

- **(a)** one, only if the `Core` walk or the image walk changes a decided shape. The case known now:
  where `measureText`, `raw`/`fromRaw` and `hashDistance` run. The goal writer's order of preference,
  not confirmed by the user: Novis source when no codec is needed (`hashDistance` is a Hamming distance
  over two `bytes`); a plan step or an output of `run` when it is one more step of a pipeline (`raw`);
  a new `source` case or a new export only when neither fits (`fromRaw`, `measureText`). A new export
  or a new `source` case changes 0120 § 3 and is the record; Novis source or a plan step does not.
- **(b)** one, only if the spike forces 0246 § 5's fallback, a pinned blocking-pool thread per call.
  It is written from the failing test as its evidence. The fallback reverses the user's call 4, so the
  session then reports `BLOCKED` for the user before goal `ext-host` builds on either model. The tests
  are never softened to pass.

Either record states its tradeoffs. Anything else a session meets is decided and recorded in the
module doc or the WIT file's own comments, never `BLOCKED`.

**Tradeoffs of this goal.** Performance: nothing on the request path; the spike measures nothing and
changes no shipped crate. Memory: nothing at run time; the world text is source. Usability: extension
authors get native types from `wit-bindgen`, and a shape that cannot cross is found before any port.
Simplicity: one new top-level folder, `wit/`, and one dev-dependency, `wit-parser`, already in
`Cargo.lock`.

**House rules for every M9 goal.** A test guest in a Rust test is **component WAT text compiled by the
`wat` crate**, so no wasm toolchain is needed to run `cargo test`; only the built-in components, from
goal `ext-image` on, need `wasm32-wasip2`, which `rust-toolchain.toml` lists. Every new `.nvs` comment
and `about.md` follows `AGENTS.md` § *Text an end user reads*. Every name in a test, a WIT file and a
record is neutral — `Shop`, `Blog`. A debug cargo command never takes `-p`.
