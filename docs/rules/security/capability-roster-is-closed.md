Every capability name that exists is on one roster, in one place, with the reasoning for what it
permits named beside it. **A name not on the roster is not a capability**, which is what makes a grant
line's unknown name checkable at boot and what makes a written query for a name nobody can grant a
compile error rather than a permanent `false`.

The roster is closed in the sense that it does not grow by accident: a new capability is one variant
and one arm in the one place a name maps to the field that grants it, which is also where a refusal
gets the spelling it prints. Two capabilities that answer different questions stay two names — being
able to read a file is not permission to run it
(`rule:security/script-spawn-capability`), and opening a connection an operator named is not opening
one a program chose.

A member that reaches nothing declares that by entering the table with no capability rather than by
being absent from it (`rule:security/capability-declaration-is-one-table`), so the standard library's
own surface is a closed claim rather than a list with an exception column.
