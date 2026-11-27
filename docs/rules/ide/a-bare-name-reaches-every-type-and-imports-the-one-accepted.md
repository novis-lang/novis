A bare name at a statement or expression position is offered every type the file's program can load —
the `Core` registry's classes and enums, and each declaration in the workspace index that the file's
`autoload` map loads — from the first character typed, and accepting one that no short name reaches
inserts its last segment and adds the `use` line for it. The registry is the roster a `Core\` separator
already lists, read before the separator is written, so this is
`rule:ide/completion-offers-only-what-the-compiler-derived` and not an exception to it. The `Ns\` list
after a namespace, the type list after `use` and after `new`, and the members an `extend` block of
another file adds read the same test.

**A declaration is loadable when the program loaded it or the map resolves its name to its file.** The
first half is the analysis's own symbol table. The second is a path comparison and never a disk probe:
the name's longest matching prefix among the map's ordinary entries, one of that prefix's roots, the
remaining segments as directories and the last one plus `.nvs`, compared with the path the index holds.
It is `AutoloadMap::loads_file`, beside `resolve` and sharing its prefix walk, so the editor and the
compiler read one implementation of the map, and both sides are canonical paths, so the comparison needs
no conversion. A type declared inside a class, `Shop\Cart::State`, is tested by its class's name, since
it is loaded with that class. Where two roots of one prefix both hold the file, it is offered: the label
is keyed by name and appears once. `Core` is always offered, since no program autoloads into it.

**An `autoload links` root loads nothing** (`rule:programs/autoload`), so its types are not offered at a
bare name, a `Ns\` list, `use` or `new`. **A file whose map has no ordinary entry is offered every
type**, as before the narrowing: a file no program loads, a new class file before any entry maps it, a
program that only uses `require`, and a map of `links` lines alone. An empty list there would offer
nothing to start from.

**Four lists stay workspace-wide.** The namespaces offered inside an `autoload` prefix or path string,
because that is where a developer adds a module to the map. The short names a `use` line must not take,
because a type in the namespace in force answers to its short name whether the map loads it or not. A
link's route argument keeps reading the `links` roots. And find references, rename, the type hierarchy,
the code lens counts and the unused-member dimming (`rule:ide/five-features-are-one-reference-index`),
because a change to a framework class must reach every module that uses it.

**This is not a security boundary.** The compiler and the map are, and a type outside the map is still
a compile error wherever it is written. A module that must not be used from another is held by a
`[[restrict]]` entry (`rule:packaging/a-restriction-limits-where-a-declaration-is-used`).

**The spelling offered is the shortest one that resolves, and the editor makes it resolve.** A type an
import or the namespace in force already reaches is offered by that short name and edits nothing else. Any
other type is offered by its last segment, with the qualified name beside it, and the item carries one more
edit: `use Qualified\Name;` after the last `use` the cursor's namespace has, failing that after the
`namespace Name;` line in force, failing that after the open tag. Where the short name is already taken in
the file, or there is no such line to write after — a bracketed namespace with no `use` in it, a shebang
script with no open tag — the item is the qualified name and edits nothing else. **A type the file already
writes by its qualified name keeps that spelling.** Where `Core\Arr::map(…)` is written, `Arr` is offered as
`Core\Arr` with no `use` line, and so is a type declared inside it, `Core\Arr::Order`: a `use` line would
leave the file with two spellings of one class. That set is read off the buffer's tokens, because the
index records no use of a `Core` class and is only as new as its last build. After `use`, every type is
its qualified name, because a declaration's name is absolute
(`rule:statements/a-qualified-name-is-absolute`).

**Matching is the client's.** The server sends the whole list once and the editor filters it as the name is
typed, so `cs` finds `Core\Str` by the editor's own match across the separator: an item's filter text is
both of its spellings. **Ranking is the server's where the match ties**, in this order: the variables the
body declared, imported types, types in the namespace in force, types already written somewhere in the
file, the rest of `Core`, the rest of the workspace, and the reserved words last. Every tier
is read off a table an arm already reads — the body's scope, the file's imports, the index's occurrences.
A member list after `->` or `::` has no tiers.

**Two cursors inside an access are bare positions.** A cursor touching the end of an access's receiver
is writing the receiver, so `Request|` is offered the type list with its `use` line rather than the
members of a class that does not resolve. A cursor past the member name with a `(` between the receiver
and itself is in the arguments or past the call, so `Core\Debug::dump(|)` is offered what `f(|)` is,
although the access node spans its own arguments.

- **Memory:** one item per type on every bare-position request, a few hundred for the registry alone, two
  more lexings of the document, and one string per qualified name it writes. All are built and dropped
  with the answer. The loadable test adds no table: it is one prefix walk and one path comparison per
  indexed type, and a module's lists get shorter, so each answer is smaller to send and to filter.

**Declined:**
- **Accepting a type writes its qualified name.** It edits nothing else, but it is not what the developer
  would have written by hand, which is the short name and a `use` line.
- **A `CompletionList` marked `isIncomplete`, so the server matches fuzzily itself.** The client would
  ask again on every keystroke. The editor already matches across a separator, and a round trip per
  character is latency spent to do the same match twice.
- **Namespaces offered as items.** Typing `Core` already lists every `Core` type, and the separator's
  list takes over at the `\`.
- **A per-module allowlist or blocklist of namespaces in a file of its own.** It states what the map
  already states, the two drift, and the editor would then hide code that compiles or offer code that
  does not. A blocklist also fails open for every module added later.
- **Sorting another module's types last instead of leaving them out.** Each one is a compile error where
  it is offered.
- **Probing the disk per item through `AutoloadMap::resolve`.** One file-system call per indexed type per
  request, for an answer the index's path already gives.
- **Narrowing references and rename to the map.** A change to a framework class must reach every module
  that uses it.
