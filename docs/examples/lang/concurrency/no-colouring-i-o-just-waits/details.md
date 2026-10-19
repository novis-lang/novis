`async`, `await` and `Fiber` are not keywords or classes. `async function` and `await $x` do not
compile, and `new Fiber(…)` is `E0303`.

A task runs until it waits. Two tasks that never wait run one after the other, each from its first
line to its last.

A task may start its own group of tasks. The inner `Core\Task::all` or `Core\Task::map` returns
before the outer task goes on.

What a task prints with `echo` is kept until the group is done. Then each task's output is printed
in the order the tasks were written, not in the order they finished. For `Core\Task::map`, that is
the order of the array. The output of a task that was stopped or that threw is printed too.

A static property has one value, and every task of the program reads and writes that same value.
