A bare name at a statement or expression position is offered every type the server holds — the `Core`
registry's classes and enums and every declaration in the workspace index — from the first character
typed, and accepting one that no short name reaches inserts its last segment and adds the `use` line for
it. The registry is the roster a `Core\` separator already lists, read before the separator is written, so
this is `rule:ide/completion-offers-only-what-the-compiler-derived` and not an exception to it.

**The spelling offered is the shortest one that resolves, and the editor makes it resolve.** A type an
import or the namespace in force already reaches is offered by that short name and edits nothing else. Any
other type is offered by its last segment, with the qualified name beside it, and the item carries one more
edit: `use Qualified\Name;` after the last `use` the cursor's namespace has, failing that after the
`namespace Name;` line in force, failing that after the open tag. Where the short name is already taken in
the file, or there is no such line to write after — a bracketed namespace with no `use` in it, a shebang
script with no open tag — the item is the qualified name and edits nothing else. After `use`, every type is
its qualified name, because a declaration's name is absolute
(`rule:statements/a-qualified-name-is-absolute`).

**Matching is the client's.** The server sends the whole list once and the editor filters it as the name is
typed, so `cs` finds `Core\Str` by the editor's own match across the separator: an item's filter text is
both of its spellings. **Ranking is the server's where the match ties**, in this order: the variables the
body declared, imported types, types in the namespace in force, types already written somewhere in the
file, the rest of `Core`, the rest of the workspace, and the reserved words last. Every tier
is read off a table an arm already reads — the body's scope, the file's imports, the index's occurrences.
A member list after `->` or `::` has no tiers.

What it spends is one item per type on every bare-position request, a few hundred for the registry alone,
built and dropped with the answer.
