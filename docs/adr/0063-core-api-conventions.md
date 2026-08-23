# ADR 0063 — `Core` API conventions: one shape for every built-in

- **Status:** Accepted
- **Date:** 2026-08-23
- **Scope:** the *shape* of every member of every `Core` class — argument order, optional arguments,
  failure signalling, naming, mutation, callbacks — and the standing rules that decide which of PHP's
  ~1,900 built-ins survive at all. Not in scope: which tier a subsystem lands at, which is
  [ADR 0051](0051-standard-library-tiers.md); and not the member list itself, which is
  [docs/spec/01-core-library.md](../spec/01-core-library.md).
- **Amends:** [0007](0007-explicit-type-system.md) § 5 — the illustrative stdlib signature
  `array_map(callable, array<T>)` becomes subject-first, per § 1 R1. [0011](0011-functions-and-constants-are-class-members.md)
  — its `Core\Str::len` / `Core\Arr::map($f, $a)` examples become `Core\Str::length` / `Core\Arr::map($a, $f)`,
  and § 2's "the exact roster is stdlib design" is answered by the spec file this ADR names.
  [0027](0027-callable-is-closures-only.md) and [0031](0031-callable-is-the-only-closure-type.md) — the same
  `Core\Arr::map(callable, array<T>)` example, same flip.
  [0057](0057-intrinsic-literal-folding.md) — its `Core\Time::format` intrinsic is the instance method
  `DateTime::format`, per R18; the folded argument is unchanged and so is everything else about it.
  [0051](0051-standard-library-tiers.md) § 3 — the Core roster gains `Core\Path`, `Core\Out`, `Core\Bytes`
  and `Core\Error`, all split out of entries it already lists.
- **Amended by:** none.
- **Relates to:** 0004, 0024, 0033, 0036, 0047, 0053

> **In short:** PHP's built-ins have no API. Argument order flips between neighbouring functions
> (`array_map(f, a)` / `array_filter(a, f)`), failure is signalled four different ways in the same
> extension, options are `int` bitmasks, and entire subsystems ship twice — once procedural, once
> object-oriented. MWL fixes the shape *before* writing the library, with **twenty rules** in § 1–§ 2. The
> load-bearing ones: **the subject is always parameter 1**; **optional arguments are one trailing shape
> literal**, never flags; **nothing mutates and nothing takes a reference**; **failure throws, absence is
> `?T`, and `false` is never a return value**; and **no operation is reachable two ways** — no procedural
> twin of a class, no class wrapper on a static, no mutable/immutable pair, no `from`/`tryFrom` pair, and
> no methods on scalars or `array<T>`. Applying these plus [ADR 0051](0051-standard-library-tiers.md)'s six
> tests takes PHP's ~1,900 functions to roughly **450 members across ~30 domain classes**, covering strictly
> more ground.

## Context

- [ADR 0011](0011-functions-and-constants-are-class-members.md) decided *where* built-ins live (`Core`
  domain classes) and [ADR 0051](0051-standard-library-tiers.md) decided *which* subsystems exist and at
  what tier. Neither says anything about what a member looks like, and both explicitly defer it. Writing
  ~450 signatures without that rule set first is how PHP's library became what it is: every function was
  locally reasonable and the whole is unlearnable.
- The cost being controlled is the one [ADR 0051](0051-standard-library-tiers.md) names as expensive:
  **API surface** (priority 4). A consistent surface is smaller than an inconsistent one of the same size,
  because a developer who has learned ten members can predict the eleventh.
- PHP duplicates along **two axes**, and only one of them is obvious:
  1. **Function families** — 11 sort functions, 12 `strpos` variants, 12 `array_diff`/`array_intersect`
     variants, 9 `printf` variants. These are visible in the function list.
  2. **Whole subsystems shipped twice**, procedural and OOP — every `date_*` function is an alias of a
     `DateTime` method, every `intl` class has a procedural twin, `mysqli` exists entirely twice, files are
     reachable through `fopen`, `SplFileObject` *and* `DirectoryIterator`. These are invisible in a function
     list because half of each pair is a class.
  Only auditing both axes produces a library with one way to do things.
