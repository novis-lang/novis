---
milestone: M9
position: last
---
# Loop goal 194 — `nvs ext` builds, inspects, tests and pins an extension, and a bundle carries one

ADR 0246 § 10 makes `nvs ext` the one tool an extension author needs beside their language's
compiler, and makes `nvs build --compile` embed the extensions a program's configuration lists. This
goal builds both, and proves the two templates on all three platforms:

```text
nvs ext new --lang rust geo       # a project that builds a component against nvs:ext@1.0.0
cargo build --release --target wasm32-wasip2
nvs ext build                     # component + manifest + Novis source -> geo.nvsx, checked to load
nvs ext test                      # the project's .nvs tests, with geo.nvsx loaded and no nvs.toml
nvs ext inspect geo.nvsx          # the manifest and the I/O it requests; --source prints the source
nvs ext verify geo.nvsx           # every check a boot makes, with nothing instantiated
nvs ext pin geo.nvsx              # the [[extension]] entry, with its sha256, ready to paste
```

1. **`new` and `build`**, and the `nvsx.toml` an author writes.
2. **`inspect`, `verify` and `pin`.**
3. **`test`.**
4. **A bundle carries its extensions.** `nvs build --compile` embeds each `.nvsx` the build's
   configuration lists, with its pin, and the bundle loads them as the entries would.
5. **A CI job** builds, loads and tests both templates on Linux, Windows and macOS.
6. **The help and the feature proofs** of every `nvs ext` command.

## Why here

`nvs ext build` is goal `ext-host`'s packer behind a command, `verify` is its load checks, `test` runs a
program that calls an extension, which goal `ext-compiler` makes possible, and `inspect` prints the I/O
request goal `ext-grants` defines. So this goal is the last of the five ADR 0246 is validated by. It
sits in front of goal `ext-image` because that goal's build script packs the built-in components with
the same packer, and the `wasm32-wasip2` target this goal adds to `rust-toolchain.toml` is the one that
build needs. It carries `position: last` because every goal it lands behind is pinned.

What is on disk as this goal is written: `nvs` has no `ext` subcommand (`crates/nvs-cli/src/main.rs:270`'s
`Command`). A bundle's payload is a flat list of `(path, text)` source entries
(`crates/nvs-cli/src/bundle.rs:286`), `nvs build --compile` reads no configuration
(`crates/nvs-cli/src/main.rs:1540` calls `bundle::build(&file, output)`), and the gap
`data/gaps/nvs-cli/nvsx-entries-are-not-embedded-yet.json` records that `.nvsx` entries are not
embedded. The only CI job that builds wasmtime is `extension-sandbox` (`.github/workflows/ci.yml:252`),
gated by the `probe` lane of `tools/nv/cmd/ci-changes.ts:58`. `rust-toolchain.toml` lists three host
targets and no wasm target. `rule:packaging/nvs-ext-is-the-authoring-tool` and
`rule:packaging/a-nvsx-dependency-embeds-in-the-same-payload` both say **Not on disk**.

## Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that makes it untrue:

- `docs/rules/packaging/nvs-ext-is-the-authoring-tool.md` loses **Not on disk** at Stage 4, and
  `docs/rules/packaging/a-nvsx-dependency-embeds-in-the-same-payload.md` at Stage 5. Each record's
  `guardedBy` names the tests of that stage.
- `crates/nvs-cli/src/bundle.rs`'s module doc (lines 1-60) — "source, not artifacts" and the layout of
  the manifest gain the extension entry, and § *Known gap* loses its gap.
- `data/gaps/nvs-cli/nvsx-entries-are-not-embedded-yet.json` — deleted by the session that closes it,
  in Stage 5.
- `docs/reference/tools/10-cli.md` § *nvs build --compile* (line 273) — what a bundle carries.
- `.github/workflows/ci.yml`'s comment above `extension-sandbox` (line 248) says wasmtime is paid for
  "before M9"; it is rewritten when the new job lands beside it.

`docs/ground-rules.md`, the `docs/rules/*.md` chapters, `docs/novis.md` and `website/` are generated
and are regenerated, never edited.

## Stage 1 — the floor

None is carried: after goal `goal-closeout` the suites, the `.nvst` trees and `nv verify` are the
safety net. Every check of goals `ext-host`, `ext-compiler` and `ext-grants` stays green as a test of
the tree, and a session that turns one red repairs it in the same slice.

## Stage 2 — `new`, `build` and `nvsx.toml`, the keystone

**Does:** Adds `nvs ext new` with Rust and C templates and `nvs ext build`, which packs a built module
with its manifest and its Novis source into one checked `.nvsx`.

