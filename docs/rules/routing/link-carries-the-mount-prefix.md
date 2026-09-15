`Core\Router::url` prepends the prefix of the mount the request arrived through, in front of the
substituted path. One compiled table then serves the same module at `/ModuleA`, at `/ModuleB` or at
`/` with no recompile — a path is source state and a mount is deployment state, so the prefix is
known only where the request is — and it is why link generation goes through this member rather than
concatenating a declared path: a link assembled from the declaration is wrong the day the module is
mounted anywhere but the root.

A program run off the command line is mounted nowhere and the prefix is empty; nothing else about the
link changes (`rule:routing/link-name-and-params-are-checked`). `urlAbsolute`'s configured origin
is the other half of an absolute link, and is read from the resolved unit rather than from any header.

The prefix is read off the request the door already wrote it on
(`rule:routing/a-request-reads-its-mount`), never derived from the request target, and all three of
`url`, `urlAbsolute` and `urlSigned` join it — a signed link survives a remount because what is
signed is the route's name and its parameters, not the path. A test describes its door with
`Core\Test::request`'s `mount` key, so what a mounted deployment writes is pinned without a server.
