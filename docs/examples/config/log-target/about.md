`[log] target` sets where Novis writes the record of a failure that it reports itself.

When a request fails, Novis first runs the program's own error handler. If that does not run, it
runs the script that `[log] handler` names. If that fails too, Novis writes the record itself.
`[log] target` is where that record goes.

The value is `stderr` (the standard error stream), `file:<path>`, or `syslog`. When the key is not
set, it is `stderr`. A service that does not set it writes to `logs/nvs.log` in the data folder.
`[log] max_size` and `[log] keep` limit the size of a file.

Only the person who runs the server can set this key. A program cannot change it. Novis checks
the value when it reads the configuration file, and a value it does not know is an error.

The example prints the target and shows that a program cannot change it.
