# ADR 0017 — The compiled-unit cache revalidates lazily and swaps one pointer, never a watcher or a restart

- **Status:** Accepted
- **Date:** 2026-08-20
- **Scope:** `mwl-host`'s compiled-unit cache, the M7 HTTP server's requirement to pick up an edited source
  file with no restart and no dropped request
- **Relates to:** [0002](0002-error-propagation.md) (a failed revalidation surfaces as a checked diagnostic,
  never a panic or an unwind), [0004](0004-memory-for-simplicity.md) (the memory a stale generation is
  allowed to hold, and for how long), [0005](0005-config-changeability.md) (`opcache.validate` and its
  revalidation-rate directive are `System`-class, not something a request can loosen),
  [0006](0006-isolated-script-execution.md) (the content-addressed compiled-unit cache and the "only
  compiled code is shared" boundary this ADR extends)

> **In short:** the cache is keyed by content, not by path: `UnitKey { path, content_hash } →
> CompileState`, and a `Ready` entry is write-once — nothing already in the map is ever mutated or
> torn. Sitting in front of it is one small, separately-locked indirection, `path → current
> content_hash`. Revalidating a path means re-stating (mtime+size, rate-capped) or re-hashing the
> file, compiling the new content through the *same* single-flight machinery a cold compile already
> uses, and then swapping the path's pointer to the new hash — publishing only if nobody has moved it
> since. A request that resolved a file before the swap keeps running against the `Arc` it already
> holds; a request that resolves it after sees the new content or, if the new content fails to
> compile, the failure as ordinary checked-return data. No filesystem watcher, no stop-the-world
> phase, no second process, and no request-serving core is ever blocked on a compile.

## Context

M7 requires that editing a `.mwl` file on disk be visible to the next request without restarting the
process — and, in the same breath, that every request stay exactly as isolated as it would be behind a
fresh subprocess ([0006](0006-isolated-script-execution.md)): a compile in flight for one file must not
stall a request that never touches it, an old request must not be yanked out from under a version of the
code it started running against, and nothing about noticing a file changed may reintroduce the shared
mutable state the whole architecture exists to avoid.

The plan already has half of this decided, in the *Compiled-unit cache* sketch:

```rust
enum CompileState {
    Compiling(broadcast::Receiver<Result<Arc<CompiledUnit>, Arc<Diagnostics>>>),
    Ready(Arc<CompiledUnit>),
    Failed(Arc<Diagnostics>),
}
// DashMap<UnitKey { path, content_hash }, CompileState>
```

That key is the detail worth noticing: it is `{path, content_hash}`, not `{path}`. Two versions of the same
file are two different map entries, never the same entry overwritten. That makes every `Ready` entry
**write-once** — a property this ADR relies on rather than one it has to add. What is missing is the piece
that turns "content-addressed cache" into "hot-reloadable cache": something that maps a *path* to whichever
`content_hash` is current right now, and a rule for updating that mapping safely while readers are using it
concurrently, from multiple cores, without ever taking a lock a request-serving core would have to wait on.

## Investigation

**Why not a filesystem watcher (`inotify`/`ReadDirectoryChangesW`/`FSEvents`/`kqueue`).** It looks like the
obvious answer and was the first one considered. Rejected:

- It is four platform-specific APIs to keep correct instead of one, on a project already carrying three
  platforms in CI.
- Watch descriptors are a finite OS resource; a web root with thousands of files is exactly the shape that
  exhausts `inotify`'s default limits, which turns a hot-reload feature into a startup failure on some
  hosts and not others.
- It silently degrades on network filesystems (NFS, SMB — an ordinary shared-hosting or container-volume
  deployment), where change events are unreliable or absent. A mechanism whose correctness depends on the
  filesystem underneath is not one this project can make a security or availability claim about.
- It does not remove the need for a stat/hash check anyway — a watcher can still miss the replace-via-rename
  window common to editors and deploy tools — so it would be additional machinery layered *on top of* the
  lazy check below, never a replacement for it. Additional machinery that does not remove a requirement is
  not paying for itself.
- It is a background subsystem whose activity is not attributable to any request, which is the opposite of
  [0004](0004-memory-for-simplicity.md)'s accounting requirement.

**Why lazy, stat-gated revalidation instead.** The plan's existing staleness rule — `stat (mtime+size) →
BLAKE3 content hash → atomic swap`, governed by `opcache.validate = never|mtime|hash` — is PHP's own
`opcache.validate_timestamps` model, which every operator deploying a PHP-shaped runtime already
understands. It costs nothing when nothing changed (one `stat`, compared against the cached mtime+size), it
needs no new subsystem, and its cost is paid on the request that happens to touch the file, which is exactly
where [0004](0004-memory-for-simplicity.md) wants it accounted. `mtime` is a cheap pre-filter; `hash` is the
authoritative fallback for the filesystems (containers, some network mounts) where mtimes lie. Neither
signal is trusted alone for the actual cache key — only the content hash is, which is what makes the whole
scheme safe rather than merely fast.

**Why a rate cap on the stat check.** A `stat` per request is cheap once; it is not free at the request
volumes M7 is verified against (10k+ concurrent). The same directive family gets a companion,
`opcache.revalidate_freq` (seconds, default matching PHP's), so a hot path hitting one file re-validates at
most once per interval regardless of request rate — the same amortisation PHP already made operators
comfortable with, not a new number to learn.

**Why the pointer swap is small and separately locked, not a rebuild of the whole map.** A design that
gives every path revalidation a single global version counter, compared against on every request, was
considered and rejected: it would serialise unrelated files' updates against each other's readers for no
reason, and it reintroduces exactly the kind of shared mutable checkpoint the thread-per-core model exists
to avoid. Keying the swap per path, and using the content hash itself as the compare-and-swap token, needs
no separate generation counter: "publish this new hash only if the path still names the hash I started
from" is answered by one lookup-and-compare inside the map shard's existing lock — the same lock DashMap
already takes for a cold insert. A slower compile of an *older* edit that finishes after a faster compile of
a *newer* one simply loses that compare and is discarded; nothing was lost, because the newer content was
already correctly published.

## Decision

**The cache gains one small indirection in front of the content-addressed store it already has, and
revalidating a path means resolving through that indirection with the same single-flight compile machinery a
cold compile already uses.**

```rust
struct PathEntry {
    content_hash: Blake3Hash,
    last_checked: Instant,     // gates the revalidate_freq rate cap
}
// DashMap<CanonicalPath, PathEntry>            — the swappable pointer, one per path
// DashMap<UnitKey { path, content_hash }, CompileState>   — unchanged, write-once per entry
```

Resolving a `require`/`include`/an inbound request's entry file:

1. Look up `PathEntry` for the canonical path. If `opcache.validate = never`, or the path was checked within
   `revalidate_freq`, use its `content_hash` as-is — no syscall.
2. Otherwise `stat` (and, under `hash` or on an `mtime` mismatch, re-hash) the file. If the observed content
   matches `PathEntry.content_hash`, update `last_checked` and continue — still no compile.
3. On an observed change: resolve/compile `UnitKey { path, new_hash }` through the *existing* `Compiling` /
   `Ready` / `Failed` state machine, off the dedicated compile pool, exactly as a cold compile does. Callers
   racing to revalidate the same path to the same new hash single-flight on the same broadcast, for the same
   reason concurrent cold hits already do.
4. On success, write `new_hash` into `PathEntry` — but only if `PathEntry.content_hash` still equals the
   hash this revalidation started from. If it does not, a fresher revalidation already won; the result of
   this one is simply dropped, not applied backwards.
5. On failure, `PathEntry` is left unchanged — the path still names the last content that compiled — but the
   `Failed` entry the resolution just reached is what *this* caller's resolution returns. A request that
   resolves the path after this point and lands on the same `content_hash` sees the same `Failed` state,
   because it is the same `UnitKey`; a request that resolves it *before* the edit, and already holds the old
   `Arc<CompiledUnit>`, is entirely unaffected — it never re-resolves mid-execution.

**A file is resolved once per isolate, the first time execution reaches it — never re-resolved on a second
reference within the same run.** This is what makes a change invisible to a request already in flight: the
`Arc<CompiledUnit>` a running isolate holds was cloned out of the cache at the moment of first touch, and an
`Arc` is never mutated in place, only ever replaced at the `PathEntry` layer above it. The same property is
why no lock a request-serving core takes is ever held across a compile: steps 1–2 above are a `stat` and a
map lookup; step 3 happens on the compile pool, never on a core serving other requests; only the final
pointer write in step 4 touches the map again, and that write is the same brief per-shard lock a cold insert
already pays.

**Every other request-local limit still applies exactly as if this were a fresh subprocess.** The compiled
code is the *only* thing this ADR, or [0006](0006-isolated-script-execution.md) before it, ever shares
across requests. A request's heap arena, its `[limits]`/`[limits.hard]` memory and CPU ceilings
([0005](0005-config-changeability.md)), its own `Core\Request`/`Core\Server`/`Core\Session` state, and its
`catch_unwind` panic containment ([0002](0002-error-propagation.md)) are unaffected by anything in this
document — a hot-reload event changes *what code a future request compiles to*, never how isolated any
request's execution of that code is. That isolation was already total before this ADR; this ADR only says
how the one deliberately shared thing may change safely underneath it.

**`opcache.validate` and its rate cap are `System`-class** ([0005](0005-config-changeability.md)): a request
cannot loosen how often, or whether, the process re-checks source files. Letting a request set
`validate = never` for itself would be a way to pin a version of the code past a since-shipped fix, and
letting it lower `revalidate_freq` would be a way to force a `stat`/hash storm on a hot file. Neither is a
request-local decision.

## Consequences

**Positive**

- No new subsystem, no new dependency, no new failure mode to reason about: the same content-addressed
  cache, the same single-flight compile state machine, and the same `Arc`-immutability the rest of the
  architecture already leans on, with one small map added in front.
- A compile of one changed file never blocks a request-serving core, including the core whose request
  noticed the staleness — that request keeps running against the version it already resolved while the new
  one compiles on the pool.
- Cross-core visibility is free: the compiled-unit cache was always the one process-wide structure shared
  across the otherwise shared-nothing per-core runtimes, so a swap performed from any core's detection is
  visible to every core's next lookup with no broadcast, no IPC, and no second cache to keep in sync.
- Memory is bounded the way [0004](0004-memory-for-simplicity.md) requires: a stale `UnitKey` entry is
  retained only for as long as some in-flight request still holds a clone of its `Arc<CompiledUnit>` (bounded
  by that request's own wall-clock/CPU limit) or until eviction reclaims it once nothing references it — so
  the number of live generations of one file is bounded by how many *distinct* generations currently-in-flight
  requests happen to span, which is itself bounded by concurrency in flight, not by how many edits have ever
  happened. O(in-flight), never O(requests served).
- A client cannot trigger recompilation at all — only the file's own content changing does — so this
  mechanism adds no new attacker-reachable trigger. Combined with the rate cap, the worst case an operator
  faces from an *accidental* hot file plus frequent edits is bounded `stat` overhead, never a compile storm:
  laziness means only files a request actually resolves ever recompile, regardless of how many files a
  deploy touched.

**Negative**

- **A revalidated file that now fails to compile fails the request that resolves it, not silently — matching
  PHP's own `validate_timestamps` behaviour, not the more forgiving "keep serving the last good version"
  policy.** Considered and rejected: silently continuing to serve stale code after an edit — especially a
  security fix — is a worse failure mode than a loud compile error, and diverging from PHP's observable
  behaviour here would cost priority 2 to buy availability priority 3 does not actually need, since only
  requests newly resolving the broken file are affected at all.
- **Two data structures instead of one** (`PathEntry` index plus the content-addressed store) is a small
  cost against priority 4. Accepted because collapsing them back into one — a single `path → CompileState`
  map, mutated in place — is exactly the design that would make a `Ready` entry no longer write-once, which
  is the property every other guarantee in this document rests on.
- **Old generations are not reclaimed by anything but "nothing references it any more."** A pathological
  pattern — one file edited continuously while a slow request holds an old generation open indefinitely —
  retains that generation until the request ends or hits its own CPU/wall-clock limit. Accepted, because that
  ceiling already exists ([0005](0005-config-changeability.md)) and bounding it twice would be a second copy
  of the same rule.

## Alternatives rejected

- **A filesystem watcher pushing invalidation.** See *Investigation* — platform-divergent, resource-bounded
  in a way that scales badly with a large web root, unreliable on network filesystems, and additive to the
  lazy check rather than a replacement for it.
- **Restart (or a supervisor that respawns) the process on any change.** This is the requirement's explicit
  exclusion, and it is also strictly worse than what it would replace: every in-flight request is dropped,
  not merely the ones touching the changed file, which fails the isolation requirement this ADR is answering
  at the same time it fails the no-restart one.
- **A global generation counter compared on every request.** Serialises unrelated files against one another
  for no benefit over a per-path compare-and-swap, and reintroduces a piece of shared, request-path-visible
  mutable state the architecture otherwise avoids.
- **Mutating a `Ready` entry's `Arc<CompiledUnit>` in place** (e.g. `Arc<Mutex<CompiledUnit>>` or an
  `ArcSwap` inside the entry) instead of adding the path-level indirection. Rejected because it means the
  content-addressed key no longer identifies immutable content — the entire reason `Ready` entries can be
  read without synchronisation in the first place — for a saving of one small map that this ADR's other
  guarantees are not worth spending.
- **Trusting `mtime` alone**, without a content hash. Already excluded by the existing `hash` validation
  mode; restated here because it is the natural first instinct and it fails on exactly the filesystems
  (containers, some network mounts, coarse clocks) an operator is least likely to notice failing on until an
  edit silently does not take effect.

## Revisiting

Reopen if a deployment wants **push-based** invalidation for reasons beyond correctness — e.g. a
multi-process or multi-host fleet wanting sub-second propagation of an edit without waiting for the next
request to a given process to notice. That is a distribution problem (how do N processes agree on "current"
without a shared filesystem's stat clock), not a concurrency-safety one, and it should extend this design
rather than replace it: the `PathEntry` swap is exactly the point such a signal would feed.

Verification, to land with M7 since the mechanism does not exist before it:

- A compile-counter assertion, extending the one M7 already commits to: editing a file mid-load triggers
  exactly one recompile even under many concurrent requests racing to notice the same change.
- A request that resolved a file before an edit completes on the pre-edit compiled unit even if the edit (and
  a successful recompile) lands before that request finishes; the next request to resolve the same path gets
  the new one.
- A revalidation that fails to compile fails only requests that resolve the path afterwards; requests already
  running are unaffected.
- Two edits landing close together resolve to the second edit's content, never the first's, regardless of
  which recompile happens to finish first.
- A `stat` storm test: N requests per second against one `mtime`-validated file perform at most
  `N ⁄ revalidate_freq` stats, not N.
- Nothing above changes the state-bleed suite's result: it must still pass with hot-reload active, since nothing
  here touches a request's arena, limits, or `Core` accessor state.
