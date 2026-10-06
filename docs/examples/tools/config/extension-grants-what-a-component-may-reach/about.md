An extension can read files, write files and send HTTP requests only when it is granted that. It
gets only what three parties all allow.

The first party is the person who runs the server. They write `grants` on the extension's
`[[extension]]` entry in `nvs.toml`. `read` and `write` are lists of folders, and `connect` is a
list of hosts. An entry without `grants` gives the extension no files and no hosts.

The second party is the extension itself. Its manifest lists what it asks for. If it does not ask to
write files, it cannot write, even when `grants` has a `write` folder. `nvs ext inspect` prints what
an extension asks for.

The third party is the program that calls the extension. An extension never reaches more than that
program may reach. If the program may read only one folder, the extension reads only that folder.

**Good to know:** a path with `..` or a symbolic link never leaves a granted folder. A request from
the extension is checked like a request from `Core\Http\Client`.
