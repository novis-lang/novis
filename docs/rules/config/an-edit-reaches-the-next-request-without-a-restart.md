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
switching a `current` link between two releases never builds a program from both. A slow copy into
the directory being served can still be seen half done if it pauses for longer than `settle`,
because no compiler can tell a pause from the end; switching a link to a finished directory is the
deploy that is always safe.

Where a design choice makes this rule harder to keep, the rule wins over latency, simplicity and
memory, and never over security.

A compile that fails leaves the pointer where it was
(`rule:config/a-broken-edit-fails-the-requests-that-resolve-it`), and a deleted file a program still
reaches is that case. The table keeps, per path, the unit in force and the one it replaced, so a
reverted edit is a pointer swap and not a compile.

There is no file-system watcher, no stop-the-world phase and no second process, and a client cannot
trigger a recompile — only the program's own files changing do. The cost is one `stat` per loaded
file, and one per listed directory, per `revalidate_freq`, off the request path. Only a program some
request has loaded is checked, so a deploy that touches many files recompiles only the programs in
use. The table is the one structure every core shares, so a swap made by the check is what every
core's next lookup reads, with no broadcast and no second cache to keep in step. A warm start in
production reads source too: every file a program reached is `stat`ed at the first check, which is
the price of seeing a change at all.

- **Memory:** each unit keeps the list of files its compile read, with a stamp and a digest per
  file. The path pointer is a second map beside the content-keyed table, one entry per entry
  file. Beyond the two units each path keeps, an older unit is freed when the last request holding
  it ends, so units stay in proportion to entry files, never to edits.

**What is on disk.** All of it for the program. The quiet time is measured from the newest
modification time among the files the program's last compile read. `nvs serve <file>` keeps its
entry unresolved, but resolves its mount root once at boot, so its static files still come from the
release it started with. A `[[server.mount]]` table resolves every entry and root at boot, so a link
switch does not reach a server that mounts one.

**Declined:**
- **A file-system watcher pushing invalidation.** It is four platform APIs, its watch descriptors are
  a finite OS resource, and it degrades silently on network file systems. It also misses a replace
  by rename, so the stamp check would still be needed beside it.
- **A restart, or a supervisor that respawns the process, on a change.** It drops every in-flight
  request, not only the ones that touch the changed file.
- **One global generation counter compared on every lookup.** It serialises unrelated files'
  updates against one another's readers. Swapping each path's pointer only if it still names the key
  the check started from needs no counter, and a slower compile of an older edit loses that compare.
- **Mutating a `Ready` entry in place instead of the path pointer.** A `Ready` entry is read
  without synchronisation because its content never changes. Collapsing the two maps into one
  `path → state` map gives that up, and every guarantee above rests on it.
- **Trusting `mtime` alone.** It fails on containers, some network mounts and coarse clocks, which
  are the file systems least likely to be noticed failing.
- **Checking every file on the request path.** It is correct, and it puts the whole-program `stat`
  and every recompile in front of whichever request arrives first. A request that arrives during a
  `git pull` could also compile a program from half of the old tree and half of the new one.
- **Pushing an edit to every process of a fleet.** Sub-second propagation across processes or hosts
  is a distribution problem, not a concurrency one. A signal for it would feed the same pointer swap.
