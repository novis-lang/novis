# Handoff

## State

**Goal `test-doubles` — `Core\Test`'s double half — has just started; nothing of it has landed yet.**
Goal `decided-closures`'s whole list is this goal's Stage 1 floor.

Settled before the first session: ADR 0079 §§ 10, 11 and 16 hold the design and the goal prose's
§ *Standing decisions* pre-authorises every call a session meets — no new syntax, a double is an
ordinary `Core`-owned object, strict always. The six ratchet keys were re-owned by hand on 2026-09-18
(`crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:24-30` name this goal and goal
`bigint`), and the two `Core\Test` bullets left `docs/agent/carried-gaps.md` § *Unowned* the same day,
so stage 0 is the `test.rs` module-doc section alone.

## Next group

**Stage 2, the checker** — one file set: `crates/nvs-types/src/expr/calls.rs`,
`crates/nvs-types/src/expr/args.rs`, `crates/nvs-types/src/core_lib.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-diagnostics/src/lib.rs`.

- [ ] **The two rows** — `crates/nvs-stdlib/src/test.rs:321`'s `CLASS` gains `double` and `partial`
      with a written `T` and a return type of `T`; `crates/nvs-stdlib/src/registry.rs:532`'s `CoreTy`
      is where a written type parameter is already spelled.
- [ ] **`T` must be an interface, and the shape is checked against its method set** —
      `crates/nvs-types/src/expr/calls.rs:126`'s `check_written_type_args` is where the written `T`
      arrives; the refusals are stage 2's items 1 and 2, two new codes.
- [ ] **Each closure's signature against the method's** — item 3, the ordinary assignability check
      over the closure literal's declared types.
- [ ] **The two `reject/` cases and the `core/` case** from ADR 0079 § *Verification*'s § 10 bullet,
      so the checker's half has its proof before the runtime's half exists: a double that fails to
      check needs no runtime.

## Backlog

- **Stage 3, the runtime** — `crates/nvs-stdlib/src/test.rs`, `crates/nvs-stdlib/src/instance.rs`,
  `crates/nvs-runtime/src/object.rs`, `crates/nvs-runtime/src/closure.rs`,
  `crates/nvs-runtime/src/dispatch.rs`, and `crates/nvs-ir/src/lower/expr.rs` if the descriptor is
  built at lowering. The one open design call — where the per-site descriptor is built — is the
  session's, recorded in `test.rs`'s module doc.
- **Stage 4, the two assertions** — `test.rs`, `registry.rs` (the method-reference `CoreTy`),
  `crates/nvs-types/src/expr/args.rs`. Shares files with stage 2; take it in the same session if
  stage 2 lands under the ceiling.
- **Stage 5, `assertCompletes`** — `test.rs` beside `advance`, one file.
- **Stage 6, the rulebook and the gate** — `docs/rules/testing.json`, the ratchet, `python tools/rules.py --render`.
- When this goal's last check goes green the driver takes goal `bigint`.
