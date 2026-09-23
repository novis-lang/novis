Every diagnostic code carries a card beside its `Code::new` declaration: one to three plain sentences
saying what the error means and how to fix it, and a short wrong-then-right example where the
sentences alone would leave the reader guessing. `nvs agent show E0621` prints it. The `///` above the
declaration stays what it is — the contributor's account, citing rules — and the card is written for
the person who met the error, in the voice of AGENTS.md § *Text an end user reads*.

A terminal rendering of diagnostics ends with one line naming `nvs agent show <code>`, once per run
and not once per diagnostic, so a run with many errors does not repeat it. The JSON rendering carries
no such line: a tool that reads it already has the code.

A code lands with its card, as a `Core` member does (`rule:core-api/reference-card`). The codes that
landed before cards existed are named in a list of codes still owing one, a test holds the list and
the cards together, and a code is deleted from the list in the commit that gives it its card.
