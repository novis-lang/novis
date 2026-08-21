# Working on MWL

MWL is a JIT-compiled, memory-safe language for web servers and the command line. It exists to run web
requests and CLI programs **securely and fast**.

This is the only file you always need to read. It carries the rules that are easy to get wrong, plus a
table telling you the *one* other file to open for whatever you are doing.

**Every fact in this repository has exactly one home.** If two documents state the same thing, the one
named below is authoritative and the other is a bug — fix it rather than reconciling it in your head.

## Where to look

Read this file, then **one** row below. Do not read the docs tree breadth-first: it is several hundred KB
and only grows as ADRs are added, and most of it is reasoning you only need when you are about to overturn
a decision.

**First, run `sh .claude/brief.sh`.** One call, well under 15 KB: the plan's status block, the current and
next milestone, the one-line title and status of every ADR, what the guard tests actually hold with their
thresholds, and what exists on disk. It stores no facts — it slices the live files and names each source,
so it cannot go stale, and it says so loudly if a slice comes back empty. It deliberately does not print
each ADR's full rule, so it stays small as more ADRs are added — open the row below for whatever you're
touching to get that.

| Doing this | Open this |
|---|---|
| Deciding what to build next; scoping a milestone; checking what exists | [docs/implementation-plan.md](docs/implementation-plan.md) — the status block at the top, then your milestone. This is the plan of record. |
| Exceptions, the call ABI, helper signatures, panic containment | [ADR 0002](docs/adr/0002-error-propagation.md). It holds the only normative copy of the calling convention, and it *supersedes* any unwinding language you find elsewhere. |
| Extensions, wasm, WIT, `.mwlx` | [ADR 0003](docs/adr/0003-extension-system.md) |
| Weighing memory against safety, speed or simplicity | [ADR 0004](docs/adr/0004-memory-for-simplicity.md) |
| `mwl.ini`, `ini_set`, limits, capabilities | [ADR 0005](docs/adr/0005-config-changeability.md). Holds the only copy of the directive layout. |
| `spawn script`, isolates, the request boundary | [ADR 0006](docs/adr/0006-isolated-script-execution.md) |
| The built-in HTTP server, live cache invalidation, picking up an edited `.mwl` file without a restart | [ADR 0017](docs/adr/0017-hot-reload-without-restart.md). Holds the only copy of the path-pointer-swap mechanism, why it needs no filesystem watcher, and why a request-serving core is never blocked on a recompile. |
| Types, `uint`, `array<T>`, unions, `mixed`, conversions, array keys | [ADR 0007](docs/adr/0007-explicit-type-system.md). Holds the only copy of the type grammar, the conversion table, the arithmetic result types and the list of deliberate divergences from PHP. |
| `string` vs `bytes`, the UTF-8 guarantee, text/binary conversion | [ADR 0009](docs/adr/0009-string-and-bytes.md) — **Proposed**, not yet Accepted: the default length/indexing granularity awaits a cost measurement (see its *Revisiting*). Holds the only copy of the `string`/`bytes` split and the conversion rule between them. |
| `enum`, enum cases, backing type, anything enum-shaped | [ADR 0010](docs/adr/0010-enums-are-a-value-type.md). Holds the only copy of enum semantics — a closed, named integer type like C#'s, not PHP's class-like construct; PHP's enum design is deliberately disregarded in full. |
| `static`, `global`, scoping, closure capture, where state may live at all | [ADR 0008](docs/adr/0008-static-and-global.md). Holds the only copy of the list of storage classes, and the one place `static`'s five PHP meanings are sorted into kept and rejected. |
| Free functions, global constants, the `Core` namespace, where a built-in lives | [ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md). Holds the only copy of the rule that every callable and every constant is a class member, and the `Core` domain-class shape built-ins are organised into. |
| `$_SERVER`, `$_GET`/`$_POST`, `$_SESSION`, `$_ENV`, `$GLOBALS`, `$_REQUEST`, `$argv`, or anything else PHP populates ambiently | [ADR 0012](docs/adr/0012-no-superglobals.md). Holds the only copy of the rule that no variable is ever host-populated — each becomes a `Core\Server`/`Core\Request`/`Core\Session`/`Core\Env`/`Core\Cli`/`Core\Script` call, and `$GLOBALS`/`$_REQUEST` have no replacement at all. |
| Comparing two objects with `<`/`>`/`<=`/`>=`/`<=>`, operator overloading, `Comparable`, `compareTo` | [ADR 0013](docs/adr/0013-comparable-interface.md). Holds the only copy of the rule that ordering two objects requires implementing `Comparable`; PHP's ambient property-walk fallback is rejected outright, and there is no cross-class overload. |
| Property hooks, `__get`/`__set`, `PropertyObserver`, undefined properties, `__call`/`__callStatic` | [ADR 0014](docs/adr/0014-property-observer.md). Holds the only copy of the rule that a property access runs its own hook first and a declared `PropertyObserver` second; accessing an undeclared property is always a hard error, and `__call`/`__callStatic` are not implemented at all. |
| `class_alias`, `use … as …`, trait-use `as`, or a `type` alias | [ADR 0015](docs/adr/0015-no-name-aliasing.md). Holds the only copy of the rule that nothing gets a second runtime-reachable name — `class_alias` does not exist, import renaming is rejected, and trait composition keeps only `insteadof` — while a `type` alias is kept as a distinct, compile-time-only synonym for a type expression, never for a single bare class. |
| The VS Code extension, the PhpStorm plugin, `mwl-lsp`/`mwl-fmt` client wiring, syntax highlighting, or what "IDE integration" does and doesn't cover yet | [ADR 0016](docs/adr/0016-ide-integration.md). Holds the only copy of the rule that language smarts and formatting live exactly once, in `mwl-lsp`/`mwl-fmt`, with a thin client per editor — PhpStorm's LSP-bridge-before-native phasing and the deferred debugger-UI wiring are both decided there, not left to be inferred from M10's task list. |
| A decision with no ADR — thread-per-core, value layout, safepoints, the unit cache, shared-nothing requests | [docs/adr/README.md](docs/adr/README.md) § *Decisions taken at project start* for **why**; the plan's § *Architecture* for the **mechanics**. That split is deliberate. |
| Any measured number, or checking whether an architecture assumption still holds | the guard tests in [benches/abi-probe/](benches/abi-probe/). The tests are the source of truth; docs quote them and can lag. |
| What the language should *do* | nothing yet — `docs/spec/` is unwritten. Say so rather than inferring semantics. |

