# Next session prompt

## State

Milestone **M2** (HIR/types/IR) is close to done; **M3** (baseline Cranelift backend —
`Hello, World!`) is the loop's target, per `.claude/loop-goal.md`. Run `python .claude/brief.py` first,
then read `docs/implementation-plan.md`'s M2/M3 paragraphs — the plan is the one home for status
detail, this file only points.

On disk: `mwl-diagnostics`, `mwl-syntax`, `mwl-hir`, `mwl-types`, `mwl-ir`, `mwl-cli` (`ast`, `check`),
`fuzz/`, `benches/abi-probe`, and the `examples/hello.mwl` the acceptance command names. Workspace is
green (build/test/clippy/fmt). **`mwl-codegen` and `mwl-runtime` still do not exist.**

**The front end now reaches the acceptance program end to end.** The two gaps
`.claude/loop-goal.md` named first are closed: `mwl-types` checks a file's top-level statements as one
synthesized frame ([ADR 0008](docs/adr/0008-static-and-global.md) § 2), and `mwl-ir`'s new
`lower_script` lowers that same frame, with `echo` lowering beside it (`Helper::EchoStr`). A snapshot
test in `crates/mwl-ir/src/lower.rs` lowers `examples/hello.mwl`'s exact shape. `mwl check
examples/hello.mwl` is clean *because it is correct now*, not because nothing looked.

**Nothing is blocked.** Every remaining gap between here and the acceptance command is enumerated with
its already-decided design in `.claude/loop-goal.md` § *The gaps that actually sit on the path*. Read
that section before picking work; do not restate it here.

**ADRs 0051-0061 are decided and wired** into CLAUDE.md, `docs/adr/README.md`, the plan and the spec. No
code implements any of them yet. Four carry obligations landing **before** M8 — each ADR's own
*Verification* section is the one home for its split. One of the four moved this session:
[ADR 0053](docs/adr/0053-iteration-and-generators.md)'s "the IR must model a suspension point inside a
loop body" is now discharged as a *representation* decision, recorded in `crates/mwl-ir/src/lib.rs`'s
§ *Design choices worth knowing before widening this further*. Read that bullet before adding any
`Terminator` variant — in particular, `switch` and the resumption dispatch must share **one** N-way
terminator. The transform itself stays at M4.

## Next

**Create `mwl-runtime` and `mwl-codegen`, and add `mwl run`.** This is the whole remaining path to the
acceptance command, and it is now the only thing on it. `.claude/loop-goal.md` holds the decided design
for each; three points from it that are easy to miss:

- Give each new crate its own `[lints]` block with `unsafe_code = "deny"` and narrow reasoned allows —
  **not** `lints.workspace = true`, which is `forbid` workspace-wide and makes a JIT unimplementable.
  Both already have a `[workspace.dependencies]` entry, so creating the directory wires them in.
- ADR 0018's debug-flags probe check lands **in the first `mwl-codegen` commit**, with the safepoint
  poll — the narrow-backend authorization does not extend to deferring it — and is not considered
  landed until the `benches/abi-probe` guard test holding its all-bits-off cost exists.
- `mwl run` checks first and, on any diagnostic, reports and exits non-zero exactly as `mwl check`
  already does, rather than running anyway.

The runtime surface the slice actually needs is small: whatever backs `Helper::EchoStr` (raw bytes to
stdout, no escaping) plus the refcount retain/release and safepoint helpers `crates/mwl-ir/src/ir.rs`
already names. Land the runtime half first — it is testable on its own, without a backend.

## Backlog

- Inline HTML at file scope (`?>text<?mwl`) is not lowered — `mwl-ir`'s known gaps say why, and that
  it is the same `Helper::EchoStr` call `echo` already emits. Needs `mwl-types` to stop no-oping
  `StmtKind::InlineHtml` at the same time.
- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT` —
  `ImplementsClause.by_field` has parsed since M1 and is still unread by `mwl-hir`/`mwl-types`
  ([ADR 0043](docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md) §§ 4-5).
- `for` loops in `mwl-ir` (reuses `LoopFrame` verbatim), then `switch` — which per the new design-choices
  bullet must introduce the N-way terminator the generator resumption dispatch will also use, not a
  `Branch` chain. `mwl-ir`'s own module docs hold the known-gap list.
- The `mixed` runtime type-tag representation — `mwl-ir` known-gap item 5, real design work,
  pre-authorized.
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
