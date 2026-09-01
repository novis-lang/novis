# Handoff

## State

**Stage 9 has opened: `Core\Html::escape` is on disk and the acceptance check that named it passes.**
ADR 0024 § 3's narrow, sink-named launderer, as one row in `crates/nvs-stdlib/src/html.rs` — the five
characters `&<>"'` as references always, no flag, and ADR 0024 § 5's last bullet on top of it: an
unterminated directional control becomes `U+FFFD` through `nvs_render::bidi::for_each_unterminated`,
called directly rather than through `nvs_render::text::substitute`, whose ADR 0086 § 1 table would
also rewrite a newline as `␊`. That module's own doc is the home of why the escape set has no
argument and why the bidi row is here.

**`Core\Html` is one member of four.** `sanitize` and ADR 0122's WHATWG parser wait on `Core\Xml`'s
tree. `Core\Html\Markup` waits on nothing: `crates/nvs-runtime/src/ctx.rs:208` already declares it a
sink carrier, `crates/nvs-types/src/expr/quals.rs:376` already refuses a `secret` conversion and
`E0417` already refuses a non-literal one — so `"<b>" as Core\Html\Markup` **type-checks today and
then ICEs**, at `crates/nvs-ir/src/lower/convert.rs:351`, whose own panic message names the missing
row and says it waited on `Core\Html` existing. It exists now.

**The next acceptance check is `validate_launders_only_what_it_actually_validated`**, which is why it
leads the group below rather than the Markup work.

**The pack still did not print ADR 0024 § 5**, which this session's own bidi row comes from — `[context]
adrs` in `docs/agent/loop-goal.toml` gained 0024 § 3 but not § 5, so it was sliced by hand again. Add
`0024 § 5`; for the group below, add `0007 § 2` (the conversion rows `convert.rs` lowers).

## Next group

**Stage 9's remaining launderers and the carrier, over `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-stdlib/src/validate.rs`, `crates/nvs-stdlib/src/html.rs` and `tests/conformance/core/`.**

- [ ] **`Core\Validate` launders only what it actually validated** — the acceptance check names
      `validate_launders_only_what_it_actually_validated` in `-p nvs-stdlib`. The module doc at
      `crates/nvs-stdlib/src/validate.rs:152` already argues why a blanket `Qual::Launder` over the
      class would be the false confidence ADR 0024 § 3 refuses, so the test is that claim asked of the
      rows: `crates/nvs-stdlib/src/registry.rs:1142` is where the class is registered, and
      `crates/nvs-stdlib/src/html.rs:236` is the shape the sibling test takes.
- [ ] **`Core\Html\Markup`, ADR 0024 § 5's only raw-write bypass** — register the carrier class beside
      `crate::html::CLASS` at `crates/nvs-stdlib/src/registry.rs:1268`, with the one slot
      `crates/nvs-runtime/src/ctx.rs:218`'s `CARRIER_TEXT_SLOT` fixes, and lower the conversion the
      checker already admits: `crates/nvs-ir/src/lower/convert.rs:351` is the panic arm and names the
      row. `crates/nvs-stdlib/src/cli.rs:1857`'s `TEXT` is the worked carrier class.
- [ ] **`Markup + Markup` is `Markup`** — ADR 0024 § 5's composition rule, which is the one operator
      row the carrier needs and the reason templating stays cheap.
      `crates/nvs-types/src/expr/operators.rs:1719` already refuses the cast and is where the class is
      named in that crate.

## Backlog

- `Core\Html::sanitize` and ADR 0122's WHATWG parser wait on `Core\Xml`'s tree — `docs/adr/0122-*.md`.
- Stage 9's other three checks: `mail_sends_against_an_operator_named_endpoint_and_no_other`,
  `storage_over_local_disk_is_gated_on_the_same_fs_capability`,
  `plural_category_answers_from_the_carried_cldr_data` — `docs/agent/loop-goal.toml` § Stage 9.
- Hooks are bypassed on both directions of an erased property access —
  `nvs_runtime::write_erased_property`, ADR 0036 § 4's own gap.
- `Core\IO`'s `truncate` and `lock`, and `Core\Cli::displayWidth` — `docs/implementation-plan.md`.
- `[log] target` is not read yet — `crates/nvs-stdlib/src/log.rs`.
