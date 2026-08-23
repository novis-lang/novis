# Next session prompt

## State

**`mwl run examples/hello.mwl` prints `Hello, World!` and exits 0.**
`.claude/loop-goal.md`'s acceptance command passes through the real pipeline — `mwl-syntax` →
`mwl-hir` → `mwl-types` → `mwl-ir` → `mwl-codegen` → `mwl-runtime` — with nothing stubbed. That
closes M3's *vertical slice*, not M3. Run `python .claude/brief.py` first, then read
`docs/implementation-plan.md`'s M3 paragraph for what the milestone still owes.

On disk: `mwl-diagnostics`, `mwl-syntax`, `mwl-hir`, `mwl-types`, `mwl-ir`, `mwl-runtime`,
**`mwl-codegen`**, `mwl-cli` (`ast`, `check`, `run`, `run --dump-ir`), `fuzz/`, `benches/abi-probe`.
Workspace is green: build, test, clippy, fmt, and `cargo test --release -p mwl-abi-probe`.

**`mwl-codegen` landed this session.** Read `crates/mwl-codegen/src/lib.rs`'s module docs before
touching it — they hold the six-item scope list, and each item says what unblocks it. In short: native
instructions for typed scalars, a 16-byte `Value` materialized only at a call boundary, ADR 0002's
compare-and-branch after every call, phis as Cranelift block parameters, the safepoint poll, and
ADR 0018's debug-flags check at every statement boundary. What it does **not** lower: any MWL-level
call, `Concat`, arrays, objects, and integer `Div`/`Mod` (refused on purpose — `sdiv` traps the process
on a zero divisor, which is a request-isolation failure, not a wrong answer).

**ADR 0018's probe check is fully landed**, including the thing that ADR's own *Verification* list says
it isn't landed without: `benches/abi-probe`'s
`an_all_bits_off_debug_probe_stays_in_the_safepoint_cost_class`, measuring **0.119 ns per site** against
a 5 ns guard and a 1.70 ns checked-return frame. `mwl_runtime::mwl_probe_stmt` is its slow path.

## Next

Pick **one**. The first two are the largest single unblockers; the third is what M3's own *Verify*
bullet still names.

1. **Lower `InstKind::Call`/`New` in `mwl-codegen`, and compile a file's methods alongside its script
   frame.** This is the biggest single gap: it is what stops `mwl run` from executing anything with a
   class in it, and it is also the missing site for ADR 0018's *call-site* (`TRACE`/`PROFILE`) probe,
   which that ADR puts in the single `emit_call` path. Needs a symbol table over the unit's own
   functions (`mwl_ir::ir::InstKind::Call::target` is a `"Class::method"` label today, so `mwl-cli`
   must lower each method under the matching name) and an object representation for the receiver.
   `mwl-cli`'s `run_run` currently lowers only `lower_script`; its module doc says so.
