`nvs reload` makes a running server use the files on disk now. Run it after you edit code or
configuration, or at the end of a deploy. The server immediately checks its configuration files and
every program it has loaded. The command returns when every compile has ended.

It prints one line for each server, and one `error:` line with the file and the line for each
error. The exit code is `0` when every server replied and uses the new files. It is `1` when a
configuration file or a program has an error. It is also `1` when a server did not reply, and the
line for that server says so. It is `2` when no server is running for this data folder. The data
folder is where the server keeps its files, and `nvs serve` and `nvs reload` find it the same way.

A server that is shutting down does not reload. Its line says `is stopping, so it did not reload`.
This line does not make the exit code `1`. When every server is shutting down, the exit code is `2`.

Only the account that runs the server, or an administrator, can run `nvs reload` on it. A request
cannot start a reload.

The server also checks the files by itself every `[opcache] revalidate_freq`. If you or a coding
agent run `nvs reload` after each batch of edits, set `revalidate_freq = "1h"`. A forgotten reload
then still takes effect within one hour.

**Good to know:** a program with an error is not replaced. Its requests fail with the error until
the file is fixed. The server does not run the older version.
