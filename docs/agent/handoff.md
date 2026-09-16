# Handoff

## State

**Goal `unowned-closures`, stage 4 open.** `Core\Debug`'s two gaps are struck and the module owns
both answers. Inside a request a dump is now one record at `Debug` through
`Ctx::write_log_record` — so `[log] target`, `[log] level` and `[log] format` answer for it, and the
unconfigured fallback is the diagnostic channel rather than the program's output; outside a request
nothing moved. `[debug] inline` is read in force at each dump, renders the same nodes for the body's
own carrier and hands the block to `Ctx::append_inline_debug`, which the isolate's finish path
appends with `Ctx::flush_inline_debug` — after the body member has declared what the body is, so a
JSON body is never modified, and after the markup, so a block never lands inside a tag.
`render` answering `Core\Cli\Text` under every sink is now stated as a **bound**: `instanceof`
against a `Core` class is `E0496` by decision ([0125](../decisions/0125.md) § 4), so the union the
other reading needs is inert. `rule:errors/debug-dump`'s signature line is amended to match.

The compress case the goal's stage-4 check names is on disk at that path. It was written last
session under a different slug and has been renamed; the case itself is unchanged. Nothing is
blocked.

**The pack was missing two `[context]` fields for this item.** `modules` names no
`crates/nvs-stdlib/src/debug.rs`, `crates/nvs-runtime/src/ctx/`, `crates/nvs-host/src/isolate.rs` or
`crates/nvs-config/src/tree.rs`, and `rules` names neither `errors/debug-dump` nor
`errors/renderings` — the two the item's own gaps cite. Both cost a fetch.

## Next group

**Stage 4: the codec descriptor that has to nest** — one file set:
`crates/nvs-stdlib/src/json.rs`, `crates/nvs-stdlib/src/db/mod.rs` and `crates/nvs-render/src/lib.rs`,
plus `nvs_runtime::CodecField`, which the first two both widen. `python tools/owners.py` prints
what is left: three items, all owned by retired goal `m8-stdlib-depth`.

- [ ] **An `array<T>` of inline shapes needs an element description that nests** —
      `crates/nvs-stdlib/src/json.rs:151` gap 1 (`rule:core-classes/derive-field-list`). The element's
      contract has nowhere to ride: `nvs_runtime::CodecField` carries one and a list has spent it on
      the element's wire type, so `decode_as` refuses the field before reading the document. This is
      the same widening `array<array<T>>` waits on, so decide the descriptor's shape here and let the
      two doors follow.
- [ ] **A hydration's skipped field has no call site to emit its default from** —
      `crates/nvs-stdlib/src/db/mod.rs:268` gap 3, the other door onto the same descriptor
      (`rule:core-classes/derive-attribute`). `crates/nvs-stdlib/src/json.rs:166` gap 2 carries the `Decided:` sentence for
      the shared half — default constants on `CodecField`, a `ClassDesc` method lookup — so read that
      before choosing, and amend the rule in the same slice if the answer changes it.
- [ ] **A `#[Test]` result is a producer, so § 22's three output formats are one record rendered** —
      `crates/nvs-render/src/lib.rs:39` gap 1 (`rule:errors/renderings`, `rule:errors/record-producers`).
      Independent of the two above and the cheapest of the three; take it first if the descriptor
      question turns out to need a decision.

## Backlog

- `crates/nvs-runtime/src/record.rs` gaps 1 and 2 — a `secret` inside a container, and an enum case
  walking to its integer; both carried in [carried-gaps.md](carried-gaps.md), neither this goal's.
- Nine gaps deferred to milestones the program has already passed — `python tools/owners.py`'s second
  block; they are owed by a goal or by nobody, and no goal names them.