One file set: `crates/nvs-cli/src/main.rs:270` (`Command`) and `:1533` (the dispatch),
`crates/nvs-cli/src/ext.rs` (new), `crates/nvs-cli/templates/ext/rust/` and
`crates/nvs-cli/templates/ext/c/` (new), `rust-toolchain.toml`, goal `ext-host`'s packer in
`crates/nvs-ext/src/`, `crates/nvs-cli/tests/ext_command.rs` (new).

- **The command.** `Command::Ext` with six subcommands, `new`, `build`, `inspect`, `test`, `verify` and
  `pin`, each with the help line `rule:packaging/nvs-ext-is-the-authoring-tool`'s table gives it. The
  ones a later stage builds print that they are not built yet and exit non-zero until it lands.
- **`nvsx.toml`.** One key for each field of the manifest (ADR 0246 § 3): the class, the exported
  interface, the path of the built module, the folder of the Novis source, each method's signature
  written as Novis text with its qualifiers and its help text, the class's constants, the settings
  block, the I/O it requests (as goal `ext-grants` defined it) and the largest linear memory. An
  unknown key is refused with the file and the line. The manifest it writes is the JSON
  `{"manifest": 1, ...}` goal `ext-host`'s loader reads, and nothing else writes one.
- **`nvs ext new --lang rust|c <dir>`** writes a project into an empty folder, or refuses a folder
  that is not empty. Both templates hold an `nvsx.toml`, the world's WIT copied from the binary's
  own `wit/` folder, one exported function, one Novis source file and one `.nvs` test, with neutral
  names. The Rust template builds with the `wit-bindgen` crate and has an empty `[workspace]` table,
  so a project inside another workspace builds alone. The C template builds with wasi-sdk's `clang`
  for `wasm32-wasip2` after `wit-bindgen c`, and its two build commands are written in one place that
  Stage 6's tool reads.