Each ADR opens with a metadata block and reaches `## Decision` within ~60 lines. Read those two. The
`## Context`, `## Investigation` and `## Alternatives rejected` sections are for when you intend to
*change* the decision — skip them otherwise.

## The priority ordering

Highest first. A lower item is spent to buy a higher one, never the reverse. Reasoning and bounds:
[ADR 0004](docs/adr/0004-memory-for-simplicity.md).

1. **Security and request isolation** — not traded for anything.
2. **Correctness of language semantics** — PHP-compatible observable behaviour.
3. **Latency and throughput on the request path.**
4. **Simplicity** — of the language surface first, then of the implementation.
5. **Memory footprint** — last, and spent deliberately to buy any of the above.

When choosing between designs:

- Prefer the safer, faster or simpler one even when it holds more memory. MWL is **not** a low-footprint
  runtime; "allocates less" is not on its own a reason to change anything.
- If a change spends memory, **say what it spends** — per request or per task — in the doc comment or ADR
  that records it.
- Memory must stay attributable to a request and under an enforceable cap, and must be O(in-flight) rather
  than O(requests served). Growth with total traffic is a leak, not a trade-off.
- Bytes *moved* are not cheap. An allocation or extra cache miss on a hot path is a latency question
  (priority 3), not a footprint one.
- Saving memory at the cost of an invariant every future contributor must remember is the wrong direction —
  that is the account the unsafe modules are already drawing on.

## Ground rules enforced elsewhere

Each bullet is the one-sentence rule; the full mechanism, the exact spellings rejected, and the reasoning
live only in the ADR it links to. **When you add a new decision, add one bullet here — not a paragraph.**
If you find yourself restating more than a sentence, that detail belongs in the ADR instead.

- **`unsafe` is forbidden workspace-wide**; only `mwl-runtime`, `mwl-codegen` and `benches/abi-probe` opt
  down to `deny` with narrow, reasoned allows. Lint policy is in [Cargo.toml](Cargo.toml).
- **Nothing unwinds through a JIT frame** — every call returns a checked status instead, never
  `extern "C-unwind"` ([ADR 0002](docs/adr/0002-error-propagation.md)).
- **Pure-Rust dependencies by default**, enforced by [deny.toml](deny.toml) in CI. Deviations are argued
  individually.
- **Extensions are sandboxed wasm, never `dlopen`** ([ADR 0003](docs/adr/0003-extension-system.md)).
- **An isolate shares nothing but compiled code, and spends its parent's budget** — the same value-crossing
  rules as cross-core worker dispatch, limits accounted at the request tree's root, never per isolate
  ([ADR 0006](docs/adr/0006-isolated-script-execution.md)).
