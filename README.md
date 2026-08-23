# MWL — Modern Web Lang

A JIT-compiled, memory-safe language for web servers and the command line, with PHP 8.5 syntax as its
starting point and no build step: change a file, run it.

> **Status: pre-alpha, milestone M0.** The architecture is validated by working spikes but the language
> does not run yet. `Hello World` is milestone M3. Nothing here is stable.

## What it is trying to be

- **No build step.** Edit a `.mwl` file and run it. Compilation happens on load and is cached.
- **Actually compiled.** Cranelift emits native code. There is no interpreter tier.
- **Parallel in-language.** `async`/`await` for overlapping I/O plus isolated workers for real multicore
  CPU work — with no `async` function colouring, so any function may do I/O.
- **One process, many requests.** A single server process handles unlimited concurrent requests, each
  fully isolated, all sharing one in-memory compiled-code cache.
- **Isolation you can reach from the language.** `spawn script 'job.mwl'` runs another file with its own
  heap, its own globals and its own slice of the caller's budget — the isolation PHP can only get by
  starting another interpreter, at microseconds instead of tens of milliseconds
  ([ADR 0006](docs/adr/0006-isolated-script-execution.md)).
- **Typed on purpose.** Every parameter, property and variable declares its type, and no value changes
  type behind your back: conversions are explicit and throw rather than quietly yielding `0`. Unions and
  `mixed` are there for the cases that genuinely are dynamic. `uint` gives you the whole 64-bit range PHP
  cannot represent, and arrays keep PHP's ordered hash while gaining declarable, nestable element types —
  `array<array<uint>>` ([ADR 0007](docs/adr/0007-explicit-type-system.md)).
- **Memory-safe and contained.** Written in Rust with `unsafe` confined to three audited modules. A
  runtime bug or a resource-limit breach kills one request, never the process.
- **Fast and simple first; memory is what pays for that.** MWL targets server-class hardware, so where a
  design can be safer, faster or simpler by holding more memory, it holds more memory — deliberately, within
  an enforced per-request cap ([ADR 0004](docs/adr/0004-memory-for-simplicity.md)). It is not a
  low-footprint runtime, and sizing it means sizing for concurrency.
- **Extensible without giving up any of that.** Extensions are sandboxed WebAssembly components: one
  precompiled binary runs on every platform, written in whatever language you like, and a crashing or
  hostile extension harms one request rather than the process.
- **A migration target for PHP.** `mwl convert` transpiles existing PHP projects, including their `.phpt`
  test suites. Note the direction of travel: MWL takes PHP's *syntax*, not its type discipline, so PHP
  files are converted rather than dropped in — the converter infers the types PHP has no syntax for and
  writes them into the output for you to review.

## Design in one page

| | |
|---|---|
| Pipeline | source → AST → HIR → types → IR → Cranelift → native |
| Execution | baseline JIT now, optimising tier later, no interpreter |
| Errors | checked return status, never unwinding ([ADR 0002](docs/adr/0002-error-propagation.md)) |
| Concurrency | thread-per-core executors, stackful coroutines, isolated cross-core workers |
| Priorities | security → semantics → latency → simplicity → memory footprint ([ADR 0004](docs/adr/0004-memory-for-simplicity.md)) |
| Types | static and mandatory; explicit checked conversions; unions plus `mixed`; `int` and `uint`; string-keyed ordered arrays with declarable nested element types ([ADR 0007](docs/adr/0007-explicit-type-system.md)) |
| Scoping | `static` is a class-member modifier only — static members and late static binding kept, function-scope `static` and `static fn` rejected, no `global` ([ADR 0008](docs/adr/0008-static-and-global.md)) |
| OOP-only | Every function is a method, every constant a class constant — no free function, no global constant, no exception for built-ins. Built-ins live under the reserved `Core` namespace, one domain class per grouping (`Core\Str`, `Core\Arr`, `Core\Math`, …) ([ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md)) |
| Values | 16-byte tagged, refcounted, copy-on-write arrays and strings |
| Requests | shared-nothing; only compiled code is shared |
| Isolates | `spawn script` runs another `.mwl` file in-process with a fresh heap, on the caller's budget ([ADR 0006](docs/adr/0006-isolated-script-execution.md)) |
| Config | root-owned `mwl.toml` states defaults; a script may retune its own limits within operator-set ceilings ([ADR 0005](docs/adr/0005-config-changeability.md)) |
| Serving | built-in HTTP/1.1 + h2c; FastCGI optional and later |
| Extensions | built-in, sandboxed wasm (`.mwlx`), or statically linked native — never `dlopen` ([ADR 0003](docs/adr/0003-extension-system.md)) |

This is the short form. The fuller decision table, with the sequencing each choice implies, is in
[docs/implementation-plan.md](docs/implementation-plan.md); the reasoning behind each choice and the
measurements backing it are in [docs/adr/](docs/adr/README.md).

## Repository layout

Four of these exist today. The rest are the shape the workspace grows into; the right-hand column is
the milestone that creates each one, since a crate is added when its milestone starts rather than
sitting empty. `mwl-cli` is the one exception to "created when its milestone starts": M1's own plan
names `mwl ast` as its verification tool, so the crate was scaffolded early with just that one
subcommand — `run`/`test` and the rest of the CLI still arrive at M3.

