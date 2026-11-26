Says what a program is allowed to do. A capability is one permission, such as reading files,
connecting to a host, starting a script, or using a database, a mail server or a mailbox. Each key
of this block is one capability.

Some keys have a default, and the default depends on the kind of run. A command-line run, such as
`nvs run`, reads and writes any file. Server work, such as a request, reads only the application
folder. That is the folder of the file the server started from. A key with no default allows
nothing until you write it.

A key you write replaces its default completely. `[]` allows nothing. A value can be `true`, or a list
of the places it covers, such as the folders a program may read or the hosts it may connect to.

**Good to know:** a program can read its own capabilities, but it cannot change them while it runs. So
`nvs.toml` is the one place that says what a program may do.
