A completion row shows what the declaration would: a method as `name(params)` with its return type at
the right, a property or a type alias as its name with its type at the right, a constant or an enum case
as `NAME = value` with its type at the right where one is declared, and a type offered by a shorter
spelling than its qualified name with the namespace it is declared in written after the name, in
brackets. A type offered by its qualified name gets nothing after it: the label already says where it is.
These are LSP's `labelDetails` — the `detail` an editor draws directly after the label, the `description`
it draws at the right — and a row without them, a keyword or a variable, shows its `detail` at the right
as before. The qualified name of a type and the qualified signature of a member stay the item's `detail`,
which an editor shows only in the panel beside the list.

**What a row documents is not on the row.** A list names every type in reach and every member of a
class, and a card on each row would be most of every response, sent on every keystroke and read for one
row. So every item carries a key under `data` — the qualified name of the type, or the owner and the
member's name, and the document's URI stamped on by the server — and `completionItem/resolve` reads the
card for the one item the editor shows: a `Core` member's reference card from its registry row, a `Core`
class's, enum's, constant's or case's own line from the same registry, a user declaration's `///` run.
It is the text hovering the declaration shows, rendered once, so a list and a hover never disagree about
a name. An item nobody documented resolves to itself.
