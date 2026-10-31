`nvs dap` is the debug adapter. Your editor starts it when you debug a program. You do not run it
yourself.

In VS Code, you add a configuration of type `nvs` to `.vscode/launch.json`, or you start one from
the Run and Debug view. The configuration names the `.nvs` file in `program`. `mode` is `run`,
`test` or `serve`. The default is `run`.

While the program runs, you can set breakpoints and step through the code. A breakpoint can have a
condition, a hit count or a log message. After a step out, the Variables view shows the value the
function returned. *Uncaught Exceptions* stops the program at an exception that no `catch` handles.

While the program is stopped, the *Novis Tasks* view in Run and Debug shows each task under the
task that started it. For example, the two functions you give to `Core\Task::all` are shown under
the task that called it. Select a task to open its file at the line where it stopped.

What the program prints appears in the debug console as text.

**Good to know:** `nvs dap` only starts programs. It cannot attach to a program that is already
running, so you cannot debug a production server.
