`Core\Task::all` and `Core\Task::map` run several tasks at the same time. When one task throws an
error, the whole group stops.

The first error wins. The other tasks are cancelled, and none of them is still running when your
`catch` runs. The error you catch is the one the task threw. It has the same class, the same message,
and your own fields if you wrote your own error class. Nothing wraps it.

A cancelled task runs nothing more. Its own `catch` does not run, and neither do the lines after it.

**The examples below** show a group where one task fails, an error class of your own crossing out of a
task, and a page that loads three things and reports the one that was missing.
