# Handoff

## State

**ADR 0135 is accepted, and it answers both halves of the question that blocked `Core\Db::open`.** A
fixed-key shape parameter is **one** `CoreTy::Shape(&[&[CoreField]])` — a list of *arms* — and it flattens
at the call site into one ABI argument per field of the arms merged in order, exactly as ADR 0063 R2's
options bag already does. The ADR's own body is the rule; the two things worth carrying forward are why the
obvious answers lose. The union is **not** `CoreTy::Union([Shape(A), Shape(B)])` because `Ty::Union` is
sorted by member `TypeId` and a shape's field order **is** the ABI, so the interner would reorder the arms
under it. And a runtime shape value passed as one argument loses because no runtime representation of a
shape exists at all (`ExprKind::ObjectLiteral` has no lowering), so it trades a merged field list in
`lower_call_args` for a new value kind in the IR and the runtime, to serve one member.

**No new diagnostic code is needed, and that was nearly forced rather than chosen.** `E0453`/`E0454` widen
from "option" to "shape key", and a *missing* required key is an ordinary argument-type mismatch — an
*extra* key is the one that ever needed a code, because ADR 0036 § 3's width subtyping would otherwise
accept it. Both type diagnostic bands are full, so a new code would have opened a third band as a side
effect of adding a parameter kind.

**Nothing is built.** `CoreTy::Shape` does not exist, `Core\Db::open` is still absent, and the goal's one
open acceptance check — `a_db_open_target_in_a_denied_range_fails` — still does not run, which stays the
ordinary state of an open item. The two module docs that said the decision was unmade now point at 0135
instead: `nvs_stdlib::db`'s known gap 1 and `nvs_stdlib::registry`'s § *The options bag*.

## Next group

**One feature down its spine, ADR 0135 §§ 1-5 in order. File set:
`crates/nvs-stdlib/src/registry.rs`, then `crates/nvs-types/src/ty.rs` with
`crates/nvs-ir/src/lower/call.rs`, then `crates/nvs-stdlib/src/db.rs`.** Take the first two together if the
budget holds — the second cannot compile without the first.

- [ ] **Add `CoreField` and `CoreTy::Shape`, with the three invariant tests** (0135 §§ 1-2).
      `crates/nvs-stdlib/src/registry.rs:189` is the enum, `crates/nvs-stdlib/src/registry.rs:604` is
      `CoreTy::Options` for the new variant to sit beside, and
      `crates/nvs-stdlib/src/registry.rs:626` is `CoreOption`, which `CoreField` is modelled on except that
      its `default` is an `Option<Const>` — `None` is required. The tests are
      `a_shape_is_only_ever_a_whole_parameter`, `a_shapes_arms_are_pairwise_disjoint` and
      `a_shape_field_is_never_nullable`, beside `crates/nvs-stdlib/src/registry.rs:2380` and
      `crates/nvs-stdlib/src/registry.rs:2589`.
- [ ] **Rename `Ty::Options` to `Ty::CoreShape`, give it per-field required-ness, and flatten a shape
      argument** (0135 §§ 3-4). `crates/nvs-types/src/ty.rs:271` is the variant and its doc comment,
      `crates/nvs-ir/src/lower/call.rs:198` and `crates/nvs-ir/src/lower/call.rs:275` are the two
      `CheckedTy::Options` sites that do the flatten, and `crates/nvs-cli/src/meta.rs:270` is the third
      reader. `crates/nvs-diagnostics/src/lib.rs:830` and `:835` are `E0453`/`E0454`, whose wording widens
      from "options bag" to "shape key".
- [ ] **`Core\Db::open`'s five edits, and `a_db_open_target_in_a_denied_range_fails` with them** (0067
      §§ 2-3, and 0135 § 3 for the twelve ABI slots). `crates/nvs-stdlib/src/db.rs:66` is the known gap
      this closes; the field list is `docs/spec/01-core-library.md` § 18 and is not written anywhere else.

## Backlog

- Both type diagnostic bands are full — `E04xx` at `E0499` and `E07xx` at `E0799` — so the next genuinely
  new type diagnostic needs a band opened; `crates/nvs-diagnostics/src/lib.rs`'s own legend owns that, and
  its line 2759 records that `E0800` was deliberately passed over once already.
- `[context]` gaps this session paid for: `adrs` wanted `0036 § 3` and `0047`'s *In short*, and there is no
  selector for `docs/spec/01-core-library.md` § 18 at all — three peeks the pack could have carried.
- MySQL runs only `query` of the four members that send — `nvs_stdlib::db`'s known gap 2.
- Whether a server under `MARIADB_CLIENT_BULK_UNIT_RESULTS` *continues* past a refused set is the one fact
  that reopens ADR 0067 § 4's N-executions decision; it needs a real server.
