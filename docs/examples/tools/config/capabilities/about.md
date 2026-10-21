`[capabilities]` lists what a program may do outside its own memory.

`fs.read` and `fs.write` are for files. `script.spawn` starts a script, and `process.exec` starts a
program. `net.connect`, `net.listen` and `net.local` are for the network. Others cover databases,
mail and the shared cache.

With no `[capabilities]`, a command-line program may use any file, connect to public hosts and
loopback, and run any program. A server program may read its application folder, except `nvs.toml` and names that start
with `.`. It may write only in folders from `Core\IO::temporaryDir`, and connect to public hosts.
Both may start a script from that folder. Other capabilities need a grant.

A grant is `true` for all, or a list of directories, hosts or names. `false` or `[]` allows nothing.
A key you write replaces its default.

A call without its grant throws a `RuntimeError`. Its message names the method, the capability and
the argument.

**Good to know:** paths compare whole folder names, so `data2` is not inside `data`. `fs.read` does
not allow running a script.
