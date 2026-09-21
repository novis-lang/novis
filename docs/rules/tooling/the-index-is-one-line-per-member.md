`nvs agent index` prints exactly one line for every member the registry holds, one for every enum,
exception and attribute beside them, and one for every chapter of the reference the binary embeds and
every heading in it. It is derived at the call and kept nowhere (`rule:testing/roster-is-derived`'s
shape), so a member that lands owes its line at once and no list is ever stale. The one list written by
hand is which chapters are embedded, and a test holds it to the files under `docs/reference/`.

Completeness is the property the command exists to have. An agent that greps a complete list learns
something from an empty result — that the name it guessed does not exist — and learns nothing at all
from an empty result over a list that merely happens not to mention it. That is why the index is
enumerated from the registry rather than written, and why the guard is a member-for-member
correspondence rather than a count.

A line carries the member's signature in the spec's own spelling and the capability the call is gated
on, written after it in brackets: `Core\IO::read(string $path): string  [fs.read]`. The capability is
joined from `rule:security/capability-declaration-is-one-table`'s one table at render time, never
copied onto a member row — that table's own rule refuses a per-member field, and this reads it rather
than reshaping it. So an agent learns the gate from the name of the thing it is about to call, which is
where every arm of `0167`'s investigation was stopped.

A chapter's line is `<id>  chapter: <title>` and a heading's is `<id>#<slug>  section: <heading>`, the
slug being the heading in lowercase with every run of anything but a letter or a digit written as one
`-`. Neither holds a `(`, a `<` or a space, so the symbol is cut from these lines as from a member's,
and a `#` is in no other symbol, so a section can never shadow one.

A line carries no behaviour: what `header: true` does to a row is the card's answer, which is what
`show` is for.
