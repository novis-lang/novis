A PHP built-in's name typed at a top-level identifier position completes to an item shaped by the
migration table's outcome for that name and by whether its destination is registered in `Core`. Four
shapes, and three insert nothing:

| Row | Item |
|---|---|
| `member` or `language`, destination registered | inserts it, with the signature the registry holds |
| `member` or `language`, destination not registered | appears, names the milestone, inserts nothing |
| `dropped` | appears, gives the row's reason and rewrite, inserts nothing |
| `open`, or no row at all | appears, says undecided, inserts nothing |

A missing row and an `open` row are one case, exactly as `bun nv migration` treats them. "Inserts
nothing" is asserted as the absence of an edit, not as an empty string.

A row whose cell names more than one destination is prose the converter may not guess at, and is a
genuine ambiguity there. It is not one here: completion has a person in the loop, so such a row becomes
one item per destination and the developer picks. This is the single place the editor may offer more than
the converter, and it follows from the human, not from a better table; the converter's own tier for that
row is unchanged.

The shape table is what keeps this layer inside `rule:ide/completion-offers-only-what-the-compiler-derived`
rather than beside it: a *name* may come from an audited table, but the text an editor types on a
developer's behalf still comes only from something the compiler can resolve. The PHP spelling never
reaches a file, which is what `rule:statements/nothing-gets-a-second-name` requires of it.