2. **The error path's refcount cleanup.** `mwl-codegen` returns a non-`OK` status without releasing the
   frame's live refcounted locals — its known gap 3. `mwl-ir` models no error edge at all
   (`InstKind::HelperCall`'s own doc comment), so both halves land together, and the IR half is the
   design decision: a call-shaped instruction needs a status and a cleanup successor.
3. **The rest of M3's *Verify* bullet:** a throw crossing several JIT frames and being caught, a helper
   panic terminating with `FATAL` and leaving the process able to run the next script, an MWL-level
   backtrace resolved from MWL's own frame chain, `mwl run --dump-asm`, and the benched typed-arithmetic
   loop with a guard in `benches/`. The first two depend on item 1 or 2; `--dump-asm` does not and is
   small (`--dump-ir` already exists as the model).

## Known red, and not caused by any of the above

`cargo deny check` **fails today**, on `main`, and has since before `mwl-codegen` existed: 13 RUSTSEC
vulnerability advisories against `wasmtime` 41.0.4 (several sandbox-escape class) plus three unmaintained
transitive crates, `bitmaps`/`im-rc`/`sized-chunks`. `bans`, `licenses` and `sources` are clean. Nothing
MWL ships runs Wasmtime — it reaches the lock only through `benches/abi-probe`'s optional `wasm-probe`
feature — so this is not a runtime exposure, but it is a real CI-red and it blocks
[ADR 0003](docs/adr/0003-extension-system.md)'s M9 sandbox. Deliberately **not** a drive-by fix: the
advisories want wasmtime >= 46, and the pin at 41 exists because M0's spike #4 confirmed 41 coexists with
the pinned Cranelift 0.128 (`Cargo.toml` says so at the dependency). Moving it is a paired wasmtime +
Cranelift bump that has to re-run that spike and the `benches/abi-probe` guard tests. Worth its own
session; say so rather than patching `deny.toml`.

## Backlog

- `docs/implementation-plan.md`'s M4 paragraph was trimmed this session only as far as getting
  `python .claude/brief.py` back under its milestone budget required — it was re-deriving each ADR's rule
  inline, which `CLAUDE.md` forbids. The rest of the plan has not had
  [DOC_CLEANUP_PROMPT.md](DOC_CLEANUP_PROMPT.md)'s pass in a while; that is the user's call to run.
- Inline HTML at file scope (`?>text<?mwl`) is not lowered — `mwl-ir`'s known gaps say why, and that it
  is the same `Helper::EchoStr` call `echo` already emits. Needs `mwl-types` to stop no-oping
  `StmtKind::InlineHtml` at the same time. Now visibly reachable: `mwl run` would print it.
- The safepoint/debug-flags loads use `MemFlags::new().with_notrap()` so nothing may treat them as
  redundant. Cranelift has no volatile and does no LICM over loads today, and MWL is single-threaded
  until M5 — but a watchdog thread setting `CPU_LIMIT` on another core is what M5 introduces, and that is
  when the load needs to become an `atomic_load` or be proven safe. Named in `mwl-codegen`'s
  `emit::ctx_word`.
- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT` —
  `ImplementsClause.by_field` has parsed since M1 and is still unread by `mwl-hir`/`mwl-types`.
- `for` loops in `mwl-ir` (reuses `LoopFrame` verbatim), then `switch` — which per `mwl-ir`'s
  design-choices bullet must introduce the N-way terminator the generator resumption dispatch will also
  use, not a `Branch` chain. `mwl-codegen` will need the matching arm; `Terminator` is `#[non_exhaustive]`
  and already falls through to an `Unsupported` naming it.
- The `mixed` runtime type-tag representation — `mwl-ir` known-gap 5, and now also `mwl-codegen`'s known
  gap 5: `ty::tag_of` is the one place that has to answer it, and it currently refuses.
- A string literal still allocates per evaluation (`mwl-runtime` known gap 3 / `mwl-codegen` known gap 4).
  The bytes now live in the unit's data section; what is missing is an immortal `StrHeader` with a pinned
  refcount, which is `mwl-runtime`'s layout decision to make.
- A `set`-hooked property is exempted from ADR 0022's constructor check rather than verified against the
  hook's own writes.
- [0054](docs/adr/0054-decimal-scalar-type.md)'s `m` literal suffix and untyped-until-placed fractional
  literals in `mwl-syntax`; [0053](docs/adr/0053-iteration-and-generators.md)'s `Iterable`/`Iterator` as
  reserved interface names alongside `Comparable`/`PropertyObserver`/`Stringable`, and `$obj[$k]` on a
  non-array refused with a diagnostic naming that ADR.
- [ADR 0061](docs/adr/0061-compile-time-autoload-and-program-discovery.md), off the hot path and
  independently landable: the `autoload` keyword and its two statement forms in `mwl-syntax` (grammar in
  [`docs/spec/00-overview.md`](docs/spec/00-overview.md) § 2), then the `mwl-hir` resolver half — probe an
  unresolved `QName`, load, `collect_*`, repeat, report `E_UNDECLARED` only at the fixpoint. Three new
  `E03xx` codes to allocate, starting at **`E0315`**. Parser half first; it is testable alone. Its
  exact-on-disk-name rule is ADR 0062 § 3's `check_path_case` in `mwl-hir::requires` — reuse it rather
  than writing a second comparison.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two documents state the same thing, the one CLAUDE.md names is
authoritative and the other is a bug — including this file, which is overwritten, never appended to.
