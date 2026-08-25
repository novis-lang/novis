# Handoff

## State

**Stage 0 is open, and it outranks everything below it.** [loop-goal.md](loop-goal.md) § *Stage 0*
holds items 1 to 17; **1 to 11, 16 and 17 are done**, so what is open is **items 12 to 15**, one
named test each at `stage = "0 catch-up"` in [loop-goal.toml](loop-goal.toml). `loop.py`
short-circuits at stage 0, so **Stage 3 is shut until those four clear**.

**Item 11 landed**: `Helper::SecretEq` → `mwl_runtime`'s `mwl_secret_eq` over
`subtle::ConstantTimeEq`. Two things had to be settled first and both are recorded in the crate docs
that own them — the qualifier does not survive `mwl_ir::ty::Ty`, so `mwl-types` records
`ExprInfo::SecretEquality` at the comparison and the lowering reads it back; and `erase_checked_ty`
had **no arm for any of the six qualified atoms**, so `secret string` did not lower at all. `mwl-ir`
gap 11 is rewritten to what is left of it: a `secret`-against-`mixed` pair still short-circuits.

**The workspace manifest was broken before this session started** — `members = ["benches/*"]` globbed
the untracked `benches/userland/`, so *every* `cargo` command failed. `Cargo.toml` now excludes it;
the playbook carries the trap. `benches/userland/` and `tools/bench.py` are still untracked, and are
item 15's `php_ratio` material — commit them with that item.

Verify is green (1545 tests, 73 suites, clippy and fmt clean); valgrind clean on a fresh-operand
`secret ==` probe. Conformance **434**, differential 89.

`examples/collect.mwl` still exits 1 at `Core\Out::capture`, genuinely blocked behind ADR 0088's sink
carriers; it is M4S work, not a slice to open ahead of the sinks.

## Next group — Stage 0 items 12 and 13, then 14, then 15

**Items 12 and 13 are one group and share `crates/mwl-stdlib/src/uri.rs`** — take them together.
Items 14 and 15 share no file with them or with each other; item 15 is by far the largest (a second
array representation with a degrade path, ~300–500 lines across `array.rs` and the ABI), so do not
size the set from it.

- [ ] **Item 12 — `$s as ?Uri` and `$s as ?Uuid` compile, `isValid` deleted from both.** ADR 0066
      §§ 1, 3. `Uri::isValid` is not written yet (`grep isValid` finds only `Encoding` and `Json`), so
      the work is the roster, not a deletion. Anchors: `crates/mwl-stdlib/src/uri.rs:63` (the module
      doc already naming ADR 0066 §§ 1, 3), the `?T` row at
      `crates/mwl-stdlib/src/registry.rs:215`, `Core\Uri`'s `CLASS` at
      `crates/mwl-stdlib/src/uri.rs:287` and its slots at `:456`. `crates/mwl-uuid` does not exist —
      `Core\Uuid` lives in `crates/mwl-stdlib/src/uuid.rs`. Closing test:
      `a_parse_roster_conversion_yields_null`-shaped, `-p mwl-types`; read the exact name out of
      `loop-goal.toml`'s item 12 block before writing it.
- [ ] **Item 13 — `Core\Uri` compares by normalized components**, with a round-trip property test.
      `uri.rs`'s own module doc owns the rule. Same file set as item 12, which is why they are one
      group; the comparison itself will want `mwl_runtime::identity`'s object-identity answer.
- [ ] **Item 14 — a call-stack limit rides the safepoint's emit site** (ADR 0020 § 1). Anchors:
      `crates/mwl-ir/src/ir.rs:266` (`InstKind::Safepoint`), the three emit sites at
      `crates/mwl-ir/src/lower/control.rs:204`, `lower/exception.rs:491` and `lower/generator.rs:650`,
      the codegen side at `crates/mwl-codegen/src/lib.rs:956`, and `mwl-ir` gap 14
      (`crates/mwl-ir/src/lib.rs:262`), which says safepoints are reserved rather than functional.
- [ ] **Item 15 — a second array representation with a degrade path**, plus the `php_ratio`
      benchmark that `benches/userland/` and `tools/bench.py` are already written for. Its own item in
      `loop-goal.md` § *Stage 0* holds the shape.

## Backlog

- `Core\Out::capture` — the last `spec-members-outstanding.txt` key; behind ADR 0088's sink carriers.
- `decodeAs<T>` for `Core\Json` — `json` gap 2, now that a written call-site type argument lands.
- ADR 0088's registry-wide qualifier classification for `Core` member rows — `docs/implementation-plan.md` § *Open now*.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s gap list.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- ADRs 0091/0092/0093 are decided and unbuilt; their milestones are M4/M6/M7/M8/M10.
