`#[After]` runs after a test and exists only for residue that outlives the isolate — a file, a row,
a remote object. Teardown is otherwise unnecessary here: the isolate dies and takes everything
inside it with it, which is what leaves this marker one narrow job rather than a general hook.

Every such residue is capability-bearing, so there is nothing `#[After]` can usefully do until a
test run can hold a capability.
