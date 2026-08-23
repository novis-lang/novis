# ADR 0051 — The standard library's tiers: what is `Core`, what ships native, what is an extension

- **Status:** Accepted
- **Date:** 2026-08-23
- **Scope:** which of PHP's bundled extensions MWL implements, and at which of
  [ADR 0003](0003-extension-system.md)'s tiers; the decision procedure applied to any future stdlib
  candidate. Not in scope: the API of any individual `Core` class, which each stdlib milestone designs.
- **Amends:** [0003](0003-extension-system.md) — the three tiers were defined by *what each is*, never by
  *how to choose between them*; § 2 below is the missing procedure. Its "fine-grained primitives" rule for
  Tier 0 survives unchanged as test 4. [0011](0011-functions-and-constants-are-class-members.md) — the
  domain-class roster is extended, and § 5 adds a rule about what may claim the `Core` prefix at all.
- **Amended by:** [0063](0063-core-api-conventions.md) — § 3's Core roster gains `Core\Path`, `Core\Out`,
  `Core\Bytes` and `Core\Error`, each split out of an entry already listed here; the six placement tests and
  every tier assignment are unchanged. That ADR fixes the *shape* of the members this one places.
- **Relates to:** [0004](0004-memory-for-simplicity.md) (the priority ordering this ADR applies; in
  particular that footprint is spent last), [0052](0052-closed-doors.md) (what is not implemented at any
  tier, and why each closure is structural), [0055](0055-extension-qualifier-declarations.md) (how an
  extension participates in the qualifier system, which is what makes Tier 1 usable for anything a
  request's data touches), [0024](0024-taint-tracking-for-injection-sinks.md) and
  [0033](0033-secret-qualifier-for-confidential-values.md) (test 2's sinks and launderers).

> **In short:** MWL does not inherit PHP's extension partition. That partition tracks 1997 C build
> engineering — separate `.so` files, `dl()`, ini load order, per-module globals — not any property worth
> preserving, which is why `ctype` is an extension and `str_pad` is not. A candidate is placed instead by
> **six ordered tests** into one of five outcomes: **Core** (Tier 0, always present), **Native** (Tier 2,
> statically linked in the default distribution, capability-gated and feature-flaggable), **Ext** (Tier 1,
> a sandboxed `.mwlx`), **Dropped**, or **already answered by MWL's architecture**. The costs that decide
> this are **API surface** (priority 4) and the **unsandboxed dependency set** (priority 1) — not binary
> size, and emphatically not runtime memory, which for an uncalled `Core` class is zero.

## Context

- PHP's extension list is an accident of history the language now has to keep. It ships two MySQL APIs,
  four XML APIs, SHA-256 in both `hash` and `openssl`, and `iconv` alongside `mbstring`. Adopting its
  partition would import every one of those duplications along with the boundary.
- **What "core bloat" costs in MWL is not what it costs in PHP.** A `Core\Zip` nobody calls is dead code in
  the binary, not a per-process module-globals struct instantiated at every startup. Per
  [ADR 0004](0004-memory-for-simplicity.md) footprint is the last thing protected anyway, so "it makes the
  runtime fatter" is close to a non-argument. Binary size is real but cheap: Cargo features cover it, and
  Tier 2 already assumes the operator may build from source.
- The two costs that *are* expensive are the ones the priority ordering ranks highest and fourth-highest:
  every `Core` class is a permanent public API developers must learn and we must keep compatible, and every
  crate in the default binary is unsandboxed code holding our process's authority. "Don't bloat the core"
  is therefore read as **do not grow the API surface, and do not grow the unsandboxed dependency set**.
- Two properties of ADR 0003's own design decide most placements, and both are easy to state wrongly:
  - **A Tier 1 guest is instantiated fresh per request**, and M9's verification explicitly requires that an
    extension storing state in a global cannot observe it on the next request. Anything whose defining
    feature is state outliving a request — above all a **connection pool** — therefore cannot be an
    extension. Having the host own the pool and hand the guest a handle means writing the client natively
    with extra steps.
  - **Coroutine suspension is *not* the constraint it appears to be.** A host import can suspend the
    calling coroutine, so an I/O-bearing extension is possible in principle. Database and cache clients are
    native because of connection lifetime, not because of blocking.

## Decision

### 1. Five placements

| Placement | Meaning |
|---|---|
| **Core** | Tier 0. Compiled into every `mwl` binary, reachable under the `Core` namespace, no build flag. |
| **Native** | Tier 2. Statically linked subsystem in the default distribution; gated at runtime by an [ADR 0005](0005-config-changeability.md) capability, removable at build time by a Cargo feature. |
| **Ext** | Tier 1. A sandboxed `.mwlx` wasm component, first-party or third-party. |
| **Dropped** | Not implemented at any tier. Structural closures live in [ADR 0052](0052-closed-doors.md); the rest are named in § 3 with their replacement. |
| **Answered** | The problem is removed by MWL's architecture; there is nothing to port. |

### 2. The six tests, applied in order

1. **Does it need runtime privilege?** Direct heap access, the request lifecycle, the compiler's own
   tables, or state that outlives a request (a connection pool). If yes it cannot be Tier 1 — a sandbox
   boundary is exactly what it needs to cross.
2. **Is it an injection sink or a launderer?** SQL text, HTML output, headers, filesystem paths, argv, logs
   ([ADR 0024](0024-taint-tracking-for-injection-sinks.md),
   [0033](0033-secret-qualifier-for-confidential-values.md)). A launderer is **always** Core: ADR 0024 § 3
   is explicit that only a `Core` function whose contract names one sink may remove a qualifier.
3. **Does it wait on the outside world?** Sockets, files, child processes, timers. Forces a capability
   grant, and — together with test 1 — usually forces Native.
4. **Is per-call cost near call overhead?** ADR 0003's own Tier 0 rule. `Core\Str::len` cannot pay a
   boundary crossing; a 1 µs formatting call comfortably can.
5. **Does it parse hostile bytes?** Image codecs, archive readers, metadata and document parsers. This is
   where PHP's CVE history lives, and it is Tier 1's *headline* case rather than its consolation prize: the
   sandbox is worth its cost precisely where the input is attacker-controlled.
6. **Would two of these exist?** One spelling per job — the rule ADRs [0021](0021-single-file-inclusion-construct.md),
   [0034](0034-legacy-cast-syntax-rejected.md), [0045](0045-and-or-xor-keyword-operators-rejected.md),
   [0049](0049-single-open-tag-and-single-exit-keyword.md) and [0050](0050-list-destructuring-spelling-rejected.md)
   already apply to syntax, applied to the library.

### 3. The roster

**Core.** `Core\Str`, `Core\Arr`, `Core\Math`, `Core\IO` — absorbing `standard`, SPL's data structures,
`ctype`, and, because [ADR 0009](0009-string-and-bytes.md) guarantees `string` is UTF-8, nearly all of
`mbstring`. `Core\Time` (`date`, `calendar`), immutable only. `Core\Regex`
([ADR 0056](0056-regex-engine-policy.md)). `Core\Json`. `Core\Hash`, absorbing `openssl`'s digest half.
`Core\Random`, secure by default. `Core\Reflect` and `Core\Ast`
([ADR 0019](0019-reflection-and-ast-parsing-are-core-features.md)). `Core\Session`. `Core\Encoding`
(`iconv`, over `encoding_rs`, sited at the `bytes`/`string` boundary where conversion is naturally
failable). `Core\Xml`, one API replacing six extensions, and `Core\Html`, whose escaper and sanitizer are
ADR 0024 launderers. `Core\Uri` (PHP 8.5's `uri`). `Core\Mime` (`fileinfo`, by magic bytes rather than
libmagic's rule interpreter). `Core\Compress` (`zlib`, plus brotli and zstd, because all three are HTTP
`Content-Encoding` values the built-in server needs on the request path). `Core\Zip`. `Core\Decimal` and
`Core\BigInt` ([ADR 0054](0054-decimal-scalar-type.md)). `Core\Os` (`posix`, minus fork). `Core\Cli`
(`readline`). `Core\Uuid`. `Core\Cache` ([ADR 0059](0059-cross-request-state-is-explicit.md)). `Core\Csv`.
`Core\Test`. Plus `Core\Fatal`/`Core\Log` ([ADR 0020](0020-error-escalation-ladder.md)) and
`Core\Attributes` ([ADR 0046](0046-attributes-shape-literal-metadata.md)), already scheduled for M8.

`Core\Zip` is Core rather than Ext despite passing test 5, because its real defects are not the
memory-safety bugs a sandbox contains: `../` and absolute-path entries, symlink entries and decompression
bombs are *policy*, and the policy must be non-optional. A sandboxed decoder gets the memory cap for free
and the traversal rules not at all.

**Native.** `Core\Db` (`pdo` and its drivers) and the Redis backend behind `Core\Cache`, both by test 1 —
connection lifetime. `Core\Crypto`: `openssl`'s primitive half merged with `sodium`, AEAD-only, with no
ECB, no unauthenticated CBC and no cipher-name-as-string; TLS via `rustls`. `Core\Http\Client` (`curl`) and
`Core\Net` (`sockets`, `stream_*`), over the runtime's own reactor rather than a second event loop.
`Core\Mail`, an SMTP client with structured headers, replacing `mail()`. `Core\Process`
([ADR 0044](0044-core-process-argv-only-no-shell.md)). The Redis driver ships behind a Cargo feature that
defaults **on**, while its `net.connect` capability stays deny-by-default: distribution and authority are
separate decisions, and only the second is security-relevant.

**Ext.** Internationalization — a first-party component carrying its CLDR data in its own wasm data
section, under its own namespace (§ 5), with a **batch-shaped API**: collation exposes sort-key generation
and whole-array sort, because sorting 10,000 strings through a per-comparison boundary would be roughly
130,000 crossings. Locale-independent Unicode algorithms — case mapping, NFC/NFD normalization, grapheme
segmentation — stay in `Core\Str`. Also Ext: `gd`, `imagick` and `exif` (test 5, the headline case); `bz2`,
`xsl`, `tidy`, `enchant`, `soap`, `ftp`, `ldap`, `snmp`, `dba`; the alternative serialization formats
(`yaml`, `msgpack`, `cbor`, `igbinary`, `protobuf`); and the remaining brokers and clients (`amqp`,
`kafka`, `mongodb`, `memcached`, `ssh2`). Vendor-C database drivers (`oci8`, `odbc`, `pdo_dblib`,
`pdo_firebird`) are Tier 2 built from source by the operator who needs them, never in a default binary.

**Dropped, with a replacement.** The procedural `mysqli`/`pgsql`/`sqlite3` APIs, by test 6 — one database
API. `filter`: its `filter_input` half dies with [ADR 0012](0012-no-superglobals.md)'s superglobals, and its
*sanitizing* filters are half-escaping that produces the false confidence ADR 0024 exists to prevent, so
only the genuine validators survive, as `Core\Validate`. `gettext` and `setlocale`, because they mutate
**process-global** C state, which is unsound in a thread-per-core runtime and would leak across requests —
translation is ICU MessageFormat with locale as an explicit argument, and **MWL has no ambient locale at
all**. `pcntl`, whose `fork()` is a correctness hazard in a threaded process and whose use cases are already
served by `spawn worker` ([ADR 0006](0006-isolated-script-execution.md)) and coroutines, leaving only a
narrow `Core\Signal` for graceful shutdown. `imap`, which PHP itself demoted in 8.4. `phar`, replaced by
[ADR 0048](0048-portable-single-file-executables.md) and closed off in ADR 0052 along with the stream
wrapper it rides on. `calendar`. `gmp` as such, replaced by `Core\BigInt` over `num-bigint` rather than the
C, LGPL GMP.

**Answered by the architecture.** `opcache` ([ADR 0042](0042-on-disk-artifact-cache-format.md),
[0017](0017-hot-reload-without-restart.md)); `xdebug` ([ADR 0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md),
[0041](0041-timeline-export-and-gc-spawn-trace-events.md), [0016](0016-ide-integration.md)); `swoole`,
`parallel`, `event` and `pthreads`, since the runtime *is* this; `apcu`
([ADR 0059](0059-cross-request-state-is-explicit.md)); `mysqlnd` and PHP's own test extensions.

### 4. C dependencies: two questions, not a case-by-case argument

AGENTS.md's "pure-Rust by default, deviations argued individually" is narrowed to a standing test, so the
answer does not depend on who argues it:

1. **Does attacker-controlled data reach this code?** If no — accept it under ordinary audit.
2. If yes — accept it only with a **demonstrable, exceptional verification record**. Otherwise it must be
   confined to wasm.

SQLite passes (2): its test suite is orders of magnitude larger than its source and it is continuously
fuzzed. Almost nothing else clears that bar, which is the point. A codec, an archive reader or an XSLT
engine fails it and is therefore Tier 1 — including when the only implementation is C, since compiling a C
library to wasm is the standing answer when the test fails.

### 5. `Core` means *always present*

Nothing outside Tier 0 may register a class under the `Core` namespace. A `use Core\Intl;` that compiles in
development and fails in production would make ADR 0011's reserved namespace conditional, which is a worse
failure than an unfamiliar namespace. A first-party extension is named like any other extension, and its
tier is therefore visible at the use site.

## Consequences

- **The stdlib is smaller than PHP's and covers more.** Six XML extensions become one class; `mbstring` and
  `iconv` collapse into `Core\Str` plus a boundary conversion; two database APIs become one. Against that,
  Core gains things PHP leaves to userland precisely because getting them wrong is a security bug — an HTTP
  client, SMTP, cache, CSV, UUID, a test surface, and [ADR 0060](0060-application-security-protocols.md)'s
  protocol roster.
- **Internationalization stops being coupled to runtime releases.** CLDR ships roughly twice a year, and
  PHP's ICU version is pinned to whatever the distribution built against — a chronic operational complaint.
  A new `.mwlx` replaces it.
- **Tier 1 gains two flagship first-party users**, image decoding and intl, which is what M9's verification
  should demonstrate. An image codec makes the security claim legible in a way a compression benchmark
  does not.
- **A cost this ADR accepts:** the default binary's unsandboxed dependency set is larger than a minimal
  runtime's, because `Core\Db`, `Core\Http\Client`, `Core\Crypto` and the Redis driver are all in it. Each
  is audited under [deny.toml](../../deny.toml), each is removable by a Cargo feature, and each is there
  because test 1 or test 2 put it there, not for convenience.
- **A cost this ADR does not hide:** placing intl at Tier 1 means locale-correct formatting requires
  installing something. That is the deliberate trade for keeping multi-megabyte CLDR data out of every CLI
  binary and every ADR 0048 single-file executable.

## Alternatives rejected

- **Mirror PHP's extension list one-for-one.** Maximum familiarity and the simplest `mwl convert` story.
  Rejected: it imports every duplication named in *Context*, and the partition encodes C build constraints
  MWL does not have.
- **Everything in Core; no tiers for first-party code.** Simplest to document, and every program has every
  feature. Rejected on priority 1: it would put image and archive decoders — the exact code that
  historically turns an upload into arbitrary code execution — in-process with every in-flight request, and
  it makes the unsandboxed dependency set unbounded.
- **A minimal core with everything else an extension**, in the `pip` or `npm` shape. Rejected on priorities
  1 and 4 together: a launderer or an injection sink cannot be third-party (ADR 0024 § 3), and leaving
  HTTP, JSON, crypto and dates to the ecosystem produces five incompatible implementations of each, which
  is the state of affairs PHP's own userland demonstrates.
- **Keep "deviations argued individually" for C dependencies.** Maximum flexibility. Rejected: a growing
  list of individually reasoned exceptions is precisely how a pure-Rust default stops being one.

## Verification

- **M8:** the § 3 roster is what the milestone builds, and each `Core` class named there has a conformance
  suite. A test asserts that no class outside Tier 0 registers a name beginning `Core\` (§ 5), and that
  attempting it is a load-time diagnostic naming this ADR.
- **M8:** a CI check enumerates the default binary's C dependencies and fails on any addition not recorded
  against § 4's two questions, so the test is enforced rather than remembered.
- **M9:** the extension end-to-end verification uses the image codec, and the benchmark ADR 0003 currently
  asserts rather than measures is committed for it. The intl component is built in the same run to prove
  the embedded-data and batch-API shapes hold — specifically, that sorting 10,000 strings costs one
  boundary crossing rather than one per comparison.
