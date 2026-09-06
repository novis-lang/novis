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
