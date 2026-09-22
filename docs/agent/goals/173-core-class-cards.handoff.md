# Handoff

## State

**Goal 173 — every Core class carries its card — has just started; nothing of it has landed yet.** Goal `limit-handler-reach`'s whole list is this goal's Stage 1 floor.

ADR 0203 already holds the design and nothing about it is a session's to re-decide: `CoreClass::doc`
is an `Option<&'static ClassDoc>` with one `short`, `nvs_lsp::card` shows it on
`completionItem/resolve`, `nvs meta` prints it under the class's `doc` key, and
`every_registry_row_carries_a_reference_card` in `crates/nvs-stdlib/src/registry.rs` lets exactly the
classes in `CLASSES_STILL_OWING_A_CARD` ship without one. `python tools/class-cards.py` prints that
list. The work is the list: a card per class, in the class's own module, and the name deleted from
the list in the same commit. The voice is AGENTS.md § *Text an end user reads*, and the goal's
standing decisions say what a card is and is not.

## Next group

**Stage 2: the cards** — one file set per session: `crates/nvs-stdlib/src/registry.rs` and the
modules whose classes the session cards, taken in the list's order.

- [ ] **Read the list** — `python tools/class-cards.py`; the first module in it is where the session
      starts, and every class that module declares is carded before the next module is opened.
- [ ] **Card a class** — a `/// `Core\X`'s own card — `rule:core-api/reference-card`.` const of
      `ClassDoc { short: "…" }` directly above the module's `CLASS` row, `doc: Some(&…)` on the row,
      and the name deleted from `CLASSES_STILL_OWING_A_CARD`; `crates/nvs-stdlib/src/json.rs` is the
      shape of a module whose members are carded.
- [ ] **Hold the pair together** — `cargo test --lib every_registry_row_carries_a_reference_card`
      names a class carded and still listed, or listed and not carded.

## Backlog

- Nothing this goal does not reach: it is one stage, repeated until the script prints that every
  class carries its card.
- When this goal's last check goes green the driver takes goal `tools-install`.
