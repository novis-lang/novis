---
milestone: dossier
---
# Loop goal 173 — every Core class and every diagnostic code carries its card, and the binary's help reaches every feature

Every class in `nvs_stdlib::registry::CLASSES` carries a `ClassDoc`: one or two plain sentences saying
what the class is for, beside the cards its members already carry. Once this goal is green the
completion list in an editor says what a `Core` class does the moment its name is highlighted, `nvs
meta` prints the line under the class's `doc` key for the website, and the registry test's list of
classes still owing a card is empty and stays empty — a new class lands with its card or does not land.

The same goal then brings every existing feature up to the *Help* proof ADR 0216 added: `nvs agent
find` reaches every configuration key, command, flag and diagnostic code, every code carries a plain
card that `nvs agent show` prints, and `dossier.py` owes *Help* for the whole roster. It is the same
work as the class cards — a field, a guard with a "still owing" list, and a drain — so it sits here
rather than in a goal of its own.

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

## Stage 3 — every key, command, flag and code is an index entry

Build `rule:tooling/the-index-names-every-key-command-and-code` in `crates/nvs-cli/src/agent.rs`:
derive one index line per configuration key from the configuration schema, per command, subcommand
and flag from `Cli::command()`, and per code from the diagnostics table, in the line shapes the rule
gives. `find` and `show` reach each by its exact name. The guard is
`every_key_command_flag_and_code_is_an_index_entry` in `crates/nvs-cli/tests/agent.rs`: a
name-for-name correspondence between each table and the index, never a count. When it is green the
rule's `status` becomes `shipped` and the test goes in its `guardedBy`.

## Stage 4 — every diagnostic code carries its card

Build `rule:tooling/a-diagnostic-code-carries-its-card` in `crates/nvs-diagnostics/src/lib.rs`: a card
beside each `Code::new`, `nvs agent show <code>` printing it, and the one closing line in the terminal
rendering. Every code that has no card yet goes into `CODES_STILL_OWING_A_CARD`, and the test
`every_code_carries_its_card_or_is_listed` holds the list and the cards together, as the class-card
test does. Then drain the list in file order, one band per session. The stage is green when
`no_code_still_owes_a_card` passes, and the rule's `status` becomes `shipped`.

## Stage 5 — Help is owed by the whole roster

Teach `tools/dossier.py` the *Help* proof: for each feature, run `nvs agent find` with the name the
roster knows it by and `nvs agent show` on the line it returns, and owe the proof when either comes
back empty or when the text is missing what the rule lists. Add the policy key `help` to
`tools/data/dossier-policy.toml`, switched off, then switch it on, write the help every feature
still owes, and stop when `python tools/dossier.py --gate` says the whole roster owes nothing.

## Standing decisions

- **Stage 2 comes first.** The class cards are part of what *Help* reads for a `Core` class, so stage
  5 cannot pass before stage 2 has.
- **The proof key is `help`, and it is last in `PROOFS`**, so the gate names `hostile, about, help`
  exactly when the proof is owed.
- **A code card is for the person who met the error.** What the error means and how to fix it, in one
  to three sentences, plus a wrong-then-right example only where the sentences leave the reader
  guessing. The `///` above the declaration stays the contributor's account and is not rewritten.
- **An index entry is an exact name, never a text search.** A session tempted to make `find` match
  text under a heading re-reads ADR 0216 § 3 first.
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
