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
- **A migration target for PHP.** `mwl convert` transpiles existing PHP projects, including their `.phpt`
  test suites.

## Design in one page

| | |
|---|---|
| Pipeline | source → AST → HIR → types → IR → Cranelift → native |
| Execution | baseline JIT now, optimising tier later, no interpreter |
| Errors | checked return status, never unwinding ([ADR 0002](docs/adr/0002-error-propagation.md)) |
| Concurrency | thread-per-core executors, stackful coroutines, isolated cross-core workers |
| Values | 16-byte tagged, refcounted, copy-on-write arrays and strings |
| Requests | shared-nothing; only compiled code is shared |
| Config | root-owned `mwl.ini`; scripts may tighten limits, never widen them |
| Serving | built-in HTTP/1.1 + h2c; FastCGI optional and later |

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
docs/adr/           architecture decision records
docs/spec/          normative language reference
```

Crates for later milestones — `mwl-host`, `mwl-http`, `mwl-db`, `mwl-regex`, `mwl-config`, `mwl-cache`,
`mwl-lsp`, `mwl-fmt`, `mwl-dap`, `mwl-test`, `mwl-convert`, `mwl-pkg` — are added when their milestone
starts, rather than sitting empty.

## Building

Requires the pinned toolchain in [`rust-toolchain.toml`](rust-toolchain.toml) (`rustup` installs it
automatically) and, on Windows, the MSVC C++ build tools for linking.

```sh
cargo build            # debug; dependencies are still built with opt-level 2
cargo test             # unit + integration
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Roadmap

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
| M8 | Stdlib and database drivers | |
| M9 | LSP, formatter, debugger, profiler, package manager | |
| M10 | PHP → MWL transpiler | |
| M11 | Optimising JIT tier | |

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

Contributions are accepted under the same dual licence.
