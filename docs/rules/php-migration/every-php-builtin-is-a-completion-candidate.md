Every function and type name in the PHP inventory — `tools/data/php-builtins.txt`, generated from the
same PHP build the differential suite uses as its oracle — is a completion candidate from the day the
layer ships. What an item *says*, and what it may insert, is the migration table's row for that name
in `docs/spec/02-php-migration.md`. Two files, two jobs, and neither is copied into the other.

The split is the whole design: coverage of the migration table never decides whether the feature
works, only how good one answer is. The inventory is complete by construction; the table is filled in
one PHP domain at a time, and `bun nv migration` reports how far it has got. A name
with no row is not missing from the list — it is an item that says *undecided*, which reads as *Novis
has audited this and owes you an answer*, where a name that does not appear reads as *Novis cannot do
this*. A missing row and an `open` row are one case.

The PHP spelling never reaches a file. A PHP name that resolved at runtime would be the second name
`rule:statements/nothing-gets-a-second-name` removes, so the editor is the only correct place for it:
its output is the Novis spelling, bounded by
`rule:php-migration/an-item-inserts-only-a-registered-member`.
