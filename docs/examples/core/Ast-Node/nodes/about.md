Every node inside this one, however deep it sits, in the order they appear in the source.

`children` goes one step down. `nodes` keeps going: the children, their children, and so on to the
bottom of the tree. On the node `Core\Ast::parse` gives you, that is every node of the file at once,
which is what you want when you are looking for something and do not care where it sits.

The node you call it on is not in the result. Both members answer what the node contains, and a node
does not contain itself. A node with nothing inside it gives you an empty array.

**Good to know:** the array is built when you ask for it, not kept on the node. Asking every node of
a large file for its own `nodes` walks the tree once per node, so keep the result if you need it
twice.

**The examples below** show how much deeper this goes than `children`, how big each class in a file
is, and which constructs a file uses.