- Three properties of MWL make most of PHP's conventions not merely ugly but unavailable:
  **arrays are copy-on-write values** (so a by-reference mutator has no performance argument left),
  **types are declared and checked** (so `false`-on-failure and `int` flag masks throw away information
  the compiler already has), and **there is no ambient state** — no locale, no default timezone, no
  internal array pointer, no error globals — because a thread-per-core runtime cannot have any.

## Decision

### 1. The shape rules

Every `Core` member obeys all twenty. A proposed member that cannot is a design bug, not an exception.

| # | Rule | Reason |
|---|---|---|
| **R1** | **The subject is parameter 1**, always — including for callback-taking and needle-taking members. `Arr::map($array, $fn)`, `Str::replace($subject, $search, $replacement)`, `Arr::contains($haystack, $needle)`. | The single most-cited PHP complaint; a rule with no exceptions is learnable in one sentence. |
| **R2** | Then required arguments in dataflow order, then **at most one trailing optional shape literal** ([ADR 0036](0036-anonymous-object-shapes.md)) declared as a `type` alias. No `bool` flag parameters, no `int` bitmasks, no positional optional tails longer than one. | Named, order-free, structurally checked, and a compile-time-constant bag folds to a constant. |
| **R3** | **Nothing mutates and nothing takes a reference.** No `&$out`, no out-parameters, no in-place variants. The result is the return value. | COW makes it free: a refcount-1 argument is mutated in place by the implementation, exactly as PHP's own `sort()` does after a copy-on-write check. |
| **R4** | **Failure throws; absence is `?T`.** `false` is never returned to signal failure, no member returns an error code, and there are no error globals (`json_last_error`, `error_get_last`). A `?T` return means the absence is an ordinary, expected outcome. | `strpos()` returning `0\|false` is PHP's most productive bug source; unions make it unnecessary. |
| **R5** | A **fixed verb lexicon**: `is…`/`has…`/`contains`/`startsWith` → `bool`; `find…` → `?T`; `indexOf`/`keyOf` → `?uint`/`?K`; `count…` → `uint`; `to…`/`from…` for conversion and static construction. `try…`, `…OrNull`, `…Safe`, `…Ex` are **banned** — R4 already covers them. | One verb per meaning, so a name predicts a return type. |
| **R6** | **Symmetric operations get symmetric names**: `encode`/`decode`, `split`/`join`, `pack`/`unpack`, `escape`/`unescape`, `trimStart`/`trimEnd`, `startsWith`/`endsWith`, `indexOf`/`lastIndexOf`, `first`/`last`. If one half exists, the other's spelling is decided by rule. | |
| **R7** | **Members are full words.** Class names may abbreviate from a closed list (`Str`, `Arr`, `Fs`, `Io`, `Uri`, `Db`, `Id`); members may not, except the conventional mathematical spellings `abs`, `min`, `max`, `sqrt`. So `Core\Str::length`, not `::len`. | Removes a per-name judgement call that PHP made differently every time. |
| **R8** | **One range convention**, shared by `Core\Str` and `Core\Arr`: `(offset, ?length)`, negative offset counts from the end, negative length stops that many from the end, `null` length runs to the end. | Stated once, never varies. |
| **R9** | **Callbacks always receive `($value, $key)`, in that order**, and a closure may declare fewer parameters than the call site passes. | Kills the whole `ARRAY_FILTER_USE_KEY`/`USE_BOTH` flag family and the need for `map`/`mapWithKey` pairs. |
| **R10** | **Haystack before needle, subject before pattern.** A corollary of R1, stated because PHP violates it in `in_array`, `str_replace` and `preg_match` simultaneously. | |
| **R11** | **No mode strings.** No `fopen($p, "r+b")`, no `hash("sha256", …)`, no `MB_CASE_TITLE`. Enums ([ADR 0010](0010-enums-are-a-value-type.md)), always. | Typo-proof, completable, and [ADR 0047](0047-literal-and-enum-case-types.md) lets a parameter accept a closed subset of cases. |
| **R12** | **Units are types.** Durations are a `Duration`, never "seconds here, microseconds there". Byte sizes are `uint` bytes. | PHP's `sleep`/`usleep`/`time_nanosleep` split is a units bug waiting to happen. |
| **R13** | **A `string` member never takes an encoding argument.** UTF-8 is the type's guarantee ([ADR 0009](0009-string-and-bytes.md)); all conversion happens at the `bytes`↔`string` boundary in `Core\Encoding`, where it can fail honestly. | This is what removes the entire `mb_*` twin set. |
| **R14** | **Anything with a lifetime is an object.** No `resource`, no integer handles, no `$link`-first convention. `Core` never exposes the `resource` atom at all; it survives in [ADR 0007](0007-explicit-type-system.md) § 3's grammar only for extension-supplied opaque handles ([ADR 0003](0003-extension-system.md)). | A handle has nowhere to enforce a capability and no methods; an object has both. |
| **R15** | **One name, one signature.** MWL has no overloading; optional arguments are the only variance. Two behaviours need two names, and R5/R6 decide them. | |
| **R16** | **A domain class is a singular noun**, and its object types nest under it: `Core\Time`, `Core\Time\Instant`, `Core\Time\Duration`. Never `Core\Times`, never `Core\TimeUtils`, never `Core\TimeHelper`. | |

