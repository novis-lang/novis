# Handoff

## State

**M4's frontier is Stage 5, and item 33 is closed end to end.** ADR 0092's record model and
plaintext rendering are the leaf crate **`nvs-render`**; `Core\Debug::dump`/`render` are
registered `Core` members; and § 5's redaction row is now closed at *both* halves — the
call-site refusal (`E0724`) and the `secret`-typed property, which renders as
`nvs_render::Node::Redacted`.

- **The property half is a descriptor bit, and the vertical is four crates long**:
  `nvs_types::expr::type_is_secret` (`crates/nvs-types/src/expr/mod.rs:88`) →
  `nvs_ir::lower::field_slots` (`crates/nvs-ir/src/lower/mod.rs:391`) →
  `nvs_ir::ir::Class::secret_fields` → `nvs_runtime::ClassTable::set_secret_fields`
  (`crates/nvs-runtime/src/object.rs:713`) → `ClassDesc::field_is_secret`, read by
  `nvs_stdlib::debug::object_body` (`crates/nvs-stdlib/src/debug.rs:426`). The plan's
  *Open now* paragraph is the one home for why each hop exists.
- **`field_reprs` is now `field_slots` and returns a pair.** The representation and the
  `secret` bit come off one join over one table on purpose — two walks could disagree about
  which declaration won a slot, and a redaction naming the wrong slot discloses the value.
- **What still has no bit is a container**: an `array<T>` element and an ADR 0036 shape
  literal's field. Both are ADR 0033's unmodelled container axis; `nvs_stdlib::debug`'s known
  gap 1 is their one home.
- **`nvs-render` depends on `nvs-syntax` and that edge is temporary** — ADR 0087's bidi
  predicate, called rather than restated. It inverts (a **move** of `nvs_syntax::bidi` down)
  the moment `nvs-runtime` or `nvs-diagnostics` becomes a dependent. That crate's module doc
  § *Where this sits* is the one home for it.
- **`Ctx` has a second sink.** `write_diagnostic` (`crates/nvs-runtime/src/ctx.rs:960`) writes
  to `OutputSink::Stderr` by default and is not routed through the capture stack.

## Next group

**Stage 5's next item, whatever `python tools/loop.py --list` names first.** Item 33 owned this
session's whole file set and nothing in it is left open, so the next group is chosen from the
tools rather than from here — `holes.py` for a refusal site, `loop.py --list` for a named case
the stage still owes.

- [ ] **Take the first open Stage 5 item `python tools/loop.py --list` names**, and read its
      anchors with one `python tools/peek.py --locate` call before opening anything.
- [ ] **Then the next one it names**, if it shares that file set.

## Backlog

- A `secret` value inside an `array<T>` element or an ADR 0036 shape field is not redacted —
  `nvs_stdlib::debug` known gap 1, ADR 0033's container axis.
- An enum case dumps as its backing integer — `nvs_stdlib::debug` known gap 2, ADR 0010 § 5.
- ADR 0092 § 6's `Throwable` producer belongs to `nvs-runtime`'s fatal path — known gap 3.
- `nvs_syntax::bidi` moves down into `nvs-render` once a second dependent exists — that
  crate's module doc § *Where this sits*.
- A `require` whose path is not a string literal runs nothing, silently — `nvs_hir::requires`.
- ADR 0092 § 4's HTTP rows (a dump under `[debug] inline`, and never in a JSON body) are M7's.
