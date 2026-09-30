A program can read which mount the server chose for a request, and the path that is left after the
prefix of that mount.

The server removes the prefix before the program runs. `Core\Request::path()` returns the part
after the prefix, so the same program works at `/`, at `/shop` and under a pattern.
`Core\Request::mount()` returns the rest. Its `prefix()` method returns the part that the server
removed. Its `captures()` method returns the text that each `*` in the `scan` pattern of the mount
matched, in order. `Core\Router::url` adds the prefix to every link that it builds.

Inside a request, `mount()` always returns a value. A request that came through a mount at `/`
gets `""` from `prefix()` and an empty array from `captures()`.

**In plain words:** the program is an office in a large building. The mount tells the office which
entrance the visitor used.

**Good to know:** the captures are `tainted`, which means text that the client chose, because the
client chooses the URL. The prefix is plain text. A program that answers no request, such as a
command-line program, gets a `LogicError` from `mount()`.
