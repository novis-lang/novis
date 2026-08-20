# Working on MWL

MWL is a JIT-compiled, memory-safe language for web servers and the command line. It exists to run web
requests and CLI programs **securely and fast**; the architecture and its measurements live in
[docs/adr/](docs/adr/README.md), and this file is only what is easy to get wrong.

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

- **`unsafe`** is forbidden workspace-wide; only `mwl-runtime`, `mwl-codegen` and `benches/abi-probe` opt
  down to `deny` with narrow, reasoned allows. Lint policy is in [Cargo.toml](Cargo.toml); the policy
  itself is in [docs/adr/](docs/adr/README.md).
- **Nothing unwinds through a JIT frame.** Errors propagate as a checked `i32` status after every call, and
  helpers are `extern "C"` wrapping `catch_unwind` — never `extern "C-unwind"`
  ([ADR 0002](docs/adr/0002-error-propagation.md)). `panic = "unwind"` is load-bearing in every profile:
  `abort` would turn a containable bug into a process kill.
- **Pure-Rust dependencies by default**, enforced by [deny.toml](deny.toml) in CI. Deviations are argued
  individually.
- **Extensions are sandboxed wasm, never `dlopen`** ([ADR 0003](docs/adr/0003-extension-system.md)).
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

## Where decisions go

A choice that would be expensive to reverse gets recorded with its reasoning. Its own numbered ADR if the
reasoning is subtle or contested; otherwise a paragraph in *Decisions taken at project start* in
[docs/adr/README.md](docs/adr/README.md). Crates for later milestones are created when their milestone
starts, not left sitting empty.
