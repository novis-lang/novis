# Next session prompt

## State

Milestone **M2** (HIR/types/IR) is close to done; **M3** (baseline Cranelift backend —
`Hello, World!`) is the loop's target, per `.claude/loop-goal.md`. Run `python .claude/brief.py` first,
then read `docs/implementation-plan.md`'s M2/M3 paragraphs — the plan is the one home for status
detail, this file only points.

On disk: `mwl-diagnostics`, `mwl-syntax`, `mwl-hir`, `mwl-types`, `mwl-ir`, **`mwl-runtime`**,
`mwl-cli` (`ast`, `check`), `fuzz/`, `benches/abi-probe`, and the `examples/hello.mwl` the acceptance
command names. Workspace is green (build/test/clippy/fmt).

**`mwl-runtime` landed this session** — the half of M3 testable without a backend. It owns
[ADR 0002](docs/adr/0002-error-propagation.md)'s ABI and the `mwl_helper!` macro that supplies the
mandatory `catch_unwind`, the 16-byte tagged `Value`, the refcounted `MwlStr`, the `Ctx` whose two hot
words back the safepoint poll and ADR 0018's probe check, and nine of the ten `mwl_ir::Helper` entry
points including `mwl_echo_str`. Read `crates/mwl-runtime/src/lib.rs`'s module docs before touching it —
they hold the known-gap list (arrays/objects have no representation, so `Helper::ArrayTruthy` has no
entry point; a string literal still allocates; no coroutine yielder; no panic hook) and each gap says
what unblocks it.

**`mwl-codegen` still does not exist.** That is now the only crate between here and the acceptance
command.

**ADRs 0051-0061 are decided and wired** into CLAUDE.md, `docs/adr/README.md`, the plan and the spec. No
code implements any of them yet. Four carry obligations landing **before** M8 — each ADR's own
*Verification* section is the one home for its split. [ADR 0053](docs/adr/0053-iteration-and-generators.md)'s
"the IR must model a suspension point inside a loop body" is already discharged as a *representation*
decision in `crates/mwl-ir/src/lib.rs`'s § *Design choices worth knowing before widening this further* —
read that bullet before adding any `Terminator` variant.

## Next

**Create `mwl-codegen`, and add `mwl run`.** This is the whole remaining path to the acceptance command.
`.claude/loop-goal.md` holds the decided design; four points from it and from this session that are easy
to miss:

- Give `mwl-codegen` its own `[lints]` block with `unsafe_code = "deny"` and narrow reasoned allows —
  **not** `lints.workspace = true`, which is `forbid` workspace-wide and makes a JIT unimplementable.
  Copy `crates/mwl-runtime/Cargo.toml`'s block; it restates the whole workspace policy with only that one
  line changed, and explains why in a comment. The `[workspace.dependencies]` entry already exists, so
  creating the directory wires the crate in.
- ADR 0018's debug-flags probe check lands **in the first `mwl-codegen` commit**, with the safepoint
  poll — the narrow-backend authorization does not extend to deferring it — and is not considered
  landed until the `benches/abi-probe` guard test holding its all-bits-off cost exists.
  `mwl_runtime::{SAFEPOINT_OFFSET, DEBUG_FLAGS_OFFSET}` are the offsets to load from; use them rather
  than restating a number.
- `mwl run` checks first and, on any diagnostic, reports and exits non-zero exactly as `mwl check`
  already does, rather than running anyway. It builds a `Ctx::stdout()`, calls the compiled script
  frame through `mwl_runtime::call`, and calls `Ctx::flush_output` before returning — Rust's stdout is
  line-buffered and `echo "Hello, World!"` has no trailing newline.
- The runtime symbols to register with `cranelift_jit::JITBuilder::symbol` come from
  `mwl_runtime::symbols()`. Mapping an `mwl_ir::Helper` tag to one of those names is `mwl-codegen`'s job
  by design: `mwl-runtime` deliberately does not depend on `mwl-ir`.

## Known red, and not caused by any of the above

`cargo deny check` **fails today**, on `main`, and did before this session: 13 RUSTSEC vulnerability
advisories against `wasmtime` 41.0.4 (several sandbox-escape class) plus three unmaintained transitive
crates, `bitmaps`/`im-rc`/`sized-chunks`. `bans`, `licenses` and `sources` are clean. Nothing MWL ships
runs Wasmtime yet — it reaches the lock only through `benches/abi-probe`'s optional `wasm-probe` feature —
so this is not a runtime exposure, but it is a real CI-red and it blocks
[ADR 0003](docs/adr/0003-extension-system.md)'s M9 sandbox on a version that must be fixed before then.
It is deliberately **not** a drive-by fix: the advisories want wasmtime >= 46, and the pin at 41 exists
because M0's spike #4 confirmed 41 coexists with the pinned Cranelift 0.128 (`Cargo.toml` says so at the
dependency). Moving it is a paired wasmtime + Cranelift bump that has to re-run that spike and the
`benches/abi-probe` guard tests. Worth its own session; say so rather than patching `deny.toml`.

## Backlog

- Inline HTML at file scope (`?>text<?mwl`) is not lowered — `mwl-ir`'s known gaps say why, and that
  it is the same `Helper::EchoStr` call `echo` already emits. Needs `mwl-types` to stop no-oping
  `StmtKind::InlineHtml` at the same time.
- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT` —
  `ImplementsClause.by_field` has parsed since M1 and is still unread by `mwl-hir`/`mwl-types`
  ([ADR 0043](docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md) §§ 4-5).
- `for` loops in `mwl-ir` (reuses `LoopFrame` verbatim), then `switch` — which per the design-choices
  bullet must introduce the N-way terminator the generator resumption dispatch will also use, not a
  `Branch` chain. `mwl-ir`'s own module docs hold the known-gap list.
- The `mixed` runtime type-tag representation — `mwl-ir` known-gap item 5. `mwl-runtime`'s `Tag` enum is
  now the other half of that question: it already numbers all ten of the plan's tags, four with nothing
  behind them.
- A `set`-hooked property is exempted from ADR 0022's constructor check rather than verified against the
  hook's own writes.
- [0054](docs/adr/0054-decimal-scalar-type.md)'s `m` literal suffix and untyped-until-placed fractional
  literals in `mwl-syntax`; [0053](docs/adr/0053-iteration-and-generators.md)'s `Iterable`/`Iterator` as
  reserved interface names alongside `Comparable`/`PropertyObserver`/`Stringable`, and `$obj[$k]` on a
  non-array refused with a diagnostic naming that ADR.
- [ADR 0061](docs/adr/0061-compile-time-autoload-and-program-discovery.md), off the `Hello, World!` path
  and independently landable: the `autoload` keyword and its two statement forms in `mwl-syntax` (grammar
  in [`docs/spec/00-overview.md`](docs/spec/00-overview.md) § 2), then the `mwl-hir` resolver half — probe
  an unresolved `QName`, load, `collect_*`, repeat, report `E_UNDECLARED` only at the fixpoint. Three new
  `E03xx` codes to allocate, starting at **`E0315`** (`E0314` is now taken). Parser half first; it is
  testable alone. Its exact-on-disk-name rule is ADR 0062 § 3's `check_path_case` in
  `mwl-hir::requires` — reuse it rather than writing a second comparison.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two documents state the same thing, the one CLAUDE.md names is
authoritative and the other is a bug — including this file, which is overwritten, never appended to.
