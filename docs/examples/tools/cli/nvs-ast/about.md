`nvs ast <file>` parses one file and prints its syntax tree (the structure that the parser builds
from the code).

The tree is printed with one level of indentation for each node, and each node has the position of
its text in the file. The command only parses. It does not check names or types, so it can print a
tree for a file that has errors in `nvs check`.

`--json` prints the tree as JSON for a tool. Each node has `kind`, `span` and `children`. Comments
and whitespace are nodes too.

The command prints a tree for a file with syntax errors too. With `--strict`, it prints no tree
when the parser reported an error.

**Good to know:** the JSON does not contain the text of a literal. Use the `span` of the node to
read the text from the file.
