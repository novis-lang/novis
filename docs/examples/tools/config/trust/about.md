A server does not start when an account other than its own can write to its configuration files.

A configuration file grants capabilities, so a person who can change the file can change what a
program may do. The rule covers every file that the configuration reads, the directory of each
file, and each secret file that it names. If the check fails for one of them, the start stops with
the error `E0607`.

On Linux, a group that can only read the files passes the check. A group that can write them does
not pass. After `chmod 0664` on `nvs.toml`, the group can write the file, and the next start fails.

**Good to know:** the server applies this check. In this version, `nvs run` and `nvs config check`
do not apply it. The chapter "Installing on a host" lists the permissions that pass on Windows and
on Linux, the commands that set them, and the meaning of each error message.
