---
milestone: M9
position: last
---
# Loop goal 192 — a program calls an extension's class as it calls a `Core` class, type-checked and with its qualifiers enforced

ADR 0246 § 8 decided that a loaded manifest's class becomes a signature-table entry like a `Core`
class, that its calls are checked by the same qualifier code, and that codegen calls a per-export
trampoline. This goal builds that over goal `ext-host`'s loaded set:

```nvs
use Shop\Ledger;                                   // a class an `[[extension]]` entry loads

int $total = Ledger::sum([3, 4]);                  // checked against the manifest's signature
Ledger::sum("3, 4");                               // does not compile: a string is not array<int>
Ledger::record($note);                             // $note is tainted: does not compile when the manifest declares a sink
```

1. **Registration.** A layer of the signature table the loaded set writes: each manifest's class is a
   `ClassSignature` whose methods carry the manifest's parameter qualifiers in the same `MethodSig`
   fields `Core` uses. A wrong argument type does not compile.
2. **The call.** Codegen emits a direct call to a per-export trampoline, and values convert by 0246
   § 1's table in both directions, every row of it.
3. **Qualifiers.** Contagion, a declared sink refusing `tainted`, a declared source tainting its
   result, and `secret` refused both ways, each a `.nvst` case.
4. **The source section.** Loading makes the extension's namespace resolvable by `autoload` with no
   line written, and a source file outside that namespace refuses the load.
5. **Tools.** `nvs check`, `nvs test` and the language server read manifests from the resolved
   configuration and never instantiate a component.

## Why here

**Behind goal `ext-host`**, which loads a `.nvsx` with every check 0246 § 4 lists, runs a call per
request under the request's budget, maps a failure to its class, and packs a `.nvsx` from WAT. This goal
needs all four: a manifest to register, a call path for the trampoline to reach, `ExtensionError` for
the cases to catch, and the packer for its fixtures.

**In front of goal `ext-grants`**, whose adversarial suite calls extensions from programs (a file read
outside the grant, a runaway loop, a reload that adds an extension) and so needs this goal's call path
and its conformance fixtures. **In front of goal `ext-tooling`**, whose `nvs ext test` runs `.nvs`
tests against a built file through this goal's compiler path.

It carries `position: last` because every goal it sits behind is pinned.

## Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that lands the
behaviour and not before:

- The *Not on disk* paragraph of `rule:packaging/extension-calls-are-statically-typed` (Stage 2); of
  `rule:packaging/a-value-crosses-as-its-wit-type` and `rule:packaging/values-cross-as-handles`
  (Stage 3); of `rule:security/extension-contagion`, `rule:security/extension-declares-sink-or-source`,
  `rule:security/extension-manifest-only-tightens`, `rule:security/extension-cannot-launder` and
  `rule:security/secret-does-not-cross-an-extension` (Stage 4); of
  `rule:packaging/an-extension-package-carries-two-payloads` (Stage 5).
- `crates/nvs-types/src/signatures.rs:740`'s comment ("`Core` first, so a user declaration can never be
  collected under a name the stdlib already owns") gains the extension layer when Stage 2 seeds it.
- `crates/nvs-test`'s module doc, which lists the sections a case may carry, gains Stage 2's section.

`docs/ground-rules.md`, the `docs/rules/*.md` chapters and `website/` are generated and are
regenerated, never edited.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. After goal `goal-closeout` that is nothing: the
suites, the `.nvst` trees and `nv verify` are the safety net. Goal `ext-host`'s checks pass. A program
under a configuration with no `[[extension]]` entry compiles to the same code as before, and every
`Core` call is checked exactly as before.

## Stage 2 — registration, the keystone

**Does:** Registers each loaded manifest's class in the signature table, so a call to it is
type-checked like a `Core` call, and lets a conformance case load a fixture extension.

Two file sets, in this order. The fixture and the runner: `crates/nvs-test/src/case.rs`,
`crates/nvs-test/src/run.rs`, a new `tests/conformance/ext/fixtures/`, a new
`crates/nvs-ext/tests/fixtures.rs`. The checker: `crates/nvs-types/src/signatures.rs`,
`crates/nvs-types/src/core_lib.rs`, a new `crates/nvs-types/src/ext_lib.rs`, the compile entry that
hands the loaded set to the checker.

- **The fixture extension.** One fixture, with a neutral name (`Shop\Ledger`), written as component
  WAT text in `tests/conformance/ext/fixtures/` and packed by `nvs-ext`'s packer into a committed
  `.nvsx` beside it. Its exports cover what the stages below need: every row of 0246 § 1's table, a
  parameter declared a sink, a return declared a source, an `err` of each case, a trap, and a source
  section. A stage that needs a load refusal adds a small fixture of its own that does not load
  (Stage 4's `secret` return, Stage 5's file outside the namespace). A Rust test fails when a
  committed fixture differs from what the packer builds from its text, so a fixture never goes stale.