```
crates/
  mwl-diagnostics   spans, source maps, error rendering                            exists
  mwl-syntax        lexer (inline HTML + PHP mode), parser, AST                        M1
  mwl-hir           name resolution, namespaces, class graph                           M2
  mwl-types         declared types, unions, narrowing, no inference                    M2
  mwl-ir            CFG/SSA IR, safepoints, refcount ops                               M2
  mwl-codegen       Cranelift backend  [audited unsafe]                                M3
  mwl-runtime       values, arrays, coroutines, scheduler  [audited unsafe]            M3
  mwl-cli           the `mwl` binary (`ast`, `check` so far)                exists / M3
  mwl-stdlib        Core domain classes, native builtin static methods                 M4S
  mwl-test          .mwlt runner                                                       M4
  mwl-host          Transport trait, unit cache, the Isolate boundary                  M5
  mwl-config        mwl.toml registry, changeability classes, overlays                  M6
  mwl-cache         content-addressed artifact cache                                   M6
  mwl-http          hyper h1 + h2c transport, optional rustls                          M7
  mwl-regex         two-tier engine + `preg_*` layer                                   M8
  mwl-db            driver trait + mysql / pgsql / sqlite / mssql                      M8
  mwl-ext           .mwlx loader, WIT host, per-request instancing                     M9
  mwl-fmt           formatter                                                         M10
  mwl-lsp           tower-lsp language server                                         M10
  mwl-dap           debug adapter                                                     M10
  mwl-pkg           package manager                                                   M10
  mwl-convert       PHP→MWL transpiler, .phpt→.mwlt                                   M11
  mwl-fcgi          optional FastCGI transport                                        M13
editors/
  vscode            TextMate grammar, language-configuration.json, LSP client        M10
  phpstorm          file-type registration, LSP-bridge plugin (Kotlin/Gradle)         M10
benches/
  abi-probe         architecture invariants + cost baselines  [audited unsafe]     exists
docs/adr/           architecture decision records                                  exists
docs/spec/          normative language reference                                unwritten
```

`editors/` sits outside the Cargo workspace — the VS Code extension is TypeScript/Node tooling, the
PhpStorm plugin is Kotlin/Gradle/IntelliJ Platform tooling — and is a thin client over `mwl-lsp`/`mwl-fmt`
in both cases, never a second implementation of language smarts or formatting
([ADR 0016](docs/adr/0016-ide-integration.md)).

[`benches/abi-probe`](benches/abi-probe/) is worth knowing about early. Several decisions in `docs/adr/`
depend on how Cranelift, `corosensei` and Wasmtime behave rather than on MWL's own code, so a dependency
bump can invalidate them silently. It checks them continuously: that a throw propagates and a runtime
panic is *contained* across native frames, that a coroutine can suspend from beneath live JIT frames, that
a wasm guest cannot read past the host heap or outlive its deadline, that an OS process still costs orders
of magnitude more than a task — and that native unwinding through JIT frames is still unavailable, which is
the premise the calling convention exists for.

## Building

Requires the pinned toolchain in [`rust-toolchain.toml`](rust-toolchain.toml) (`rustup` installs it
automatically) and, on Windows, the MSVC C++ build tools for linking.

```sh
cargo build            # debug; dependencies are still built with opt-level 2
cargo test             # unit + integration
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The cost baselines are release-mode figures, so their guard tests are skipped in a debug build. The
extension-sandbox probes pull in Wasmtime and sit behind a feature flag, so day-to-day builds do not pay
for it:

```sh
cargo test --release -p mwl-abi-probe                          # cost guards
cargo test --release -p mwl-abi-probe --features wasm-probe    # + sandbox probes
cargo bench -p mwl-abi-probe                                   # track the numbers
```

## Roadmap

The full plan — milestones broken into deliverables, with their verification criteria and the reasoning
behind each design decision — is [docs/implementation-plan.md](docs/implementation-plan.md).

| | Milestone | State |
|---|---|---|
| M0 | Project setup, CI, architecture spikes | **done** |
| M1 | Lexer, parser, diagnostics | **next** |
| M2 | HIR, type system, IR | |
| M3 | Baseline Cranelift backend → **Hello World** | |
| M4 | Language completeness, test runner | |
| M5 | Concurrency: coroutines, channels, workers, script isolates | |
| M6 | `mwl.toml`, capabilities, limits, artifact cache | |
| M7 | Built-in HTTP server | |
| M8 | Stdlib, database drivers, the `mwl:ext` WIT world | |
| M9 | Extension system: `.mwlx` loading, sandboxing, `mwl ext` tooling | |
| M10 | LSP, formatter, debugger, profiler, package manager | |
| M11 | PHP → MWL transpiler | |
| M12 | Optimising JIT tier | |
| M13 | Optional FastCGI transport | if a deployment target needs it |

## Contributing

Read [CLAUDE.md](CLAUDE.md) first — it carries the priority ordering every design choice is judged
against, the invariants that are easy to break, and a table pointing at the *one* document to open for a
given piece of work. Every fact in this repository has a single home; if two documents disagree, the one
CLAUDE.md names is right and the other is a bug.

## Licence

MWL is [MIT](LICENSE). Contributions are accepted under the same licence.

Every third-party component compiled into the `mwl` binary is permissively licensed and attributed in
[THIRD-PARTY-LICENSES.txt](THIRD-PARTY-LICENSES.txt), with each component's own copyright notice and
licence text reproduced in full. That file is **generated** from the resolved dependency graph and
**embedded in the binary**, so a copy of `mwl` carries its notices without the repository:

```sh
mwl info                # build and host facts, plus every component and its licence
mwl info --licenses     # the same, plus every licence text in full
mwl -i                  # the same command, under PHP's spelling

python tools/gen-attribution.py           # regenerate after changing a dependency
python tools/gen-attribution.py --check   # what CI runs; fails if the notice is stale
```

The generator fails closed: a licence it has no policy for, or one missing from
[deny.toml](deny.toml)'s allow list, stops the build instead of quietly omitting a notice. `cargo deny`
decides what may be *linked*; this decides what must be *shipped*
([ADR 0065](docs/adr/0065-third-party-attribution-and-mwl-info.md)).
