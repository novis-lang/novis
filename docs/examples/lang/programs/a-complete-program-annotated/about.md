A short tour of what almost every Novis program has in its first ten lines.

A program is a file of statements that run from top to bottom. Every variable has a type. You write
the type once, where you create the variable, and the variable keeps that type. A `foreach` loop
also names the type of each value it reads from a list.

To convert a value to another type, you use `as`. For example, a form sends text, and `as` converts
that text to a number you can add. If the value cannot be converted, the program stops with an
error.

Every built-in function is a method of a class, such as `Core\Str::length`. A program cannot read a
file, call another server or start another program until the person who runs it allows that.

**The examples below** show this one task at a time: adding the numbers a form sent, sorting a list
with the one option that `sort` accepts, and building a JSON reply.
