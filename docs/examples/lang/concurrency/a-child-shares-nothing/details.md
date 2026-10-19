A static property with an array value starts at its declared value in the child too. An entry the
parent added is not there, and an entry the child adds does not reach the parent.

A child started with `on: "here"` runs on the same core as the parent. It shares nothing with the
parent either.

A task in `Core\Task::all` can start a child. All tasks of the program share one value of each static
property, but the child still gets its own copy.

The child is a separate program. It does not need the classes the parent declares, and it cannot read
the parent's variables. A child that uses `$x` without declaring it does not compile. The error is
`E0301`, and `spawn script` throws `RuntimeError` in the parent.
