# MWL Specification — 01: The `Core` library

**This file is authoritative for every `Core` signature.** It is the member list
[ADR 0011 § 2](../adr/0011-functions-and-constants-are-class-members.md) deferred and
[ADR 0051 § 3](../adr/0051-standard-library-tiers.md) placed. The *shape* rules every entry obeys are
[ADR 0063](../adr/0063-core-api-conventions.md) — read that first; this file applies it and does not
re-argue it. Where a class has its own ADR (`Core\Regex`, `Core\Decimal`, `Core\Reflect`, `Core\Process`,
`Core\Cache`, …) that ADR owns the semantics and this file owns only the signatures.

## How to read an entry

Every member is `public static` on its domain class unless the entry shows a `$receiver->`, which marks an
instance method. The `public static function` prefix is omitted throughout; `Core\Str::length` is written
`length(string $s): uint`.

The **Replaces** column names the PHP built-ins an entry subsumes. It is the source for `mwl convert`'s
mapping table (M11) and doubles as the audit trail for
[ADR 0063 § 3](../adr/0063-core-api-conventions.md): a PHP name that appears nowhere in this column is
either in that ADR's removal list or genuinely absent from PHP.

**Qualifier** is the [ADR 0024](../adr/0024-taint-tracking-for-injection-sinks.md)/[0033](../adr/0033-secret-qualifier-for-confidential-values.md)
classification, and every member has one:

| Mark | Meaning |
|---|---|
| *(blank)* | **contagious** — a `tainted`/`secret` argument yields a `tainted`/`secret` result. The default, and the overwhelming majority |
| **sink** | refuses a qualified argument in the named position |
| **launder** | removes a qualifier; its contract names the sink it launders for |
| **neutral** | the result never carries a qualifier from the argument (a `bool`, a count, a hash of a secret) |

Two conventions apply throughout and are not repeated per entry:

- **Ranges** are `(offset, ?length)` — negative offset counts from the end, negative length stops that many
  from the end, `null` runs to the end ([ADR 0063](../adr/0063-core-api-conventions.md) R8).
- **A variadic member takes no options shape.** R2 requires the options shape to be last, and a variadic
  parameter already is; where both are wanted the member takes an `array` instead of a variadic.

## What is a member and what is an operator

`Core\Arr` covers whole-array operations only. Element access stays in the language, and this is what
keeps the two from becoming two spellings of one thing:

| Operation | Spelling | Note |
|---|---|---|
| read an element | `$a[$k]` | throws on a missing key ([ADR 0063](../adr/0063-core-api-conventions.md) R4) |
| read a possibly-absent element | `$a[$k] ?? $default` | the absence-tolerant spelling; there is **no** `Arr::get` |
| write an element | `$a[$k] = $v` | |
| remove an element | `unset($a[$k])` | permitted on an array element; refused on a declared object property ([ADR 0028](../adr/0028-closing-the-remaining-magic-methods.md)) |
| ask whether a key exists | `Arr::hasKey($a, $k)` | `isset()`/`array_key_exists` both collapse here |

The same division applies elsewhere: `**` is exponentiation, so there is no `Math::pow`; `%` is integer
modulo, so `Math::mod` exists only for the `float` case; `instanceof` is an operator, so `Core\Reflect` has
no `isA`.

## Milestones

**M4S** builds every class in §§ 1–12: they are pure, need no capability, no reactor, no driver, and no
open file — so they can be implemented, tested and depended upon before the HTTP server exists. **M8**
builds §§ 14–17, which need one of those things, under the same rules and the same conformance checks.

§ 13 is neither: each of its entries is pure, but each also waits on something outside `Core`.
`Core\Program` needs [ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md)'s `autoload`,
`Core\Ast` needs [ADR 0019](../adr/0019-reflection-and-ast-parsing-are-core-features.md)'s inert-AST
surface, `Core\Attributes`' retrieval body is M8 by [ADR 0046](../adr/0046-attributes-shape-literal-metadata.md)
(its `#[...]` *syntax* is M4), and `Core\Reflect`, `Core\Decimal`, `Core\BigInt` and `Core\Test` want a
finished object representation under them. They land with whichever milestone closes their dependency, to
this same contract.

---

# Part I — the pure core (M4S)

## 1. `Core\Str`

`string` is guaranteed-valid UTF-8 ([ADR 0009](../adr/0009-string-and-bytes.md)), so **no member takes an
encoding argument** (R13) and there is no `mb_` twin of anything. Indexing granularity — code point versus
grapheme for `length`, `at` and `slice` — is the one open question in ADR 0009 and is settled there, not
here; every signature below is written granularity-agnostically and the conformance suite pins whichever
that ADR lands on.

### Inspection

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `length` | `length(string $s): uint` | `strlen`, `mb_strlen` | neutral |
| `isEmpty` | `isEmpty(string $s): bool` | `$s === ""` | neutral |
| `at` | `at(string $s, int $index): string` | `$s[$i]`, `mb_substr($s,$i,1)` | |
| `contains` | `contains(string $haystack, string $needle): bool` | `str_contains`, `strstr` as a predicate | neutral |
| `startsWith` | `startsWith(string $s, string $prefix): bool` | `str_starts_with` | neutral |
| `endsWith` | `endsWith(string $s, string $suffix): bool` | `str_ends_with` | neutral |
| `indexOf` | `indexOf(string $haystack, string $needle, {from?: int, caseInsensitive?: bool}): ?uint` | `strpos`, `stripos`, `mb_strpos`, `mb_stripos` | neutral |
| `lastIndexOf` | `lastIndexOf(string $haystack, string $needle, {before?: int, caseInsensitive?: bool}): ?uint` | `strrpos`, `strripos`, `mb_strrpos` | neutral |
| `countOf` | `countOf(string $haystack, string $needle): uint` | `substr_count` | neutral |
| `compare` | `compare(string $a, string $b): int` | `strcmp`, `strncmp`, `strnatcmp` | neutral |
| `compareCaseless` | `compareCaseless(string $a, string $b): int` | `strcasecmp`, `strncasecmp`, `strnatcasecmp` | neutral |
| `naturalOrder` | `naturalOrder({caseInsensitive?: bool}): callable` | the comparator behind `natsort`/`natcasesort` | neutral |

`strcoll` and every locale-sensitive comparison are **not** here: MWL has no ambient locale
([ADR 0051](../adr/0051-standard-library-tiers.md)), and locale-aware collation is the intl extension's
batch-shaped API.

