# Next session prompt

## State

Milestone **M2** (HIR/types/IR) is close to done; **M3** (baseline Cranelift backend → `Hello, World!`) is
the loop's target, per `.claude/loop-goal.md`. Run `python .claude/brief.py` first, then read
`docs/implementation-plan.md`'s M2/M3 paragraphs — the plan is the one home for status detail, this file
only points.

On disk: `mwl-diagnostics`, `mwl-syntax`, `mwl-hir`, `mwl-types`, `mwl-ir`, `mwl-cli` (`ast`, `check`),
`fuzz/`, `benches/abi-probe`. Workspace is green (build/test/clippy/fmt). `mwl-codegen` and `mwl-runtime`
do not exist yet — create them at M3 with their **own** `[lints]` block (`unsafe_code = "deny"`, narrow
reasoned allows), never `lints.workspace = true`, which is `forbid`.

**ADRs 0051–0060 landed** — the standard-library scoping set, decided in full and already wired into
CLAUDE.md, `docs/adr/README.md`, the plan and the spec. No code implements any of them yet. Three carry
obligations that land **before** M8 and therefore concern the work in front of you:

- [ADR 0053](docs/adr/0053-iteration-and-generators.md) — `Iterable`/`Iterator` are the only iteration
  interfaces (`ArrayAccess`/`Countable` do not exist), and generators lower to an explicit **state
  machine**, not to a coroutine. `mwl-ir` must be able to represent a suspension point inside a loop body.
- [ADR 0054](docs/adr/0054-decimal-scalar-type.md) — `decimal` is a scalar. Needs an `m` literal suffix in
  `mwl-syntax`, conversion/arithmetic rows in `mwl-types`, and i128 lowering at M4.
- [ADR 0055](docs/adr/0055-extension-qualifier-declarations.md) — the WIT world must carry a
  `tainted`/`secret` axis. Nothing to do now; it constrains M8/M9 and is recorded so it is not forgotten.

Nothing is blocked. The three gaps sitting directly on the acceptance command's path are spelled out, with
their already-decided designs, in `.claude/loop-goal.md` § *The three gaps that actually sit on the path* —
read that section before picking work.

## Next

**The script body is a function** — still the largest of those three gaps, and the one everything else
waits on. `mwl-types::check::check_stmts` walks declarations only (`_ => {}` swallows every top-level
statement) and `mwl-ir` exposes only `lower_method`, so a top-level `echo "...";` is today neither
type-checked nor lowered. Confirmed: `echo $undefinedThing;` at file scope passes `mwl check` clean, while
the identical line inside a method reports `E0301`.
[ADR 0008](docs/adr/0008-static-and-global.md) § 2 already decides the shape — a file's top-level
statements are one synthesized frame whose variables are locals — so this is reuse of
`check_method`/`lower_method`, not a second walk. Land the checker half first; it is testable on its own
with the fixtures `check.rs` already has.

**Before widening `mwl-ir` further**, spend a short pass deciding how a suspension point inside a loop body
will be represented, and record it in `crates/mwl-ir/src/lib.rs`'s module doc under *Design choices worth
knowing before widening this further*. This is a **design note, not an implementation** — ADR 0053's
state-machine transform itself belongs in M4. The point is that the script-body work is about to widen the
IR, and foreclosing the representation is the expensive mistake, exactly as it would have been for
ADR 0018's probe ids.

## Backlog

- `echo` lowering: no `StmtKind::Echo` arm in `mwl-ir`'s `lower.rs`, no `InstKind`/`Helper` behind it —
  `.claude/loop-goal.md` fixes the CLI semantics (raw stdout, no escaping).
- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT` —
  `ImplementsClause.by_field` has parsed since M1 and is still unread by `mwl-hir`/`mwl-types`
  ([ADR 0043](docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md) §§ 4-5).
- `for` loops in `mwl-ir` (reuses `LoopFrame` verbatim), then `switch` (needs a distinct frame kind and
  PHP fallthrough semantics decided) — `mwl-ir`'s own module docs hold the known-gap list.
- The `mixed` runtime type-tag representation — `mwl-ir` known-gap item 5, real design work,
  pre-authorized.
- A `set`-hooked property is exempted from ADR 0022's constructor check rather than verified against the
  hook's own writes.
- **New, from this session's ADRs, all small and independently landable:**
  [0054](docs/adr/0054-decimal-scalar-type.md)'s `m` literal suffix and untyped-until-placed fractional
  literals in `mwl-syntax`; [0053](docs/adr/0053-iteration-and-generators.md)'s `Iterable`/`Iterator` as
  reserved interface names alongside `Comparable`/`PropertyObserver`/`Stringable`, and `$obj[$k]` on a
  non-array refused with a diagnostic naming that ADR.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two documents state the same thing, the one CLAUDE.md names is
authoritative and the other is a bug. Keep work small and commit each finished step. When a session ends,
overwrite this file with the next prompt rather than appending to it.
