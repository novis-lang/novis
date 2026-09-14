# Handoff

## State

**Goal `m4-refusals` — Stages 3 and 4 are closed. Stage 5 (`throw` and `clone` through a tag) is next.**
`python tools/holes.py` reports **8** refusal sites, `UNATTRIBUTED: 0`, **13** guarded, and
`crates/nvs-ir/tests/refusals.rs`'s `CEILING` is **8** to match.

- The statement roster at `crates/nvs-ir/src/lower/stmt.rs:240` is arms rather than a catch-all: `E0101`
  for `var $x;`, `E0204`/`E0209`/`E0203` for `global`/function-scope `static`/`goto`, `E0215`/`E0216` for
  a free function or constant, and `E0233` for a declaration inside a body.
- `StmtKind::Error` and the wildcard stay `unreachable!`. Neither is a refusal with a code to name, which
  `crates/nvs-ir/src/lib.rs:200` sanctions and `holes.py`'s `CONSTRUCT` does not match; the wildcard is
  there only because `StmtKind` is `#[non_exhaustive]`, so the playbook bullet's first way out is the one
  taken and `nvs-syntax`'s surface is untouched.
- Stage 4 lowers: `$f(...)` over a `callable` answers the closure the callee already holds, retained once
  where that read borrowed a slot this frame does not own. Probed against PHP over five callee shapes —
  a local, a closure literal, a parameter round-trip, a bound instance-method reference and a call's own
  result — and the two outputs are byte-identical.
- Two codes the roster names had no conformance case and now have one, under
  `tests/conformance/reject/`: `a-var-declaration-with-no-initializer-is-refused-at-the-missing-equals`
  and `goto-is-refused-at-the-keyword`. `goto` is refused by no rule fragment under `docs/rules/`, so
  that case's `--TEST--` line cites none — see `## Backlog`.
- Nothing is blocked.

## Next group

**Stage 5: `throw` and `clone` through a tag** — one file set: `crates/nvs-ir/src/lower/exception.rs`,
`crates/nvs-ir/src/lower/expr.rs`, and the runtime throw path those two share under
`crates/nvs-runtime/src/`. `docs/agent/loop-goal.md` § *Stage 5* is the spec; PHP's wording is the
authority on every message, so each operator's differential case decides it.

- [ ] **A tagged `throw` operand is untagged behind one check** — `crates/nvs-ir/src/lower/exception.rs:30`,
      whose assert is the last `Ty::Object` claim on that path. An object inside the `Throwable` tree takes
      the path a `Ty::Object` operand already takes; anything else throws PHP's `Error` in PHP's wording
      ("Can only throw objects", "Cannot throw objects that do not implement Throwable"). The checker
      passes `mixed`/`object` here on purpose (`crates/nvs-types/src/expr/members.rs:451`), which the
      goal's § *Standing decisions* settles as a shape to lower rather than refuse.
- [ ] **A tagged `clone` operand, through the same helper family** — `crates/nvs-ir/src/lower/expr.rs:5295`.
      `clone null` is "__clone method called on non-object". One implementation, not a second convention:
      this is the erased receiver's throw path (`crates/nvs-ir/src/lib.rs:319`) applied to an operator.
- [ ] **A differential case per operator** — `tests/differential/lang/`, suffix `matches-phps`, since the
      wording is what is being pinned. Both cases run the same erased throw path the tagged sites reach,
      `crates/nvs-ir/src/lib.rs:319`. `CEILING` falls to 6 in the same slice.

## Backlog

- Stage 3's roster codes are guarded but unchecked by its own acceptance check: `want` at
  `docs/agent/loop-goal.toml:9594` still names only the six earlier codes.
- `goto`'s refusal has no rule fragment; `docs/decisions/0007.md:173` is its only home — `docs/rules/`.
- `docs/agent/loop-goal.md`'s Stage 3 and Stage 4 line anchors are stale — that file.
- Taking `#[non_exhaustive]` off `StmtKind` would make a new variant fail to compile in `nvs-ir`; it is a
  decision about `nvs-syntax`'s public surface and was not taken — `docs/agent/playbook.md`.
