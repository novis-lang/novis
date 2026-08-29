# Handoff

## State

**Stage 6's item 20 is one construct short of two.** `await` is a prefix expression now:
`ExprKind::Await` in the AST, `crates/nvs-syntax/src/parser/expr.rs:2049` parsing it with a
`parse_unary` operand exactly as `clone` does, arms in all five `ExprKind` walkers, and a
`nvs-types` refusal under a new `E0776` beside `spawn script`'s `E0703`. `examples/isolate.nvs`
parses whole and reports only those two codes; it used to stop at `E0319` on the `await` line.

**`await` is contextual, not reserved** — `crates/nvs-syntax/src/token.rs`'s module doc owns why, and
`Parser::at_await_operand` owns the disambiguation and the one spelling it claims (`await($x)` is the
operator, not a call). `docs/spec/00-overview.md` § 2 is the grammar's home.

**What `ScriptResult` should be is narrower than it looked.** The example writes `$r->ok` and
`$r->output`, and a `Core` instance has **no property a program can reach**
(`crates/nvs-stdlib/src/registry.rs:775` — `slots` is read by helper bodies, not by source). The
shape `Core\Task::all` already answers with is read with `->` and lowers today
(`examples/tasks.nvs:19`), so a shape type is the candidate and a registered class is the thing that
would need new machinery. Deciding that is the next slice, not a settled fact.

**Orientation gaps, unchanged from the previous session:** `[context] adrs` prints ADR 0023 § 2 only
— ADR 0006's `## Decision`, *What is and is not shared*, *Failure is a value* and *Output is captured
by default* are the sections a Stage 6 slice is written against and none is in the manifest.
`[context] modules` has no pattern for `nvs-syntax/src/parser/`, `nvs-types/src/expr/`,
`nvs-stdlib/src/registry.rs` or `nvs-cli/src/`.

## Next group

**`ScriptResult` and the type of the handle — the same two files each time.** File set:
`crates/nvs-types/src/expr/mod.rs` and `crates/nvs-types/src/expr/args.rs`, with
`crates/nvs-stdlib/src/registry.rs` read-only for the first.

- [ ] **`ScriptResult`'s shape, decided and recorded** — `->ok`, `->output`, `->error` are what
      `examples/isolate.nvs` reads, and ADR 0006 § *Failure is a value* is what they mean. Decide
      **shape vs. registered class** on the fact above and record it in the module doc that owns the
      answer. Anchors: `crates/nvs-types/src/expr/args.rs:799` (`bind_callable_shape`, the one place
      a `Ty::Shape` is built from a written argument today), `crates/nvs-types/src/expr/mod.rs:766`
      (the `Await` arm answering `mixed`), `crates/nvs-stdlib/src/registry.rs:775`.
- [ ] **`spawn script` types to the handle and `await` types to the result** — the pair, replacing
      both refusals' `mixed` with the two types, so `$r->ok` is a `bool` at the use site with no
      cast. Anchors: `crates/nvs-types/src/expr/mod.rs:742` (`E0703`'s site) and `:766` (`E0776`'s).
      Retire whichever code the slice makes unreachable, and never reuse the number.
- [ ] **`await` lowers** — the roster comment at `crates/nvs-ir/src/lower/expr.rs:423` is the proof
      that must be re-subtracted, and `crates/nvs-runtime/src/script.rs` plus
      `crates/nvs-host/src/isolate.rs` are the seam the lowering calls. Take this only after the two
      above; it is the slice that closes the acceptance check.

## Backlog

- ADR 0006's four sections are not in `[context] adrs` — `docs/agent/loop-goal.toml`.
- `Core\Secret::reveal()` is still absent, so item 18's escape hatch is open at both ends —
  `docs/plan/m5.md`.
- The 1000-case conformance floor is M4's residue; the suite reads 966 — `docs/implementation-plan.md`.
- No `.nvst` case pins the `await` refusal, deliberately: `E0776` is retired by the group above.
