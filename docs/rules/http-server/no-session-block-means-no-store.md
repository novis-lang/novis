`[session]` absent is not "sessions off with a default backend". It is a session surface that
throws on `Core\Session::start`, naming the block to write. The premise that a deployment with
nothing configured is safe (`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`) has
one answer for a store nobody chose: no store. Every other `Core\Session` member already throws
until `start` has run (`rule:core-classes/session-is-started-explicitly`), so a program that
cannot use sessions can say so at the root.

A block that names `shared` but a tree with no `[cache.shared]` is the same shape one step later:
`start` reaches the one store that block names, and says which block is missing when there is
none, rather than opening a store of its own.
