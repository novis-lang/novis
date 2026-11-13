A link from one module to a page of another module on the same server.

A module is one program on the server, with its own entry file. `autoload links` names the folders
of the other modules. Your program can link to their routes with `Core\Router::url`, but it cannot
call their code. A link to a route that does not exist does not compile.

The server picks the mount that the link uses. It takes the first match of this list:

1. A mount from the same `[[server.mount]]` block. Every module of one `scan` is in one block.
2. A mount on the same host. The link is a path.
3. A mount on another host with an `origin`. The link starts with that origin.

**Good to know:** if two mounts match at the same step, the server uses the block with
`canonical = true`. With no such block, or with two, the server does not start.
