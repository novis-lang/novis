# Loop goal

Finish milestone **M4** *and* **M4S Part I** — read those two paragraphs in `docs/implementation-plan.md`
for scope; do not re-derive them here. Two milestones in one loop is deliberate: `Core\Arr`'s entire
contract rests on the copy-on-write array M4 builds, and building the array without its only real consumer
is how you get a representation that has to be rebuilt.

The one thing standing under all of it: **`mwl_ir::Ty::Object` refcounts nothing.** No field layout, no
instance dispatch, no array. `InstKind::New`/`FieldGet`/`FieldSet`/`ArrayNew`/`ArrayGet`/`ArraySet`/
`ArrayAppend` all return `CodegenError::Unsupported`, and an *instance* `Call` is refused for the same
reason (`mwl-codegen` known gap 1). Nothing else either milestone owes has a site to attach to until that
lands. **Start there.**

## Acceptance

**The checks themselves live in [`loop-goal.toml`](loop-goal.toml), and only there.** Every fixture, its
exact expected output, the two suites and every named guard test are in that file as data; the driver reads
it directly, so there is nothing here that could drift out of sync with what actually runs. Read it, or run
`python tools/loop.py --list` for the same thing as a summary.

The driver runs those checks in order, short-circuiting on the first failure, so the ledger line each
iteration writes tells you exactly how far the loop got. Every check must pass. Nothing else counts as done
— not a passing unit test, not a session claiming `DONE`. What the check kinds mean, how the native/WSL legs
and the valgrind sweep are ordered, and why: [coordinator.md](coordinator.md) § *The acceptance test*.

**The expected output in that file is frozen; a fixture's *source* is not.** A fixture may be corrected to
a spelling the docs already fix — its author had to write `Core\Arr::sort`'s option bag and `Iterator<T>`'s
shape from the spec, and a spelling error there is a bug in the fixture, not a decision. What may never
change is the expected output, or the fixture's reason for existing. Re-freeze, record why in the commit
message, move on. That is not licence to weaken a check to make it pass.


## Standing decisions — pre-authorized, do not stop the loop for these

Every one of these was settled with the user before the loop started. Implement it; do not re-open it.

- **Decide and record; never `BLOCKED` for a design call.** The object header and field layout, the array's
  ordered-hash representation, the `mixed` runtime type tag, the `Throwable` constructor's exact signature,
  and how generics are represented in `mwl-types` are all yours to settle under AGENTS.md's priority
  ordering. Record each in the home AGENTS.md already names — a paragraph in `docs/adr/README.md`
  § *Decisions taken at project start*, or the crate's own module doc. **Do not open a numbered ADR for
  these.** Reserve `BLOCKED` for a decision that is expensive to reverse *and* has no safe default.
- **The exception surface is [docs/spec/01-core-library.md](../spec/01-core-library.md) § 10, not
  ADR 0020 § 1.** ADR 0020 explicitly deferred the exact shape to stdlib work, and § 10 *is* that work.
  So: `Throwable` is the root and user classes extend it directly; the tree is `LogicError`,
  `RuntimeError` (with `IOError`, `ParseError`, `TimeoutError`) and `ArithmeticError`; members are
  **readonly properties** — `$e->message`, `$e->previous`, `$e->backtrace`, `$e->location` — not
  `getMessage()`/`getTraceAsString()` accessors; and there is **no `Exception` and no `Error`** class.
  Amend ADR 0020 § 1 to point at § 10 rather than restate it, and migrate the three M3 fixtures.
  `$e->backtrace` is the array form M3 carried over; the string form it landed becomes a rendering of it.
