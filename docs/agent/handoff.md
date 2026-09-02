# Handoff

## State

**ADR 0135's checker and IR halves are both on disk.** `Ty::CoreShape` carries a `required` flag per
key; `nvs_types::core_lib` records § 3's fill list for a shape parameter — the merged list's order,
each slot's own default or `Const::Null` — and `nvs_ir::lower::shape_fills` reads it, so a written
shape literal flattens into one ABI argument per slot exactly as a bag does. The fills ride a second
`ConstArg` variant, `RequiredShape`, which `MethodSig::required` skips: a shape parameter records
what its unwritten slots pass **and** stays required, which one `ConstArg::Options` entry could not
express (`crates/nvs-types/src/defaults.rs`'s variant doc owns why). `check_options_arg` now also
refuses a key the merged list requires and the literal omits, as `E0402` — § 3 flattens a key into
an argument, so a missing one is a call one argument short.

**Two things are still missing, and only one is a gap.** `Core\Db::open` has no registry row yet
(`nvs_stdlib::db` known gap 1), which is the next group's first slice and the reason the goal's one
open acceptance check, `a_db_open_target_in_a_denied_range_fails`, still does not run — an item
still open, not a regression. § 2's *exactly one arm accepts it* remains unwritten and unstatable on
a merged list; `Ty::CoreShape`'s own known gap owns it, and it also swallows "a key required by only
one arm is required by neither".

**The refusal landed with a unit test, not a call-site test.** No registry row declares a required
shape key, so nothing can write `Core\Foo::bar({...})` and watch it fail; the rule is held by
`missing_required_keys`' own test in `expr/args.rs`, and the `.nvst` and `core_members.rs` cases are
owed by the `open` slice below.

`[context] adrs` in `docs/agent/loop-goal.toml` now names `0135 §1/§2/§3`, in both that file and
`docs/agent/goals/5-database.toml` — the two had already diverged over the `0133` entries, which
this session did not touch.

## Next group

**`Core\Db::open`, its tests, then § 2's arms. File set: `crates/nvs-stdlib/src/db.rs` with
`crates/nvs-types/tests/core_members.rs` and `tests/conformance/core/`, then
`crates/nvs-types/src/ty.rs` with `crates/nvs-types/src/core_lib.rs`.** The first two items share
one build and one file set; the third is its own session and its own ADR reading.

- [ ] **`Core\Db::open`'s five edits** (0067 § 3, 0135 §§ 1 and 3).
      `crates/nvs-stdlib/src/db.rs:501` is `connect`'s row and the block the `Db\Settings` arms go
      beside; `crates/nvs-stdlib/src/db.rs:66` is known gap 1, to delete with the row. The merged
      list is the server arm's fields, then the SQLite arm's `path`, then the trailing bag's
      `shared` — so the helper is one ordinary `args: [N]` over that order, and § 3's `host` is
      `Qual::Sink` on the *field*.
- [ ] **The cases that row makes writable** (0135 §§ 2 and 3, 0067 § 3).
      `a_db_open_target_in_a_denied_range_fails` is the goal's open check;
      `crates/nvs-types/tests/core_members.rs:150` is the shape the unknown-key case already takes,
      and a missing required key is now `E0402` from
      `crates/nvs-types/src/expr/args.rs:688`. Three `.nvst` cases make the member reachable.
- [ ] **§ 2's arm selection** (0135 § 2). `crates/nvs-types/src/ty.rs:292` is `CoreShape` and the
      known gap to close by carrying the arms (or per-field masks);
      `crates/nvs-types/src/core_lib.rs:568` is `merge_shape_arms`, which would then keep both the
      merged list and the arms, and `crates/nvs-types/src/expr/args.rs:603` is the one reader.

## Backlog

- § 2's disjointness proof `a_shapes_arms_are_pairwise_disjoint`, a static registry check (ADR 0135 § 2).
- `a_shape_is_only_ever_a_whole_parameter` and `a_shape_field_is_never_nullable` (ADR 0135 §§ 1, 3).
- `ParamDoc::shape` for a union's merged key set, each `desc` naming its arm (ADR 0135 § 4).
- The four MySQL/MariaDB/SQL Server drivers past PostgreSQL (docs/plan/m8.md).
- ADR 0067 § 5's `inList` expansion arity in the statement-cache key, over a real server (0067 § 1).
