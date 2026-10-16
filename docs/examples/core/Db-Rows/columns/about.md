Describes the columns a query answered, one `Core\Db\Column` for each.

`Core\Db\Rows::columns` returns what the statement described. There is one entry per column, and the
entries come in the order the query asked for them. Each entry is a `Core\Db\Column`, which gives you
the column's name, the type it was declared as, and whether it may hold `null`.

The description belongs to the statement, not to the rows. A query that matched no rows describes its
columns too, so a program can print the headings of a table before it knows whether there is anything
under them. For the same reason `queryAs` describes the columns the query asked for, and the class it
builds has nothing to say about them.

Every call returns the same description, and none of them costs a copy.

**The examples below** list the columns a query answered, print the headings when no rows matched, and
write the first line of a CSV export.