- **Type variables stay compiler-owned.** Implement real `<T>` machinery in `mwl-hir`/`mwl-types` —
  declaration, substitution, and call-site inference from argument types — but expose it only to
  declarations the compiler owns, which is what [ADR 0007](../adr/0007-explicit-type-system.md)'s
  *Revisiting* already assumes and [ADR 0053 § 2](../adr/0053-iteration-and-generators.md) states
  outright. User code gets exactly two things: implementing a compiler-owned generic interface at a
  concrete type (`implements Iterator<User>`), and the explicit call-site type argument
  `Core\Attributes::get<T>` needs. **User-defined generic classes stay deferred** — that is a
  language-surface decision ADR 0007 parks, not this loop's to take.
- **`Core` is native Rust, all of it.** Every §§ 1–12 member is a Rust function in `crates/mwl-stdlib`,
  reached through a signature table the checker knows and a helper symbol codegen emits — the mechanism
  the existing runtime helpers already use. The plan's § *Architecture* calls Tier 0 "compiled into the
  binary, native, direct heap access, no boundary"; that is meant literally, and no part of `Core` is
  written in MWL. The crate is **`mwl-stdlib`**, the name ADR 0003 § *Tier 0*, the README and the
  workspace manifest already use — the M4S paragraph's one-off `mwl-core` was the outlier and is fixed.
- **Spec §§ 1–12 only.** § 13 (`Reflect`, `Ast`, `Attributes` retrieval, `Program`, `Decimal`, `BigInt`,
  `Test`) is **out**: each depends on something outside `Core` — ADR 0061's autoload, ADR 0019's inert
  AST, or a finished object representation. The spec file's own *Milestones* line says §§ 1–13 and is
  wrong; correct it to §§ 1–12 and note where each § 13 entry actually lands. ADR 0046's `#[...]`
  *syntax* is still in scope — only the retrieval body is not, which is what M4 already says.
- **Settle [ADR 0009](../adr/0009-string-and-bytes.md) early, by measurement.** It is *Proposed*, and
  `Core\Str::length`/`at`/`slice` cannot be conformance-tested until its granularity question lands — the
  spec says as much. Implement both granularities behind one seam, take the cost measurement its
  *Revisiting* asks for, write the figure into `a_grapheme_index_costs_more_than_a_code_point_index` in
  `benches/abi-probe/tests/perf_guards.rs` (where measured numbers live, per AGENTS.md), pick the default
  it justifies, and move the ADR to **Accepted**. Do this before writing `Core\Str`'s conformance cases.
- **No cycle collector.** Refcounting only. A cyclic object graph in a CLI script is retained until the
  process exits; record that boundary in `mwl-runtime`'s module doc and scope Stage 6's leak check to
  acyclic fixtures so it stays a true signal. The plan's § *Architecture* already fixes the policy — the
  wholesale request-heap drop makes cycles structurally unable to accumulate in the server, and the
  optional mark-sweep collector belongs with M5/M6, where the stop-the-world path and the request arena
  exist. Do not build it here.
- **Dependencies: choose them, record them.** `Core\Regex`, `Core\Time`, `Core\Hash`, `Core\Random`,
  `Core\Uri` and `Core\Csv` all need outside code. Pick pure-Rust crates against
  [ADR 0051 § 4](../adr/0051-standard-library-tiers.md)'s two-question test, keep `cargo deny check`
  green, and record each pick with its reasoning in that module's own doc comment. Stop only if a needed
  capability has no pure-Rust option at all — that is a real `BLOCKED`, naming the capability.
- **The runner lives in `crates/mwl-test`**, which the README's crate table already reserves; `mwl-cli`
  exposes it as the `mwl test` subcommand. **The `.mwlt` format is a superset of `.phpt`'s sections**,
  per M4. Two sections are new:
  `--ORACLE--` holds a PHP twin whose output the case's own output must match byte for byte, which is how
  Stage 4's differential leg proves PHP compatibility instead of freezing a belief about it; and
  `--ORACLE-DIVERGES--` holds a one-line reason plus that case's own `--EXPECT--`, for the divergences
  ADR 0063 took deliberately. PHP 8.5 is on `PATH` on both legs.
