# ADR 0042 — The on-disk artifact cache is one immutable, self-describing file per compiled unit, verified before it is ever mapped executable

- **Status:** Accepted
- **Date:** 2026-08-22
- **Scope:** the on-disk half of the "Code cache" decision the plan already names (content-addressed,
  BLAKE3) and M6's "content-addressed artifact cache with integrity verification and a refusal to use a
  world-writable cache directory." This ADR is the only copy of the file layout, the header shape, the
  write/verify/evict mechanics, and the `opcache.*` directives that govern it. It does not touch the
  in-process `DashMap<UnitKey, CompileState>` cache or the hot-reload pointer swap — those stay exactly as
  [ADR 0017](0017-hot-reload-without-restart.md) defines them.
- **Amended by:** 0078

> **In short:** the disk cache is a directory of immutable files, one per compiled unit, addressed by
> `BLAKE3(source content ‖ env_hash)`, where `env_hash` covers the target triple, the CPU feature bitset,
> the compiler build and the loaded extension set — folding the
> execution environment into the *address itself* rather than the *header*, so an artifact built for the
> wrong machine, the wrong compiler build or a different set of extensions is a plain cache miss, never a
> file that gets opened and then rejected. Each file still carries a self-describing header (magic, format
> version, the same `env_hash`, a BLAKE3 checksum of the payload) as defense in depth against a hash
> collision or a hand-placed file. A reader `mmap`s the file read-only, hashes the mapped bytes in place, and only calls `mprotect`
> to make it executable once the checksum matches — the existing W^X discipline, extended one step
> earlier. A writer compiles to a temp file, `fsync`s it, and does one atomic rename onto the final,
> content-addressed path; if that path already exists, the writer's own copy is simply discarded, never
> overwritten. No lock file, anywhere, ever. Eviction rides the already-expensive cold-compile path at a
> small probability — the same shape as PHP's own `session.gc_probability`/`gc_divisor` — so a warm cache
> hit never pays for it. And the checksum is explicitly **not** claimed as a defense against a hostile
> co-resident writer, only against corruption: that threat is closed by refusing a world-writable cache
> directory and requiring it be owned by the runtime's own account, a permission check, not a hash.

## Context

- The plan already commits to "content-addressed on-disk cache (BLAKE3) + in-process `Arc` sharing" and
  M6 already commits to "integrity verification" and "a refusal to use a world-writable cache directory,"
  plus a verification target of warm-cache `nvs run` startup being fast. None of the plan's own text says
  *what a cache entry looks like on disk*, *how a reader tells a good entry from a bad one before trusting
  it*, or *what makes the cache stop growing forever* — three questions M6 cannot be scoped against without
  an answer.
- Unlike the in-process cache, this store is read and written by **independent, non-communicating OS
  processes** — every `nvs run` invocation is a fresh process with no shared memory, no shared lock, and no
  guarantee any two invocations overlap in time. Any design that assumes a live coordinator (a lock file, an
  in-memory index) does not fit this shape at all.
- The stated goal is explicitly dual: fast (a one-off CLI script must not pay a JIT compile on every
  invocation) *and* safe (a corrupted, truncated, or wrong-environment file on disk must never become a
  crash, a wrong result, or — the sharper version — arbitrary code the runtime executes because it trusted
  a file it should not have).

## Investigation

- **Single packed store (sqlite index + blob file, or a hand-rolled B-tree).** Fewer inodes, one fsync
  point. Rejected: sqlite is exactly the class of C dependency `deny.toml`'s pure-Rust-by-default policy
  exists to keep out (the one existing exception, `rusqlite`, is taken for a *database feature*, not for
  infrastructure that could be a plain file store instead — a materially different argument for a
  materially different need). Worse, a shared, mutable index file needs real write coordination across
  concurrent, non-communicating processes; get that wrong and a corrupted *index* takes out every entry at
  once, which is a strictly worse blast radius than the one this ADR is scoped to bound.
- **A shared manifest/LRU-index file next to per-unit blobs.** Same shape of rejection: a second
  shared-mutable structure every reader must trust and every writer must keep consistent is exactly what
  [ADR 0017](0017-hot-reload-without-restart.md) already rejected once, for the identical reason, when it
  chose a per-path pointer over a global generation counter.
