The message has two lines. The first is ``Core\IO::read needs the capability `fs.read` for <path>,
which is not granted``, with the full path of the file. The second is ``help: grant it in nvs.toml
under `[capabilities.fs]` ``.

`fs.read` covers every call that looks at a file, such as `Core\IO::exists`, and not only
`Core\IO::read`. `fs.write` is a separate capability. A grant of `read` for a folder does not let your
program write there.

A grant lists folders. Novis resolves each path to its real full path before it checks it. A `..` that
leads out of a granted folder is denied, and so is a symbolic link that points out of one. A path
with `..` that ends inside a granted folder is allowed.

A file that does not exist inside a granted folder is not a denial. The call throws `IOError`, a
subclass of `RuntimeError`, with the operating system's message. A path that does not exist outside
every granted folder is denied like any other path.

`spawn script "job.nvs"` needs `script.spawn`. Its message starts with ``spawn script` needs the
capability `script.spawn` ``, and the `help:` line names `[capabilities.script]`.