- **M4's CLI-program bullet is scoped down, deliberately.** It asks for "an argument-parsing
  file-processing tool", but `Core\Cli` (§ 15) and `Core\IO` (§ 14) are Part II / M8 and capabilities do
  not exist until M6 — so neither argv nor file reading is reachable here. `examples/report.mwl` is that
  bullet's computational half; record in the plan that its argv/file half moves to M8 with §§ 14–15.
- **ADR 0018's `BRANCH` probe lands with the branch lowering it needs** — the last of that ADR's three
  sites still not emitted (`mwl-codegen` known gap 2), and the object work touches terminator lowering
  anyway.
- **Backlog items are off-path unless the goal needs them.** If a slice is not on the path to the
  acceptance list, put it in `## Backlog` and move on.
- **Doc trimming is not loop work, ever.** Nothing measures doc size (AGENTS.md § *Length targets*), and
  [doc-cleanup.md](doc-cleanup.md)'s pass is never run from inside the loop.

## The gaps that actually sit on the path

Named because none is visible from either milestone's text, and each is work rather than a question:

- **A `catch` binding is function-scoped today**, so two clauses on one `try` cannot both bind `$e` —
  `E0406` fires on the second. PHP allows it, and every PHP program in existence writes it. This is a
  decide-and-record call, not a `BLOCKED`: either scope a `catch` binding to its own handler block, or
  keep the current rule deliberately and say why in `mwl-types`' own module doc. `examples/errors.mwl`
  sidesteps it with four distinct names, so it is not on the acceptance path either way.
- **The constructor is spelled `function constructor(...)`**, and the parent call is
  `parent::constructor(...)` — `mwl-types`' own tests and `ctor_init.rs` are the authority.
  [ADR 0043](../adr/0043-interface-default-methods-and-delegation-replace-traits.md)'s illustrative
  examples omitted `function` and have been corrected; if another doc example does the same, it is a bug.
- **`implements Iterable<int>` does not parse yet.** A generic interface in an `implements` clause is
  [ADR 0053 § 2](../adr/0053-iteration-and-generators.md)'s one narrow extension, and it is the parse
  error `examples/iterate.mwl` hits today.
- **`mwl_ir::lower::lower_file` skips interfaces and enums entirely**, for the same reason instance calls
  are refused: there is no object representation to lower them onto.
- **`lower_try` panics on a second `catch` clause and on `finally`.** `examples/errors.mwl` needs both,
  and a typed `catch` needs `instanceof`, which needs the class hierarchy at runtime.
  `mwl_ir::lower::is_global_throwable` accepts exactly three names today.
- **Integer `Div`/`Mod` are still refused** in `mwl-codegen` — `sdiv` traps the whole process on a zero
  divisor, which is a request-isolation failure. The throw path exists now, so this is a checked divisor
  plus a `Terminator::Throw`. `examples/core.mwl` uses `%`, so it is on the path.
- **A `throw` must satisfy the return check.** A method declared `: int` whose body always throws counts
  as returning. Cheap, and real code needs it.
- **A `THROWN` still leaks a temporary in flight** inside the expression that threw
  (`mwl_ir::lower::Lowering::landing_block`). Stage 6's valgrind leg will find it. It needs an
  owned-temporaries stack threaded through `lower_expr`.
- **A string literal still allocates on every evaluation** (`mwl-codegen` known gap 4). `Core\Str`'s
  conformance suite will make this loud.
- **`for` and `switch` are still absent from `mwl-ir`.** `switch` needs the N-way terminator generator
  resumption also wants, so ADR 0053's state-machine lowering and `switch` pair naturally.
- **`crates/mwl-ir/src/lib.rs`'s module doc is a slice-by-slice changelog** of exactly the kind AGENTS.md
  forbids. It is the one doc in the repo genuinely owed a trim. Backlog, not a reason to stop.
