The inventory, the migration table and the `Core` registry are joined once, at build time, into a
static sorted table compiled into the language server. The server reads no file under `docs/` at
runtime: a shipped binary does not depend on the documentation tree, and a lookup answers in the time
a lookup takes.

The join is a consistency check, and the check is a benefit the layer buys rather than a cost it pays.
A destination spelling in the migration table that matches no registry row, and that no spec section
schedules, is a typo in the docs or a member the registry renamed — and it fails the build instead of
shipping as a suggestion that cannot resolve. A destination that is merely unbuilt is not a mismatch:
the spec names it where the registry does not, and it becomes an inert item under
`rule:php-migration/an-item-inserts-only-a-registered-member`.

Nobody writes a mapping for the editor. The migration table is the converter's table too, and the rule
that neither translation table is copied into the other binds this reader the same way; a mapping the
editor owned would drift, and the drift would surface as the editor and the converter disagreeing
about one name.
