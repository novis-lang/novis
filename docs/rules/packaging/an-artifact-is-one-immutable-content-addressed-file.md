The on-disk artifact cache is a fan-out directory of content-addressed files:

```
<cache_dir>/<key[0:2]>/<key[2:]>.nvsc
```

**The unit of code is a source file, under a small program list.** Three kinds of file share the
path scheme, the header, the verify and the eviction:

- **A source file's entry** holds the machine code of every function whose first statement is in
  that file, each under its own key, and the literals they reach. Its key is
  `BLAKE3(BLAKE3("nvs file entry" ‖ each function key) ‖ env_hash)`, so two programs that share a
  file whose functions compile alike share its entry.
- **The program list** is each entry's key in program order, under the unit's key, `key =
  BLAKE3(content_hash ‖ env_hash)`.
- **The hint** names the last list written for one program, under a key from its entry file's
  path, its unit kind and its optimisation level. It is the one file the store replaces, by the
  same rename, and it is how a process with nothing in memory reuses functions after an edit.

**A function's key is BLAKE3 over the code it would compile to:** the target and its flags, its
lowered Cranelift IR with each target renumbered by first use, and each target's name free of its
position. A unit function is named by its Novis name and a literal by the digest of its contents.
Every slot offset, static slot, literal and callee the emit read is an operand or a relocation
in that IR, so the key moves exactly when the code would. A function's statement, edge, query and
index numbers are relocations the placer writes, so its bytes do not depend on where it sits in
the program. An edit writes the entry of the file it changed, a new list and the hint, and a warm
hit compiles nothing: the list names every entry, and their code is placed as stored.

A stale or missing hint costs reuse and never correctness: every function reused is found under
its own key in an entry that passed its checksum.

`content_hash` is BLAKE3 over every file the program
reached, each as its name, its text and its folder. The name is there because a diagnostic and a
throw's frame print it. The folder is there because a relative path given as a string literal compiles to an absolute
path joined to it (`rule:programs/relative-paths-resolve-from-their-file`), so the same file in another
folder is another program. The content hash is computed once per unit and shared with the in-memory
`UnitKey`, so a unit's bytes cross BLAKE3 one time however many caches it lands in. `env_hash` is the single environment digest of
`rule:config/the-extension-set-is-in-every-unit-key` — target triple, CPU feature bitset, compiler
build and the loaded extension set — and the same value keys the in-memory cache, so one process
cannot disagree with its own disk cache about what a unit was compiled against.

**The environment is in the address, not only in the header.** An artifact built for another machine,
another CPU-feature set, another compiler build or another extension set is a path this process never
looks up; the cost is one failed `open`, the same as any miss, never an open-then-reject. The extension
set belongs in the key because codegen emits a direct call to an extension trampoline, so the loaded
set is a codegen input like any other.

Both digests are cryptographic on purpose. One `nvs serve` shares one `env_hash` across every tenant's
source, so a key a tenant could collide on purpose would hand their artifact to another tenant's
request, and `rule:packaging/a-writer-publishes-by-one-atomic-rename-and-never-a-lock` is true only of a
collision-resistant key. What a non-cryptographic hash would save is small, and it is spent off the
request path. No shared index, no lock file and no manifest exist anywhere, and the hint is a
pointer nothing trusts: every entry is
independently creatable, verifiable and discardable, so a bug or an attack against one entry has a
blast radius of exactly one entry. The store is read and written by independent processes that share
no memory and no lock and may never overlap in time, so a design that needs a live coordinator does
not fit it. The `[opcache]` directives that govern the store are
`rule:config/opcache-file-cache-directives-are-system`'s.

The cost of an environment in the address is disk: a cache directory shared across a mixed fleet
(several CPU generations, a compiler upgrade half rolled out) holds one copy of a unit per environment
rather than one in total. Checking the environment after opening the file would change only when that
storage is paid, not whether.

**Declined:**
- **One packed store — sqlite, or a hand-rolled B-tree — instead of a file per entry.** sqlite is the
  kind of C dependency the pure-Rust policy keeps out, and a shared mutable index needs write
  coordination across processes that never talk; a corrupt index takes out every entry at once.
- **A shared manifest or LRU index beside per-entry files.** It is a second shared-mutable structure
  every reader trusts and every writer keeps consistent, the shape the in-memory cache already rejects
  for a per-path pointer (`rule:config/an-edit-reaches-the-next-request-without-a-restart`).
- **One file per program.** An edit to one function then compiles and writes the whole program
  again, so a reload grows with the program and never with the edit.
- **One file per function.** An object file's headers cost about 1 KB per function on disk, and a
  hit opens one file per function.
- **A relocatable object file per source file.** Each needs a Cranelift module of its own, which
  declares the whole program once per file. A piece is the function's bytes, alignment and
  relocations, and the store's own placer reads five relocation kinds; a function with any other
  kind is never stored.
- **A key from a list of the tables a function reads, kept beside the emit.** A table the list
  forgot would reuse stale code, which is a security bug. The emitted IR is the record of what
  the emit read, and it cannot forget one.
