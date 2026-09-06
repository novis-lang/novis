A warm hit parses, checks and lowers the same source in full; only the Cranelift walk and the pages it
would have produced come off the disk instead. That is the direct consequence of
`rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header` carrying no class
metadata: the loading process needs the lowered IR to build the descriptors the payload's undefined
names resolve against, and to name a function it needs by position in that shared program.

The cost is named rather than hidden, and it is judged by one number: the margin by which a warm hit
beats the cold compile it replaces, with the front end inside neither arm. **A cache that does not beat
compiling is a cache to delete.** The guard states the margin it holds the loader to, and a change that
brought place-relocate-bind within that factor of codegen is the one thing that reopens the file
format — with an IR class list versioned alongside the file, never a serialised runtime type.

A cold run pays its compile once per distinct (content, environment) pair, ever; every later run, in any
process, is an `mmap`, a verify, a relocate and an `mprotect`.
