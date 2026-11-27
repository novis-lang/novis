A warm hit parses, checks and lowers the same source in full; only code generation and the pages it
would have produced come off the disk instead. That is the direct consequence of
`rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header` carrying no class
metadata: the loading process needs the lowered IR to build the descriptors the stored code's
symbols resolve against, and to name each function it places.

**A server that keeps its edit state skips the unchanged part of the front end too.** Under
`[opcache] incremental` a recompile reuses each source file's checked result whose key did not
move (`rule:config/an-edit-reaches-the-next-request-without-a-restart`). That reuse is kept in the
serving process's memory and never written to disk, so a process with nothing in memory still
checks every file. A cold compile generates each function once: it compiles the functions the
store misses, and places every function from the same pieces it stores.

The cost is named rather than hidden, and it is judged by one number: the margin by which a warm hit
beats the cold compile it replaces, with the front end inside neither arm. **A cache that does not beat
compiling is a cache to delete.** The guard states the margin it holds the loader to, and a change that
brought placing and relocating within that factor of codegen is the one thing that reopens the file
format — with an IR class list versioned alongside the file, never a serialised runtime type.

A cold run pays its compile once per distinct function and environment, ever; every later run, in
any process, is a read, a verify, a placement, the relocations and an `mprotect`.

**Declined:**
- **A checked result written to disk, so a fresh process skips the front end.** It serialises the
  checker's types, so every change to them becomes a format bump, and it buys the front end only
  for the first compile of a process.
- **A second walk through the JIT on a cold compile.** With function reuse the placer runs on every
  compile anyway, and one walk halves a cold compile's code generation.
