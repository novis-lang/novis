Says whether the program is allowed to do something, so it can take another route when it is not.

A deployment decides what each program may do: read files, connect to another server, run another
script, reach a database. Each of these permissions is called a capability and has a name, such as
`fs.read`. `Core\Cap::has` takes one of those names and returns `true` or `false`. A feature that is
useful but not required checks first, and does something else when the answer is `false`.

**Good to know:** the check changes nothing. A program can never give itself a capability it was not
given. A `true` is also not a promise about one file or one server. It means the capability was
granted for something. The exact file is checked again when the program opens it, and that check can
still say no.

**The examples below** check what a program may do, show what a `true` does not cover, and switch an
optional feature off.
