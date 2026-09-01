# Handoff

## State

**ADR 0126's `property<T>` is closed.** Lowering landed earlier; both of § 5's and § 1's remaining
refusals landed this session. A write through a key whose public set holds a `readonly` property is
`E0782` at the write, asked over the *set* because which name the key holds is what the access does not
know — `crates/nvs-types/src/expr/assign.rs`'s `reject_readonly_write_through_key`, reached from
`check_write_target` so all four write spellings answer there. A `property<T>` whose `T` declares no
public property is `E0799` at the conversion —
`crates/nvs-types/src/expr/operators.rs`'s `reject_empty_property_key_set`, whose doc owns why
`crate::lower` cannot host it and why `as` being the type's only source makes the conversion complete.

Still owed on item 35: nothing. The keyed access inherits ADR 0036 § 4's own gap — hooks bypassed on
both directions — which is recorded on `nvs_runtime::write_erased_property` and closes for every caller
at once; it is in the backlog below, not in this item.

**The driver's acceptance check fails on stage 9, and none of stage 9 exists.**
`html_escape_launders_for_the_html_sink_and_for_no_other` needs `Core\Html`, and `nvs-stdlib` has no
`src/html.rs` and no `Core\Html` row at all. The type system already knows the *name*
(`crates/nvs-types/src/expr/quals.rs:376` and `crates/nvs-types/src/expr/operators.rs:1719` refuse a
cast to `Core\Html\Markup`); the class behind it is what is missing. That is the next group.

**The orientation pack did not print ADR 0126 §§ 1 or 5**, which item 35's own slices are specified by —
`[context] adrs` in `docs/agent/loop-goal.toml` carries 0024 § 3, 0051 § 3 and 0063 but no 0126 section,
so two sessions in a row have sliced them by hand. Add `0126 §§ 1, 4-5`; for the group below, add
`0024 §§ 3, 5`.

## Next group

**`Core\Html`, stage 9's first launderer, over `crates/nvs-stdlib/src/registry.rs`, a new
`crates/nvs-stdlib/src/html.rs`, `crates/nvs-stdlib/src/lib.rs` and `crates/nvs-types/src/core_lib.rs`.**

- [ ] **`Core\Html::escape`, the class and the member** — ADR 0024 § 3's narrow, sink-named launderer,
      `escape(tainted string): string`. The five edits of `docs/agent/conventions.md` § *A `Core` member*,
      with `crates/nvs-stdlib/src/json.rs` as the worked example: the row in
      `crates/nvs-stdlib/src/registry.rs:1079`'s `CLASSES`, the card, the `nvs_helper!` body, the
      `address()` arm, three `.nvst` cases. ADR 0051 § 3 places the class in Core.
- [ ] **The laundering is the *return type*, and it is what the named test asserts** — a `tainted`
      argument in and an unqualified `string` out, decided by `crates/nvs-types/src/core_lib.rs:335`'s
      `qual_of` over the row's `CoreTy`. Write
      `html_escape_launders_for_the_html_sink_and_for_no_other` in `nvs-stdlib` against the registry
      row — ADR 0024 § 3's "never generic" is the assertion: no other sink's parameter admits what
      `escape` returned, and no other member launders for HTML.
- [ ] **`Core\Html\Markup`, ADR 0024 § 5's only raw-write bypass** — the runtime already renders it as a
      sink carrier (`crates/nvs-runtime/src/ctx.rs:223`'s `is_carrier`, joined at
      `crates/nvs-stdlib/src/registry.rs:1669`'s `class_renders`) and the checker already refuses the
      cast (`crates/nvs-types/src/expr/quals.rs:376`), so what is owed is the registered class those two
      already speak for.

## Backlog

- The hooked-property gap on the erased path — ADR 0036 § 4's own, recorded on
  `crates/nvs-runtime/src/object.rs`'s `write_erased_property`; closes for every caller at once.
- Stage 9's other four tests: `Core\Validate` laundering only what it validated, `Core\Mail`,
  `Core\Storage`'s `fs` capability, CLDR plural categories — `docs/agent/loop-goal.toml:2543`.
- Stage 7: reading `[log] target` — `docs/implementation-plan.md`'s *Open now*.
- Stage 2: `Core\IO\File::truncate` and `::lock` — same field.
- Stage 3: `Core\Cli::displayWidth` — same field.
