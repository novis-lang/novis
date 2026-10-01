Short of a checked conversion, the only way to remove `tainted` is a `Core` member whose contract
states which **one** sink it is safe for: an HTML escape for HTML text, an identifier quote for a
dynamic table or column name, a path containment check for a path component, and one per sink as each
class is designed.

There is deliberately no generic `sanitize()` or `clean()`. A value safe for HTML text is not safe for
a shell argument or a path, and a single catch-all invites exactly the false confidence the qualifier
exists to prevent. The roster grows by adding a named member to the class that owns the sink, never by
widening an existing one.

**One launderer checks nothing in the bytes: `Core\Path::fromCwd`**, for a file path typed on the
command line. It throws while a request is being answered, so request data never passes through it.
The user who typed the path can already open any file the process can, and the `fs` grants still
bound every file its answer names. A request that opens a file named by its own data keeps
`Core\IO::within($base, $path)`. `fromCwd` removes `tainted` and nothing else: a `secret` argument
does not compile, as at every launderer.

Which return type a launderer takes is a predicate rather than a per-member choice
(`rule:security/launderer-answers-a-carrier`). Where no built-in launderer fits, the way out is
`rule:security/assert-trusted` — written, greppable, and carrying a reason — and never a silent cast.
