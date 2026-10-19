A statement is one step of a program. Most statements are an expression followed by `;`: a call, an
assignment, an `echo`. The value is thrown away, and the work still happens. A `;` on its own is an
empty statement.

A declaration is a statement too. Write the type, or write `var` and the type comes from the value.
A variable belongs to the whole function, so each name is declared once.

Braces `{ … }` group statements into a block. A block does not open a new scope. You may read a
variable only where every path has given it a value. A variable assigned in one branch of an `if`
is readable after the `if` when every branch assigns it.

**The examples below** show the statements, a variable declared before its value is
known, and a program that prices an order. The last one reads a variable that only one branch
declares, and does not compile.
