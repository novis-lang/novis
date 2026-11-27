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

**A recompile does the work the edit reached, not the program's.** Machine code is reused function
by function from the store (`rule:packaging/an-artifact-is-one-immutable-content-addressed-file`):
a function whose key did not move is placed as stored, in every mode. Under `[opcache]
incremental`, `true` in `development` and `false` in `production`
(`rule:config/a-startup-default-is-never-flipped`), the compile pool also keeps two things per
written entry path: each function's code, so a reuse reads no disk, and each source file's checked
result, keyed on its text and the declarations its last check read. A recompile then checks a file whose text moved and each file that reads a declaration
whose signature moved, and no other. A body edit checks one file, compiles one function and
writes one source-file entry at every program size. The `program reloaded` record carries those
counts (`rule:config/a-reload-command-checks-now-and-waits`), and
`benches/scaling/compiler/reload.nvs` holds them flat from 55 to 440 files. Two costs still grow
with the program: every file is stamped or hashed, and the whole program's code is placed again.
With `incremental` off, both are freed when the compile ends, so a production
server's memory between deploys is its units alone.

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
trigger a recompile — only the program's own files changing do. The one local exception is
`nvs reload` (`rule:config/a-reload-command-checks-now-and-waits`): the serving account or an
administrator runs the same check at once, through a private endpoint no request can reach, and it
compiles only what that check finds moved. It is also the only thing that skips `settle`, because
its caller says the writes are done. The cost is one `stat` per loaded
file, and one per listed directory, per `revalidate_freq`, off the request path. Only a program some
request has loaded is checked, so a deploy that touches many files recompiles only the programs in
use. The table is the one structure every core shares, so a swap made by the check is what every
core's next lookup reads, with no broadcast and no second cache to keep in step. A warm start in
production reads source too: every file a program reached is `stat`ed at the first check, which is
the price of seeing a change at all.

- **Memory:** each unit keeps the list of files its compile read, with a stamp and a digest per
  file. The path pointer is a second map beside the content-keyed table, one entry per entry
  file. Beyond the two units each path keeps, an older unit is freed when the last request holding
  it ends, so units stay in proportion to entry files, never to edits. Under `incremental`, each
  entry path also keeps its files' checked results and its functions' code for the unit in force
  only: about what `nvs check` peaks at for the program plus its machine code, about 200 MiB at
  440 files. It is per program, never per
  request or per edit.

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
- **Compiling a function on its first call.** A recompile would then be one function's work, and
  the cost moves onto whichever request calls it first. Every file a server serves is compiled
  before its socket is bound, and nothing on the request path is made on first use.
- **A file's checked result keyed on its own digest alone.** It cannot see a signature edit in
  another file that this one reads. The key holds the declarations the last check read, which the
  language server already records.
- **Sharing unchanged code between unit versions, for now.** Placing the whole program again costs
  29 MB at 440 files and about 1–2 s at 100 times that. Sharing needs every call between functions
  to go through a table per version, one more load per call on the request path. It waits for a
  measured program that needs it.
- **Pushing an edit to every process of a fleet.** Sub-second propagation across processes or hosts
  is a distribution problem, not a concurrency one. A signal for it would feed the same pointer swap.