- **`wasm32-wasip2`** joins `rust-toolchain.toml`'s `targets`, if no earlier goal added it.
- **`nvs ext build`** reads `nvsx.toml`, componentizes a core module with `wit-component` or takes a
  component as it is, writes `nvs.manifest` and `nvs.source`, writes the `.nvsx`, and then runs every
  load check boot runs (the function Stage 3's `verify` calls). It prints the file's path and its pin.
  A WASI preview 1 module is refused, and the message names `wasm32-wasip2`.
- **Pinned by** the Stage 2 checks. The test guests are WAT modules compiled by the `wat` crate; the
  real templates are built by Stage 6's tool.

## Stage 3 — `inspect`, `verify` and `pin`

**Does:** Adds the three commands that read a `.nvsx` without running it: what it declares, whether
it loads, and the entry that pins it.

One file set: `crates/nvs-cli/src/ext.rs`, goal `ext-host`'s load checks in `crates/nvs-ext/src/`,
`crates/nvs-cli/tests/ext_command.rs`, `crates/nvs-cli/src/config.rs:1` (where `nvs config check`
prints a refusal).

- **`nvs ext inspect <file>`** prints the class, each method's Novis signature, the settings block,
  the I/O the manifest requests and its largest memory. `--source` adds each source file under its
  path. Every string from the file is printed with control characters escaped, because a manifest is
  text an author wrote and a terminal reads it.
- **`nvs ext verify <file>`** runs every load check ADR 0246 §§ 3-4 lists, instantiates nothing, and
  prints `ok` with the pin, or every refusal with a non-zero exit. It is the same function boot calls,
  so the two cannot disagree.
- **`nvs ext pin <file>`** prints the `[[extension]]` entry with `path` as given and `sha256`, ready
  to paste. A manifest that requests I/O adds one comment line naming what it requests. It never
  writes a `grants` key: granting is the operator's act (`rule:security/extension-grants-are-an-intersection`).
- **Pinned by** the Stage 3 checks: an entry `pin` prints passes `nvs config check`.

## Stage 4 — `test`

**Does:** Adds `nvs ext test`, which runs the project's `.nvs` tests with the built file loaded and no
`nvs.toml`.

One file set: `crates/nvs-cli/src/ext.rs`, `crates/nvs-cli/src/testing.rs:1` (the runner `nvs test`
uses), `crates/nvs-cli/tests/ext_command.rs`.

- **What it runs.** The project's test folder, by the path `nvs test <dir>` takes, with the `.nvsx`
  the last `nvs ext build` wrote loaded as an entry. A built file older than its module is refused,
  and the message says to run `nvs ext build`.
- **What it reads and grants.** No `nvs.toml` is read, so the extension holds no I/O, as it would
  under a fresh operator's entry. A test that needs a grant names a configuration with `--config`,
  whose entry for the built file must carry that file's pin; a different pin is refused with the
  line `nvs ext pin` would print.
- **The rule is now whole.** `rule:packaging/nvs-ext-is-the-authoring-tool` loses **Not on disk**.
- **Pinned by** the Stage 4 checks.

## Stage 5 — a bundle carries its extensions

**Does:** Makes `nvs build --compile` embed every `.nvsx` its configuration lists, with its pin, and
makes the bundle load them as the entries would.

One file set: `crates/nvs-cli/src/bundle.rs` (`embedded` at line 101, `build` at 164, `manifest` at
286, `write_bundle` at 301, `parse_manifest` at 398), `crates/nvs-cli/src/main.rs:1540`,
`crates/nvs-cli/tests/bundle.rs`, `data/gaps/nvs-cli/nvsx-entries-are-not-embedded-yet.json`.

- **The build reads the configuration.** `bundle::build` resolves the configuration as every other
  command does, and embeds each `[[extension]]` entry's file with its pin and its `grants` as
  written. A file whose digest is not its pin refuses the build, naming the entry.
- **The payload gains an extension entry.** Beside the source text entries, an entry holds the
  `.nvsx` bytes, the pin and the grant. The footer's format version goes up by one. A bundle is a
  copy of the binary that built it, so a binary reads only its own version.
- **The bundle loads them first.** At run, the embedded entries are loaded from the payload, pin
  checked again, before any entry the run's own `nvs.toml` lists. A class both declare is refused,
  as any duplicate is. The run's own capabilities are still the caller's set, so the intersection
  holds in a bundle too. The built-in components need no entry: they are in the binary
  (`rule:packaging/the-first-party-components-are-built-in`).
- **The gap closes.** `data/gaps/nvs-cli/nvsx-entries-are-not-embedded-yet.json` is deleted, and
  `rule:packaging/a-nvsx-dependency-embeds-in-the-same-payload` loses **Not on disk**.
- **Pinned by** the Stage 5 checks, with a fixture component packed from WAT.

## Stage 6 — the CI job

**Does:** Builds, loads and tests both templates on Linux, Windows and macOS in CI, through one
`bun nv` tool that also runs on a contributor's machine.

One file set: `tools/nv/cmd/ext-templates.ts` (new), `tools/nv/test/ext-templates.test.ts` (new),
`tools/nv/cmd/ci-changes.ts:41` (`LANES`), `.github/workflows/ci.yml:252` (the job the new one sits
beside).

- **`bun nv ext-templates --lang rust|c|all`** writes each template into `.agent-tmp/ext-templates/`
  with the `nvs` it is given, runs the template's own build commands, then `nvs ext build`,
  `nvs ext verify` and `nvs ext test`, and calls the extension from a program under an `nvs.toml`
  holding the entry `nvs ext pin` printed. It prints one line per template:
  `rust template: built, verified, tested and called`. It deletes what it wrote.
- **The C leg needs wasi-sdk and `wit-bindgen`.** Where `WASI_SDK_PATH` is not set it prints
  `c template: skipped, no wasi-sdk at WASI_SDK_PATH` and passes, unless `CI` is set, where it fails.
  Only the CI job installs wasi-sdk; no contributor needs it (ADR 0247 § 4's reasoning).
- **The job.** `extension-templates` in `.github/workflows/ci.yml`, a matrix of Linux, Windows and
  macOS, installs a pinned wasi-sdk with its checksum checked and a pinned `wit-bindgen-cli`, builds
  `nvs`, and runs `bun nv ext-templates --lang all`. It carries a `timeout-minutes`, as every job does.
- **The lane.** `tools/nv/cmd/ci-changes.ts:41`'s `LANES` gains `ext`: `crates/nvs-ext/`,
  `crates/nvs-cli/src/ext.rs`, `crates/nvs-cli/templates/`, `wit/`, `Cargo.lock`,
  `rust-toolchain.toml`, `tools/nv/cmd/ext-templates.ts` and `.github/workflows/`, and the
  `changes` job emits it (`rule:testing/ci-lanes`).
- **Pinned by** the Stage 6 checks. The Rust leg runs here; the C leg and the three platforms run in
  CI. The loop never pushes, so the job's first verdict comes with the next push the user makes, and
  the handoff says so.

## Stage 7 — the help and the feature proofs

**Does:** Writes the reference sections, the help and the feature proofs of every `nvs ext` command,
and grows `nvs build --compile`'s.

One file set: `docs/reference/tools/10-cli.md:273` (the sections go after `# nvs build --compile`),
`docs/examples/tools/cli/`, `tests/hostile/tools/cli/`, `crates/nvs-cli/tests/agent.rs:382`.

- **Three sections**, each a feature by its heading, each with a fenced example:
  `# nvs ext new and nvs ext build` (with the `nvsx.toml` keys), `# nvs ext inspect, nvs ext verify
  and nvs ext pin`, and `# nvs ext test`. Their features are `tools:cli/nvs-ext-new-and-nvs-ext-build`,
  `tools:cli/nvs-ext-inspect-nvs-ext-verify-and-nvs-ext-pin` and `tools:cli/nvs-ext-test`.
  `# nvs build --compile` says what a bundle carries.
- **The Help proof.** `nvs agent find` reaches each command by the words its user types, as
  `crates/nvs-cli/tests/agent.rs:382` reaches `autoload`, and `nvs ext --help` lists all six.
- **The feature proofs** (`rule:testing/feature-proofs`): an `about.md`, the examples
  `bun nv proofs --id` says each owes, one attack each, and the Rust tests of Stages 2 to 5 marked
  `// covers:`. An attack is a program `nvs run` delivers, whose fixture the command's Rust test
  reads. A fixture is component WAT packed into a committed `.nvsx`, in the shape goal
  `ext-compiler` set, and goal `ext-grants`' test in `crates/nvs-ext/tests/fixtures.rs` already
  checks every fixture under `tests/hostile/` and `docs/examples/` against its text. The session chooses each, for example: a manifest whose help text holds terminal escapes
  and a megabyte of text (`inspect`), a source file that names a namespace outside the extension's
  (`build`), and a test that asks the guest to read the configuration file (`test`).
  `tools:cli/nvs-build-compile` gains an example that calls a bundled extension.
- **Pinned by** the Stage 7 checks.

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
  web-app set, without MessageFormat. **`nvs ext new` ships Rust and C templates only.** `Novis\Image`
  and `Novis\Intl` are built into every binary and always on. The build compiles the component crates
  for `wasm32-wasip2`, and libwebp is prebuilt by a `bun nv` tool, committed beside its source hash,
  and rebuilt and compared by CI.
- **The record writer's calls, standing but not confirmed by the user.** The manifest is JSON written
  from an `nvsx.toml`. `Novis\` is reserved. The error variant `invalid|parse|runtime` throws
  `LogicError|ParseError|RuntimeError`. One instance per extension per request, and a second task
  waits. A third-party settings block is `[ext.<name>]`. `nvs check` and the language server read
  manifests and never instantiate. **A bundle embeds the `.nvsx` files the build's configuration
  lists.**
- **This goal writer's calls, not confirmed by the user.** A method's signature is written in
  `nvsx.toml` as Novis text, so an author writes the language they call it from. `nvs ext build`
  compiles nothing: it takes the module the author's toolchain built, and refuses WASI preview 1.
  The world's WIT a project builds against is copied from the binary, never fetched. `pin` never
  writes `grants`. `nvs ext test` grants nothing unless `--config` names a configuration. A bundle
  embeds each entry's `grants` as written, and its embedded entries load before the run's own. The
  CI job runs through a `bun nv` tool, so the job and a contributor run one definition.
- **No record slot.** ADR 0246 decided all of this. A question it does not answer is settled under
  AGENTS.md's priority ordering and written in `crates/nvs-cli/src/ext.rs`'s module doc.
- **A libc import outside the world is a refusing stub.** If the C template's libc imports a WASI
  interface outside the world, goal `ext-grants`' standing decision applies: the host links it as a
  stub whose every function returns its "access denied" error. The template is not changed to avoid it.
- **No new host dependency for C.** `nvs` does not embed `wit-bindgen`'s C generator; a C author
  installs `wit-bindgen-cli` and wasi-sdk, as the template's README says.
- **Not checked:** whether `rustup` installs a target newly listed in `rust-toolchain.toml` on a
  toolchain that is already installed. If it does not, Stage 6's tool runs `rustup target add
  wasm32-wasip2` and says so.
- **A scratch project lives under `.agent-tmp/`** (AGENTS.md rule 10), and never under the system
  temporary folder. A Rust project under the repository joins the root workspace unless its
  `Cargo.toml` has its own `[workspace]` table, which is why the template carries one.
- **Tradeoffs.** Performance: nothing on the request path; a bundle with extensions starts by
  checking each embedded pin, one digest per entry. Memory: a bundle is larger by its `.nvsx` files,
  read once at start. Usability: an author goes from nothing to a pinned entry in five commands, and
  a single-file executable carries its extensions. Simplicity: one command with six subcommands,
  one more file format (`nvsx.toml`), one more CI job, and C authors install two tools.
- **Test guests in Rust tests are component WAT compiled by the `wat` crate**, so `cargo test` needs
  no wasm toolchain. A debug cargo command never takes `-p`.
- **Every name in a template, a test, an example and a fixture is neutral** — `Geo`, `Greeting`,
  `Shop`, `example.com`.
- **Every comment in a new `.nvs` and every new `about.md` follows `AGENTS.md` § *Text an end user
  reads* at the first write**, and so do the templates' comments, which an author reads first.
  `bun nv proofs --comments <paths>` is run over the proofs before the wrap.
