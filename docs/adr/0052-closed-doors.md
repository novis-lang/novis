# ADR 0052 — Four closed doors: no FFI, no stream wrappers, no cross-request state, no `eval`

- **Status:** Accepted
- **Date:** 2026-08-23
- **Scope:** four mechanisms PHP has that Novis will not implement at any tier, each because it contradicts a
  commitment the project has already made rather than because it is unpopular or hard. Ordinary triage
  outcomes — extensions dropped because something better replaces them — stay in
  [ADR 0051](0051-standard-library-tiers.md) § 3.
- **Amends:** [0003](0003-extension-system.md) — its `dlopen` rejection covered *loading a shared library*
  but not FFI, which reaches the same place through a different door; § 1 closes it by name.
  [0008](0008-static-and-global.md) — its storage-class table is exhaustive for *language* constructs; § 3
  extends the same rule to library calls that mutate process-global state.
- **Amended by:** none.

> **In short:** four doors stay shut. **No FFI** — it is `dlopen` with friendlier syntax, and ADR 0003
> already rejected `dlopen` because it destroys the two claims the product rests on. **No stream wrappers
> and no scheme dispatch** — a path argument is a filesystem path, always, which retires `phar://`
> deserialization, `php://filter` chains, remote inclusion and a large part of PHP's SSRF surface in one
> decision. **No cross-request state** — shared memory, SysV IPC, and every userland call that mutates
> process-global configuration are the same violation at different scales. **No `eval`** — it breaks the
> static `require` graph, the artifact cache, definite-assignment analysis and taint tracking
> simultaneously, and `spawn script` already covers dynamic code, isolated and budgeted. None of the four
> has an opt-in, an ini flag, or a "trusted mode."

## Context

- The [ADR 0051](0051-standard-library-tiers.md) placement procedure asks *which tier* a capability belongs
  at. These four have no correct tier: each is unsafe in-process, unavailable in a sandbox, and unsafe as
  a build-time option, so answering "which tier" for them is a category error.
- Each has a genuine constituency in PHP, and none is being closed because it is unused. FFI is how modern
  PHP wraps libraries without writing C. Stream wrappers are how `file_get_contents` reads a URL. APCu is
  in nearly every production deployment. `eval` underpins a generation of template engines. The argument in
  every case is that the mechanism trades away something Novis has ranked above it.
- Writing them down matters more than it looks. Three of the four are currently closed only by not having
  been built, which is not a decision — it is an absence, and absences get filled by whoever needs them
  next.

## Decision

### 1. No FFI, and no native module of any kind

There is no `FFI`-equivalent, no `com_dotnet`-equivalent, and no mechanism by which userland code causes
native code to be loaded into the process.

[ADR 0003](0003-extension-system.md) rejected `dlopen` on two grounds: it destroys memory safety, because
one bad write corrupts arbitrary memory, and it destroys request isolation, because one segfault takes down
every in-flight request in Novis's single process. FFI reaches both outcomes from userland instead of from an
extension author, which makes it strictly worse: the code doing the unsafe pointer arithmetic is now
written by an application developer under deadline rather than by someone who chose to write an extension.

Tier 1 exists so this is unnecessary. Wrapping an existing C or Rust library is the *stated purpose* of the
wasm component tier, and it delivers the same capability with a memory boundary, a capability grant, a CPU
deadline and a memory cap. "Compile the library to wasm" is the answer, and
[ADR 0051](0051-standard-library-tiers.md) § 4 already makes it the standing one.

### 2. A path is a filesystem path: no stream wrappers, no scheme dispatch

No `Core` function that takes a path interprets a scheme prefix, and there is no registry by which userland
or an extension adds one. `Core\IO::read("php://filter/...")` looks for a file with that name and does not
find it.

PHP's decision to make every filesystem function accept a URL, and to let userland register new schemes, is
the root of an entire vulnerability taxonomy: `phar://` metadata triggering unserialization on any file
operation that touches such a path — one of the most reliable RCE primitives in PHP's history — `php://filter`
chains that convert an arbitrary file read into arbitrary code execution, and `data://`/`http://` turning
every local file-inclusion bug into a remote one.

The mechanism's actual benefit is polymorphism over "things you can read bytes from." That is available
without any of the above, as an ordinary interface implemented by ordinary types and resolved statically —
which is what `rule:iteration/two-interfaces`'s `Iterable` and the `Core\IO` stream types
provide. What is refused is specifically **dispatch on the textual content of a path**.

### 3. No cross-request state, at any scale

Nothing a request does is observable by another request except through an explicit, capability-gated store.
This closes two things that look unrelated and are the same violation:

- **Shared memory and SysV IPC** — `shmop`, `sysvshm`, `sysvsem`, `sysvmsg`, and APCu's cross-process
  segment. *Strict shared-nothing requests* is a priority-1 commitment and load-bearing for the whole
  design; a shared segment reintroduces exactly the channel [ADR 0006](0006-isolated-script-execution.md)
  exists to eliminate. There is no safe-if-careful version, because the entire isolation argument is that
  carefulness is not a mechanism.
