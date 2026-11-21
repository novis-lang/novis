`nvs reload` makes a running server use the files on disk now. Run it after you edit code or
configuration, or at the end of a deploy. The server checks its configuration files and every
program it has loaded at once. The command returns when every compile has ended.

It prints one line for each server, and one `error:` line with the file and the line for each
error. The exit code is `0` when every server uses the new files. It is `1` when a configuration
file or a program has an error. It is `2` when no server is running for this data folder.

Only the account that runs the server, or an administrator, can run `nvs reload` on it. A request
cannot start a reload.

The server also checks the files by itself every `[opcache] revalidate_freq`. If you or a coding
agent run `nvs reload` after each batch of edits, set `revalidate_freq = "1h"`. A forgotten reload
then still takes effect within one hour.

**Good to know:** a program with an error is not replaced. Its requests fail with the error until
the file is fixed. The server does not run the older version.
