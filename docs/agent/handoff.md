# Handoff

## State

**ADR 0135's registry half is on disk.** `CoreTy::Shape(&'static [&'static [CoreField]])` and `CoreField`
are in `crates/nvs-stdlib/src/registry.rs`; the variant's own doc is the rule. The three invariants ADR
0135 §§ 1-3 name are held by `a_shape_is_only_ever_a_whole_parameter` — which also refuses an empty arm
list and an empty arm, the same walk — `a_shapes_arms_are_pairwise_disjoint` and
`a_shape_field_is_never_nullable`. All three iterate `CLASSES` and are **vacuous until a row declares a
shape**; `Core\Db::open` is the first that will.

**Nothing checks or lowers one yet.** `nvs_types::ty::Ty` still spells the bag `Options` with no per-field
required-ness, `nvs_ir::lower::lower_call_args` flattens only a bag, and `Core\Db::open` is still absent —
so the goal's one open acceptance check, `a_db_open_target_in_a_denied_range_fails`, still does not run.
That is an item still open rather than a regression.

**`orient.py` printed no section of ADR 0135**, although the item was entirely about §§ 1-3: `[context]
adrs` in `docs/agent/loop-goal.toml` names 0133 and 0067 and not 0135. Three peeks paid for it — the
backlog's first line is the fix.

## Next group

**0135 down its remaining spine, then the member it exists for. File set: `crates/nvs-types/src/ty.rs`
with `crates/nvs-types/src/core_lib.rs`, then `crates/nvs-ir/src/lower/call.rs` with
`crates/nvs-diagnostics/src/lib.rs`, then `crates/nvs-stdlib/src/db.rs`.** The first two share a build and
the second cannot compile without the first; the third is its own session's worth.

- [ ] **Rename `Ty::Options` to `Ty::CoreShape`, give its fields per-field required-ness, and intern
      `CoreTy::Shape` into it** (0135 §§ 1 and 3). `crates/nvs-types/src/ty.rs:271` is the variant, whose
      `Vec<(String, TypeId)>` pair gains the required flag; `crates/nvs-types/src/ty.rs:461` is its
      formatting arm and `crates/nvs-types/src/ty.rs:748` the interning helper.
      `crates/nvs-types/src/core_lib.rs:516` is where `CoreTy::Options` becomes one and where
      `CoreTy::Shape`'s **merged, name-deduplicated** field list is built, and
      `crates/nvs-types/src/core_lib.rs:235` synthesizes the `ConstArg::Options` defaults entry. Only
      about eight sites spell `Ty::Options`; the ~130 other hits are `CoreTy::Options` construction and do
      not move.
- [ ] **Flatten a shape argument at the call site, and widen `E0453`/`E0454` from "option" to "shape
      key"** (0135 §§ 2-3). `crates/nvs-ir/src/lower/call.rs:82` is `lower_call_args`,
      `crates/nvs-ir/src/lower/call.rs:198` reads the bag off the last parameter and
      `crates/nvs-ir/src/lower/call.rs:275` checks a written literal's keys against the declared ones —
      arm selection is "exactly one arm accepts it", and an unfilled slot passes the field's default or
      `Const::Null`. `crates/nvs-diagnostics/src/lib.rs:830` and `:835` are the two codes, whose *text*
      widens; ADR 0135 is explicit that no third code is added.
- [ ] **`Core\Db::open`'s five edits, and `a_db_open_target_in_a_denied_range_fails` with them** (0067
      §§ 3 and 18, 0135 § 3). `crates/nvs-stdlib/src/db.rs:495` is `CLASS` and
      `crates/nvs-stdlib/src/db.rs:499` the `connect` row `open` sits beside;
      `crates/nvs-stdlib/src/db.rs:1545` is `CONNECT_DOC`, the card block the new card joins in row order;
      `crates/nvs-stdlib/src/db.rs:6030` is `address`. The helper is an ordinary `args: [12]` — the server
      arm's ten fields, the SQLite arm's `path`, then the bag's `shared`. This test is the driver's one
      failing acceptance check.

## Backlog

- `[context] adrs` in `docs/agent/loop-goal.toml` carries no 0135 entry; §§ 1-3 are what the group needs.
- A shape parameter's reference card: `ParamDoc::shape` at `crates/nvs-stdlib/src/registry.rs:789` says
  "an options bag" and `docs/agent/conventions.md` § *A `Core` member* says `shape` is filled for a
  fixed-key shape parameter — one of the two is about to be wrong.
- `every_member_parameter_carries_a_qualifier_classification` reads `Qual` off a parameter's own type; a
  `CoreTy::Shape` carries its classification on the **field** (0135 § 3), so that gate needs the extra
  level before `open`'s row lands — `crates/nvs-stdlib/src/registry.rs`.
- `crates/nvs-cli/src/meta.rs:322`'s `ty_string` has no `CoreTy::Shape` arm and falls into its wildcard;
  `nvs meta` will print a shape parameter as whatever that arm says until it gains one.
- `nvs_stdlib::db` known gap 2 onward: MySQL runs only `query` of the four members that send.
