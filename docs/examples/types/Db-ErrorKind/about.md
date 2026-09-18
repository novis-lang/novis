Says why the database refused a statement, in the same words on every backend.

When a statement fails, the error your program catches carries one of these cases beside the server's
own message. They are conditions rather than codes: a duplicate key is a duplicate key whether the
server called it `23505`, `1062` or `SQLITE_CONSTRAINT_UNIQUE`, so a program written against one
database still reads correctly on the next. Some of the cases are worth acting on — a duplicate key is
usually something to tell the person using your app about, and a deadlock is usually worth trying
again. `Other` carries everything a driver's own table does not name.

**Good to know:** the server's raw `SQLSTATE` is still there beside the case, for the rare condition
that matters to you and has no case of its own. Nothing is lost by the normalising; it only saves you
from writing a table of vendor codes yourself.
