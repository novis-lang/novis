An unqualified class, interface or enum name that resolves to nothing carries one `use` line per type
it could have meant, and the editor offers each as a quick fix. The checker computes it where it raises
`E0303`: every type the program declares, and every `Core` type the registry names, whose last segment is
the written name is a candidate (`nvs_hir::imports::candidates`), and each becomes a suggestion whose
replacement is `use Qualified\Name;` inserted at the place every `use` line this server writes goes
(`nvs_hir::imports::import_site`) — after the last `use` the namespace has, failing that after the
`namespace Name;` line, failing that after the open tag. The editor's code action is the same translation
of a suggestion it already makes for the casing fix and the legacy-cast fix
(`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`): the checker chose the text, and
the server computes nothing.

**One candidate is a machine-applicable fix; several are each offered for a person to choose.** The
checker cannot say which `Request` was meant when two namespaces declare one, and a code action never
makes a choice (`rule:ide/a-code-action-writes-only-what-is-already-determined`), so the ambiguous case
is several actions and the unique case is one that `source.fixAll.nvs` may apply on save. Three names get
no fix at all: a qualified one, because it is absolute and no import changes what it means
(`rule:statements/a-qualified-name-is-absolute`); a short name an existing `use` already resolves to
something undeclared, because that import is the mistake and a second `use` of the same short name would
not compile; and one written in a file with no place to put the line — a bracketed namespace with no
`use` in it, a script with no open tag — because the checker names a place or none, and never invents
one.

What it spends is a walk over the symbol table and the registry on the checker's error path, and
nothing on a program that compiles.