### Extraction

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `slice` | `slice(string $s, int $offset, ?int $length = null): string` | `substr`, `mb_substr` | |
| `before` | `before(string $s, string $needle, {last?: bool}): ?string` | `strstr($h,$n,true)`, `strrchr` as a prefix | |
| `after` | `after(string $s, string $needle, {last?: bool}): ?string` | `strstr`, `stristr`, `strrchr` | |
| `split` | `split(string $s, string $separator, {limit?: int}): array<string>` | `explode` | |
| `chunk` | `chunk(string $s, uint $size): array<string>` | `str_split`, `mb_str_split`, `chunk_split` | |
| `lines` | `lines(string $s): array<string>` | `explode(PHP_EOL, …)`, `file()`'s split half | |
| `graphemes` | `graphemes(string $s): array<string>` | `grapheme_*` (intl, for the split case) | |
| `codePoints` | `codePoints(string $s): array<uint>` | `mb_str_split` + `mb_ord`, `unpack("N*", …)` | neutral |

### Transformation

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `join` | `join(array<string> $parts, string $separator = ""): string` | `implode`, `join` | |
| `replace` | `replace(string $s, string $search, string $replacement, {caseInsensitive?: bool, limit?: uint}): string` | `str_replace`, `str_ireplace` | |
| `replaceAll` | `replaceAll(string $s, array<string> $pairs, {caseInsensitive?: bool}): string` | `str_replace` with array arguments, `strtr` | |
| `replaceRange` | `replaceRange(string $s, int $offset, ?int $length, string $replacement): string` | `substr_replace` | |
| `trim` | `trim(string $s, {characters?: string}): string` | `trim` | |
| `trimStart` | `trimStart(string $s, {characters?: string}): string` | `ltrim` | |
| `trimEnd` | `trimEnd(string $s, {characters?: string}): string` | `rtrim`, `chop` | |
| `padStart` | `padStart(string $s, uint $length, string $padding = " "): string` | `str_pad` + `STR_PAD_LEFT` | |
| `padEnd` | `padEnd(string $s, uint $length, string $padding = " "): string` | `str_pad` + `STR_PAD_RIGHT` | |
| `repeat` | `repeat(string $s, uint $times): string` | `str_repeat` | |
| `reverse` | `reverse(string $s): string` | `strrev` (grapheme-aware here, unlike PHP's byte reversal) | |
| `wrap` | `wrap(string $s, uint $width, {breakWith?: string, cutLongWords?: bool}): string` | `wordwrap` | |
| `lower` | `lower(string $s): string` | `strtolower`, `mb_strtolower` | |
| `upper` | `upper(string $s): string` | `strtoupper`, `mb_strtoupper` | |
| `upperFirst` | `upperFirst(string $s): string` | `ucfirst` | |
| `lowerFirst` | `lowerFirst(string $s): string` | `lcfirst` | |
| `fold` | `fold(string $s): string` | `mb_convert_case(…, MB_CASE_FOLD)` — for caseless comparison | |
| `normalize` | `normalize(string $s, NormalForm $form): string` | `Normalizer::normalize` | |
| `fromCodePoint` | `fromCodePoint(uint $codePoint): string` | `chr`, `mb_chr` | neutral |
| `format` | `format(string $template, mixed ...$arguments): string` | `sprintf`, `vsprintf`, `printf`, `vprintf`, `fprintf`, `vfprintf` | |

`format` is an [ADR 0057](../adr/0057-intrinsic-literal-folding.md) intrinsic: a literal template has its
placeholder count and types checked against the argument list at compile time, which turns PHP's
`printf`-argument-mismatch bug family into a diagnostic. `ucwords` and title casing are **not** here —
word segmentation is locale-dependent and belongs to intl.

Enums: `NormalForm { Nfc, Nfd, Nfkc, Nfkd }`.

## 2. `Core\Arr`

`array<T>` is an insertion-ordered hash with copy-on-write value semantics
([ADR 0007 § 5](../adr/0007-explicit-type-system.md)). Every member is pure (R3); an implementation
mutates in place whenever the argument's refcount is 1, so purity costs nothing. `T` is a type variable —
the stdlib is parametric where user code is not.

### Inspection

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `count` | `count(array<T> $a): uint` | `count`, `sizeof` | neutral |
| `isEmpty` | `isEmpty(array<T> $a): bool` | `empty($a)`, `count($a) === 0` | neutral |
| `isList` | `isList(array<T> $a): bool` | `array_is_list` | neutral |
| `hasKey` | `hasKey(array<T> $a, int\|string $key): bool` | `array_key_exists`, `isset` | neutral |
| `contains` | `contains(array<T> $haystack, T $needle): bool` | `in_array` (always strict) | neutral |
| `keyOf` | `keyOf(array<T> $haystack, T $needle): ?(int\|string)` | `array_search` | neutral |
| `keys` | `keys(array<T> $a): array<int\|string>` | `array_keys` | |
| `values` | `values(array<T> $a): array<T>` | `array_values` | |
| `first` | `first(array<T> $a): ?T` | `reset`, `current`, `$a[array_key_first($a)]` | |
| `last` | `last(array<T> $a): ?T` | `end`, `$a[array_key_last($a)]` | |
| `firstKey` | `firstKey(array<T> $a): ?(int\|string)` | `array_key_first`, `key` | neutral |
| `lastKey` | `lastKey(array<T> $a): ?(int\|string)` | `array_key_last` | neutral |

PHP's internal array pointer (`current`/`key`/`next`/`prev`/`reset`/`end`/`each`) has no equivalent: a
mutable cursor inside a copy-on-write *value* is incoherent, since copying the array would copy its
iteration position. `foreach` and the four members above cover every use.

### Structure

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `slice` | `slice(array<T> $a, int $offset, ?int $length = null, {preserveKeys?: bool}): array<T>` | `array_slice` | |
| `splice` | `splice(array<T> $a, int $offset, ?int $length, array<T> $replacement = []): array<T>` | `array_splice` (returning, not by-reference) | |
| `chunk` | `chunk(array<T> $a, uint $size, {preserveKeys?: bool}): array<array<T>>` | `array_chunk` | |
| `append` | `append(array<T> $a, T ...$values): array<T>` | `array_push`, `$a[] = $v` in expression position | |
| `prepend` | `prepend(array<T> $a, T ...$values): array<T>` | `array_unshift` | |
| `withoutFirst` | `withoutFirst(array<T> $a): array<T>` | `array_shift`'s remainder (`first` gets the element) | |
| `withoutLast` | `withoutLast(array<T> $a): array<T>` | `array_pop`'s remainder (`last` gets the element) | |
| `pad` | `pad(array<T> $a, int $size, T $value): array<T>` | `array_pad` | |
| `reverse` | `reverse(array<T> $a, {preserveKeys?: bool}): array<T>` | `array_reverse` | |
| `flip` | `flip(array<T> $a): array<int\|string>` | `array_flip` | |
| `flatten` | `flatten(array<T> $a, {depth?: uint}): array<T>` | `array_merge(...$a)`, `iterator_to_array` on a recursive iterator | |
| `fill` | `fill(uint $count, T $value): array<T>` | `array_fill` | |
| `fillKeys` | `fillKeys(array<int\|string> $keys, T $value): array<T>` | `array_fill_keys` | |
| `range` | `range(int $start, int $end, {step?: int}): array<int>` | `range` | neutral |
| `combine` | `combine(array<int\|string> $keys, array<T> $values): array<T>` | `array_combine` | |
| `toPairs` | `toPairs(array<T> $a): array<array<int\|string\|T>>` | manual `foreach` | |
| `fromPairs` | `fromPairs(array<array<int\|string\|T>> $pairs): array<T>` | manual `foreach` | |
| `column` | `column(array<array<T>> $a, int\|string $column, {indexBy?: int\|string}): array<T>` | `array_column` | |

### Set operations

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `merge` | `merge(array<T> $a, array<U> ...$others): array<T\|U>` | `array_merge`, the `+` operator | |
| `mergeRecursive` | `mergeRecursive(array<T> $a, array<U> ...$others): array<T\|U>` | `array_merge_recursive` | |
| `replace` | `replace(array<T> $a, array<U> ...$others): array<T\|U>` | `array_replace` | |
| `replaceRecursive` | `replaceRecursive(array<T> $a, array<U> ...$others): array<T\|U>` | `array_replace_recursive` | |
| `diff` | `diff(array<T> $a, array<T> $b, {on?: SetOn, by?: callable, comparator?: callable}): array<T>` | `array_diff`, `array_udiff`, `array_diff_key`, `array_diff_assoc`, `array_diff_ukey`, `array_udiff_assoc` | |
| `intersect` | `intersect(array<T> $a, array<T> $b, {on?: SetOn, by?: callable, comparator?: callable}): array<T>` | `array_intersect` and its five variants | |
| `unique` | `unique(array<T> $a, {by?: callable}): array<T>` | `array_unique` | |
| `countValues` | `countValues(array<T> $a): array<uint>` | `array_count_values` | neutral |

Twelve `array_diff*`/`array_intersect*` functions become two members plus a three-case enum. That single
row is the largest reduction in the library.

### Iteration and aggregation

Every callback receives `($value, $key)` and may declare fewer parameters (R9), which is what removes
`ARRAY_FILTER_USE_KEY`, `ARRAY_FILTER_USE_BOTH` and the need for `…WithKey` twins.

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `map` | `map(array<T> $a, callable $fn): array<U>` | `array_map` | |
| `mapKeys` | `mapKeys(array<T> $a, callable $fn): array<T>` | `array_combine(array_map(...), …)` | |
| `filter` | `filter(array<T> $a, callable $predicate): array<T>` | `array_filter` and its two flags | |
| `reduce` | `reduce(array<T> $a, callable $fn, U $initial): U` | `array_reduce` | |
| `each` | `each(array<T> $a, callable $fn): void` | `array_walk`, `array_walk_recursive` | |
| `find` | `find(array<T> $a, callable $predicate): ?T` | `array_find` | |
| `findKey` | `findKey(array<T> $a, callable $predicate): ?(int\|string)` | `array_find_key` | neutral |
| `any` | `any(array<T> $a, callable $predicate): bool` | `array_any` | neutral |
| `all` | `all(array<T> $a, callable $predicate): bool` | `array_all` | neutral |
| `groupBy` | `groupBy(array<T> $a, callable $key): array<array<T>>` | nothing — the most-written PHP userland helper | |
| `sum` | `sum(array<int\|float\|decimal> $a): int\|float\|decimal` | `array_sum` | neutral |
| `product` | `product(array<int\|float\|decimal> $a): int\|float\|decimal` | `array_product` | neutral |
| `average` | `average(array<int\|float\|decimal> $a): ?float\|?decimal` | `array_sum($a)/count($a)`, with the empty case answered | neutral |
| `min` | `min(array<T> $a): ?T` | `min` with an array argument | |
| `max` | `max(array<T> $a): ?T` | `max` with an array argument | |

### Ordering

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `sort` | `sort(array<T> $a, {by?: callable, order?: Order, comparator?: callable, preserveKeys?: bool}): array<T>` | `sort`, `rsort`, `asort`, `arsort`, `usort`, `uasort`, `natsort`, `natcasesort`, `array_multisort` | |
| `sortByKey` | `sortByKey(array<T> $a, {order?: Order, comparator?: callable}): array<T>` | `ksort`, `krsort`, `uksort` | |

Eleven sort functions plus `array_multisort` become two members. Descending is `{order: Order::Desc}`,
key-preservation is an option rather than a letter in the name, and `by` — a key-extractor closure — is
the thing `usort` callbacks are written to emulate.

Enums: `Order { Asc, Desc }`, `SetOn { Values, Keys, Both }`.

## 3. `Core\Math`

`**` is exponentiation and `%` is integer modulo, so neither has a member (R17). Division by zero throws
`ArithmeticError` (R4), which is why `fdiv`'s silent `INF` has no equivalent. Overflow throws rather than
becoming a `float` ([ADR 0007](../adr/0007-explicit-type-system.md)).

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `abs` | `abs(int\|float\|decimal $n): int\|float\|decimal` | `abs` | neutral |
| `sign` | `sign(int\|float\|decimal $n): int` | `$n <=> 0` | neutral |
| `min` | `min(T $a, T $b): T` | `min` with scalar arguments | neutral |
| `max` | `max(T $a, T $b): T` | `max` with scalar arguments | neutral |
| `clamp` | `clamp(T $n, T $low, T $high): T` | `min(max(…))` | neutral |
| `ceil` | `ceil(float\|decimal $n): float\|decimal` | `ceil` | neutral |
| `floor` | `floor(float\|decimal $n): float\|decimal` | `floor` | neutral |
| `truncate` | `truncate(float\|decimal $n): float\|decimal` | `(int)` truncation | neutral |
| `round` | `round(float\|decimal $n, {precision?: int, mode?: RoundMode}): float\|decimal` | `round` and its four `PHP_ROUND_*` constants | neutral |
| `intDiv` | `intDiv(int $a, int $b): int` | `intdiv` | neutral |
| `mod` | `mod(float $a, float $b): float` | `fmod` (integer `%` is the operator) | neutral |
| `gcd` | `gcd(int $a, int $b): int` | `gmp_gcd` | neutral |
| `lcm` | `lcm(int $a, int $b): int` | `gmp_lcm` | neutral |
| `sqrt` | `sqrt(float $n): float` | `sqrt` | neutral |
| `cbrt` | `cbrt(float $n): float` | `pow($n, 1/3)` | neutral |
| `hypot` | `hypot(float $a, float $b): float` | `hypot` | neutral |
| `exp` | `exp(float $n): float` | `exp` | neutral |
| `log` | `log(float $n, {base?: float}): float` | `log`, `log10`, `log2`, `log1p` | neutral |
| `sin` `cos` `tan` | `sin(float $radians): float` (and the rest) | `sin`, `cos`, `tan` | neutral |
| `asin` `acos` `atan` | `asin(float $n): float` (and the rest) | `asin`, `acos`, `atan` | neutral |
| `atan2` | `atan2(float $y, float $x): float` | `atan2` | neutral |
| `sinh` `cosh` `tanh` | hyperbolic, same shape | `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh` | neutral |
| `toRadians` | `toRadians(float $degrees): float` | `deg2rad` | neutral |
| `toDegrees` | `toDegrees(float $radians): float` | `rad2deg` | neutral |
| `isNan` | `isNan(float $n): bool` | `is_nan` | neutral |
| `isFinite` | `isFinite(float $n): bool` | `is_finite`, `is_infinite` | neutral |
| `toBase` | `toBase(int $n, uint $base): string` | `decbin`, `dechex`, `decoct`, `base_convert` | neutral |
| `fromBase` | `fromBase(string $s, uint $base): int` | `bindec`, `hexdec`, `octdec`, `base_convert` | neutral |
| `format` | `format(int\|float\|decimal $n, {decimals?: uint, decimalSeparator?: string, groupSeparator?: string}): string` | `number_format` | neutral |

Constants: `PI`, `TAU`, `E`, `EPSILON`, `INT_MAX`, `INT_MIN`, `UINT_MAX`, `FLOAT_MAX`, `FLOAT_MIN`, `NAN`,
`INFINITY` — replacing `M_PI`, `M_E`, `PHP_INT_MAX`, `PHP_FLOAT_EPSILON` and the rest of PHP's global
constants ([ADR 0011](../adr/0011-functions-and-constants-are-class-members.md)).

`Math::format` takes explicit separators because MWL has no ambient locale; locale-correct number
formatting is intl's `NumberFormatter` equivalent, at Tier 1.

Enums: `RoundMode { HalfUp, HalfDown, HalfEven, HalfOdd, Up, Down }`.

## 4. `Core\Time`

PHP has ~40 `date_*` procedural functions that are aliases of `DateTime` methods, *and* a
`DateTime`/`DateTimeImmutable` mutable/immutable pair. Both duplications are gone (R17, R20): there are
objects only, and every one is immutable. There is **no ambient timezone** — no process default, no
per-request default — so a `Zone` is an explicit argument at every instant↔calendar conversion.

### Entry points on `Core\Time`

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `now` | `now(): Instant` | `time`, `microtime`, `date_create` | neutral |
| `monotonic` | `monotonic(): Duration` | `hrtime` — for measuring, never for wall-clock | neutral |
| `sleep` | `sleep(Duration $d): void` | `sleep`, `usleep`, `time_nanosleep`, `time_sleep_until` | neutral |
| `fromEpoch` | `fromEpoch(int $seconds, {nanos?: uint}): Instant` | `DateTime::setTimestamp` | neutral |
| `fromIso` | `fromIso(string $text): Instant` | `strtotime` on an ISO-8601 string, `DateTime::__construct` | |
| `parse` | `parse(string $text, string $format, Zone $zone): DateTime` | `DateTime::createFromFormat`, `strptime` | |
| `at` | `at(int $year, uint $month, uint $day, Zone $zone, {hour?, minute?, second?, nanos?}): DateTime` | `mktime`, `gmmktime`, `DateTime::setDate` | neutral |

`Core\Time::parse` is an [ADR 0057](../adr/0057-intrinsic-literal-folding.md) intrinsic — a literal format
string is validated and its plan prepared at compile time. PHP's free-form `strtotime` is **not**
implemented; see `DateTime::shift` below for what replaces its relative half.

### `Core\Time\Instant` — an absolute point on the timeline

| Member | Signature | Notes |
|---|---|---|
| `in` | `$i->in(Zone $zone): DateTime` | the only instant→calendar conversion; a zone is never implicit |
| `toEpochSeconds` | `$i->toEpochSeconds(): int` | replaces `getTimestamp`, `date("U")` |
| `toEpochMillis` | `$i->toEpochMillis(): int` | |
| `plus` / `minus` | `$i->plus(Duration $d): Instant` | replaces `date_add`, `date_sub`, `modify` |
| `since` | `$i->since(Instant $earlier): Duration` | replaces `date_diff`, `DateInterval` arithmetic |
| `compareTo` | `$i->compareTo(Instant $other): int` | `Instant` implements `Comparable` ([ADR 0013](../adr/0013-comparable-interface.md)), so `<`/`>` work directly |
| `toIso` | `$i->toIso(): string` | replaces `date(DATE_ATOM)` |

### `Core\Time\DateTime` — a civil date and time in a zone

| Member | Signature | Notes |
|---|---|---|
| `format` | `$d->format(string $pattern): string` | [ADR 0057](../adr/0057-intrinsic-literal-folding.md) intrinsic. Replaces `date`, `gmdate`, `idate`, `strftime`, `date_format` |
| `shift` | `$d->shift(string $expression): DateTime` | the closed relative grammar — `"+2 weeks"`, `"next monday"`, `"start of month"`. Folded at compile time for a literal, parsed at run time otherwise, one implementation for both ([ADR 0063 § 4](../adr/0063-core-api-conventions.md)). Throws on anything the grammar does not accept, so it launders a `tainted` argument |
| `plus` / `minus` | `$d->plus(Duration $d): DateTime` | the computed-offset spelling; `shift` is the literal one |
| `with` | `$d->with({year?, month?, day?, hour?, minute?, second?, nanos?}): DateTime` | replaces `setDate`, `setTime`, `setISODate` |
| `startOf` / `endOf` | `$d->startOf(Unit $u): DateTime` | distinct operations at a DST boundary, which is why both exist |
| `toInstant` | `$d->toInstant(): Instant` | |
| `date` / `timeOfDay` / `zone` | `$d->date(): Date` | component views |
| `weekday` | `$d->weekday(): Weekday` | replaces `date("N")` |
| `dayOfYear` | `$d->dayOfYear(): uint` | replaces `date("z")` |
| `isLeapYear` | `$d->isLeapYear(): bool` | replaces `date("L")`, `checkdate`'s year half |

`Core\Time\Date` and `Core\Time\TimeOfDay` are the zone-free component types, with the same `plus`/`minus`/
`with`/`compareTo`/`format` shape. `checkdate` has no equivalent because constructing an invalid date
throws.

### `Core\Time\Duration` and `Core\Time\Zone`

| Member | Signature | Notes |
|---|---|---|
| `Duration::seconds` | `seconds(int $n): Duration` | plus `nanos`, `millis`, `minutes`, `hours`, `days`, `weeks` |
| `$d->toSeconds` | `$d->toSeconds(): int` | plus `toMillis`, `toNanos` |
| `$d->plus` / `minus` / `multipliedBy` / `negated` | | `Duration` is `Comparable` |
| `Zone::of` | `of(string $id): Zone` | IANA identifier; throws on an unknown one. Replaces `DateTimeZone` |
| `Zone::UTC` | constant | the only zone that is ever a default, and only where written explicitly |
| `$z->offsetAt` | `$z->offsetAt(Instant $i): Duration` | replaces `getOffset` |

Enums: `Weekday { Monday … Sunday }`, `Month { January … December }`,
`Unit { Nanos, Millis, Second, Minute, Hour, Day, Week, Month, Quarter, Year }`.

PHP's `calendar` extension (`cal_days_in_month`, `easter_date`, the Julian/Jewish/French converters) is
dropped outright ([ADR 0051](../adr/0051-standard-library-tiers.md)); `cal_days_in_month` is
`$d->startOf(Unit::Month)->plus(…)` arithmetic, and the rest has no place in a Tier 0 library.

## 5. `Core\Regex`

Semantics — the two-tier engine, the step budget, why the *pattern* is a sink while the subject is not —
are [ADR 0056](../adr/0056-regex-engine-policy.md). `preg_match`'s `$matches` out-parameter is a returned
`?Match` (R3), and there are no `PREG_*` flag constants (R11).

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `compile` | `compile(string $pattern, {caseInsensitive?, multiline?, dotAll?, ungreedy?}): Pattern` | the `/…/imsx` delimiter-and-modifier syntax | **sink** (pattern) |
| `matches` | `matches(string $subject, Pattern\|string $pattern): bool` | `preg_match` as a predicate | neutral |
| `match` | `match(string $subject, Pattern\|string $pattern, {from?: int}): ?Match` | `preg_match` + `$matches` + `PREG_OFFSET_CAPTURE` | **sink** (pattern) |
| `matchAll` | `matchAll(string $subject, Pattern\|string $pattern): array<Match>` | `preg_match_all` + `PREG_PATTERN_ORDER`/`PREG_SET_ORDER` | **sink** (pattern) |
| `replace` | `replace(string $subject, Pattern\|string $pattern, string $replacement, {limit?: uint}): string` | `preg_replace` | **sink** (pattern) |
| `replaceWith` | `replaceWith(string $subject, Pattern\|string $pattern, callable $fn, {limit?: uint}): string` | `preg_replace_callback`, `preg_replace_callback_array` | **sink** (pattern) |
| `split` | `split(string $subject, Pattern\|string $pattern, {limit?: int, keepEmpty?: bool}): array<string>` | `preg_split` and its four flags | **sink** (pattern) |
| `quote` | `quote(string $literal): string` | `preg_quote` | **launder** (for the pattern sink) |

`$match->group(int\|string)`, `$match->groups()`, `$match->offset()` and `$match->text()` replace the
positional-array shape. `preg_last_error` has no equivalent: a failure throws (R4).

## 6. `Core\Json`

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `encode` | `encode(mixed $value, {pretty?: bool, escapeUnicode?: bool}): string` | `json_encode` and its 15 `JSON_*` flags | |
| `decode` | `decode(string $json): mixed` | `json_decode`, `json_last_error`, `json_last_error_msg` | |
| `decodeAs` | `decodeAs<T>(string $json): T` | hand-written hydration | |
| `isValid` | `isValid(string $json): bool` | `json_validate` | neutral |

`decode` throws `ParseError` on malformed input — there is no flag to choose between throwing and
returning `null`, and no error-code accessor (R4). A class participates by implementing
`Core\Json\Codec`, which declares `toJson(): mixed` and a static `fromJson(mixed $value): static`; there is
no magic hook and no structural encoding of public properties
([ADR 0063 § 4](../adr/0063-core-api-conventions.md)). A `secret` value cannot be encoded at all
([ADR 0033](../adr/0033-secret-qualifier-for-confidential-values.md)).

## 7. `Core\Encoding` and `Core\Bytes`

`Core\Encoding` sits exactly at the `bytes`↔`string` boundary, which is the one place a conversion can
honestly fail ([ADR 0009](../adr/0009-string-and-bytes.md)).

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `decodeText` | `decodeText(bytes $b, Charset $charset): string` | `iconv`, `mb_convert_encoding`, `utf8_decode` | |
| `encodeText` | `encodeText(string $s, Charset $charset): bytes` | `iconv`, `mb_convert_encoding`, `utf8_encode` | |
| `isValidText` | `isValidText(bytes $b, Charset $charset): bool` | `mb_check_encoding` | neutral |
| `toBase64` / `fromBase64` | `toBase64(bytes $b): string` | `base64_encode`, `base64_decode` | |
| `toBase64Url` / `fromBase64Url` | `toBase64Url(bytes $b): string` | `strtr(base64_encode(…))` idiom | |
| `toHex` / `fromHex` | `toHex(bytes $b): string` | `bin2hex`, `hex2bin`, `unpack("H*")` | |
| `toBase32` / `fromBase32` | `toBase32(bytes $b): string` | nothing — needed by TOTP ([ADR 0060](../adr/0060-application-security-protocols.md)) | |

`quoted_printable_encode`/`_decode` and `convert_uuencode`/`_decode` are dropped; quoted-printable survives
only inside `Core\Mail`, which is the one thing that ever needed it.

`Core\Bytes` is the binary counterpart of `Core\Str`, with the same subject-first shape: `length`, `at`,
`slice`, `concat`, `indexOf`, `compare`, `fill`, `repeat`, plus `pack(string $format, mixed ...$values)`
and `unpack(bytes $b, string $format): array<mixed>` (replacing `pack`/`unpack`, with the format string an
ADR 0057 intrinsic). There is no `bytes` literal — see [00-overview § 5](00-overview.md).

Enums: `Charset { Utf8, Utf16Le, Utf16Be, Latin1, Windows1252, Ascii, … }`.

## 8. `Core\Path`

Pure string algebra over paths. **No member touches the disk**, so none needs a capability and all are
available in the wasm browser target ([ADR 0025](../adr/0025-wasm-browser-target.md)). Everything that
reads or writes is `Core\IO` (§ 14) — that split is the point.

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `basename` | `basename(string $path, {withoutExtension?: bool}): string` | `basename`, `pathinfo(…, PATHINFO_BASENAME)` | |
| `dirname` | `dirname(string $path, {levels?: uint}): string` | `dirname` | |
| `extension` | `extension(string $path): ?string` | `pathinfo(…, PATHINFO_EXTENSION)` | |
| `withExtension` | `withExtension(string $path, ?string $extension): string` | manual string surgery | |
| `join` | `join(string $base, string ...$segments): string` | `$a . "/" . $b` | |
| `split` | `split(string $path): array<string>` | `explode(DIRECTORY_SEPARATOR, …)` | |
| `normalize` | `normalize(string $path): string` | the lexical half of `realpath` — resolves `.`/`..` **without** touching the disk | |
| `isAbsolute` | `isAbsolute(string $path): bool` | manual checks | neutral |
| `relativeTo` | `relativeTo(string $path, string $base): ?string` | nothing | |

Constant: `Path::SEPARATOR` (replacing `DIRECTORY_SEPARATOR`). `Path::normalize` is **not** a launderer:
`../` removal is not path-traversal safety, because the base directory is not part of the input. The
launderer is `Core\IO::within`, in § 14, which is where the base is known.

## 9. Collections: `Core\ObjectMap`, `Core\ObjectSet`, `Core\Heap`

`array<T>` is list, stack, queue and dictionary, so `SplStack`, `SplQueue`, `SplDoublyLinkedList`,
`SplFixedArray`, `ArrayObject` and `ArrayIterator` are dropped as restatements of it. These three survive
because an insertion-ordered `int|string`-keyed hash cannot express them
([ADR 0063 § 4](../adr/0063-core-api-conventions.md)).

| Type | Members | Replaces |
|---|---|---|
| `ObjectMap<K, V>` | `set`, `get`, `has`, `remove`, `count`, `keys`, `values`, `clear`; `Iterable` | `SplObjectStorage` used as a map, `spl_object_id` side tables |
| `ObjectSet<T>` | `add`, `has`, `remove`, `count`, `union`, `intersect`, `difference`, `clear`; `Iterable` | `SplObjectStorage` used as a set |
| `Heap<T>` | `push`, `peek`, `pop`, `count`, `isEmpty` | `SplPriorityQueue`, `SplMinHeap`, `SplMaxHeap` |

`ObjectMap`/`ObjectSet` key on identity. `Heap` orders by [ADR 0013](../adr/0013-comparable-interface.md)'s
`Comparable`, or by a comparator given at construction. These are the only mutable `Core` types, because a
persistent structure would give up the O(log n) that justifies their existence at all; they are objects,
so [ADR 0007](../adr/0007-explicit-type-system.md)'s reference semantics apply and nothing about R3 is in
question.

## 10. `Core\Error` — the exception types

A small closed set on one axis. PHP's 13 SPL exception classes are not reproduced: their boundaries are
undefined in practice, and half of the parallel `Error` tree (`TypeError`, `ArgumentCountError`, most
`ValueError`) is unreachable because those programs do not compile.

```
Throwable                     // the root; user classes extend it directly
  ├─ LogicError               // a bug in the program: bad argument, bad state, bad index
  ├─ RuntimeError             // the world said no
  │    ├─ IOError             // a file, socket or process failed
  │    ├─ ParseError          // input did not match a format this code declared
  │    └─ TimeoutError        // a deadline passed
  └─ ArithmeticError          // overflow (ADR 0007), division by zero
```

Members are readonly properties, not `getX()` accessors: `$e->message`, `$e->previous`, `$e->backtrace`,
`$e->location`. `Throwable`'s message is a `secret` sink
([ADR 0033](../adr/0033-secret-qualifier-for-confidential-values.md)). Resource-limit reports are **not**
`Throwable` at all and never reach a `catch` ([ADR 0020](../adr/0020-error-escalation-ladder.md)).
Domain-specific errors are user-defined classes; `Core` does not attempt to enumerate them.

## 11. `Core\Random`, `Core\Uuid`, `Core\Hash`

`Core\Random` is a CSPRNG, always — PHP's `rand`/`mt_rand`/`shuffle`/`str_shuffle`/`array_rand` and its
newer `Random\Randomizer` OOP twin all collapse into it, and none of the insecure generators survives
under any name.

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `Random::int` | `int(int $min, int $max): int` | `rand`, `mt_rand`, `random_int` | neutral |
| `Random::bytes` | `bytes(uint $count): bytes` | `random_bytes`, `openssl_random_pseudo_bytes` | neutral |
| `Random::token` | `token(uint $bytes = 32): string` | `bin2hex(random_bytes(…))` idiom | neutral |
| `Random::pick` | `pick(array<T> $a): ?T` | `array_rand` | |
| `Random::sample` | `sample(array<T> $a, uint $count): array<T>` | `array_rand` with a count | |
| `Random::shuffle` | `shuffle(array<T> $a): array<T>` | `shuffle`, `str_shuffle` | |

`Core\Random\Seeded` is a separate object with the same members, constructed from an explicit seed. It is
not a second spelling of the above: its guarantee is reproducibility, not unpredictability, and making the
distinction a *type* is what stops a test helper being reached for in production code. `srand`/`mt_srand`
have no equivalent, because seeding the global generator is exactly what that separation removes.

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `Uuid::v4` | `v4(): Uuid` | `uniqid`, `com_create_guid`, userland UUID libraries | neutral |
| `Uuid::v7` | `v7(): Uuid` | nothing — time-ordered, for database keys | neutral |
| `Uuid::parse` | `parse(string $s): Uuid` | manual validation | |
| `Uuid::isValid` | `isValid(string $s): bool` | a regex | neutral |
| `Hash::of` | `of(bytes\|string $data, Digest $digest): bytes` | `hash`, `md5`, `sha1`, `crc32`, `openssl_digest` | neutral |
| `Hash::hmac` | `hmac(bytes\|string $data, secret bytes $key, StrongDigest $digest): bytes` | `hash_hmac` | neutral |
| `Hash::equals` | `equals(bytes $a, bytes $b): bool` | `hash_equals` — constant-time | neutral |
| `Hash::stream` | `stream(Digest $digest): Hash\Stream` | `hash_init`/`hash_update`/`hash_final`, `HashContext` | |

`Digest` carries every algorithm including `Md5`, `Sha1` and `Crc32`, because checksum interop genuinely
needs them. `StrongDigest` is the closed subset ([ADR 0047](../adr/0047-literal-and-enum-case-types.md))
that the HMAC and signature members declare, so `Hash::hmac($m, $k, Digest::Md5)` is a compile error naming
the reason. Password hashing takes no algorithm argument at all and is in § 16.

## 12. `Core\Uri`, `Core\Validate`, `Core\Csv`, `Core\Out`

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `Uri::parse` | `parse(string $uri): Uri` | `parse_url` | |
| `Uri::isValid` | `isValid(string $uri): bool` | `filter_var(…, FILTER_VALIDATE_URL)` | neutral |
| `Uri::encodeComponent` / `decodeComponent` | `encodeComponent(string $s): string` | `rawurlencode`, `rawurldecode` | |
| `Uri::encodeFormValue` / `decodeFormValue` | `encodeFormValue(string $s): string` | `urlencode`, `urldecode` (the `+`-for-space variant) | |
| `Uri::parseQuery` | `parseQuery(string $query): array<string>` | `parse_str` — returns, never populates variables | |
| `Uri::buildQuery` | `buildQuery(array<string> $parameters): string` | `http_build_query` | |
| `$uri->with` | `$uri->with({scheme?, host?, port?, path?, query?, fragment?}): Uri` | manual reassembly | |
| `$uri->resolve` | `$uri->resolve(string $reference): Uri` | nothing | |

`Uri::parse` is an ADR 0057 intrinsic. Note what is **not** here: `Core\Uri` never decides whether a URL
may be *fetched* — that is `Core\Http::allowUrl` in § 16, the SSRF launderer
([ADR 0058](../adr/0058-outbound-request-policy.md)).

`Core\Validate` is what survives of `filter`: the genuine validators only. Its *sanitizing* filters are
dropped, because half-escaping produces exactly the false confidence
[ADR 0024](../adr/0024-taint-tracking-for-injection-sinks.md) exists to prevent — **no `Validate` member
launders anything.**

`isEmail`, `isUrl`, `isIp`, `isIpV4`, `isIpV6`, `isMac`, `isDomain`, `isAscii`, `isPrintable`,
`isInteger`, `isFloat`, `isBoolean`, `oneOf(mixed $value, array<mixed> $allowed): bool` — all
`(subject, …): bool`, all neutral. Replaces `filter_var`'s validate half and its 20 `FILTER_*` constants.

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `Csv::parse` | `parse(string $text, {separator?, quote?, escape?, header?: bool}): array<array<string>>` | `str_getcsv`, the parsing half of `fgetcsv` | |
| `Csv::format` | `format(array<array<string>> $rows, {separator?, quote?, header?: array<string>}): string` | `fputcsv`'s formatting half | |
| `Out::capture` | `capture(callable $fn): string` | `ob_start`/`ob_get_clean` | |
| `Out::filtered` | `filtered(callable $fn, callable $filter): string` | `ob_start($callback)` | |

`Core\Out` has exactly these two members. A buffer is scoped to a closure and nests by call nesting, so
PHP's global `ob_*` stack — start in one function, end in another, ten functions to inspect the stack — has
no equivalent, and neither does implicit flushing.

## 13. Compiler-facing surfaces

These are `Core` classes whose semantics live in their own ADRs; this file fixes only that they follow the
same shape rules.

| Class | Owns | ADR |
|---|---|---|
| `Core\Reflect` | `forClass`, `forObject`, `typeOf(mixed): TypeKind`, and the visibility-respecting walk. Replaces `ReflectionClass` **and** `get_class`, `get_object_vars`, `get_class_methods`, `method_exists`, `property_exists`, `class_exists`, `is_a`, `is_subclass_of`, `class_implements`, `spl_object_id` | [0019](../adr/0019-reflection-and-ast-parsing-are-core-features.md) |
| `Core\Ast` | `parse(string): Node` returning inert typed data | [0019](../adr/0019-reflection-and-ast-parsing-are-core-features.md) |
| `Core\Attributes` | `get<T>`, `all<T>` — structural, not a `Reflect` walk | [0046](../adr/0046-attributes-shape-literal-metadata.md) |
| `Core\Program` | `implementing<T>()` | [0061](../adr/0061-compile-time-autoload-and-program-discovery.md) |
| `Core\Decimal`, `Core\BigInt` | the non-operator members of the `decimal` scalar and arbitrary-precision integers. Replaces `bcmath`, `gmp` | [0054](../adr/0054-decimal-scalar-type.md) |
| `Core\Test` | `assert(bool, {message?})`, `assertEquals`, `assertThrows(callable, string $class)`, `assertMatches`. The runner is M10 tooling, not this surface | — |

`Core\Reflect::typeOf` is the single replacement for PHP's 14 `is_*` predicates plus `gettype`: they are
only meaningful on a `mixed`, and the checker already knows every other case.

---

# Part II — the capability-bearing half (M8)

Everything below needs a capability grant, the reactor, an open handle or a driver. It is listed at one
line per member because the semantics are owned by [ADR 0051](../adr/0051-standard-library-tiers.md)'s
roster and by each subsystem's own ADR; the shape rules are identical to Part I's.

## 14. `Core\IO`

Every member is an [ADR 0024](../adr/0024-taint-tracking-for-injection-sinks.md) **path sink** and requires
an `fs.read` or `fs.write` capability. `resource` is never exposed — an open file is a `Core\IO\File`
object (R14).

- **Whole-file:** `read(string $path): bytes`, `readText(string $path, {charset?}): string`,
  `write(string $path, bytes|string $data)`, `append(…)`, `lines(string $path): Iterable<string>` —
  replacing `file_get_contents`, `file_put_contents`, `file`, `readfile`, `fpassthru`.
- **Metadata:** `exists`, `isFile`, `isDir`, `isReadable`, `isWritable`, `size`, `modifiedAt`, `stat` —
  replacing `file_exists`, `is_file`, `is_dir`, `filesize`, `filemtime`, `fileperms`, `stat`, `lstat`.
- **Manipulation:** `copy`, `move`, `remove`, `makeDir`, `removeDir`, `list(string $path): array<string>`,
  `walk(string $path): Iterable<string>`, `temporaryFile`, `temporaryDir` — replacing `copy`, `rename`,
  `unlink`, `mkdir`, `rmdir`, `scandir`, `glob`, `opendir`/`readdir`/`closedir`, `tempnam`, `tmpfile`,
  `sys_get_temp_dir`, and the `DirectoryIterator` family.
- **Resolution:** `canonicalize(string $path): string` (`realpath`), and `within(string $base, tainted
  string $path): string` — **the path-traversal launderer**: it resolves and then proves containment,
  which is the check `Core\Path::normalize` structurally cannot make.
- **Handles:** `open(string $path, FileMode $mode): File`; `$file->read`, `->readLine`, `->write`,
  `->seek`, `->tell`, `->truncate`, `->flush`, `->lock`, `->close`. `FileMode` is an enum, never a
  mode string (R11) — replacing `fopen`'s `"r+b"` grammar and the whole `fread`/`fgets`/`fwrite`/`fseek`/
  `ftell`/`feof`/`flock`/`fstat` family plus `SplFileObject`.
- **Standard streams:** `IO::stdin()`, `IO::stdout()`, `IO::stderr()`.

No stream wrappers, no `php://`, no `phar://`, no user-registered protocols
([ADR 0052](../adr/0052-closed-doors.md)).

## 15. Request-facing: `Core\Server`, `Core\Request`, `Core\Response`, `Core\Session`, `Core\Env`, `Core\Cli`

These replace PHP's superglobals ([ADR 0012](../adr/0012-no-superglobals.md)); every value they return that
originates outside the process is `tainted` ([ADR 0024](../adr/0024-taint-tracking-for-injection-sinks.md)).

- `Core\Request`: `method`, `path`, `query`, `body`, `header`, `headers`, `cookie`, `files`, `clientIp` —
  replacing `$_GET`, `$_POST`, `$_FILES`, `$_COOKIE`, `$_REQUEST`, `filter_input`.
- `Core\Response`: `setStatus`, `setHeader`, `addCookie`, `write`, `redirect`, `sendFile` — replacing
  `header`, `headers_sent`, `setcookie`, `setrawcookie`, `http_response_code`.
- `Core\Server`: the request's own environment — replacing `$_SERVER`.
- `Core\Session`: `get`, `set`, `remove`, `clear`, `regenerate`, `destroy` — replacing all ~25 `session_*`
  functions. May not use `Core\Cache` ([ADR 0059](../adr/0059-cross-request-state-is-explicit.md)).
- `Core\Env`: `get(string): ?tainted string`, `all()`, and the constants `EOL`, `OS`, `VERSION`. Read-only —
  `putenv` has no equivalent, because a process-global mutation is unsound across cores.
- `Core\Cli`: `arguments(): array<tainted string>`, `readLine(): ?tainted string`, `isTty()`,
  `terminalWidth()` — replacing `$argv`, `$argc`, `readline`. Process exit is the `exit` keyword
  ([ADR 0049](../adr/0049-single-open-tag-and-single-exit-keyword.md)).

## 16. Network, data and crypto

| Class | Surface | ADR |
|---|---|---|
| `Core\Http\Client` | `get`, `post`, `send(Request)`, `stream`; `Core\Http::allowUrl` is the SSRF launderer that pins an address. Replaces all ~30 `curl_*` functions and their handle | [0058](../adr/0058-outbound-request-policy.md) |
| `Core\Net` | TCP/UDP/Unix sockets over the runtime's own reactor. Replaces `socket_*`, `stream_socket_*`, `fsockopen` — three PHP APIs for one job | [0051](../adr/0051-standard-library-tiers.md) |
| `Core\Db` | `connect`, `query`, `execute`, `transaction`, prepared statements only; SQL text is a sink. Replaces `PDO` **and** the procedural `mysqli`/`pgsql`/`sqlite3` APIs | [0051](../adr/0051-standard-library-tiers.md) |
| `Core\Crypto` | AEAD only, no ECB, no unauthenticated CBC, no cipher-name-as-string. Replaces `openssl_*`'s primitive half and `sodium_*` | [0051](../adr/0051-standard-library-tiers.md) |
| `Core\Password` | `hash(secret string): string`, `verify(secret string, string): bool`, `needsRehash(string): bool` — **no algorithm argument**. Replaces `password_hash`, `password_verify`, `crypt` | [0063](../adr/0063-core-api-conventions.md) |
| `Core\Jwt`, `Core\Csrf`, `Core\Totp`, `Core\SignedCookie` | the closed four-entry roster; a JWT's algorithm comes from the key, never the token | [0060](../adr/0060-application-security-protocols.md) |
| `Core\Process` | `run`, `spawn` — argv only, never a shell string | [0044](../adr/0044-core-process-argv-only-no-shell.md) |
| `Core\Mail` | an SMTP client with structured headers. Replaces `mail()` | [0051](../adr/0051-standard-library-tiers.md) |
| `Core\Cache` | `local`, `shared`; copy-in/copy-out. Replaces `apcu_*`, `memcached` for the local case | [0059](../adr/0059-cross-request-state-is-explicit.md) |
| `Core\Log`, `Core\Fatal` | structured logging; both are `secret` sinks | [0020](../adr/0020-error-escalation-ladder.md) |
| `Core\Debug` | coverage, tracing, profiling control; the `dump` that replaces `var_dump`, `print_r`, `var_export`, `debug_zval_refcount` | [0018](../adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md) |
| `Core\Signal` | graceful shutdown only. What remains of `pcntl_*` after `fork` is refused | [0051](../adr/0051-standard-library-tiers.md) |
| `Core\Os` | process and host facts (`pid`, `hostname`, `cpuCount`, `memoryUsage`, `loadAverage`). Replaces `posix_*` minus fork, `php_uname`, `memory_get_usage`, `getrusage`, `sys_getloadavg` | [0051](../adr/0051-standard-library-tiers.md) |
| `Core\Config` | `set(string, string): bool`, `get(string): ?string`, `restore(string): void`, `all(): array<string, string>` — the request-local overlay over `mwl.toml`. Replaces `ini_set`, `ini_get`, `ini_restore`, `ini_get_all`, `set_time_limit`. String-in/string-out because the directive name is dynamic; the registry parses with the same parser the boot path uses | [0005](../adr/0005-config-changeability.md), [0064](../adr/0064-configuration-file-format.md) |

## 17. Documents and formats

| Class | Surface | Note |
|---|---|---|
| `Core\Html` | `escape` (the auto-applied launderer), `sanitize`, `Markup` | [ADR 0024](../adr/0024-taint-tracking-for-injection-sinks.md) owns both launderers |
| `Core\Xml` | one API replacing DOM, SimpleXML, XMLReader, XMLWriter, `xml_parser_*` and XSLTProcessor. Its **tree** API and its **streaming** reader/writer are different jobs, not twins — the tree materialises, the stream does not, and no operation is available through both | the one place in this file where two shapes of the same subsystem coexist, stated explicitly so it is not read as an exception to R17 |
| `Core\Compress` | gzip, deflate, brotli, zstd — one API replacing `gzopen` handles, `deflate_init` contexts and `zlib.*` stream filters | |
| `Core\Zip` | Core rather than an extension because `../` entries, symlink entries and decompression bombs are *policy*, and policy must be non-optional | [ADR 0051 § 3](../adr/0051-standard-library-tiers.md) |
| `Core\Mime` | type detection by magic bytes, not by libmagic's rule interpreter | |

---

## Counting the result

| | PHP | MWL |
|---|---|---|
| Global functions / `Core` members | ~1,900 | ~450 |
| Sort functions | 11 + `array_multisort` | 2 |
| `array_diff`/`array_intersect` variants | 12 | 2 |
| `strpos` variants | 12 | 4 |
| `printf` variants | 9 | 2 |
| Date APIs | ~40 procedural + 2 mutable/immutable class trees | 1 immutable object tree |
| XML APIs | 6 | 1 |
| Socket APIs | 3 | 1 |
| Ways to run a program | 7 | 1 |
| Ways to hash | 3 | 1 |
| Exception classes in the stdlib | 13 SPL + 8 `Error` | 7 |

The reduction is entirely in restatements. The library covers strictly more than PHP's: an HTTP client,
SMTP, cache, CSV, UUID, a test surface, [ADR 0060](../adr/0060-application-security-protocols.md)'s
protocol roster, and typed date arithmetic are all things PHP leaves to userland.
