# ADR 0131 — A temporary directory dies with its script, and the runtime's sweep never throws

- **Status:** Accepted
- **Date:** 2026-09-01
- **Scope:** `Core\IO::temporaryDir` — the owned root it creates under, who deletes what it creates and
  when, the two sweeps and their predicates, `nvs tmp clean`, and the `[io] temp_root` and
  `[debug] keep_temporary` keys. Not in scope: the member's capability check (unchanged, stated in its
  registry card per [0117](0117-an-implemented-core-member-documents-itself-in-the-registry.md)), the
  ordering of end-of-script user code ([0127](0127-the-end-of-a-script-is-observable.md) owns the hook
  queue, [0072](0072-core-task-structured-concurrency.md) § 6 owns `afterResponse`), and uploaded files,
  which never have a temp path at all ([0105](0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)).

> **In short:** every directory `Core\IO::temporaryDir` hands out is deleted by the runtime when its
> script ends, and the program never has to think about it again. The runtime tracks what it handed out
> and sweeps whatever still stands — after the last user code, off the request path, and **without ever
> throwing**: a directory the program already removed is the goal state, a deletion the OS refuses is a
> log line retried by the next sweep. Directories live under a Novis-owned root (`[io] temp_root`), which
> is what makes sweeping safe: at `nvs serve` boot and under `nvs tmp clean` — nowhere else — the runtime
> also removes entries whose owning process is **dead**, keyed on owner liveness and never on age, so a
> hard-killed script's leftovers go too and a long-running process's files are untouchable.
> `[debug] keep_temporary = true` (reloadable) turns deletion into a log line naming each kept path.
> There is no `temporaryFile` and no per-call persist option: a file that must outlive its script is
> storage, not a temporary ([0059](0059-cross-request-state-is-explicit.md)).

## Context

- `temporaryDir` shipped with removal as the program's own job, on the argument that a runtime sweeping
  files it knows nothing about would be deciding their lifetime. That argument dissolves once the
  lifetime is part of the member's contract: a runtime executing a declared "dies with the script" is not
  deciding anything. What the old rule actually bought was PHP's quirk pair — `tempnam` leaks unless you
  remember, `tmpfile` vanishes whether you were done or not — reproduced one half at a time.
- The deletions the sweep performs are work a correct program owed anyway; a program that skips them is
  leaking. So the default case costs nothing new, and it moves off the request path, which manual cleanup
  in the handler never did.
- The failure cases are what a manual rule cannot cover: an uncaught throw skips the cleanup code, a
  hard kill skips everything. [0127](0127-the-end-of-a-script-is-observable.md) § *Context* records the
  same lesson for observing an ending — code that must run "whatever the ending" cannot be every entry
  file's `try/finally`.

## Decision

### 1. The API is one member, unchanged in signature

`Core\IO::temporaryDir(): string` is the whole temporary-file surface. There is no `temporaryFile`: a
program that needs one temporary file needs somewhere to put the second, so the door hands out an owned
directory and the program names files inside it — plain `Core\IO` writes to paths it got from a member it
called. `docs/spec/01-core-library.md` § 14 no longer lists `temporaryFile`.

### 2. Directories live under a Novis-owned root

Every directory is created as `nvs-<pid>-<nonce>` under one root the runtime owns: `[io] temp_root` when
configured, else a `novis` subdirectory of the platform temporary directory, created private on first
use. `temp_root` is a Boot key ([0078](0078-config-reload-and-control-socket.md)); the member's
capability check is unchanged — `fs.write` for the path it creates, asked after the name is chosen.

Exclusive ownership of the root is the entire safety argument for § 4: the runtime never deletes
anything it did not create, because nothing else writes there. Sweeping a shared `/tmp` — anyone's
symlinks, anyone's names — is the classic TOCTOU surface and is exactly what this rule forbids.

### 3. The end-of-script sweep: unconditional, silent about success, loud in the log about failure

The runtime keeps a per-script list of the paths § 1 handed out — O(directories created), request-local,
attributable ([0004](0004-memory-for-simplicity.md)) — and when the script ends, deletes each entry that
still exists. It runs after the last user code: after [0127](0127-the-end-of-a-script-is-observable.md)'s
`onExit` queue on a CLI ending, after [0072](0072-core-task-structured-concurrency.md) § 6's
`afterResponse` work on a request — and off the request path, so a response never waits on a deletion.
It covers every ending the process survives: normal end, `exit`, an uncaught throw, and a request that
died mid-flight, because the worker survives it ([0106](0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md))
and the worker sweeps. A `FATAL` on the CLI kills the process and leaves § 4's case.

The sweep **never throws and never alters a response**. A path that is already gone is the goal state
reached early — a program may delete its own directory whenever it likes, and the sweep does not remark
on it. A deletion the OS refuses (a handle still held — on Windows, routinely an indexer or scanner) is
one runtime log line naming the path and the error, and the directory simply waits in the owned root for
the next sweep to try again. No retry loop, no accumulated error state.

