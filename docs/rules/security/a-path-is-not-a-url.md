No `Core` member that takes a path interprets a scheme prefix, and there is no registry by which
userland or an extension adds one. A read of `"data://text/plain,..."` looks for a file with that name and
does not find it.

Making every filesystem function accept a URL, and letting userland register new schemes, is the root
of an entire vulnerability taxonomy: metadata in an archive path triggering unserialization on any
file operation that touches it, filter chains that turn an arbitrary file read into arbitrary code
execution, and `data://`/`http://` turning every local file-inclusion bug into a remote one.

The mechanism's actual benefit is polymorphism over "things you can read bytes from," and that is
available without any of the above, as an ordinary interface implemented by ordinary types and
resolved statically. What is refused is specifically **dispatch on the textual content of a path**,
which is also what makes `rule:security/path-scope-canonicalise-then-prefix` a total rule rather than
one with a scheme-shaped hole in it.
