An extension can read files, write files and send HTTP requests only when three parties all allow
it.

The person who runs the server writes `grants` on the extension's `[[extension]]` entry in
`nvs.toml`. `read` and `write` are lists of folders, and `connect` is a list of hosts. An entry
without `grants` gives the extension no files and no hosts.

The extension's manifest lists what it asks for. If it does not ask to write files, it cannot write.
`nvs ext inspect` prints what an extension asks for.

The program that calls the extension sets the last limit. An extension never reaches more than that
program may reach.

A package that calls such an extension needs a line in `[grants]`. The line lists `fs.read` for an
extension that reads, and `net.connect` for one that connects. An extension that writes also needs
`fs.write`. Novis code inside an extension is a package too, and it needs its own line.

**Good to know:** a path with `..` or a symbolic link never leaves a granted folder.
