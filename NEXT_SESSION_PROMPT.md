# Next session prompt

## State

Milestone **M2** (HIR/types/IR) is close to done; **M3** (baseline Cranelift backend → `Hello, World!`) is
the loop's target, per `.claude/loop-goal.md`. Run `python .claude/brief.py` first, then read
`docs/implementation-plan.md`'s M2/M3 paragraphs — the plan is the one home for status detail, this file
only points.

On disk: `mwl-diagnostics`, `mwl-syntax`, `mwl-hir`, `mwl-types`, `mwl-ir`, `mwl-cli` (`ast`, `check`),
`fuzz/`, `benches/abi-probe`. Workspace is green (build/test/clippy/fmt). `mwl-codegen`, `mwl-runtime` and
`examples/` do not exist yet.

**Nothing is blocked.** Every gap between here and the acceptance command — including the two that are
merely missing files (`mwl run`, `examples/hello.mwl`) — is enumerated with its already-decided design in
`.claude/loop-goal.md` § *The gaps that actually sit on the path*. Read that section before picking work;
do not restate it here.

**[ADR 0062](docs/adr/0062-case-sensitivity-is-a-compiler-property.md) is decided *and implemented*** —
unlike 0051–0061 below, it landed with code. Nothing case-related is left open: names resolve
case-sensitively, keywords/contextual keywords/`<?mwl` are lower case only, and `require`'s literal path is
compared to the on-disk entry exactly (`E0314`), which closes for `require` the portability hole ADR 0061
§ 1 had closed only for `autoload`. Two consequences worth knowing before touching the front end: `IF` is
an ordinary `Ident` with **no** diagnostic of its own (ADR 0029/0032 make it a legal class name — see
ADR 0062 § 2 before "fixing" this), and `Core\Bytes` no longer collides with the `bytes` type keyword.

**ADRs 0051–0061 are decided and wired** into CLAUDE.md, `docs/adr/README.md`, the plan and the spec. No
code implements any of them yet. Four carry obligations landing **before** M8, so they touch the work in
front of you — each ADR's own *Verification* section is the one home for its split:

- [ADR 0053](docs/adr/0053-iteration-and-generators.md) — `Iterable`/`Iterator` are the only iteration
  interfaces; generators lower to an explicit **state machine**, so `mwl-ir` must be able to represent a
  suspension point inside a loop body.
- [ADR 0054](docs/adr/0054-decimal-scalar-type.md) — `decimal` is a scalar: `m` literal suffix in
  `mwl-syntax`, conversion/arithmetic rows in `mwl-types`, i128 lowering at M4.
- [ADR 0055](docs/adr/0055-extension-qualifier-declarations.md) — the WIT world must carry a
  `tainted`/`secret` axis. Nothing to do now; recorded so M8/M9 does not forget it.
- [ADR 0061](docs/adr/0061-compile-time-autoload-and-program-discovery.md) — `autoload` resolves names to
  files at compile time, paths relative to the declaring file. M2 half is grammar plus a fixpoint over
  `mwl-hir::requires`' existing worklist; M6/M7 add cache edges, M8 adds `Core\Program`.

## Next

**The script body is a function** — the largest gap on the path and the one everything else waits on;
`.claude/loop-goal.md` names the decided design. Confirmed by inspection: `echo $undefinedThing;` at file
scope passes `mwl check` clean, while the identical line inside a method reports `E0301`. Land the checker
half first — it is testable on its own with the fixtures `check.rs` already has — then the `mwl-ir` half.

**Before widening `mwl-ir` further**, spend a short pass deciding how a suspension point inside a loop body
will be represented, and record it in `crates/mwl-ir/src/lib.rs`'s module doc under *Design choices worth
knowing before widening this further*. This is a **design note, not an implementation** — ADR 0053's
state-machine transform belongs in M4. The script-body work is about to widen the IR, and foreclosing the
representation is the expensive mistake, exactly as it would have been for ADR 0018's probe ids.

## Backlog

- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT` —
  `ImplementsClause.by_field` has parsed since M1 and is still unread by `mwl-hir`/`mwl-types`
  ([ADR 0043](docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md) §§ 4-5).
- `for` loops in `mwl-ir` (reuses `LoopFrame` verbatim), then `switch` (needs a distinct frame kind and
  PHP fallthrough semantics decided) — `mwl-ir`'s own module docs hold the known-gap list.
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
authoritative and the other is a bug. Keep work small and commit each finished step. When a session ends,
overwrite this file with the next prompt rather than appending to it.
