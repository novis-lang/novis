The compiled-unit cache is keyed by **content**, not by path: `UnitKey { path, content_hash,
env_hash } → CompileState`, and a `Ready` entry is write-once — nothing already in the map is ever
mutated or torn. In front of it sits one small indirection, `path → current content_hash`, which is
the pointer an edit swaps.

Resolving a `require` or an inbound request's entry file walks five steps: reuse the known hash
under `opcache.validate = "never"` or inside `revalidate_freq`, with no syscall; otherwise `stat`
(and under `hash`, or on an `mtime` mismatch, re-hash) the file, and continue with no compile if
the content is unchanged; on a change, compile the new content through the same single-flight
machinery a cold compile uses, on the compile pool, never on a request-serving core; on success
swap the path's pointer, publishing only if nobody moved it since; on failure leave the pointer
alone (`rule:config/a-broken-edit-fails-the-requests-that-resolve-it`).

`mtime` is a cheap pre-filter; only the content hash is trusted as the key, so a filesystem with a
coarse clock cannot serve stale code. There is no filesystem watcher, no stop-the-world phase and no
second process, and a client cannot trigger a recompile — only the file's own content changing does.
The rate cap bounds `stat` overhead to `N ⁄ revalidate_freq` per file, and laziness means only files
a request actually resolves ever recompile, however many a deploy touched.
