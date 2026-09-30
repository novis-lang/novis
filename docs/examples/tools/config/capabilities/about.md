`[capabilities]` lists what a program may do outside its own memory, such as reading a file or
opening a connection.

Each of these actions needs a named capability, and every capability is denied until the
configuration grants it. `fs.read` and `fs.write` are for files. `script.spawn` is for starting
another script, and `process.exec` is for starting another program. `net.connect`, `net.listen`
and `net.local` are for the network. There are more for databases, mail and the shared cache.

A grant has three forms. `true` allows everything. A list allows only the directories, hosts or
names in the list. `false`, an empty list and a missing key all deny.

A call without its grant throws a `RuntimeError`. The message names the method, the capability and
the argument. The program can catch this error and continue.

**In plain words:** a program starts in a room where every door is locked. Each grant is the key
to one door.

**Good to know:** Novis resolves a path first and then compares whole directory names, so `data2`
is not inside `data`. A grant to read a file is not a grant to run it as a script.
