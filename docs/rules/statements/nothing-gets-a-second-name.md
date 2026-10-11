A class, interface, enum, method or constant is reachable under exactly the name it was declared with — its
own short name, or a fully-qualified path to it — and under no other.

`use Path\To\Name;` imports under the declared short name, full stop. Renaming an import is a diagnostic:
*imports cannot be renamed; refer to `Name` by its declared short name, or use the fully-qualified path
directly.* Nothing registers a second runtime-reachable name for a class either, and no such member is
coming later under another spelling.

The one real cost — two unrelated libraries choosing the same short class name — is paid at the call site
with the fully-qualified path: strictly more to type and strictly less to keep track of. Because no `as`
survives, making a bare `Str` resolve to an unrelated class is structurally unreachable rather than merely
refused, and the symbol table keeps exactly one entry per declared name, with no code path anywhere asking
whether a name was resolved directly or through an alias.

**One `use` names one import.** PHP's group form, `use App\Models\{User, Post};`, is `E0238` where the
`\{` is written, the sibling of the rename refusal on the other half of the same statement. The group
form does not give anything a second name, since every short name it introduces is still that name.
It is refused for simplicity: it is a second syntax for a statement that already exists, its payoff is
fewer lines, and its cost is that the short names a file introduces are no longer one per `^use` line
for a `grep`. The parser reports the brace group and skips it, so the statement still yields an import
for the prefix that was written and nothing downstream sees a half-parsed one.

**Declined:**
- **Import `as` kept for two libraries whose short names collide.** The fully-qualified name is more
  explicit for a little more typing, and a carve-out for collisions only does not stay narrow.
