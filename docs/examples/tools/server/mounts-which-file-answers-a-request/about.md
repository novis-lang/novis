A mount tells the server which program file answers a URL.

Each `[[server.mount]]` block in `nvs.toml` has a `prefix`, a `host`, or both, and names one file as
its `entry`. A request whose URL starts with the prefix runs that file. When several mounts match,
the server uses the one with the longest prefix. When no mount matches, the server sends a `404`.

The server removes the prefix before the program runs, so `Core\Request::path()` returns only the
part after the prefix. The same program therefore works under `/shop` and under `/`. A mount with
`scan` in place of `entry` is a pattern for many files. The server creates one mount for each file
that matches the pattern.

**In plain words:** a mount is the sign at the entrance of a building. The sign says which office
receives the visitors of each department.

**Good to know:** every file must be inside the folder that `[server] root` names. In production,
the entry file answers every URL under its prefix. In development, a URL that names another `.nvs`
file in the folder of the entry file runs that file.