### 2. One paradigm per operation

These four are what close PHP's second duplication axis. They are separated from § 1 because they constrain
the *library's structure*, not an individual signature.

| # | Rule |
|---|---|
| **R17** | **One paradigm per operation.** A stateless operation is a static method on a domain class; anything with identity or lifetime is an object. **Nothing is reachable both ways** — no procedural twin of a class API, and no class wrapper around a static one. |
| **R18** | **A domain class's static members either operate on a scalar/array, or construct an object. They never mirror an object's own methods.** `Time::format($instant, $fmt)` may not exist beside `$instant->format($fmt)`. |
| **R19** | **Scalars and `array<T>` never gain methods.** There is no `$s->length()` and no `$a->map()`; `Core\Str` and `Core\Arr` are the one spelling. This is what stops the twin problem regrowing after M4S. |
| **R20** | **No mutable/immutable twin types.** Every `Core` value type is immutable — there is no `DateTime`/`DateTimeImmutable` pair to choose between. |

The twin sets these rules retire, with their replacements:

| PHP's duplication | Replacement |
|---|---|
| ~35 `date_*` procedural aliases of `DateTime`; `DateTime` vs `DateTimeImmutable` | `Core\Time`, objects only, immutable only (both axes) |
| every `intl` class's procedural twin (`collator_*`, `numfmt_*`, `datefmt_*`, `msgfmt_*`, `normalizer_*`, …) | the intl extension, objects only ([ADR 0051](0051-standard-library-tiers.md) § 3) |
| `Reflection*` classes **and** `get_class`, `get_object_vars`, `get_class_methods`, `method_exists`, `property_exists`, `class_exists`, `is_a`, `is_subclass_of`, `class_implements`, `class_parents`, `spl_object_id` | `Core\Reflect` only. The `instanceof` **operator** stays — it is syntax, not a second API |
| `fopen`-family, `SplFileObject`/`SplFileInfo`, `DirectoryIterator`/`FilesystemIterator` | `Core\IO\File` and `Core\IO\Dir`; no `SplFileInfo`-shaped twin of `Core\Path` |
| `socket_*`, `stream_socket_*`, `fsockopen` | `Core\Net` |
| `gzopen` handles, `deflate_init` contexts, `zlib.*` stream filters | `Core\Compress` |
| `hash()` one-shot, `HashContext` incremental, `openssl_digest` | `Core\Hash::of()` plus a `Core\Hash\Stream` object |
| `rand`/`mt_rand`/`random_int` **and** `Random\Randomizer`/`Random\Engine` | `Core\Random` |
| DOM, SimpleXML, XMLReader, XMLWriter, `xml_parser_*`, XSLTProcessor | `Core\Xml`. Its tree API and its streaming reader/writer are **different jobs, not twins**, and the spec says so explicitly so it cannot be read as an exception to R17 |
| ~20 SPL iterator classes, `iterator_to_array`/`iterator_count`/`iterator_apply` | dropped ([ADR 0053](0053-iteration-and-generators.md)) |
| `Enum::from()` throwing beside `Enum::tryFrom()` returning null; `json_decode` with and without `JSON_THROW_ON_ERROR` | one member each, its behaviour fixed by R4 |

