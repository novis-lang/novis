A source change — an edited, added or removed file anywhere in a program — reaches the next request
without a restart or a reload, in every mode. The compiled-unit cache is keyed by **content**, not by
path: `UnitKey { path, program_digest, probe_hash, env_hash } → CompileState`, and a `Ready` entry is
write-once — nothing already in the map is ever mutated or torn. In front of it sits one small
indirection, `path → current key`, which is the pointer a change swaps.

**A unit is keyed on its whole program.** A compile records every file it read, with the stamp
(`mtime` and size) and the digest it read, and the key's digest is the whole-program digest the
on-disk artifact cache computes. A check of a unit looks at every one of those files, every
`autoload` path the compile probed, misses included, and every directory a discovery query listed
(`rule:packaging/autoload-probes-fold-into-the-cache-key`). Under `mtime` a file whose stamp did not
move is not read; under `hash` every file is re-hashed. `mtime` is only a pre-filter: the content
digest is the key, so a coarse clock cannot serve stale code.

**No request makes a file-system call to revalidate.** A request resolves a path to the unit its
pointer names, which is a map lookup; a path nothing has resolved yet is an ordinary cold compile. A
task on the compile pool checks every loaded program once per `revalidate_freq`. On a change it waits
until no file of the program has moved for `[opcache] settle`, compiles the program through the same
single-flight machinery a cold compile uses, checks every file again, and throws the compile away and
retries if anything moved while it ran. Only then does it swap the pointer, publishing only if nobody
moved it since. An idle server takes a change within `revalidate_freq` plus `settle`.

**An atomic deploy is atomic.** At the start of each compile the entry file's path is resolved
through every symlink and junction once, and the program is read through the real directory that
gives. After the compile the path is resolved again, and a different answer discards the compile, so
switching a `current` link between two releases never builds a program from both.

A compile that fails leaves the pointer where it was
(`rule:config/a-broken-edit-fails-the-requests-that-resolve-it`), and a deleted file a program still
reaches is that case. The table keeps, per path, the unit in force and the one it replaced, so a
reverted edit is a pointer swap and not a compile. Any older unit is freed when the last request
holding it ends, so units stay in proportion to entry files, never to edits.

There is no file-system watcher, no stop-the-world phase and no second process, and a client cannot
trigger a recompile — only the program's own files changing do. The cost is one `stat` per loaded
file, and one per listed directory, per `revalidate_freq`, off the request path.

**What is on disk.** The resolve-time form of this rule: the check runs inside the resolve, on the
request path, and the key's digest is the entry file's content
alone, so an edit to a `require`d or autoloaded file is not seen until the entry file changes. The
background check, `settle`, the link re-resolve and the whole-program key are not.
