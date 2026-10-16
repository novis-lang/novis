Returns every environment variable as one array, with the variable's name as the key.

The array is sorted by name. Two calls give the same order, on every operating system. Every value
is tainted: it comes from outside your program, so you must check or escape it before you use it in
a query, a URL or a command. A variable whose name or value is not valid UTF-8 text is left out of
the array. `Core\Env::get` with that name throws an error that says why.

**In plain words:** tainted means "not checked yet". You can compare the value or write it to a
log. You cannot put it into a query or a command until you have checked it.

**Good to know:** a program cannot change its environment. There is no `putenv`.
