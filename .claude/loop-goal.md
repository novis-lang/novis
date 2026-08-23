# Loop goal

Finish milestone **M3** — every item in its own *Verify* bullet in `docs/implementation-plan.md`, not just
the vertical slice the previous loop reached. Read that milestone's paragraph for scope; do not re-derive
it here.

The one thing standing under all of it: **`mwl-codegen` lowers no MWL-level call at all.** `InstKind::Call`
and `Concat` return `CodegenError::Unsupported` today. Nothing else M3 owes — a throw across frames, a
backtrace, ADR 0018's call-site probe — has a site to attach to until that lands. Start there.

## Acceptance (the driver checks this itself, every iteration)

`Test-Goal` in `.claude/loop.ps1` runs the list below. Every item must pass. Nothing else counts as done —
not a passing unit test, not an IR snapshot, not a session claiming `DONE`.

**Windows leg** (short-circuits on the first failure, so a broken iteration is cheap):

| command | must |
|---|---|
| `mwl run examples/hello.mwl` | exit 0, stdout `Hello, World!` |
| `mwl run examples/calls.mwl` | exit 0, stdout `quadruple(5) = 20` |
| `mwl run examples/throw.mwl` | exit 0, stdout `caught: boom` |
| `mwl run examples/arith.mwl` | exit 0, stdout `sum = 998000` |
| `mwl run examples/trace.mwl` | exit 0, stdout contains `#0 Deep::inner()` then `#1 Deep::outer()`, in that order |
| `mwl run examples/uncaught.mwl` | exit **non-zero**, stderr contains `Uncaught Exception: unhandled`, `#0 Boom::inner()` and `#1 Boom::outer()` |
| `mwl run --fault-inject=helper-panic examples/fatal.mwl` | exit **non-zero**, stderr contains `FATAL`, stdout contains `start` |
| `mwl run --dump-asm examples/arith.mwl` | exit 0, at least 200 bytes of output |
| `cargo test --release -p mwl-abi-probe` | green, and it contains the two guards named below |
| `cargo test -p mwl-codegen` | green, and it contains the reuse test named below |

**WSL leg** (runs only once the whole Windows leg is green — you chose both platforms, and this ordering
keeps a failing iteration from paying for it): the same eight `mwl run` checks, through
`wsl.exe -- bash -lc`, against a Linux build with its own `CARGO_TARGET_DIR`. Same expectations, byte for
byte. A JIT is exactly where an ABI or calling-convention divergence hides, which is why this leg exists at
all rather than being left to CI.

The example files **already exist and must not be changed** — they were written up front so the check is a
real signal rather than a silently-false one, the same way `examples/hello.mwl` was. If one of them needs a
different spelling because a decision below turns out to be wrong, that is a `BLOCKED`, not an edit.

### The three test names the acceptance requires by name

Fixed here so the driver can rely on them and so nobody has to guess where they live:

- `benches/abi-probe/tests/perf_guards.rs::a_typed_arithmetic_loop_contains_no_call` — compiles
  `examples/arith.mwl`'s loop and asserts **structurally** that the loop body emits no call instruction.
  This is the honest form of ADR 0007's "mandatory types pay for themselves" claim: not a timing, a
  structure. `benches/abi-probe` gains a dev-dependency on `mwl-codegen`/`mwl-runtime` to do it.
- `benches/abi-probe/tests/perf_guards.rs::a_typed_arithmetic_loop_stays_in_the_native_cost_class` —
  ns/iteration bounded **relative to** the checked-return frame cost `a_checked_return_frame_stays_cheap`
  already measures on the same machine. Self-relative, per
  [ADR 0026](../docs/adr/0026-performance-measurement-methodology.md); never an absolute figure quoted from
  another machine. Pick the ratio from what the first honest measurement shows, with headroom — and write
  the measured figure into that test's own comment, which is where measured numbers live (CLAUDE.md).
- `crates/mwl-codegen/tests/…::a_second_script_runs_after_a_contained_helper_panic` — compiles and runs two
  scripts in **one process**: the first provokes a contained helper panic, the second must still return
  normally. This is the "leaves the process able to run the next one" half of M3's bullet, which a one-shot
  CLI cannot show. `benches/abi-probe`'s `the_jit_is_still_usable_after_a_contained_panic` is the ABI-level
  version of the same claim — mirror it, do not duplicate it.

## Do this first, before any M3 work

**Clear the `cargo deny check` red.** 13 RUSTSEC advisories against the pinned wasmtime 41 (several
sandbox-escape class) plus three unmaintained transitive crates. The advisories want wasmtime ≥ 46, and the
pin at 41 exists because M0's spike #4 confirmed it coexists with the pinned Cranelift 0.128 — so this is a
**paired wasmtime + Cranelift bump**, and it has to re-run that spike and every `benches/abi-probe` guard,
not just edit two version numbers. If Cranelift's API churned, fixing `mwl-codegen` against the new version
is part of this, and it is far cheaper now than after the backend has grown calls, exceptions and a
backtrace. Do not patch `deny.toml` to silence it. If the bump proves genuinely impossible (no Cranelift
version satisfies both), that is a real `BLOCKED` — say which constraint failed.

## Standing decisions — pre-authorized, do not stop the loop for these