- **A case loads a fixture.** A new section `--EXTENSION--` lists fixture names, one per line. The
  runner (`crates/nvs-test/src/case.rs:323`'s `KNOWN`, `crates/nvs-test/src/run.rs:192`'s
  `run_case`) copies each into the case's working directory and adds its `[[extension]]` entry, with its
  pin, to the case's `nvs.toml`, creating the file when the case wrote none. A case never spells a
  digest. A name with no fixture is a parse error naming it.
- **The layer.** A new `crates/nvs-types/src/ext_lib.rs`, the shape `error_lib.rs` and `iter_lib.rs`
  are, seeds the loaded set's classes after `iter_lib::seed` at
  `crates/nvs-types/src/signatures.rs:742`. Each manifest class is a `ClassSignature`
  (`crates/nvs-types/src/signatures.rs:448`) whose `MethodSig`s (`signatures.rs:58`) carry each
  parameter's qualifier in `param_quals`, exactly where `crates/nvs-types/src/core_lib.rs:135` puts a
  `Core` member's, and each `const` member with its type. Types come from the manifest's Novis
  signature, never from the WIT. `rule:packaging/extension-calls-are-statically-typed`.
- **The checker reads no extension branch.** `crates/nvs-types/src/expr/args.rs:117` and
  `crates/nvs-types/src/expr/calls.rs:228` and `:568` read the registered signature and do not know
  where it came from (`rule:security/extension-manifest-only-tightens`).
- **The set reaches the compiler.** The compile entry the CLI and the runner use reads the manifests of
  the resolved configuration's set (goal `ext-host`'s manifest model, no engine) and hands them to the
  checker. An extension class used when the set does not load it is unknown, and the program does not
  compile.
- **Pinned by** the Stage 2 checks.

## Stage 3 — the call

**Does:** Emits a direct call to a per-export trampoline, and converts every row of 0246 § 1's table in
both directions.

Two file sets, in this order. The emit: `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-codegen/src/lib.rs`. The conversion: `crates/nvs-ext/src/` (the trampolines and the
conversion), `crates/nvs-ext/tests/convert.rs`, reading values through `nvs-runtime`'s accessors.

- **The emit.** A static call to an extension method lowers as `crates/nvs-ir/src/lower/expr.rs:1139`
  lowers a `Core` call: a direct call to a symbol, here the export's trampoline. The trampolines are
  bound beside the runtime's and `Core`'s symbols at `crates/nvs-codegen/src/lib.rs:1954`. The set is in
  `env_hash`, so an artifact never outlives the trampolines it calls
  (`rule:config/the-extension-set-is-in-every-unit-key`).
- **The conversion**, both ways, one row at a time (`rule:packaging/a-value-crosses-as-its-wit-type`):
  `bool`, `int` as `s64`, `uint` as `u64` with no conversion, `float`, `string` (UTF-8 both ways),
  `bytes` as one bulk copy, `array<T>` as `list<T>`, `array<K, V>` as `list<tuple<K, V>>` in order, a
  shape as a `record` with an optional field as an `option`, an enum in kebab-case, `?T` as
  `option<T>`, a closed union of shapes as a `variant`, a `Core` value class as its `types` record,
  `resource` as the extension's resource dropped at request end, and `mixed` as a `borrow<value>` read
  through the accessor imports (`rule:packaging/values-cross-as-handles`).
- **Where the conversion lives.** `nvs-ext` owns wasmtime and `nvs-runtime` owns the value
  representation; the conversion is in `nvs-ext` and reads values through `nvs-runtime`'s public
  accessors. A seam that has to move into `nvs-runtime` moves, and the module doc says why.
- **End to end.** Two cases call the fixture through a program: every row round-trips, and an `err`
  throws its class while a trap throws `ExtensionError` and the next call succeeds.
- **Pinned by** the Stage 3 checks.

## Stage 4 — qualifiers

**Does:** Enforces contagion, a declared sink, a declared source and the `secret` refusal at an
extension call, by the same code a `Core` call uses.

One file set: `crates/nvs-types/src/expr/quals.rs`, `crates/nvs-types/src/expr/args.rs`,
`crates/nvs-types/src/expr/calls.rs`, the manifest model's qualifier fields, and the cases.

- **Contagion.** A `tainted` argument makes every `string` and `bytes` in the result `tainted`, through
  `crates/nvs-types/src/expr/quals.rs:256`'s `tainted_result` at `crates/nvs-types/src/expr/calls.rs:228`
  and `:568`, with nothing in the manifest. `rule:security/extension-contagion`.