### 4. The orphan sweep: at `nvs serve` boot and under `nvs tmp clean`, keyed on liveness and never on age

A process that was killed outright ran no sweep. Its leftovers are reclaimed by the orphan sweep, which
walks the owned root and deletes each `nvs-<pid>-*` entry **whose pid is not alive**. It runs in exactly
two places: once at `nvs serve` boot, before traffic, and whenever the operator runs `nvs tmp clean`,
which prints each path it removes and supports `--dry-run`. It runs on no other invocation: taxing every
CLI start with a root walk to insure against a rare hard kill prices the common case for the exceptional
one, and the deliberate consequence is that on a machine where `nvs serve` never runs and nobody calls
`tmp clean`, a hard-killed script's directory persists — bounded by crash frequency, confined to one
visible root, one command to clear.

The predicate is owner liveness, **never age**. An age rule is precisely what deletes a long-running
process's files out from under it; liveness cannot, because a live owner's entries are skipped no matter
how old. Every failure mode falls the safe way — a recycled pid makes a dead owner's entry look alive
and it *leaks until a later sweep*, never the reverse — so the sweep may under-delete and can never
over-delete. `tmp clean` has no force flag that overrides liveness; the worst it can do is nothing. Like
§ 3, the orphan sweep never throws: a refused deletion is a log line (for `tmp clean`, a printed line)
and the entry waits.

### 5. `[debug] keep_temporary` keeps everything, visibly, and only the operator can say so

With `[debug] keep_temporary = true`, § 3's sweep logs each path it would have deleted instead of
deleting it — the absence of cleanup is always deliberate and always visible, never a silent leak. The
key is reloadable ([0078](0078-config-reload-and-control-socket.md)), so it can be flipped on a live
server around one problematic request and off again. There is deliberately no in-language setter and no
per-call persist option: a program that can exempt its own files from cleanup is a program that can be
made to hoard them ([0059](0059-cross-request-state-is-explicit.md) — a temporary that outlives its
script is cross-request state, and it is spelled as storage, not smuggled through this API). § 4's
orphan sweep ignores the key's kept entries only while their owner lives; `nvs tmp clean` is how a
debugging session's keepings are cleared.

### 6. The program's own deletions stay loud

`Core\IO::remove` and `removeDir` are unchanged: called by the program, a refusal throws, because a
deliberate action's failure is the program's to hear about. Only the runtime's automatic sweeps are
silent-but-logged. The two doors stay distinct; unifying them in either direction reintroduces either a
throwing sweep or a silent `remove`.

## Consequences

- The trade, under [0004](0004-memory-for-simplicity.md)'s ordering: the tracked list spends a few
  pointers per created directory, per script; the sweeps spend deletions the program owed anyway, moved
  off the request path; disk spends the rare orphan on a serverless machine until `tmp clean`. Bought:
  the entire "who cleans up, and what if we crash" question is gone from every Novis program, and no
  path of any ending leaks a temporary on a serving host.
- The registry card's removal paragraph — "removing it is the program's own job" — is replaced by this
  contract when the sweep lands, in the same slice, per [0117](0117-an-implemented-core-member-documents-itself-in-the-registry.md).
- `create_private_dir`'s callers move from `std::env::temp_dir()` to the § 2 root; the pid-liveness
  check is new host surface with a per-platform edge (pid recycling) the guard tests must pin.

## Alternatives rejected

- **Sweep on the next `temporaryDir` call.** A program that never calls again never sweeps, so expired
  data persists unboundedly — O(scripts run), a leak by [0004](0004-memory-for-simplicity.md)'s own
  test; and it bills request A for request B's deletions on the request path.
- **Wall-clock lifetimes (`delete after 10m`, systemd-tmpfiles age rules).** Deletes files a
  long-running process still holds — the "vanished before the script decided" half of the PHP quirk pair,
  rebuilt on purpose. Age appears nowhere in either sweep's predicate.
- **A per-call `persist:` option.** One forgotten argument ships the leak to production, and it is
  cross-request storage wearing the temp API's name. The operator-only key in § 5 serves the one honest
  use (debugging) with no code change to ship.
- **An orphan sweep on every CLI invocation.** An always-cost on the common case to insure the rare one;
  rejected as § 4 states, with the residue named there.

## Revisiting

If orphan accumulation on serve-less machines turns out to be common in practice — crash-prone CLI fleets
with nobody running `tmp clean` — reconsider a background orphan sweep on CLI starts, priced then, not
assumed now.

## Verification

Guard tests, named in the goal that lands this: the end-of-script sweep removes a standing directory and
says nothing about an already-removed one; a refused deletion logs and does not throw; the orphan sweep
removes a dead owner's entry, skips a live owner's, and `tmp clean --dry-run` deletes nothing;
`keep_temporary` logs each kept path and deletes none; `temporaryDir` creates under the configured
`temp_root` and not the platform default.
