Runs a function and gives you everything it printed, as a value. This replaces PHP's `ob_start` and
`ob_get_clean`.

The function uses `echo` as usual, but nothing it prints reaches the output. `capture` collects it
and returns it as a `Core\Cli\Text`. You can print that value later with `echo`, or read it as a
`string` with its `text` method. The function's own return value is not used.

You can call `capture` inside another `capture`. Each call collects only what its own function
printed. If the function throws an error, the collecting stops and the error continues as usual.

The `through` option takes a second function. It gets the collected text and returns a changed copy,
and `capture` returns that copy.

**Good to know:** output from `Core\Debug::dump` is not collected. It still appears as usual.

**The examples below** collect some output and print it later, change it with `through`, and check
what a function prints.
