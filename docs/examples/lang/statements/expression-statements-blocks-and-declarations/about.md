A statement is one step of a program. Most statements are an expression followed by `;`: a call, an
assignment, an `echo`. The value of such a statement is thrown away, and the work still happens. A
`;` on its own is an empty statement.

A declaration is a statement too. Write the type, or write `var` and the type comes from the value on
the right. A variable belongs to the whole function, so each name is declared once per function.

Braces `{ … }` group statements into a block. A block does not open a new scope, so a variable
declared inside one can still be read after the closing brace. What the position does change is
where you may read the variable: only where every path has given it a value. A variable first
assigned in one branch of an `if` is readable after the `if` when every branch assigns it.

**The examples below** show the statements a program is made of, how to declare a variable before
its value is known, and these forms together in a program that prices an order.
