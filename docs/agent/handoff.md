# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32 and 33 are closed, and item 34 is open**: twelve of its seventeen findings are
ticked, D35 closed this session. The item-34 `[[check]]`'s first unwritten case is now its
**twelfth**, `tests/conformance/core/json-decodes-a-nested-field-and-a-promoted-one.nvst` (findings
D7/D8) — none of the three cases after the eleventh exists on disk, so each of them is one
acceptance failure in the check's own order.

**`Core\Json::decodeAs<array<C>>` is the typed list decode, and the list-ness rides beside the
descriptor rather than being read off the document.** `nvs_types::expr::args::written_class_of`
records the *element* class plus a flag, `ResolvedCall::written_class_is_list` carries it, and
`nvs-ir` emits it as an `InstKind::ConstBool` in the slot after the `ClassDescConst` — so the helper
is now `args: [4]` and `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS`' docs own that ABI. A helper
guessing from the JSON's own shape was the alternative and is unsound: the checker has already given
the call site `array<C>` or `C`, and the guess can answer with the other one.

**A list refuses at its first bad element, on purpose.** ADR 0071 § 5 accumulates issues within an
object because a class's field count is a bound the *program* wrote; a list's length is a bound the
*document* wrote, so accumulating across elements would do work in proportion to what an attacker
sent. `nvs_stdlib::json::decode_each`'s doc owns that reasoning. Issue paths gain the element's
position — `1.x` — through `json::path_of`.

**A JSON array reaching a scalar `decodeAs<C>` still reports every field missing** rather than
naming the shape, because both JSON shapes are one `NvsArray` and only the list side tells them
apart (a packed array has index 0). Asymmetric, pre-existing, and in `## Backlog`.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247` and `E07xx` is `E0794`; this session added no diagnostic.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1068,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's own
ADRs; **0071 §§ 2, 4-5** is this session's proven one, and **0013 §§ 2-4**, **0046 §§ 2, 4-5**,
**0053 §§ 1-3**, **0007 §§ 2-3**, **0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**,
**0011**, **0086 § 6** and **0096 §§ 1-1a** are the earlier ones. `[context] modules` misses every
file this session edited — `crates/nvs-stdlib/src/json.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-types/src/expr/args.rs`, `crates/nvs-types/src/expr_table.rs`,
`crates/nvs-ir/src/lower/closure.rs` and `crates/nvs-ir/src/lower/expr.rs` — and still misses
`crates/nvs-types/src/retrieval.rs`, `defaults.rs`, `consts.rs`, `expr/members.rs`, `generics.rs`,
`expr/assign.rs`, `expr/iteration.rs`, `check.rs`, `returns.rs`, `attributes.rs`, `derive.rs`,
`routes.rs`, `commands.rs`, `testing.rs`, `crates/nvs-stdlib/src/arr.rs`, `serialize.rs`,
`secret.rs`, `crates/nvs-hir/src/errors.rs`, `members.rs`, `crates/nvs-ir/src/lower/generator.rs`
and `call.rs`. `orient.py` itself still warns that `crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**The item-34 `[[check]]`'s twelfth case is D7 and D8 together — what `#[Json\Derive]` may call a
field — and its file set is `crates/nvs-types/src/derive.rs` with the codec table in
`crates/nvs-runtime/src/object.rs` and both halves of the codec in `crates/nvs-stdlib/src/json.rs`.**

- [ ] **D7 a nested-class, array or enum field is a FATAL, not a decode** —
      `docs/reference/findings.md:219`. Every declared type outside the five scalars erases to
      `CodecTy::Opaque` at `crates/nvs-types/src/derive.rs:232`, and `nvs_stdlib::json` turns that
      into `Fault::fatal` before the document is even read,
      `crates/nvs-stdlib/src/json.rs:929`. The table to widen is `CodecTy` at
      `crates/nvs-runtime/src/object.rs:484`; the decode arm is
      `crates/nvs-stdlib/src/json.rs:1184` and the naming arm `crates/nvs-stdlib/src/json.rs:1232`.
      A nested class needs its own descriptor reachable from the field, which is the design call
      this slice makes and records in `derive.rs`'s module doc.
- [ ] **D8 a promoted constructor parameter is not a `#[Json\Derive]` field** —
      `docs/reference/findings.md:221`. The field walk that misses it is in
      `crates/nvs-types/src/derive.rs:217`; ADR 0071 § 2's "every field is a same-named constructor
      parameter" is already resolved to a `param` index at
      `crates/nvs-runtime/src/object.rs:520`, so a promoted parameter is the same row built from a
      different declaration site.
- [ ] **The case both findings are closed by** —
      `tests/conformance/core/json-decodes-a-nested-field-and-a-promoted-one.nvst`, named at
      `docs/agent/loop-goal.toml:339`. The shape to copy is
      `tests/conformance/core/json-decodes-a-typed-list.nvst:1`, and the encode half it must agree
      with is `crates/nvs-stdlib/src/json.rs:1184`.

## Backlog

- A JSON array under a scalar `decodeAs<C>` reports every field missing instead of naming the shape
  — `crates/nvs-stdlib/src/json.rs`'s `decode_object`, and the discriminator `decode_each` uses.
- D34 `Core\Arr::sort` on `array<decimal>` throws at run time — `docs/reference/findings.md:297`.
- The item-34 check's last two cases, `script-args-are-read.nvst` and
  `task-after-response-runs.nvst` — `docs/agent/loop-goal.toml:340`.
- Stage 9's ADR 0119 lowering, items 21–23 — `docs/agent/loop-goal.md`.
- `[context]` in `docs/agent/loop-goal.toml` misses every module and ADR named in `## State`.
