`nvs reload` makes a running server use the files on disk now. The server checks its configuration
and every loaded program. The command returns when every compile has ended.

It prints one line for each server, and an `error:` line with the file and line of each error. The
exit code is:

- `0` when every server replied and uses the new files.
- `1` when a file has an error, or a server did not reply.
- `2` when no server is running for this data folder, or every server is shutting down.

Only the account that runs the server, or an administrator, can run `nvs reload`. A request cannot
start a reload.

The server compiles only the functions that changed. In development it also checks only the files
that changed (see `[opcache] incremental`). The `program reloaded` record in the log has the counts.

**Good to know:** a program with an error does not run, and neither does its older version. Its
requests fail until the file is fixed.
