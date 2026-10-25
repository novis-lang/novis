`Core\Task::all` runs a fixed set of jobs at the same time and gives you all their results together.

You give it a shape: a value with named fields, written `{ rows: …, label: … }`. Each field holds a
function that takes no arguments. `Core\Task::all` returns a shape with the same field names, and
each field has the type that job's function returns. You read `$page->rows` straight away, with no
conversion. A variable of type `callable`, with no signature, gives you `mixed` for that field alone.

`Core\Task::all` returns only when every job has finished, so nothing from the group is still
running afterwards. The subject has to be a shape of functions. An array of functions does not
compile.

**The examples below** show three jobs read by name, the three ways to write a job, a product
page that loads its parts at the same time, and an array of functions that does not compile.
