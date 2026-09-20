The parts directly inside this node, in the order they appear in the source.

Every node of a parsed file is made of smaller parts. The file's own node has one child for each
top-level statement. A class has its properties and its methods. Each child is a node too, so you
can step down again from any of them.

This goes one step only. Use `nodes` when you want everything below a node, however deep it sits. A
node with nothing inside it, such as a number, gives you an empty array, and that is how you tell a
leaf from a branch.

**The examples below** show one step down into a class, how a leaf differs from a branch, and an
outline of a whole file printed level by level.
