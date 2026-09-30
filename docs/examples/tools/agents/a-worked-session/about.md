An example of how a coding agent finds the right method with the `nvs agent` commands.

The agent has to print the length of a name. It knows the PHP function `strlen`, so it runs
`nvs agent find strlen`. The command prints nothing, because Novis has nothing with that name. The
agent then runs `nvs agent find length` and gets two lines: `Core\Str::length` and
`Core\Bytes::length`. `nvs agent show 'Core\Str::length'` prints the description, which says that
this method counts characters and not bytes. The agent uses it, and `nvs check` accepts the
program. For the name "Zoë" the program prints `3`.

`nvs check` gives the same help. A program that calls `strlen` does not compile, and the error
names `Core\Str::length` as the replacement.

A keyword is found the same way. `nvs agent find autoload` returns a heading of the reference, and
`nvs agent show` with that heading prints the section.
