Runs a function and gives you everything it printed, as a value.

The function uses `echo` as usual, but nothing it prints reaches the output. `capture` collects it
and returns it as a value. In a command-line program the value is a `Core\Cli\Text`. In a web
request it is a `Core\Html\Markup`. The text in it was already escaped once when it was printed.
You can print the value later with `echo`, and it is not escaped a second time. `as string` converts
it to a `string`. The function's own return value is not used.

You can call `capture` inside another `capture`. Each call collects only what its own function
printed. If the function throws an error, the collecting stops and the error continues as usual.

The `through` option takes a second function. It gets the collected value and returns a changed copy
of the same class, and `capture` returns that copy.

**Good to know:** output from `Core\Debug::dump` is not collected. It still appears as usual.

**The examples below** collect some output and print it later, change it with `through`, and check
what a function prints.