- **Folding the environment into the header only, keying purely by content hash.** Works, but means every
  reader must *open* a file before learning it is useless to them — an `nvs run` on a freshly rebuilt
  compiler, or on a different machine's cache directory shared over a network mount, would do a wasted
  open+read on every wrong-environment entry sharing that content hash. Folding the environment into the
  *address* turns that into a plain, cheap "no such file" — the same cost as a cache miss, not a
  cache-hit-then-reject.
- **Compressing the payload (zstd or similar).** Rejected for this specific payload: the entire benefit of
  a warm hit is `mmap` the bytes directly as the pages the JIT would otherwise have produced. Compression
  would force a decompress-into-a-fresh-buffer step before that mapping could happen at all, spending CPU
  to save disk space that a compiled unit does not have much of in the first place, and forfeiting the
  zero-copy read this design is built around.
- **`mmap` read-only → hash the mapped bytes → `mprotect` to executable only on match**, versus reading the
  file into a heap buffer first. `mmap` lets the page cache do the I/O work exactly once and lets BLAKE3
  (already multi-GB/s single-threaded) run directly over the mapped region with no extra copy — strictly
  less I/O and less memory movement than an explicit `read()` into a buffer, for the same verification
  guarantee.

## Decision

### 1. Layout: a fan-out directory of immutable, content-addressed files

```
<cache_dir>/<key[0:2]>/<key[2:]>.nvsc
```

where `key = BLAKE3(source_content ‖ env_hash)` — the same
git-object/cargo-incremental-cache shape, chosen for the same reason: cheap to compute, no shared index to
keep consistent, and a lookup miss costs exactly one failed `open`. Folding the environment into the key
itself (not just the header) means an artifact from a different machine, a different CPU-feature set, a
different compiler build or a different set of loaded extensions is never opened at all — it simply is not
the file this process would look for.

`env_hash` is [0078](0078-config-reload-and-control-socket.md) § 4's single environment digest,
`BLAKE3(target_triple ‖ cpu_feature_bitset ‖ compiler_version_hash ‖ extension_set_hash)`. The same value
keys [0017](0017-hot-reload-without-restart.md)'s in-memory `UnitKey`, so one process cannot disagree with
its own disk cache about what a unit was compiled against. The extension component is what makes the claim
in § 6 true: codegen emits a **direct call** to an extension trampoline
([0003](0003-extension-system.md) § *Extension functions are statically typed*), so the loaded set is a
codegen input like any other, and an artifact compiled against one set must never be reused against another.

### 2. File shape

```
magic ("NVSC") | format_version: u16 | env_hash: 32 bytes
  | payload_len: u64 | BLAKE3(payload): 32 bytes | payload
```

`env_hash` is repeated here even though it is already folded into the path, as defense in
depth against a `BLAKE3` collision or a file placed at that path by hand rather than produced by this
runtime — a second, independent check bought for the cost of one comparison.

### 3. Reading: verify fully before a single page becomes executable

`mmap` the file `PROT_READ` (never starting from `PROT_EXEC`). Check `magic`/`format_version`/`env_hash`
against what this process expects — any mismatch is a cache miss, not an error. Compute
`BLAKE3` over the mapped payload bytes and compare to the header's checksum — any mismatch is a cache miss:
delete the file (it can only be corrupt or tampered, never a second valid version — see §5) and fall
through to compiling fresh. Only once the checksum matches does the payload's pages get `mprotect`'d to
`PROT_READ | PROT_EXEC`, extending the W^X discipline the JIT's own freshly-compiled pages already follow
one step earlier in the pipeline. **No failure mode here reaches a panic, a `FATAL`, or a `Throwable` — a
bad cache entry is invisible to the script being run, exactly as invisible as a cold cache would be**
([ADR 0002](0002-error-propagation.md)'s checked-return discipline, extended to a compile-pipeline internal
that never had a caller to report to in the first place).

### 4. Writing: one atomic rename, no lock file, ever

Compile → write header + payload to `<cache_dir>/<key[0:2]>/.tmp-<random>` → `fsync` the temp file → rename
onto the final `<key[2:]>.nvsc` path. Rename is atomic on every target platform this project ships for
(POSIX `rename(2)`; Windows via `ReplaceFile`/`MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`), so no reader
ever observes a torn or partial file under the final name. If the final path already exists by the time the
rename would happen, another writer already produced byte-identical content — by construction, since the
key already includes the content hash — so this writer deletes its own temp file rather than overwriting;
there is nothing to reconcile, because there is nothing that could differ. **No directory `fsync` and no
lock file are used**: correctness here does not depend on durability across a crash — the failure mode of
losing an un-fsynced-to-disk entry is "the next process recompiles it," which is a cache miss, not a
correctness or security defect, so paying for that extra sync on every write buys nothing this ADR needs to
buy.

