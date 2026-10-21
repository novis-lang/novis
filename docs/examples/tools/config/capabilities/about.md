`[capabilities]` lists what a program may do outside its own memory, such as reading a file or
opening a connection.

Each of these actions has a named capability. `fs.read` and `fs.write` are for files. `script.spawn`
is for starting another script, and `process.exec` is for starting another program. `net.connect`,
`net.listen` and `net.local` are for the network. There are more for databases, mail and the shared
cache.

With no `[capabilities]` lines, a program you start from the command line may read and write any
file. It may also connect to any public host and to loopback, and run any program. A server program
may read its application folder, except `nvs.toml` and names that start with `.`. It may write only
in the folders that `Core\IO::temporaryDir` made for it. It may connect to any public host, and may
not run another program. On both, a script inside the application folder may be started.

The application folder is the folder of the file the program started from, or the `root` of its
`[[app]]` block. Listening and the other
capabilities have no default, so they need a grant.

A grant has three forms. `true` allows everything. A list allows only the directories, hosts or
names in the list. `false` and an empty list allow nothing. A key you write replaces its default.

A call without its grant throws a `RuntimeError`. The message names the method, the capability and
the argument. The program can catch this error and continue.

**Good to know:** Novis resolves a path first and then compares whole directory names, so `data2`
is not inside `data`. A grant to read a file is not a grant to run it as a script.