### 3. What is removed, and why

Four standing reasons. The member-by-member list is in
[docs/spec/01-core-library.md](../spec/01-core-library.md); this is the rule that generated it.

1. **Pure aliases** — `sizeof`, `join`, `chop`, `key_exists`, `pos`, `fputs`, `is_integer`, `is_long`,
   `is_double`, `is_real`, `doubleval`, `ini_alter`. [ADR 0051](0051-standard-library-tiers.md) test 6.
2. **Dead or dying in PHP itself** — `ereg*`, `mysql_*`, `mcrypt`, `create_function`, `each`,
   `money_format`, `get_magic_quotes_*`, `utf8_encode`/`utf8_decode`, `strptime`, `strftime`,
   `date_sunrise`/`date_sunset`, `convert_cyr_string`, `hebrev`, `get_browser`, `highlight_*`.
3. **Already closed by a decision** — a table of ~120 functions whose removal follows from
   [0007](0007-explicit-type-system.md)/[0008](0008-static-and-global.md) (`extract`, `compact`,
   `get_defined_vars`, `parse_str`, `$$var`), [0027](0027-callable-is-closures-only.md)/[0031](0031-callable-is-the-only-closure-type.md)
   (`call_user_func*`, `func_get_args`, `function_exists`), [0052](0052-closed-doors.md) (`eval`, `dl`,
   every `stream_*` wrapper), [0020](0020-error-escalation-ladder.md) (`set_error_handler`,
   `trigger_error`, `error_get_last`, `@`), [0028](0028-closing-the-remaining-magic-methods.md)
   (`register_shutdown_function`), [0012](0012-no-superglobals.md) (`session_*`, `header`, `setcookie`,
   `filter_input*`), [0051](0051-standard-library-tiers.md) (`setlocale`, `gettext`, `pcntl_*`, `putenv`),
   [0059](0059-cross-request-state-is-explicit.md) (`apcu_*`, `shmop`, `sysv*`).
4. **Structurally wrong here** — the internal array pointer (`current`/`key`/`next`/`prev`/`reset`/`end`:
   a mutable cursor inside a COW *value* is incoherent, since copying an array would copy its iteration
   position); every by-reference mutator (R3); `array_multisort`; `strip_tags`, `addslashes`,
   `htmlentities` and the rest of the half-escapers ([ADR 0024](0024-taint-tracking-for-injection-sinks.md)
   exists to prevent the false confidence they create); `settype`/`gettype`/`strval`/`intval`
   ([ADR 0034](0034-legacy-cast-syntax-rejected.md): `as` is the only conversion spelling); `soundex`,
   `metaphone`, `similar_text`, `str_word_count`, `chunk_split` (ASCII-only algorithms that are wrong on
   UTF-8); `uniqid` (`Core\Uuid`).

The 14 `is_*` predicates collapse to one `Core\Reflect::typeOf($mixed)` returning an enum, because they are
only meaningful on a `mixed` and the checker already knows every other case.

### 4. Resolutions this ADR fixes

Each was a live design question; each is now a rule the spec file applies.

- **No ambient timezone.** A `Zone` is an explicit argument at every instant↔calendar conversion, and there
  is no process or per-request default — the same unsoundness argument [ADR 0051](0051-standard-library-tiers.md)
  makes against `setlocale`, which MWL already has no equivalent of.
- **Weak digests are available and structurally refused where unsafe.** One `Digest` enum carries MD5,
  SHA-1 and CRC32 for the interop that genuinely needs them (ETags, checksums, legacy APIs); the HMAC,
  signature and password members declare a narrower closed subset via
  [ADR 0047](0047-literal-and-enum-case-types.md), so `Hash::hmac($m, $k, Digest::Md5)` is a compile error
  naming the reason. `Password::hash()` takes no algorithm argument at all.