- **Decide and record; never `BLOCKED` for a design call.** The object/array heap layout, the `mixed`
  runtime type tag, how a throw is represented, and how `mwl-ir` models the error edge are all yours to
  settle, following CLAUDE.md's priority ordering. Record each in the home CLAUDE.md already names — a
  paragraph in `docs/adr/README.md` § *Decisions taken at project start*, or the crate's own module doc.
  **Do not open a numbered ADR for these**; that index is already pushing `brief.py`'s budgets. Reserve
  `BLOCKED` for a decision that would be expensive to reverse *and* that you cannot pick a safe default for.
- **The thing thrown is a runtime-owned `Throwable`, not a user-declared class.** `Throwable`, `Exception`
  and `Error` are global, PHP-shaped names — [ADR 0020](../docs/adr/0020-error-escalation-ladder.md) § 1
  says so and is the only home for that fact. `new Exception("…")` lowers to a **runtime helper that
  allocates the runtime's own exception value**, not through general `InstKind::New` object lowering;
  `getMessage()`/`getTraceAsString()` are runtime methods on that opaque value, not general field access.
  User-declared classes, the object heap layout and instance-method dispatch stay M4 — which is why every
  acceptance example uses `static` methods only.
- **`getTrace()` is an explicit M4 carry-over.** It returns `array<…>`, and codegen lowers no array at all;
  making it on-path would pull M4's ordered-hash-with-COW work into M3. `getTraceAsString()` lands now.
  Record the carry-over in the plan's M4 paragraph so it is not silently dropped.
- **`--fault-inject=<site>` is a real, deliberate CLI hook**, with a closed set of sites, of which
  `helper-panic` is the one the acceptance uses: the first runtime helper the script calls panics. It is a
  test hook for a failure mode that by definition has no user-facing trigger — a contained *engine* panic.
  Scope it to `mwl run` only; it must never reach `mwl serve` (M7), and say so in the flag's own doc.
- **Backtrace shape:** `#0 Class::method() at <file>:<line>`, resolved from MWL's own frame chain, never the
  platform unwinder — that is the whole point of
  [ADR 0002](../docs/adr/0002-error-propagation.md). `<file>` is the path as given on the command line, so
  the output is identical on both platforms. Source positions come from the stable per-statement ids
  [ADR 0018](../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md) already put in
  the IR — do not invent a second position table.
- **ADR 0018's call-site probe lands with the first compiled call site**, in `emit_call`'s single path, the
  same way the statement-boundary probe landed with the safepoint poll. That ADR names M3 and its argument
  is precisely that this is not retrofittable. Its `TRACE`/`PROFILE` entry/exit pair is not a later slice.
- **The error path's refcount cleanup lands with calls**, not after. `mwl-codegen` known gap 3 and the
  `mwl-ir` error edge are one piece of work; a backend that leaks on every throw is not a backend that has
  exceptions.
- **Backlog items are off-path unless the goal needs them.** `for`/`switch`, ADR 0043 `by`-delegation,
  ADR 0061 autoload, ADRs 0053/0054, inline HTML at file scope, the immortal string literal: if a slice is
  not on the path to the acceptance list, put it in `## Backlog` and move on. `arith.mwl` uses `while`
  deliberately so `for` stays off the path.
- **Doc trimming is authorized when — and only when — `brief.py` reports a truncated section.** Run
  [DOC_CLEANUP_PROMPT.md](../DOC_CLEANUP_PROMPT.md)'s pass on **that one doc**, not a repo-wide rewrite.
  Two are already over: `docs/implementation-plan.md`'s milestone section, and `crates/mwl-ir/src/lib.rs`'s
  module doc, which has become a slice-by-slice changelog of exactly the kind CLAUDE.md forbids.

## The gaps that actually sit on the path

Named because none is visible from the milestone text, and each is work rather than a question:

- **No symbol table over a unit's own functions.** `mwl_ir::ir::InstKind::Call::target` is a
  `"Class::method"` label, and `mwl-cli`'s `run_run` lowers only `lower_script` — a file's methods are never
  lowered at all. Both halves are needed before one call executes.
- **`Concat` has no lowering**, and `mwl-runtime` has no `mwl_str_concat` primitive behind it. Every
  acceptance example except `hello.mwl` concatenates.
- **`try`/`catch` is not lowered anywhere** — `mwl-ir`'s module doc lists it with `for`/`switch` as absent.
  Unlike those two it is squarely on the path, and it is the consumer of the error edge above.
- **A `throw` must satisfy the return check.** A method declared `: int` whose body always throws has to
  count as returning. The acceptance examples dodge this by declaring the throwing chain `void`, so it is
  not a stop-the-loop item — but real code needs it and it is cheap next to everything else here.
- **`--dump-asm` does not exist.** `--dump-ir` is the model, and it prints *instead of* running; match that.
  It is the smallest item on this list and depends on nothing, so it is a good first commit after the bump.
- **Integer `Div`/`Mod` are refused on purpose** in `mwl-codegen` — `sdiv` traps the whole process on a zero
  divisor, which is a request-isolation failure, not a wrong answer. `arith.mwl` avoids both. Fixing them
  properly (a checked divisor plus a throw) is welcome once exceptions work, but is not acceptance.
