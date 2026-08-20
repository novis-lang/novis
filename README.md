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
  test suites.

## Design in one page

| | |
|---|---|
| Pipeline | source → AST → HIR → types → IR → Cranelift → native |
| Execution | baseline JIT now, optimising tier later, no interpreter |
| Errors | checked return status, never unwinding ([ADR 0002](docs/adr/0002-error-propagation.md)) |
| Concurrency | thread-per-core executors, stackful coroutines, isolated cross-core workers |
| Priorities | security → semantics → latency → simplicity → memory footprint ([ADR 0004](docs/adr/0004-memory-for-simplicity.md)) |
| Values | 16-byte tagged, refcounted, copy-on-write arrays and strings |
| Requests | shared-nothing; only compiled code is shared |
| Config | root-owned `mwl.ini`; scripts may tighten limits, never widen them |
| Serving | built-in HTTP/1.1 + h2c; FastCGI optional and later |
| Extensions | built-in, sandboxed wasm (`.mwlx`), or statically linked native — never `dlopen` ([ADR 0003](docs/adr/0003-extension-system.md)) |

The reasoning behind each of these, and the measurements backing them, are in
[docs/adr/](docs/adr/README.md).

## Repository layout

```
crates/
  mwl-diagnostics   spans, source maps, error rendering
  mwl-syntax        lexer (inline HTML + PHP mode), parser, AST
  mwl-hir           name resolution, namespaces, class graph
  mwl-types         gradual type system, inference, checking
  mwl-ir            CFG/SSA IR, safepoints, refcount ops
  mwl-codegen       Cranelift backend                        [audited unsafe]
  mwl-runtime       values, arrays, coroutines, scheduler     [audited unsafe]
  mwl-stdlib        native builtin functions
  mwl-cli           the `mwl` binary
benches/
  abi-probe         architecture invariants + cost baselines  [audited unsafe]
docs/adr/           architecture decision records
docs/spec/          normative language reference
```

[`benches/abi-probe`](benches/abi-probe/) is worth knowing about early. Several decisions in `docs/adr/`
depend on how Cranelift, `corosensei` and Wasmtime behave rather than on MWL's own code, so a dependency
bump can invalidate them silently. It checks them continuously: that a throw propagates and a runtime
panic is *contained* across native frames, that a coroutine can suspend from beneath live JIT frames, that
a wasm guest cannot read past the host heap or outlive its deadline — and that native unwinding through
JIT frames is still unavailable, which is the premise the calling convention exists for.

Crates for later milestones — `mwl-host`, `mwl-http`, `mwl-db`, `mwl-regex`, `mwl-config`, `mwl-cache`,
`mwl-ext`, `mwl-lsp`, `mwl-fmt`, `mwl-dap`, `mwl-test`, `mwl-convert`, `mwl-pkg` — are added when their
milestone starts, rather than sitting empty.

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
| M0 | Project setup, CI, architecture spikes | in progress |
| M1 | Lexer, parser, diagnostics | |
| M2 | HIR, type system, IR | |
| M3 | Baseline Cranelift backend → **Hello World** | |
| M4 | Language completeness, test runner | |
| M5 | Concurrency: coroutines, channels, workers | |
| M6 | `mwl.ini`, capabilities, limits, artifact cache | |
| M7 | Built-in HTTP server | |
| M8 | Stdlib, database drivers, the `mwl:ext` WIT world | |
| M9 | Extension system: `.mwlx` loading, sandboxing, `mwl ext` tooling | |
| M10 | LSP, formatter, debugger, profiler, package manager | |
| M11 | PHP → MWL transpiler | |
| M12 | Optimising JIT tier | |

## Licence

[MIT](LICENSE). Contributions are accepted under the same licence.