- **Relative dates are a closed grammar with one implementation.** `$t->shift("+2 weeks")` is validated and
  folded at compile time for a literal argument ([ADR 0057](0057-intrinsic-literal-folding.md)) and parsed
  at runtime otherwise, sharing one implementation exactly as that ADR already requires of every intrinsic,
  so the two paths cannot diverge. Malformed input throws (R4). PHP's free-form `strtotime` is **not**
  implemented: the grammar is closed and unambiguous, with no locale-shaped date-order guessing. Parsing a
  `tainted` string through it yields a typed value and no qualifier, since a closed grammar that throws on
  anything it does not recognise is a launderer in exactly the sense
  [ADR 0024](0024-taint-tracking-for-injection-sinks.md) § 3 defines.
- **`Core\Path` is pure; `Core\IO` touches the disk.** Path algebra (`basename`, `dirname`, `extension`,
  `join`, `isAbsolute`) needs no capability, is safe in the wasm browser target
  ([ADR 0025](0025-wasm-browser-target.md)), and is constant-foldable. Everything that reads or writes —
  including `realpath`, which is `IO::canonicalize` — needs an `fs.*` capability and is an ADR 0024 sink.
  PHP conflates the two, which hides the boundary that matters.
- **Output capture is scoped.** `Core\Out::capture(fn)` runs a closure and returns what it echoed; nesting
  is call nesting. A transform over outgoing output is likewise scoped (`Out::filtered(fn, filter)`), never
  a globally installed handler — PHP's `ob_*` stack carries its complexity in exactly the part that can be
  entered from one function and left from another.