- **Nothing is untyped, and no type ever changes by itself** — every binding declares a type; `mixed` is the
  one unchecked position; `int + uint` is a compile error; overflow throws rather than becoming a `float`
  ([ADR 0007](docs/adr/0007-explicit-type-system.md)). Type *inference* belongs in `mwl convert`, never in
  the compiler.
- **`string` is guaranteed-valid UTF-8; binary data is the separate `bytes` type** — not yet Accepted, see
  the table above ([ADR 0009](docs/adr/0009-string-and-bytes.md)).
- **Every function is a method, every constant a class constant** — no free function, no global constant,
  no exception for the standard library; built-ins live under `Core` domain classes
  ([ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md)).
- **`static` marks a class member; nothing else holds state behind a function's back** — no function-scope
  `static`, no `static fn`, no `global`; a second per-isolate slot table for one would undo this decision
  ([ADR 0008](docs/adr/0008-static-and-global.md)).
- **No variable is ever populated by the host** — PHP's superglobals become `Core` accessor classes;
  `$GLOBALS`/`$_REQUEST` have no replacement at all ([ADR 0012](docs/adr/0012-no-superglobals.md)).
- **Ordering two objects requires the global `Comparable` interface** — there is no property-walk fallback,
  and no cross-class overload ([ADR 0013](docs/adr/0013-comparable-interface.md)).
- **A property access runs its own hook, then a declared `PropertyObserver`, in that order** — accessing an
  undeclared property is always a hard error, and `__call`/`__callStatic` do not exist
  ([ADR 0014](docs/adr/0014-property-observer.md)).
- **Nothing gets a second runtime-reachable name** — no `class_alias`, no import `as`, no trait-use `as`; a
  compile-time-only `type` alias for a type expression is the one exception
  ([ADR 0015](docs/adr/0015-no-name-aliasing.md)).
- **Compiled code is the only thing a request shares with any other** — an edited source file is picked up
  by revalidating a small per-path pointer, never by a filesystem watcher or a process restart, and no
  request-serving core is ever blocked on a recompile it did not ask for
  ([ADR 0017](docs/adr/0017-hot-reload-without-restart.md)).
- **Architecture assumptions are tested, not remembered.** [benches/abi-probe/](benches/abi-probe/) guards
  the ABI, coroutine, sandbox and cost claims on every CI run. If a change makes one of those tests fail,
  the ADR it points at needs revisiting — do not adjust the threshold to make it pass.

## Commands

```sh
cargo build                                                    # debug; deps still built at opt-level 2
cargo test                                                     # unit + integration
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test --release -p mwl-abi-probe                          # cost guards (skipped in debug)
cargo test --release -p mwl-abi-probe --features wasm-probe     # + sandbox probes (pulls in Wasmtime)
```

### Fuzzing on Windows: use WSL

`cargo-fuzz` (the `fuzz/` crate, `cargo +nightly fuzz run lex|parse`) needs libFuzzer, which is not
supported on native Windows at all — do this in WSL, not PowerShell/Git Bash. From a Windows shell,
`wsl.exe -- bash -lc "<command>"` runs a command straight in the default WSL distro, which mounts the
repo at `/mnt/d/swlang` (adjust the drive letter). One-time setup in that distro, first time only:

```sh
sudo apt-get update && sudo apt-get install -y build-essential clang
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
source "$HOME/.cargo/env"
rustup toolchain install nightly
cargo install cargo-fuzz --locked
```

Then, from `/mnt/d/swlang` (not `fuzz/` itself — cargo-fuzz expects the parent directory):
`cargo +nightly fuzz run lex -- -max_total_time=3600` (and `parse` likewise) for the 1h M1 verification
run; CI's `fuzz-smoke` job runs both for 60s on every push as a continuous regression check, same as the
plan's overall verification strategy calls for.

## Writing docs here

The docs are optimised for an agent that reads one file and starts working. Keep them that way:

- **State a fact once.** Put it in the home named in *Where to look*, and link to it from everywhere else.
  Summarising an ADR into another document creates a second copy that will silently go stale — that is how
  the plan came to carry a superseded ABI cost and a superseded unwinding design at the same time.
- **Never quote a measured number outside the ADR that owns it.** Numbers live with their guard test.
- **Front-load.** Decision first, reasoning below it. Assume the reader stops after the first screen.
- A choice that would be expensive to reverse gets its own numbered ADR if the reasoning is subtle or
  contested; otherwise a paragraph in *Decisions taken at project start* in
  [docs/adr/README.md](docs/adr/README.md).
- Crates for later milestones are created when their milestone starts, not left sitting empty.


## Keep work small, commit your work
- Always commit your work, when a step is done, you dont need to verify the history before, just commit everything that has changed