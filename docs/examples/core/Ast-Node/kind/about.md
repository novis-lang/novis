The name of the construct this node is, such as `If`, `Echo` or `ClassDecl`.

Every part of a parsed file is one construct of the language, and `kind` is its name. The name comes
from the grammar, not from the text of the program, so two files that write the same construct
differently still give the same name. The node you get back from `Core\Ast::parse` is the whole
file, and its kind is `File`.

This is what you branch on when you walk a tree. A tool that treats loops one way and declarations
another reads `kind` at every node and decides from there. The set of names is fixed and known, so a
name your walk does not handle is a construct you have not covered yet.

**Good to know:** the name says what the construct is, never what it says. Use `line`, `column` and
`offset` to find the part in the source when you need the text itself.

**The examples below** show what each part of a small program is, how to count the loops in a file,
and a check that a file declares classes and runs nothing else.