- **Collections.** `array<T>` is list, stack, queue and dictionary; SPL's restatements of it
  (`SplStack`, `SplQueue`, `SplDoublyLinkedList`, `SplFixedArray`, `ArrayObject`, `ArrayIterator`) are
  dropped. Exactly three structures survive, each expressing something an ordered `int|string`-keyed hash
  cannot: `Core\ObjectMap<K, V>`, `Core\ObjectSet<T>` (identity keys) and `Core\Heap<T>` (O(log n) priority
  ordering, over [ADR 0013](0013-comparable-interface.md)'s `Comparable`).
- **Exceptions are a small closed set on one axis.** `Throwable`, then `LogicError` (a bug: bad argument,
  bad state, bad index), `RuntimeError` (the world said no) with `IOError`/`ParseError`/`TimeoutError`, and
  `ArithmeticError` (overflow per [ADR 0007](0007-explicit-type-system.md), division by zero). PHP's 13 SPL
  classes are not reproduced: their boundaries are undefined in practice (`OutOfRange` vs `OutOfBounds`
  differ only in *when* the index is known), and half of the parallel `Error` tree — `TypeError`,
  `ArgumentCountError`, most of `ValueError` — is unreachable here because those programs do not compile.
  Domain errors are user-defined classes. Resource-limit reports remain outside `Throwable` entirely
  ([ADR 0020](0020-error-escalation-ladder.md)).
- **JSON uses one explicit interface, both directions.** `Core\Json\Codec` declares `toJson(): mixed` and a
  static `fromJson(mixed): static`. No magic hook survives ([ADR 0028](0028-closing-the-remaining-magic-methods.md)),
  structural encoding of public properties is rejected because it makes a class's public shape an implicit
  wire contract, and the decode half — which `JsonSerializable` lacks, forcing every PHP project to hand-write
  hydration — is part of the same interface. A `secret` property simply never appears in `toJson()`
  ([ADR 0033](0033-secret-qualifier-for-confidential-values.md)).
- **No lazy pipeline API yet.** A `Core\Seq` over `Iterable` would be a second spelling of `map`/`filter`/
  `reduce`, which [ADR 0051](0051-standard-library-tiers.md) test 6 argues against, and the question is much
  better informed once generators actually run. Deferred past M5; streaming today is `foreach` over an
  `Iterable`, which needs nothing new. Adding it later is purely additive.

### 5. Every member declares its qualifier behaviour

A `Core` member's entry in the spec file states whether it is a **sink** (refuses `tainted`/`secret`), a
**launderer** (removes a qualifier, its contract naming the sink it launders for), **contagious** (a
qualified argument produces a qualified result), or **neutral**. This is a column of the signature, not a
footnote: [ADR 0024](0024-taint-tracking-for-injection-sinks.md) § 3 and
[ADR 0055](0055-extension-qualifier-declarations.md) both rest on that classification being total, and a
member added without one is an incomplete member.

## Consequences

**Positive**

- **The eleventh member is predictable from the first ten.** That is the whole return on R1–R20, and it is
  the only defence against a ~450-member library being unlearnable.
- **Roughly 1,900 PHP functions become ~450 members** covering strictly more ground — because the
  reduction comes from removing restatements, not capability.
- **A reviewer reads authority off the call site.** `Path::` cannot touch the disk, `IO::` can; a `Digest`
  subset means an unsafe algorithm cannot reach an HMAC; a sink is visible in the spec entry.
- **`mwl convert`'s mapping table gets a rule, not just a list.** Most of PHP's surface maps by pattern
  (twin removal, flag-to-enum, subject-first reorder), so the converter can explain each rewrite it makes.

**Negative**

- **A converted program's `catch` clauses need review.** The SPL exception classes do not exist, and no
  mechanical mapping recovers the intent behind `catch (OutOfBoundsException $e)`.
- **`$a = Arr::sort($a);` is three characters longer than `sort($a);`** and reads as a copy even though COW
  makes it one. This is R3's whole cost, paid at every mutation site.
- **Every date formatting call names a zone.** Correct, and more typing than PHP for the common case.
- **The spec file is large and must stay in sync with the implementation.** M4S's conformance suite is what
  keeps it honest; a signature with no test is treated as unimplemented.
- **Familiarity is spent.** A PHP developer knows `strtotime`, `ob_start` and `SplStack` and will not find
  them. Each is deliberate, and each is named with its replacement in the spec so the diagnostic can point
  at one ([ADR 0011](0011-functions-and-constants-are-class-members.md) § 4's style).

## Alternatives rejected

- **Mirror PHP's built-ins one-for-one and rely on `mwl convert`.** Maximum familiarity, trivial conversion.
  Rejected: it imports both duplication axes wholesale and makes the ~1,900-name surface permanent, which is
  the cost [ADR 0051](0051-standard-library-tiers.md) ranks as the expensive one.
- **Named arguments instead of a shape-literal options bag.** Slightly terser and familiar from PHP 8.
  Rejected: it is a real language feature (grammar, checker rules) and it makes every parameter *name* part
  of the public compatibility surface forever, whereas a declared options `type` alias is versioned like any
  other type and reuses [ADR 0036](0036-anonymous-object-shapes.md) unchanged.
- **Keep by-reference mutators for the hot paths** (`Arr::sortInPlace`). Rejected: COW already gives the
  in-place mutation when the refcount is 1, so the only thing a second spelling buys is the aliasing rules
  R3 removes — a violation of R15 and of [ADR 0051](0051-standard-library-tiers.md) test 6 for no measurable
  gain.
- **A fuller typed collections library** (`List<T>`, `Map<K,V>`, `Set<T>`) beside `array<T>`. Rejected: two
  ways to hold a sequence, a conversion tax at every library boundary, and a standing question about which
  one a signature should declare. The three structures in § 4 are admitted precisely because they are *not*
  restatements of `array<T>`.
- **Structural JSON encoding of public properties.** Zero boilerplate. Rejected: it makes a rename a silent
  wire-format break and needs an opt-out mechanism, which is a magic hook under another name.

## Verification

- **M4S:** every member in [docs/spec/01-core-library.md](../spec/01-core-library.md) has a conformance
  test, and a mechanical check over the spec file asserts the rules that can be checked mechanically —
  R5's verb-to-return-type table, R7's full-word rule against the closed abbreviation list, R6's symmetric
  pairs both present, R4 (no member returns `bool` to signal failure, no member returns a union with
  `false`), and R2 (at most one options-shape parameter, and it is last). A member with no declared
  qualifier behaviour (§ 5) fails the same check.
- **M4S:** a check asserts no `Core` signature mentions `resource` (R14), and that no static member's name
  collides with an instance method on a type its own class constructs (R18).
- **M8:** the capability-bearing half of the roster is added under the same rules and the same checks;
  nothing about them is M4S-specific.
- **M11:** `mwl convert`'s PHP-name → `Core`-member table is generated against the spec file, so a name PHP
  has and MWL removed produces a diagnostic naming the replacement rather than an unresolved call.
