---
milestone: dossier
---
# Loop goal 173 — every Core class carries its card

Every class in `nvs_stdlib::registry::CLASSES` carries a `ClassDoc`: one or two plain sentences saying
what the class is for, beside the cards its members already carry. Once this goal is green the
completion list in an editor says what a `Core` class does the moment its name is highlighted, `nvs
meta` prints the line under the class's `doc` key for the website, and the registry test's list of
classes still owing a card is empty and stays empty — a new class lands with its card or does not land.

## Why here

It needs the field, the guard and the reader, and ADR 0203 landed all three: `CoreClass::doc`, the
`CLASSES_STILL_OWING_A_CARD` list in `every_registry_row_carries_a_reference_card`, and
`nvs_lsp::card`, which shows the line on `completionItem/resolve`. The rule is already in force for
every class added since — this goal is the catch-up for the classes that landed before the field
existed, which is the same shape as the member cards' own backfill. Nothing after it waits on it, and
it sits behind the generated proof goals because every one of them may add a class, and each of those
lands carded on its own.

## Stage 1 — the floor

Goal `limit-handler-reach`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the cards, the keystone

`python tools/class-cards.py` prints the classes still owing a card, read off the one list. For each:
write a `ClassDoc` const in the class's own module, directly above its `CLASS` row (`/// `Core\X`'s own
card — `rule:core-api/reference-card`.`), set the row's `doc: Some(&CLASS_DOC)`, and delete the class's
name from `CLASSES_STILL_OWING_A_CARD`. `cargo test --lib every_registry_row_carries_a_reference_card`
names any class deleted too early or carded and not deleted. The goal is green when the script prints
that every class carries its card.

The `short` is what a reader of a completion list sees first, and it is written to AGENTS.md § *Text an
end user reads*: what the class is for, in words a programmer already knows, no repository vocabulary,
no history, no PHP unless the class exists to replace one PHP thing. Where `docs/reference/core/<Class>.md`
has a `summary:` line, start from it and rewrite it to that voice — the page summaries are in the
repository's own voice and are not copied as they are.

## Standing decisions

- **One or two sentences, and what the class is for — never a member list.** The members' cards say what
  each member does; the class card says why the class exists. "Reads and writes files under the
  directories the program was allowed" is a card; "`read`, `write`, `append` and `delete`" is not.
- **A namespace class with no members of its own still gets a card** — one sentence saying what its
  nested classes are about.
- **A class is deleted from the list the session it gains its card**, in the same commit, and the list
  is never added to: a class that is not in it and has no card fails the test, which is the rule
  working as intended.
- **Take the list in file order, as many per session as the read budget holds.** Every card is a
  three-line edit in the module that owns the class, so a session that has the module open cards every
  class in it before moving on.
- **The page summary is a source, not the text.** `docs/reference/core/<Class>.md`'s `summary:` stays as
  it is; only the registry card is written in the end-user voice.
