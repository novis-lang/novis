Spawning an isolate requires `script.spawn`, a deny-by-default grant naming the roots that code may be
executed *from*. Being able to read a file is not permission to run it, so `fs.read` does not imply
`script.spawn`: the two answer different questions, and a template directory that is readable by
design should not become an execution root by accident.

The entry path is canonicalised and then prefix-checked against the granted roots
(`rule:security/path-scope-canonicalise-then-prefix`), which closes traversal by construction rather
than by validation. A dynamic path is allowed — a queue worker needs one — but it can only ever land
inside a root an operator wrote down.

The child's grants are the parent's effective grants, optionally narrowed at the spawn site. Nothing
widens: a parent that has dropped `net.connect` cannot regain it by spawning
(`rule:security/no-runtime-grant`).