- **Userland calls that mutate process-global configuration** — `putenv`, `setlocale`, `bcscale`,
  `mb_internal_encoding`, `date_default_timezone_set`. Each is ambient mutable state read by later,
  unrelated code, which is the shape [ADR 0008](0008-static-and-global.md) already removed from the
  language; several are also outright unsound in a multithreaded process, since the C library state they
  touch is not thread-local. The environment is read-only after startup, and locale, scale and timezone are
  always explicit arguments.

This does not close operator configuration. [ADR 0005](0005-config-changeability.md)'s `nvs.toml` and
`Core\Config::set` are governed, per-request, and cannot widen an operator's ceiling; that is a different mechanism
with a different threat model. The rule is about **userland calls whose effect outlives or escapes the
caller's own request**.

The replacement is [ADR 0059](0059-cross-request-state-is-explicit.md)'s `Core\Cache` — per-core in
process, or a real store where sharing must be genuine — where the sharing is explicit, bounded, and
visible in the capability grants.

### 4. No `eval`

There is no `eval`, no string-argument `assert`, and no `Core` function that compiles and runs a string
produced at runtime.

Four separate mechanisms depend on the set of code in a program being known before it runs. The static
`require` graph ([ADR 0021](0021-single-file-inclusion-construct.md)) is what
[ADR 0048](0048-portable-single-file-executables.md) § 3 bundles; the artifact cache
([ADR 0042](0042-on-disk-artifact-cache-format.md)) is keyed on unit content; definite assignment
([ADR 0022](0022-definite-property-initialization.md)) and taint tracking
([ADR 0024](0024-taint-tracking-for-injection-sinks.md)) are whole-program compile-time analyses. `eval`
does not weaken these one at a time — it makes all four unsound at once, and the escape hatches that would
be needed to keep them (a "no eval reached this file" analysis) are exactly as hard as not having it.

The two legitimate uses have separate, better answers already decided.
[ADR 0019](0019-reflection-and-ast-parsing-are-core-features.md) gives `Core\Ast::parse()` for inspecting
code as data, and states from its own side that a parsed AST has no path back into execution; this section
is that rule stated from the other side. [ADR 0006](0006-isolated-script-execution.md)'s `spawn script`
runs code chosen at runtime, in an isolate, spending the parent's budget — which is what a template engine
or a plugin loader actually needs, with a boundary that `eval` never had.

## Consequences

- **`nvs convert` cannot mechanically translate four PHP patterns**, and must diagnose rather than guess:
  an `FFI::cdef` call, a path with a scheme prefix, an APCu or `shm_*` call, and `eval`. Each diagnostic
  names the replacement — a `.nvsx`, a plain path, `Core\Cache`, `spawn script` — so migration is guided
  rather than blocked. This is a real migration cost and the ADR does not minimize it: an application built
  on APCu will need its caching layer reconsidered, not rewritten mechanically.
- **A class of library cannot be ported at all** — anything whose whole purpose is process-global state or
  in-process native binding. That is the intended outcome.
- **Some capability is genuinely lost.** Cross-request coordination at shared-memory latency is not
  available at any price; the floor is `Core\Cache`'s per-core store or a network round trip. § 3 spends
  latency (priority 3) to buy isolation (priority 1), which is the ordering `rule:programs/memory-priority` mandates, and it is
  worth naming as a real trade rather than a free win.
- **Nothing here is revisitable by configuration.** There is deliberately no ini directive for any of the
  four, because an option that is off by default is still a mechanism that exists, and every safety claim
  downstream would have to be qualified with "unless it is enabled."

## Alternatives rejected

- **FFI behind a capability grant.** Symmetrical with every other dangerous capability, and it would let an
  operator opt in knowingly. Rejected: the capability system's guarantees are enforced *by* memory safety,
  so a capability that removes memory safety cannot be enforced by it. Every other grant restricts what
  code may do; this one would remove the mechanism that makes grants meaningful.
- **Stream wrappers restricted to a built-in allow-list** (`file://` only, no registration). This is close
  to harmless, and would ease migration. Rejected on priority 4: it keeps a scheme-parsing step in every
  path-handling function forever, to support exactly one scheme that is also the default — surface with no
  corresponding capability.
- **`eval` restricted to code with no `require` and no closures**, so the static analyses survive. Rejected:
  the restriction is hard to state precisely and harder to enforce, and what remains is strictly less
  capable than `spawn script`, which already exists and is isolated.
- **A read-only shared segment** for caches, with writes going through a coordinator. Rejected: read-only
  is not the property that matters. Any shared mapping means one request's failure can leave another
  request's view inconsistent, and the value it holds still crosses the boundary
  [ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md) governs.

## Verification

- **M8:** a fixture per closure. A path with a scheme prefix resolves as a literal filename and fails to
  open; there is no symbol named `eval`, `FFI`, `shm_get_var` or `putenv` resolvable under `Core` or at
  file scope, and referencing one is an ordinary unresolved-name diagnostic that additionally names this
  ADR.
- **M9:** the adversarial extension suite already required by ADR 0003 covers § 1 from the other side — an
  extension cannot reach native code the host did not import for it.
- **M11:** `nvs convert` has a fixture for each of the four PHP patterns, asserting the diagnostic names the
  replacement rather than emitting a partial translation.
