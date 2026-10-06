Reads Novis source code and returns a tree that describes it.

You pass the text of a program, starting with `<?nvs` the way a file does. You get back one node for
the whole file. Every node says what kind of part it is, such as `ClassDecl`, `Echo` or `Int`, and
where that part starts. From a node you can walk down to the parts inside it.

This is the same parser the `nvs` command uses. Source the compiler accepts is accepted here. Source
it rejects throws a `ParseError` naming the line and column of the first problem.

**Good to know:** the tree is data. Nothing in it runs the code you parsed, and no node gives you the
source text back.

**The examples below** show the parts of a small program, a check for valid source, and a project
rule that forbids `exit`.