- **A sink.** A parameter the manifest declares a sink refuses a `tainted` argument, through
  `crates/nvs-types/src/expr/quals.rs:312`'s `admits_tainted_argument`.
  `rule:security/extension-declares-sink-or-source`.
- **A source.** A return the manifest declares a source is `tainted` even when every argument was
  plain.
- **`secret`, both ways.** A `secret` argument does not compile
  (`crates/nvs-types/src/expr/quals.rs:342`'s `admits_secret_argument`), and a manifest declaring a
  `secret` return does not load. `rule:security/secret-does-not-cross-an-extension`.
- **Nothing launders.** A manifest field that would admit `tainted` or remove it from a result does
  not exist, and a manifest naming an unknown qualifier does not load.
  `rule:security/extension-cannot-launder`.
- **Pinned by** the Stage 4 checks: a `.nvst` case for each behaviour, and the load refusals in
  `nvs-ext`.

## Stage 5 — the source section

**Does:** Makes the extension's Novis source resolvable by `autoload` with no line written, and refuses
a load whose source leaves the extension's namespace.

One file set: `crates/nvs-hir/src/autoload.rs`, the compile entry Stage 2 touched, `nvs-ext`'s loader.

- **Resolvable with no line.** Loading the extension adds its namespace to the program's
  `crates/nvs-hir/src/autoload.rs:259` `AutoloadMap`, with the `nvs.source` files as the root. A class
  in that source resolves and compiles like any other file
  (`rule:packaging/an-extension-package-carries-two-payloads`). Its files hold the authority of their
  own namespace, which is no grant unless the operator writes one.
- **A file outside the namespace** refuses the load, naming the entry and the file
  (0246 § 3).
- **A diagnostic in a source file** names it by the extension and its relative path, so a reader can
  find it with `nvs ext inspect --source`. The exact form is the session's call, written in the module
  doc.
- **Pinned by** the Stage 5 checks.

## Stage 6 — the tools

**Does:** Makes `nvs check`, `nvs test` and the language server read the manifests of the resolved
configuration's set, and never instantiate a component.

Two file sets. The CLI: `crates/nvs-cli/src/main.rs` (`Command::Check` at `:1473`, `Command::Test` at
`:1505`), `crates/nvs-cli/tests/check_command.rs`, `crates/nvs-cli/tests/test_command.rs`. The editor:
`crates/nvs-lsp/src/completion.rs`, `crates/nvs-lsp/src/server.rs`.

- **`nvs check` and `nvs test`** type an extension call from the configured set. A fixture whose
  instantiation would trap proves neither instantiates it.
- **The language server** offers an extension class's methods in completion
  (`crates/nvs-lsp/src/completion.rs:679`'s `class_names` and the member list) and shows the
  manifest's signature and help text on hover (`crates/nvs-lsp/src/server.rs:1128`), read from the
  workspace's resolved configuration.
- **Pinned by** the Stage 6 checks.

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

**The goal writer's calls**, not confirmed by the user: the fixture is committed `.nvsx` bytes beside
its WAT text, held to its text by a Rust test; a case loads one through a new `--EXTENSION--` section
that writes the pinned entry for it; the extension layer is a new `ext_lib.rs` seeded after `Core`;
the value conversion lives in `nvs-ext`.

**No record slot.** ADR 0246 and its rules decide everything this goal builds. A question a session
meets is decided under the priority ordering and recorded in the module doc of the crate it touches,
never `BLOCKED` and never a new record. **No new diagnostic code** unless a refusal cannot be made true
under an existing one: a wrong argument, a sink and a `secret` argument reuse the codes a `Core` call
gives.

**Tradeoffs of this goal.** Performance: an extension call is a direct call to a trampoline, about
11 ns plus the copy of its arguments (0246's figures, not re-measured here); a program that calls no
extension compiles to the same code as before. Memory: one signature per loaded class, per process,
and each call's arguments copied whole into the guest. Usability: an extension call is checked like a
`Core` call, its methods complete and hover in the editor, and its builder source needs no `autoload`
line. Simplicity: no second analysis and no extension branch in the checker; one new case section.

**House rules for every M9 goal.** A test guest in a Rust test is **component WAT text compiled by the
`wat` crate**, so no wasm toolchain is needed to run `cargo test`; only the built-in components, from
goal `ext-image` on, need `wasm32-wasip2`, which `rust-toolchain.toml` lists. Every new `.nvs` comment
and `about.md` follows `AGENTS.md` § *Text an end user reads*. Every name in a test, a fixture and a
record is neutral — `Shop`, `Blog`. A debug cargo command never takes `-p`.