### 5. What the checksum defends against — and what it explicitly does not

The header checksum defends against **corruption**: truncation, bit rot, an interrupted write from a crash
that landed at a path this scheme's own rename discipline should have prevented but a caller bypassed
(direct filesystem tampering outside `nvs`'s own writer). **It is not, and is not claimed to be, a defense
against a hostile file placed by another local principal who can write into the cache directory** — that
principal can compute a perfectly valid header and checksum over payload bytes of their own choosing. That
threat is closed the way M6 already states it must be closed: **refuse to use a world-writable cache
directory**, extended here to also require the directory be owned by the account the runtime process itself
runs as (the same class of check `ssh` applies to `~/.ssh`) — a permission and ownership check, done once at
process start, not a per-file property. A checksum answers "is this the file I wrote," never "should I trust
whoever wrote it" — conflating the two would be the actual security hole.

### 6. Eviction: piggybacked, probabilistic, off the request path entirely

Content-addressed entries never need invalidating for *correctness* — only for *growth*. That holds because
§ 1's key covers **every** input to a compile, source and environment alike; it is why the extension set had
to join `env_hash` rather than sit outside the key, and why changing that set produces ordinary cache misses
instead of needing an invalidation pass ([0078](0078-config-reload-and-control-socket.md) § 4). On a cache
**miss**
(already the expensive path: a real compile is about to happen, on the dedicated compile pool, never on a
request-serving core) — after writing the new entry, with a small configured probability, walk the cache
directory's total size and, if over the configured cap, delete oldest-by-`mtime` entries down to a hysteresis
floor below the cap, so hovering exactly at the boundary does not re-trigger a walk on every subsequent miss.
This is deliberately the same probabilistic shape as PHP's own `session.gc_probability`/`gc_divisor` —
familiar, and it means **a warm cache hit never performs a directory walk, never checks a size, and pays
nothing extra beyond §3's verify-then-map** — keeping M6's own warm-cache CLI-startup verification bullet
intact. An operator wanting deterministic control instead of probability gets `nvs cache gc` / `nvs cache
clear` as explicit commands, same idea M6 already sketches for a tampered-artifact refusal.

### 7. Directives, all `System`-class

`opcache.file_cache` (bool, default on), `opcache.file_cache_dir` (path, root-owned, defaults to a fixed
system location), `opcache.file_cache_max_size` (bytes), `opcache.file_cache_gc_probability` /
`opcache.file_cache_gc_divisor` (mirroring PHP's session-GC pair). All `System`-class per
[ADR 0005](0005-config-changeability.md), for the identical reason `opcache.validate` is: a script that
could redirect where the process reads "already-compiled, about-to-be-trusted" native code from would be
handing itself a code-injection primitive, not a performance knob.

### 8. Reuse for a later milestone's wasm module cache

M9's extension system already names "compiled-module caching in the existing content-addressed artifact
cache." Nothing here is wasm-specific — a payload is a payload, and the same key/header/verify/evict shape
applies with Wasmtime's own module-serialization versioning standing in for `compiler_version_hash`. Not
re-litigated further here since M9 has not started.

## Consequences

**Positive**

- A one-off CLI invocation pays a JIT compile exactly once per distinct (content, target, compiler build)
  triple, ever — every subsequent invocation, on any process, is an `mmap` + verify + `mprotect`, no
  compile, matching the "why should a one-shot script recompile every time" question this ADR answers.
- Corruption, truncation, and wrong-environment artifacts are all cache misses, never crashes — the runtime
  behaves as if the cache entry never existed, which is always a safe fallback since a cache miss is already
  a handled, ordinary path (a fresh compile).
- No shared mutable structure exists anywhere in this design — every entry is independently
  creatable, independently verifiable, and independently discardable. A bug or attack against one entry has
  a blast radius of exactly one entry.
- No new dependency, no lock file, no background subsystem, no directory-fsync tax — the design spends
  almost nothing beyond the one BLAKE3 hash and one file open a cache hit needs regardless of the scheme
  chosen.

**Negative**

- Folding the environment into the cache key means the *same* source content produces a *different* cache
  entry per target/CPU-feature/compiler-build combination — expected and desired, but it means a cache
  directory shared across a heterogeneous fleet (mixed CPU generations, mid-rollout compiler upgrade) stores
  one copy per combination rather than one copy total. Accepted: the alternative (one entry, checked against
  the environment after opening it) only changes *when* the extra storage cost of heterogeneity is paid, not
  *whether* it is paid, and this ADR's Investigation section already rejected the "open then reject" version
  as the strictly worse option on the read path.
- The ownership/permission check in §5 is a startup-time refusal, not a runtime one — a directory whose
  ownership changes *after* the process has already started (a shared, long-lived host reconfigured under a
  running `nvs serve`) is not re-checked mid-run. Accepted as consistent with every other `System`-class
  directive: boot-time configuration is trusted for the life of the process, exactly as
  [ADR 0005](0005-config-changeability.md) already establishes for the rest of `nvs.toml`.
- Probabilistic eviction means the cache can transiently exceed its configured cap between the misses that
  happen to trigger a sweep — bounded by how unlikely a long silent stretch of pure cache hits is in
  practice, and correctable at any time with the explicit `nvs cache gc` escape hatch.

## Alternatives rejected

- **sqlite (or any single packed index) instead of a fan-out file-per-entry directory.** See *Investigation*
  — a C dependency this project avoids by default, and a shared mutable index whose corruption takes out the
  whole cache rather than one entry.
- **Keying purely by content hash, checking environment fields only after opening the file.** See
  *Investigation* — turns a wrong-environment miss into a wasted open-then-reject instead of a plain failed
  lookup.
- **Compressing the payload.** See *Investigation* — defeats the zero-copy `mmap`-as-executable-pages design
  this ADR is built around, for a payload class (compiled native code) that is already small.
- **A directory-level `fsync` on every write, for crash durability.** Rejected: the failure this would guard
  against — losing an entry on a crash before it reaches disk — degrades to an ordinary cache miss on the
  next run, not a correctness or security defect, so the extra sync buys nothing this design needs.
- **Treating the payload checksum as sufficient protection against a hostile cache directory.** Explicitly
  rejected as a *claim* — see §5. A checksum proves content integrity, never authorial trust; only a
  permission/ownership check closes that gap, and this ADR states that distinction rather than leaving it
  implied.

## Revisiting

- **Cross-host cache sharing over a network filesystem** (a build farm or a fleet wanting to skip compiling
  the same content on every host) is not addressed here — the same open [ADR 0017](0017-hot-reload-without-restart.md)
  already left for push-based invalidation: a distribution question, not a concurrency-safety one, and it
  should extend this design (the content-addressed key already makes a shared store *safe* to read from
  multiple hosts) rather than replace it.
- **Exact default values** for `opcache.file_cache_max_size` and the GC probability/divisor pair are left to
  whoever implements M6, the same way ADR 0018 left exact Clover/lcov shape to its implementer.
- **Per-ancestor-directory ownership walking** (checking not just the cache directory itself but every
  parent up to some root) versus checking the cache directory alone is left as an implementation-time call
  at M6 — the requirement decided here is that *some* ownership/permission check happens before the
  directory is trusted at all, not the exact depth of that walk.

Verification, to land with M6 since the mechanism does not exist before it:

- A bit flipped anywhere in a cache file's payload is detected and treated as a cache miss, never a crash,
  never wrong output.
- An artifact built for a different `target_triple`/CPU-feature set/compiler version is never opened at all
  — a filesystem-level miss, not a read-then-reject.
- Two processes racing to compile and cache the same content concurrently each produce a valid entry; the
  survivor is whichever rename wins, and no reader ever observes a partial file under the final name.
- A world-writable cache directory, and a cache directory not owned by the running account, are both
  refused at startup.
- Warm-cache `nvs run` startup meets M6's own verification bullet for it, unchanged by this ADR beyond
  actually specifying the mechanism that bullet was implicitly assuming.
- Cache size stays within its configured cap (plus the accepted hysteresis window) under a sustained stream
  of distinct-content compiles, with the sweep itself never observed on the request-serving/CLI-hot path.
