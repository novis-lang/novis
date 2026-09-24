# Novis Specification — 01: The `Core` library

**This file is authoritative for every `Core` signature.** It is the member list
[ADR 0011 § 2](../decisions/0011.md) deferred and
[ADR 0051 § 3](../decisions/0051.md) placed. The *shape* rules every entry obeys are
`rule:core-api/shape-rules` — read that first; this file applies it and does not
re-argue it. Where a class has its own ADR (`Core\Regex`, `Core\Decimal`, `Core\Reflect`, `Core\Process`,
`Core\Cache`, …) that ADR owns the semantics and this file owns only the signatures. For an *implemented*
member, `rule:core-api/reference-card` puts its
reference documentation in the `nvs-stdlib` registry declaration, exposed by `nvs meta --json`, and a doc
field the registry carries wins over this file, field by field; this file remains authoritative for every
member the registry does not yet hold, and for the designed signature of everything.

## How to read an entry

Every member is `public static` on its domain class unless the entry shows a `$receiver->`, which marks an
instance method. The `public static function` prefix is omitted throughout; `Core\Str::length` is written
`length(string $s): uint`. **The `$name` in a signature is callable**: `Str::length(s: $x)` binds exactly as
it would at a user-declared method, and a trailing options shape is addressable as `options:` —
`rule:core-api/shape-rules` R2, which is also why a parameter's name is versioned here
like its type.

**A `Core` value object's readonly members are reached as readers** — `$write->affected()`, never
`$write->affected` — because a `Core` instance has no property a program can reach: its slots are
`nvs-stdlib`'s layout rather than a surface, which `nvs_stdlib::registry::CoreTy::Instance` owns. An entry
below written `readonly x: T` means the reader `->x(): T`, and the same rule is why `Core\RateLimit`'s
`Decision` and `Core\Script`'s `ExitReport` answer through members. The exception classes of § 10 are the
other roster and do have properties, which is what the entry at that section says.

The **Replaces** column names the PHP built-ins an entry subsumes. It is one of the two inputs to
[02-php-migration.md](02-php-migration.md), which is the complete PHP-name → outcome table `nvs convert`
(M11) is generated from and the only place that can answer "did we drop something real": this file states
what Novis *has*, and that one accounts for every PHP name Novis does not.

**Qualifier** is the `rule:security/tainted-qualifier`/[0033](../decisions/0033.md)
classification, and every member has one:

| Mark | Meaning |
|---|---|
| *(blank)* | **contagious** — a `tainted`/`secret` argument yields a `tainted`/`secret` result. The overwhelming majority |
| **sink** | refuses a qualified argument in the named position |
| **launder** | removes a qualifier; its contract names the sink it launders for |
| **neutral** | the result never carries a qualifier from the argument (a `bool`, a count, a hash of a secret) |

A blank cell here means *contagious was chosen*, never *nobody looked*: the classification is declared per
parameter in `nvs-stdlib`'s member registry, an unclassified `string`/`bytes` parameter **refuses** a
tainted argument, and a member that ships with one fails that crate's own test suite
(`rule:security/unclassified-parameter-refuses-tainted`). Which parameters are
sinks follows from that ADR's § 1 predicate — *the content becomes an instruction something executes,
rather than data something returns or frames* — of which one corollary is that all four
[R11](../decisions/0063.md) grammars are sinks.

Two conventions apply throughout and are not repeated per entry:

- **Ranges** are `(offset, ?length)` — negative offset counts from the end, negative length stops that many
  from the end, `null` runs to the end (`rule:core-api/shape-rules` R8).
- **A variadic member takes no options shape.** R2 requires the options shape to be last, and a variadic
  parameter already is; where both are wanted the member takes an `array` instead of a variadic.

## What is a member and what is an operator

`Core\Arr` covers whole-array operations only. Element access stays in the language, and this is what
keeps the two from becoming two spellings of one thing:

| Operation | Spelling | Note |
|---|---|---|
| read an element | `$a[$k]` | throws on a missing key (`rule:core-api/shape-rules` R4) |
| read a possibly-absent element | `$a[$k] ?? $default` | the absence-tolerant spelling; there is **no** `Arr::get` |
| write an element | `$a[$k] = $v` | |
| remove an element | `unset($a[$k])` | an array element of a named holder, and nothing else — a declared property, static or instance, and every other operand alike are refused (`rule:classes/unset-is-refused-on-a-property`) |
| ask whether a key exists | `Arr::hasKey($a, $k)` | `isset()`/`array_key_exists` both collapse here |
| combine two arrays | `Arr::overlay` / `underlay` / `appendAll` | `$a + $b` does **not** compile (`rule:types/array-combination`) |

The same division applies elsewhere: `**` is exponentiation, so there is no `Math::pow`; `%` is integer
modulo, so `Math::mod` exists only for the `float` case; `is` is an operator, so `Core\Reflect` has
no `isA`.

## Milestones

**M4S** builds every class in §§ 1–12: they are pure, need no capability, no reactor, no driver, and no
open file — so they can be implemented, tested and depended upon before the HTTP server exists. **M8**
builds §§ 14–17, which need one of those things, under the same rules and the same conformance checks.

Three entries in Part II land earlier than M8 because their dependency is not a capability:
**`Core\Task`** (§ 19) needs the M5 scheduler and lands with it; **`Core\Metrics`** and its exporter (§ 16)
need the M7 server's own series to be testable and land there; and **`Core\Router`** (§ 13) is split — the
compile-time table with M4S's other attribute pass, the matcher with M7.

§ 13 is neither: each of its entries is pure, but each also waits on something outside `Core`.
`Core\Program` needs `rule:programs/no-runtime-autoload`'s `autoload`,
`Core\Ast` needs `rule:tooling/reflection-and-source-parsing-are-core-features`'s inert-AST
surface, `Core\Attributes`' retrieval body is M8 by `rule:attributes/inert-metadata`
(its `#[...]` *syntax* is M4), and `Core\Reflect`, `Core\Decimal` and `Core\BigInt` want a finished object
representation under them. They land with whichever milestone closes their dependency, to this same
contract. `Core\Test` is the one entry whose schedule is already fixed rather than dependency-driven:
[ADR 0079](../decisions/0079.md) § 24 names the milestone for each of its pieces,
from the assertions at the M4S tail to `#[Bench]` and mutation testing at M10.

---

# Part I — the pure core (M4S)

## 1. `Core\Str`

`string` is guaranteed-valid UTF-8 (`rule:types/bytes`), so **no member takes an
encoding argument** (R13) and there is no `mb_` twin of anything. Indexing granularity — what `length`, `at`
and `slice` count in — is `rule:types/string-is-utf8`'s, not this file's: **extended
grapheme clusters**. Every signature below is written granularity-agnostically, so nothing here changes if
that ADR's own *Revisiting* ever re-opens it.

### Inspection

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `length` | `length(string $s): uint` | `strlen`, `mb_strlen` | neutral |
| `isEmpty` | `isEmpty(string $s): bool` | `$s == ""` | neutral |
| `at` | `at(string $s, int $index): string` | `$s[$i]`, `mb_substr($s,$i,1)` | |
| `contains` | `contains(string $haystack, string $needle): bool` | `str_contains`, `strstr` as a predicate | neutral |
| `startsWith` | `startsWith(string $s, string $prefix): bool` | `str_starts_with` | neutral |
| `endsWith` | `endsWith(string $s, string $suffix): bool` | `str_ends_with` | neutral |
| `indexOf` | `indexOf(string $haystack, string $needle, {from?: int, caseInsensitive?: bool}): ?uint` | `strpos`, `stripos`, `mb_strpos`, `mb_stripos` | neutral |
| `lastIndexOf` | `lastIndexOf(string $haystack, string $needle, {before?: int, caseInsensitive?: bool}): ?uint` | `strrpos`, `strripos`, `mb_strrpos` | neutral |
| `countOf` | `countOf(string $haystack, string $needle): uint` | `substr_count` | neutral |
| `compare` | `compare(string $a, string $b, {caseInsensitive?: bool, natural?: bool}): int` | `strcmp`, `strcasecmp`, `strnatcmp`, `strnatcasecmp`, and the comparator behind `natsort`/`natcasesort` | neutral |

Case-insensitivity is an option here exactly as it is on `indexOf`, `replace` and the rest, rather than a
second member name. `{natural: true}` selects a **different ordering**, not a variant of the same one —
`compare("img12", "img2")` is negative and becomes positive under it — so a natural sort is
`Arr::sort($a, {comparator: fn($x, $y) => Str::compare($x, $y, {natural: true})})`. `strncmp`'s
length-limited form is `Str::slice` first.

`strcoll` and every locale-sensitive comparison are **not** here: Novis has no ambient locale
(`rule:core-api/tier-placement`), and locale-aware collation is the intl extension's
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

`before` and `after` both **exclude the needle**, which is where `after` parts company with `strstr`: PHP
returns the needle and everything past it, and the port of a program that wanted that is
`$needle . Str::after(...)`. Two members named against the same needle that each drop it are the pair a
reader can predict. `{last: true}` cuts at the last occurrence instead, which is all `strrchr` added, and a
needle that does not occur is `null` where `strstr` is `false` (R5).

`lines` splits on `\n`, `\r\n` and a lone `\r` alike, and a trailing terminator does **not** produce a final
empty element — the platform's own line ending is never consulted, which is why there is no `PHP_EOL`
equivalent to pass it.

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
| `fromCodePoints` | `fromCodePoints(array<uint> $codePoints): string` | `implode(array_map("mb_chr", …))` | neutral |
| `format` | `format(string $template, mixed ...$arguments): string` | `sprintf`, `vsprintf`, `printf`, `vprintf`, `fprintf`, `vfprintf` | **sink** (template) |

`format` is an `rule:expressions/intrinsic-literals` intrinsic: a literal template has its
placeholder count and types checked against the argument list at compile time, which turns PHP's
`printf`-argument-mismatch bug family into a diagnostic. Its template grammar is **`printf`'s**, kept
deliberately — a closed conversion list (`%s %d %u %f %e %g %x %X %o %b %%`) with `printf`'s flag, width,
precision and `%1$s` positional syntax, minus everything that reads ambient state. It is a *grammar*, not a
mode string, so R11 does not reach it; the same is true of `Core\Regex`'s patterns, `Core\Bytes::pack`'s
format and CLDR date patterns, and those four are the only ones in the library. **All four are `tainted`
sinks**, because a grammar is an instruction rather than data
(`rule:security/sink-predicate` and `rule:security/every-grammar-is-a-sink`): a tainted `format`
template hands an attacker `%2$s` and `%999999999d`. The *arguments* stay contagious, and none of the four
gets a launderer except `Regex::quote` — a grammar is written by the program, so the fix at a rejected call
site is a literal, which `rule:expressions/intrinsic-literals` already folds.

`ucwords` and title casing are **not** here — word segmentation is locale-dependent and belongs to intl;
the mechanical rewrite for an ASCII-ish name is
`Str::join(Arr::map(Str::split($s, " "), fn($w) => Str::upperFirst($w)), " ")`.

PHP's `ctype_*` family has **no member and no replacement class**. Each one is a character-class question,
which is what `Core\Regex` is for — `ctype_alpha($s)` is `Regex::matches($s, "^\\p{L}+$")` — except the two
numeric ones, which are a *type* question and therefore an `as`: `ctype_digit($s)` is
`$s as ?uint != null` (`rule:expressions/nullable-conversion`). Adding them as members
would import ASCII-only semantics into a type that guarantees UTF-8, which is the mistake
`rule:types/bytes` exists to prevent; `Core\Validate::isAscii` and `isPrintable`
are here precisely because they *are* about the ASCII range and say so.

Enums: `NormalForm { Nfc, Nfd, Nfkc, Nfkd }`.

## 2. `Core\Arr`

`array<T>` is an insertion-ordered hash with copy-on-write value semantics
([ADR 0007 § 5](../decisions/0007.md)). Every member is pure (R3); an implementation
mutates in place whenever the argument's refcount is 1, so purity costs nothing. `T` is a type variable —
the stdlib is parametric where user code is not.

Every key is a `string`, so every key-valued **return** below is typed `string`; a key **parameter** is
`int|string`, matching the subscript normalisation of that same section. How arrays combine, and why no
member is named `merge`, is `rule:types/array-combination`.

### Inspection

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `count` | `count(array<T> $a): uint` | `count`, `sizeof` | neutral |
| `isEmpty` | `isEmpty(array<T> $a): bool` | `empty($a)`, `count($a) == 0` | neutral |
| `isList` | `isList(array<T> $a): bool` | `array_is_list` | neutral |
| `hasKey` | `hasKey(array<T> $a, int\|string $key): bool` | `array_key_exists`, `isset` | neutral |
| `contains` | `contains(array<T> $haystack, T $needle): bool` | `in_array` (always strict) | neutral |
| `keyOf` | `keyOf(array<T> $haystack, T $needle): ?string` | `array_search` | neutral |
| `keys` | `keys(array<T> $a): array<string>` | `array_keys` | |
| `values` | `values(array<T> $a): array<T>` | `array_values` | |
| `first` | `first(array<T> $a): ?T` | `reset`, `current`, `$a[array_key_first($a)]` | |
| `last` | `last(array<T> $a): ?T` | `end`, `$a[array_key_last($a)]` | |
| `firstKey` | `firstKey(array<T> $a): ?string` | `array_key_first`, `key` | neutral |
| `lastKey` | `lastKey(array<T> $a): ?string` | `array_key_last` | neutral |

PHP's internal array pointer (`current`/`key`/`next`/`prev`/`reset`/`end`/`each`) has no equivalent: a
mutable cursor inside a copy-on-write *value* is incoherent, since copying the array would copy its
iteration position. `foreach` and the four members above cover every use.

Over an `array<?T>`, the `?T` returned by `first`, `last`, `find`, `min`, `max` and `Random::pick` cannot
distinguish "absent" from "present and null". That is accepted rather than patched with a second return
shape: `isEmpty` and `hasKey` answer the question directly, and every alternative costs a union at every
call site to serve a case a program rarely has.

### Structure

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `slice` | `slice(array<T> $a, int $offset, ?int $length = null, {preserveKeys?: bool}): array<T>` | `array_slice` | |
| `replaceRange` | `replaceRange(array<T> $a, int $offset, ?int $length, array<T> $replacement = []): array<T>` | `array_splice` (returning, not by-reference) | |
| `chunk` | `chunk(array<T> $a, uint $size, {preserveKeys?: bool}): array<array<T>>` | `array_chunk` | |
| `append` | `append(array<T> $a, T ...$values): array<T>` | `array_push`, `$a[] = $v` in expression position | |
| `prepend` | `prepend(array<T> $a, T ...$values): array<T>` | `array_unshift` | |
| `withoutFirst` | `withoutFirst(array<T> $a): array<T>` | `array_shift`'s remainder (`first` gets the element) | |
| `withoutLast` | `withoutLast(array<T> $a): array<T>` | `array_pop`'s remainder (`last` gets the element) | |
| `padStart` | `padStart(array<T> $a, uint $size, T $value): array<T>` | `array_pad` with a negative size | |
| `padEnd` | `padEnd(array<T> $a, uint $size, T $value): array<T>` | `array_pad` | |
| `reverse` | `reverse(array<T> $a, {preserveKeys?: bool}): array<T>` | `array_reverse` | |
| `flip` | `flip(array<int\|string> $a): array<string>` | `array_flip` | |
| `flatten` | `flatten(array<array<T>> $a): array<T>` | one level of a hand-written recursive walk | |
| `flattenDeep` | `flattenDeep(array<mixed> $a): array<mixed>` | `iterator_to_array` on a recursive iterator, a hand-written recursive walk | |
| `fill` | `fill(uint $count, T $value): array<T>` | `array_fill` | |
| `fillKeys` | `fillKeys(array<int\|string> $keys, T $value): array<T>` | `array_fill_keys` | |
| `range` | `range(int $start, int $end, {step?: int}): array<int>` | `range` | neutral |
| `fromKeysAndValues` | `fromKeysAndValues(array<int\|string> $keys, array<T> $values): array<T>` | `array_combine` | |
| `from` | `from(Iterable<T>\|Iterator<T>\|array<T> $items, {limit?: uint}): array<T>` | `iterator_to_array`, `iterator_count`'s materialising half | |
| `column` | `column(array<array<T>> $a, int\|string $column, {indexBy?: int\|string}): array<T>` | `array_column` | |

`fromKeysAndValues` throws when the two arrays differ in length (R4). `flip` collapses duplicate values,
the last occurrence winning. `from` takes all three of the shapes
`rule:iteration/foreach-subjects` lets `foreach` take, an `array<T>` included, so a
member reading a sequence never refuses what a loop over the same value would accept. It drains its
argument once and always returns a list — a generator yields no keys (§ 5), and an array's are discarded
for the same reason `values` discards them, so the result's shape does not depend on which shape went in.
`{limit: n}` stops the *drive* after `n` elements, which is the only guard against materialising an
unbounded generator. `flatten` unwraps one
level and `flattenDeep` recurses, the same pairing as `overlay`/`overlayDeep`; neither takes a depth count,
because every real call means one of those two. Their parameter types differ because only one of the two
has a depth an element type can state: `flatten` unwraps exactly one level, so `array<array<T>>` in and
`array<T>` out is exact, while `flattenDeep`'s depth is the caller's data — a nested `T` would bind one
level too shallow over a three-deep argument and the declared answer would then claim a nesting the real
one does not have. `mixed` on both sides is therefore the *sound* spelling rather than a weak one, and a
caller that knows the depth is two reaches for `flatten` and keeps its `T`. **`{preserveKeys: false}` — the default wherever it appears — discards *every*
key and renumbers from `"0"`**; `true` keeps every key. PHP renumbers integer keys and silently keeps string
ones, which is the key-type-dependent behaviour
`rule:types/preserve-keys` removes.

`padStart`/`padEnd` **always return a list**, and take no `preserveKeys` option: padding *adds* entries, and
there is no non-arbitrary key for an added one beside an existing map's, so keeping is not a choice that can
be offered. `withoutFirst`/`withoutLast` are the opposite case — they add nothing, so every surviving key is
kept. `fill` drops PHP's `$start_index`, which produces a list that does not start at zero; the keys a caller
actually wants are `fillKeys`.

### Combining and set operations

Three members combine arrays, and **each treats every key the same way** — there is no member named
`merge`, and `array + array` does not compile. The rules, the key order each produces and `nvs convert`'s
rewrite table are `rule:types/array-combination`.

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `overlay` | `overlay(array<T> $base, array<U> ...$layers): array<T\|U>` | `array_replace`, `array_merge` over maps | |
| `overlayDeep` | `overlayDeep(array<T> $base, array<U> ...$layers): array<T\|U>` | `array_replace_recursive` | |
| `underlay` | `underlay(array<T> $base, array<U> ...$layers): array<T\|U>` | the `+` operator | |
| `appendAll` | `appendAll(array<T> $a, array<U> ...$others): array<T\|U>` | `array_merge` over lists, `array_merge(...$arrays)` | |
| `diff` | `diff(array<T> $a, array<T> $b, {on?: SetOn, by?: callable, comparator?: callable}): array<T>` | `array_diff`, `array_udiff`, `array_diff_key`, `array_diff_assoc`, `array_diff_ukey`, `array_udiff_assoc` | |
| `intersect` | `intersect(array<T> $a, array<T> $b, {on?: SetOn, by?: callable, comparator?: callable}): array<T>` | `array_intersect` and its five variants | |
| `unique` | `unique(array<T> $a, {by?: callable}): array<T>` | `array_unique` | |
| `countBy` | `countBy(array<T> $a, {by?: callable}): array<uint>` | `array_count_values`, the userland group-and-count | neutral |

`overlay` keeps the right-hand value, `underlay` the left-hand one, and both keep an existing key in its
existing position; `appendAll` discards keys and always returns a list. `overlayDeep` recurses only where
both sides hold an array and neither is a list — a list is replaced wholesale.

`unique`, `diff` and `intersect` compare by **strict identity**, as `contains` does; PHP's default
string-cast comparison (`SORT_STRING`) is not reproduced. Twelve `array_diff*`/`array_intersect*` functions
become two members plus a three-case enum, the largest single reduction in the library.
`array_merge_recursive` has no replacement: promoting two colliding scalars into a two-element array is a
data-shape change, not a merge.

### Iteration and aggregation

Every callback receives `($value, $key)` and may declare fewer parameters (R9), which is what removes
`ARRAY_FILTER_USE_KEY`, `ARRAY_FILTER_USE_BOTH` and the need for `…WithKey` twins. `reduce` prefixes that
pair with the accumulator — `($carry, $value, $key)` — since a fold has nowhere else to put it. `$key` is a
`string` wherever it is offered, on a list exactly as on a map
(`rule:types/arrays`).

`reduce`'s `U` is bound by `$initial`, so the fold's type is the seed's: a fold building a string starts
from `""`, and an empty array is that seed returned unchanged with no call made.

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `map` | `map(array<T> $a, callable(T, string): U $fn): array<U>` | `array_map` | |
| `mapKeys` | `mapKeys(array<T> $a, callable(T, string): int\|string $fn): array<T>` | `array_combine(array_map(...), …)`, the `keyBy` idiom | |
| `filter` | `filter(array<T> $a, callable(T, string): bool $predicate): array<T>` | `array_filter` and its two flags | |
| `reduce` | `reduce(array<T> $a, callable(U, T, string): U $fn, U $initial): U` | `array_reduce` | |
| `find` | `find(array<T> $a, callable(T, string): bool $predicate): ?T` | `array_find` | |
| `findKey` | `findKey(array<T> $a, callable(T, string): bool $predicate): ?string` | `array_find_key` | neutral |
| `any` | `any(array<T> $a, callable(T, string): bool $predicate): bool` | `array_any` | neutral |
| `all` | `all(array<T> $a, callable(T, string): bool $predicate): bool` | `array_all` | neutral |
| `groupBy` | `groupBy(array<T> $a, callable(T, string): int\|string $key): array<array<T>>` | nothing — the most-written PHP userland helper | |
| `sum` | `sum(array<int\|float\|decimal> $a): int\|float\|decimal` | `array_sum` | neutral |
| `product` | `product(array<int\|float\|decimal> $a): int\|float\|decimal` | `array_product` | neutral |
| `average` | `average(array<int\|float\|decimal> $a): ?(float\|decimal)` | `array_sum($a)/count($a)`, with the empty case answered | neutral |
| `min` | `min(array<T> $a): ?T` | `min` with an array argument | |
| `max` | `max(array<T> $a): ?T` | `max` with an array argument | |

**`map`, `filter` and `groupBy` preserve every key**; `mapKeys` is the only member that changes one, and
`values` renumbers. A `groupBy` bucket therefore keeps each entry's own key: a partition cannot collide
two entries, so there is nothing for the renumbering `flatten` and `appendAll` do to protect against, and
`Arr::values` over a bucket recovers the list — the other direction is not recoverable. PHP's multi-array `array_map($fn, $a, $b)` and its `array_map(null, $a, $b)` zip have no member:
they are a `foreach` over `Arr::keys`, and a zip whose element type is `array<T|U>` would defeat the
element typing that makes the rest of this class checkable.

`array_walk` and `array_walk_recursive` have no member: `foreach` is the language's own spelling, and R3
removes the by-reference mutation that was their only reason to exist
(`rule:types/array-combination`).

### Ordering

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `sort` | `sort(array<T> $a, {by?: callable, order?: Order, comparator?: callable, preserveKeys?: bool}): array<T>` | `sort`, `rsort`, `asort`, `arsort`, `usort`, `uasort`, `natsort`, `natcasesort`, `array_multisort` | |
| `sortByKey` | `sortByKey(array<T> $a, {order?: Order, comparator?: callable}): array<T>` | `ksort`, `krsort`, `uksort` | |

Eleven sort functions plus `array_multisort` become two members. Descending is `{order: Order::Desc}`,
key-preservation is an option rather than a letter in the name, and `by` — a key-extractor closure — is
the thing `usort` callbacks are written to emulate. Both sorts are stable.

`by` and `comparator` **compose** rather than conflict, on `sort` and on `diff`/`intersect` alike: `by`
decides *what* is compared and `comparator` decides *how*, so a comparator given beside an extractor sees
the extracted keys. No combination of the options is refused — one rule fewer to remember, at no cost to
implement. On `diff`/`intersect`, `on` selects the part of an entry that is compared — the value, the key,
or both — and `by`/`comparator` apply to the part it selected, so a `by` under `SetOn::Keys` maps the key.

Enums: `Order { Asc, Desc }`, `SetOn { Values, Keys, Both }`.

## 3. `Core\Math`

`**` is exponentiation and `%` is integer modulo, so neither has a member (R17). Division by zero throws
`ArithmeticError` (R4) — in `intDiv`, in `mod`, and in the `/` operator whatever the operand types, which
is exactly why `fdiv` *is* a member here: it is the one spelling left for IEEE's `INF`, and PHP has it for
the same reason. Overflow throws rather than becoming a `float`
(`rule:types/arithmetic`).

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
| `fdiv` | `fdiv(float $a, float $b): float` | `fdiv` | neutral |
| `gcd` | `gcd(int $a, int $b): int` | `gmp_gcd` | neutral |
| `lcm` | `lcm(int $a, int $b): int` | `gmp_lcm` | neutral |
| `sqrt` | `sqrt(float $n): float` | `sqrt` | neutral |
| `cbrt` | `cbrt(float $n): float` | `pow($n, 1/3)` | neutral |
| `hypot` | `hypot(float $a, float $b): float` | `hypot` | neutral |
| `exp` | `exp(float $n): float` | `exp` | neutral |
| `log` | `log(float $n, {base?: float}): float` | `log`, `log10`, `log2` | neutral |
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
| `format` | `format(int\|float\|decimal $n, {decimals?: uint, decimalSeparator?: string, groupSeparator?: string}): string` | `number_format` | |

Constants: `PI`, `TAU`, `E`, `EPSILON`, `INT_MAX`, `INT_MIN`, `UINT_MAX`, `FLOAT_MAX`, `FLOAT_MIN`, `NAN`,
`INFINITY` — replacing `M_PI`, `M_E`, `PHP_INT_MAX`, `PHP_FLOAT_EPSILON` and the rest of PHP's global
constants (`rule:classes/no-free-functions-or-constants`).

`Math::format` takes explicit separators because Novis has no ambient locale; locale-correct number
formatting is intl's `NumberFormatter` equivalent, at Tier 1.

`log1p` and `expm1` are **not** folded into `log`/`exp`: they exist for precision near zero, which an
argument or a base cannot express, and neither is common enough in web or CLI code to earn a member. Their
rewrite is the naive form, with the precision loss stated rather than hidden.

Enums: `RoundMode { HalfUp, HalfDown, HalfEven, HalfOdd, Up, Down }`.

## 4. `Core\Time`

PHP has ~40 `date_*` procedural functions that are aliases of `DateTime` methods, *and* a
`DateTime`/`DateTimeImmutable` mutable/immutable pair. Both duplications are gone (R17, R20): there are
objects only, and every one is immutable. There is **no ambient timezone** — no process default, no
per-request default — so a `Zone` is an explicit argument at every instant↔calendar conversion.

**Two arithmetics, and the type says which one you get.** An `Instant` moves by a `Duration`, which is an
exact count of nanoseconds; a `DateTime` moves by a count of a `Unit`, which is a calendar step that a DST
boundary or a short month can make longer or shorter than its nominal length. `Time::now()->plus(72h)` and
`Time::now()->in($zone)->plus(3, Unit::Day)` are different operations, and PHP's `"+3 days"` is ambiguous
between them. There is no relative-expression string anywhere in this class: everything `strtotime` spells
is a typed call, and `rule:types/duration-literal`'s `72h`/`30d` literal is what keeps them
short.

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

`Core\Time::parse` is an `rule:expressions/intrinsic-literals` intrinsic — a literal format
string is validated and its plan prepared at compile time. **A CLDR pattern is a `tainted` sink** wherever
one is taken — `Time::parse`'s `$format` and every `format(string $pattern)` below — because it is one of
R11's four grammars, per `Core\Str::format`'s note in § 1; the `$text` being parsed is data and stays
contagious. **Patterns are CLDR** (`yyyy-MM-dd HH:mm:ss`,
`EEEE, d MMMM yyyy`), not PHP's `date()` letters: both grammars are closed and the argument is almost
always a literal, so `nvs convert` rewrites one into the other mechanically, and the intl extension needs
CLDR anyway. The same patterns serve `DateTime::format`. The **subset** of CLDR field letters implemented,
and the fact that a name renders in CLDR's root locale because there is no `setlocale`
(`rule:core-api/tier-placement`), are `crates/nvs-stdlib/src/cldr.rs`'s own docs; a
letter outside the subset is a diagnostic naming itself, never a silent literal.

PHP's free-form `strtotime` is **not** implemented, in either half. Every expression it accepts is a typed
call:

| PHP | Novis |
|---|---|
| `strtotime("now")` | `Time::now()->in($z)` |
| `strtotime("+3 days")` | `Time::now()->in($z)->plus(3, Unit::Day)` — or `Time::now()->plus(72h)` for an exact offset |
| `strtotime("today")` | `Time::now()->in($z)->startOf(Unit::Day)` |
| `strtotime("next monday")` | `Time::now()->in($z)->next(Weekday::Monday)` |
| `strtotime("first day of next month")` | `Time::now()->in($z)->startOf(Unit::Month)->plus(1, Unit::Month)` |
| `strtotime("last day of this month")` | `Time::now()->in($z)->endOf(Unit::Month)` |
| `strtotime("2024-03-01")` | `Time::parse($t, "yyyy-MM-dd", $z)` |
| `strtotime($userSuppliedRelativeString)` | `Duration::parse($s)`, for the exact-duration subset only |

A relative expression arriving at run time — a `retention = "30d"` in config, a `--since=7d` flag — is a
`Duration`, never a calendar step, because a count and a unit that are not known until run time are exactly
what `Duration::parse` takes. "Next monday" is not a value a config file supplies.

### `Core\Time\Instant` — an absolute point on the timeline

| Member | Signature | Notes |
|---|---|---|
| `in` | `$i->in(Zone $zone): DateTime` | the only instant→calendar conversion; a zone is never implicit |
| `toEpochSeconds` | `$i->toEpochSeconds(): int` | replaces `getTimestamp`, `date("U")` |
| `toEpochMillis` | `$i->toEpochMillis(): int` | plus `toEpochMicros`, replacing `microtime(true)`'s two halves |
| `plus` / `minus` | `$i->plus(Duration $d): Instant` | replaces `date_add`, `date_sub`, `modify` |
| `since` | `$i->since(Instant $earlier): Duration` | replaces `date_diff`, `DateInterval` arithmetic |
| `compareTo` | `$i->compareTo(Instant $other): int` | `Instant` implements `Comparable` (`rule:classes/comparable`), so `<`/`>` work directly |
| `toIso` | `$i->toIso(): string` | replaces `date(DATE_ATOM)` |

### `Core\Time\DateTime` — a civil date and time in a zone

| Member | Signature | Notes |
|---|---|---|
| `format` | `$d->format(string $pattern): string` | `rule:expressions/intrinsic-literals` intrinsic, CLDR patterns. Replaces `date`, `gmdate`, `idate`, `strftime`, `date_format` |
| `plus` / `minus` | `$d->plus(int $count, Unit $unit): DateTime` | calendar arithmetic: adding `1, Unit::Month` lands on the same day-of-month, clamped to the month's length, and crossing a DST boundary is a 23- or 25-hour day. Replaces `date_add`, `date_sub`, `modify`, `strtotime`'s relative half |
| `next` / `previous` | `$d->next(Weekday $w): DateTime` | the nearest strictly later (earlier) day with that weekday, time-of-day preserved. Replaces `strtotime("next monday")` |
| `with` | `$d->with({year?, month?, day?, hour?, minute?, second?, nanos?}): DateTime` | replaces `setDate`, `setTime`, `setISODate` |
| `withTime` | `$d->withTime(TimeOfDay $t): DateTime` | the common half of `with`, spelled as the operation it is |
| `startOf` / `endOf` | `$d->startOf(Unit $u): DateTime` | distinct operations at a DST boundary, which is why both exist |
| `difference` | `$d->difference(DateTime $other, Unit $unit): int` | whole units from the receiver **to** `$other`, counted in the *receiver's* zone — an age in years, a term in months. Replaces `date_diff` + `DateInterval`'s `y`/`m`/`d` fields |
| `toInstant` | `$d->toInstant(): Instant` | |
| `date` / `timeOfDay` / `zone` | `$d->date(): Date` | component views |
| `weekday` | `$d->weekday(): Weekday` | replaces `date("N")` |
| `dayOfYear` | `$d->dayOfYear(): uint` | replaces `date("z")` |
| `isLeapYear` | `$d->isLeapYear(): bool` | replaces `date("L")`, `checkdate`'s year half |

`Core\Time\Date` and `Core\Time\TimeOfDay` are the zone-free component types, with the same `plus`/`minus`/
`with`/`compareTo`/`format` shape and the constructors `Date::at(int $y, uint $m, uint $d)` and
`TimeOfDay::at(uint $hour, uint $minute, {second?: uint, nanos?: uint})`. `checkdate` has no equivalent
because constructing an invalid date throws.

### `Core\Time\Duration` and `Core\Time\Zone`

| Member | Signature | Notes |
|---|---|---|
| `Duration::seconds` | `seconds(int $n): Duration` | plus `nanoseconds`, `microseconds`, `milliseconds`, `minutes`, `hours`, `days`, `weeks` — for a **computed** count; a literal one is `rule:types/duration-literal`'s `30s` |
| `Duration::parse` | `parse(string $text): Duration` | the run-time form of that same literal grammar, one implementation for both. Throws on anything it does not accept, so it **launders** a `tainted` config value |
| `$d->toSeconds` | `$d->toSeconds(): int` | plus `toMilliseconds`, `toMicroseconds`, `toNanoseconds` |
| `$d->plus` / `minus` / `multipliedBy` / `negated` | `$d->plus(Duration $d): Duration` / `$d->minus(Duration $d): Duration` / `$d->multipliedBy(int $factor): Duration` / `$d->negated(): Duration` | `Duration` is `Comparable` and `Stringable`, emitting the literal grammar so it round-trips through `parse` |
| `Zone::of` | `of(string $id): Zone` | IANA identifier; throws on an unknown one. Replaces `DateTimeZone` |
| `Zone::fixed` | `fixed(Duration $offset): Zone` | a fixed offset from UTC, for a timestamp that carries one instead of a region |
| `Zone::system` | `system(): Zone` | the host's configured zone, read once at boot. Replaces `date_default_timezone_get` |
| `Zone::UTC` | constant | the only zone that is ever a default, and only where written explicitly |
| `$z->offsetAt` | `$z->offsetAt(Instant $i): Duration` | replaces `getOffset` |

`Zone::system()` is **not** an ambient default: it is an ordinary value a program asks for and then passes
explicitly, so a call site still names the zone it converts in. What has no equivalent is
`date_default_timezone_set` — nothing installs a zone that a later conversion silently picks up, which is
the unsoundness `rule:core-api/tier-placement` rejects `setlocale` for.

Enums: `Weekday { Monday … Sunday }`, `Month { January … December }`,
`Unit { Nanosecond, Microsecond, Millisecond, Second, Minute, Hour, Day, Week, Month, Quarter, Year }` —
full words, because R7's closed abbreviation list does not reach enum cases either.

PHP's `calendar` extension (`cal_days_in_month`, `easter_date`, the Julian/Jewish/French converters) is
dropped outright (`rule:core-api/tier-placement`); `cal_days_in_month` is
`$d->startOf(Unit::Month)->plus(…)` arithmetic, and the rest has no place in a Tier 0 library.

## 5. `Core\Regex`

Semantics — the two-tier engine, the step budget, why the *pattern* is a sink while the subject is not —
are `rule:core-classes/regex-two-tiers`. `preg_match`'s `$matches` out-parameter is a returned
`?Match` (R3), and there are no `PREG_*` flag constants (R11).

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `compile` | `compile(string $pattern, {caseInsensitive?, multiline?, dotAll?, ungreedy?}): Pattern` | the `/…/imsx` delimiter-and-modifier syntax | **sink** (pattern) |
| `matches` | `matches(string $subject, Pattern\|string $pattern): bool` | `preg_match` as a predicate | neutral |
| `match` | `match(string $subject, Pattern\|string $pattern, {from?: int}): ?Match` | `preg_match` + `$matches` + `PREG_OFFSET_CAPTURE` | **sink** (pattern) |
| `matchAll` | `matchAll(string $subject, Pattern\|string $pattern): array<Match>` | `preg_match_all` + `PREG_PATTERN_ORDER`/`PREG_SET_ORDER` | **sink** (pattern) |
| `replace` | `replace(string $subject, Pattern\|string $pattern, string $replacement, {limit?: uint}): string` | `preg_replace` | **sink** (pattern) |
| `replaceWith` | `replaceWith(string $subject, Pattern\|string $pattern, callable(Match): string $fn, {limit?: uint}): string` | `preg_replace_callback`, `preg_replace_callback_array` | **sink** (pattern) |
| `split` | `split(string $subject, Pattern\|string $pattern, {limit?: int, keepEmpty?: bool}): array<string>` | `preg_split` and its four flags | **sink** (pattern) |
| `quote` | `quote(string $literal): string` | `preg_quote` | **launder** (for the pattern sink) |

`Match` replaces the positional-array shape with four members:

| Member | Signature | Answers |
|---|---|---|
| `group` | `$match->group(int\|string $group): ?string` | one group's text; `null` where the pattern declares it and this match did not reach it, and a **throw** for a group the pattern does not declare |
| `groups` | `$match->groups(): array<?string>` | every group at once, in `preg_match`'s own order: a named group under its name, then under its number |
| `offset` | `$match->offset(): int` | where the whole match starts, in `rule:types/string-is-utf8`'s unit — not `PREG_OFFSET_CAPTURE`'s bytes |
| `text` | `$match->text(): string` | the whole match, which is group `0` |

`match`'s `from` is a position in that same unit, negative counting from the end (R8), and the match is
still made against the whole subject so a look-behind sees what precedes it. One divergence from
`preg_match` is deliberate: a declared group that did not participate is **present and `null`** rather than
absent, which is what lets `group` distinguish "no text" from "no such group". `preg_last_error` has no
equivalent: a failure throws (R4). `replaceWith`'s
callback is `callable(Match): string` — the one place a `Core` callback does not receive `($value, $key)`,
because a match is one value with named parts rather than a pair. `preg_grep` has no member:
`Arr::filter($a, fn($v) => Regex::matches($v, $p))` is the same thing in the same number of characters.

## 6. `Core\Json`

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `encode` | `encode(mixed $value, {pretty?: bool, escapeUnicode?: bool}): string` | `json_encode` and its 15 `JSON_*` flags | |
| `decode` | `decode(string $json, {maxDepth?: uint}): mixed` | `json_decode`, `json_last_error`, `json_last_error_msg`, `$depth` | |
| `decodeAs` | `decodeAs<T>(string $json, {maxDepth?: uint}): T` | hand-written hydration | |
| `isValid` | `isValid(string $json): bool` | `json_validate` | neutral |

`decode` throws `ParseError` on malformed input — there is no flag to choose between throwing and
returning `null`, and no error-code accessor (R4). **`maxDepth` defaults to 512 and exceeding it throws**:
nesting depth is the one JSON input that costs unbounded work before any value exists, so the cap is on by
default rather than opt-in, exactly as PHP's `$depth` is. An integer literal too large for `int` throws
rather than degrading to `float`, because silent precision loss on a wire format is the bug
`JSON_BIGINT_AS_STRING` exists to work around. A class participates by implementing
`Core\Json\Codec`, which declares `toJson(): mixed` and a static `fromJson(mixed $value): static`; there is
no magic hook and no structural encoding of public properties
(`rule:core-api/written-participation`). An inline shape
(`rule:types/object-top`) is the one instance that needs neither, encoding as a
JSON object keyed by its field names — it has no declaration to carry a codec, and
[ADR 0071 § 7](../decisions/0071.md) owns why that is not the same rule. A `secret` value cannot be encoded at all
(`rule:security/secret-qualifier`).

Both halves are generated from a class's own declared properties by the opt-in `#[Json\Derive]`
attribute, whose rules — the field list, the `#[Json\Field(name?, skip?)]` override, and the one throw
carrying every failed field — are `rule:core-classes/derive-attribute`. Two `type` aliases are all
`Core\Json` adds for it:

| Name | Definition | Attaches to |
|---|---|---|
| `Core\Json\Derive` | `{}` | a class; generates whichever `Codec` half the class does not declare itself |
| `Core\Json\Field` | `{name?: string, skip?: bool}` | a property; renames or removes one field |

`decodeAs<T>` accepts an inline shape (`rule:types/object-top`) or a class with a
`Codec`, derived or hand-written. Over a `tainted` argument it diagnoses a `T` whose text-carrying fields are
unqualified, naming the field.

## 7. `Core\Encoding` and `Core\Bytes`

`Core\Encoding` sits exactly at the `bytes`↔`string` boundary, which is the one place a conversion can
honestly fail (`rule:types/bytes`).

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `decodeText` | `decodeText(bytes $b, Charset $charset): string` | `iconv`, `mb_convert_encoding`, `utf8_decode` | |
| `encodeText` | `encodeText(string $s, Charset $charset): bytes` | `iconv`, `mb_convert_encoding`, `utf8_encode` | |
| `isValidText` | `isValidText(bytes $b, Charset $charset): bool` | `mb_check_encoding` | neutral |
| `toBase64` / `fromBase64` | `toBase64(bytes $b): string` / `fromBase64(string $s): bytes` | `base64_encode`, `base64_decode` | |
| `toBase64Url` / `fromBase64Url` | `toBase64Url(bytes $b): string` / `fromBase64Url(string $s): bytes` | `strtr(base64_encode(…))` idiom | |
| `toHex` / `fromHex` | `toHex(bytes $b): string` / `fromHex(string $s): bytes` | `bin2hex`, `hex2bin`, `unpack("H*")` | |
| `toBase32` / `fromBase32` | `toBase32(bytes $b): string` / `fromBase32(string $s): bytes` | nothing — needed by TOTP (`rule:security/protocol-roster`) | |

`quoted_printable_encode`/`_decode` and `convert_uuencode`/`_decode` are dropped; quoted-printable survives
only inside `Core\Mail`, which is the one thing that ever needed it.

`Core\Bytes` is the binary counterpart of `Core\Str`, with the same subject-first shape and the same member
names wherever the operation is the same: `length`, `at`, `slice`, `indexOf`, `compare`, `contains`,
`startsWith`, `endsWith`, `join(array<bytes> $parts, bytes $separator = "")`, `fill`, `repeat`, plus
`pack(string $format, mixed ...$values)` and `unpack(bytes $b, string $format): array<mixed>` (replacing
`pack`/`unpack`, with the format string an `rule:expressions/intrinsic-literals` intrinsic and, on both members, a **sink** — it is one
of R11's four grammars, per `Core\Str::format` above). The three predicates are what magic-byte
sniffing needs, and `join` rather than a `concat` of its own keeps R6's pairing with `Core\Str`. There is
no `bytes` literal — see [00-overview § 5](00-overview.md).

The member *names* pair up; three of the *signatures* deliberately do not, because a byte string carries
less than a text one. `at` answers a `uint` rather than a one-byte buffer, `indexOf` has no
`caseInsensitive` option, and `compare` answers an ordering `int`.
`crates/nvs-stdlib/src/bytes.rs` owns all three and why.

Enums: `Charset` — one case per encoding in the **WHATWG Encoding Standard**, which is what `encoding_rs`
implements, named in Novis casing: `Utf8`, `Utf16Le`, `Utf16Be`, `Latin1`, `Windows1252`, `Ascii`,
`ShiftJis`, `EucJp`, `Gbk`, `Big5`, `EucKr`, and the rest of that document's index. The roster is that
standard's, not a list this file curates, so adding an encoding is a dependency update rather than a design
decision — and `iconv`'s open-ended `//TRANSLIT` and `//IGNORE` suffixes have no equivalent, since a
conversion that cannot be exact throws (R4).

Two departures from the index, because a `Charset` argument is an *instruction* where the standard's index
is a guess about a mislabelled document. `Ascii` and `Latin1` are cases of their own rather than aliases of
`Windows1252`, which is what the standard resolves those labels to; the three disagree over `0x80`-`0x9f`,
and `isValidText($b, Charset::Ascii)` — the `mb_check_encoding` call the table above replaces — would
otherwise be true of every byte string. And `replacement` is absent, since a case that maps every input to
an error is surface with no meaning behind it. `crates/nvs-stdlib/src/encoding.rs` owns both, and which
cases it converts itself rather than delegating.

## 8. `Core\Path`

Pure string algebra over paths. **No member touches the disk**, so none needs a capability and every member
is constant-foldable. Everything that reads or writes is `Core\IO` (§ 14) — that split is the point.

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

Every member accepts `/` and `\` alike as a separator on every platform and emits `Path::SEPARATOR`, so a
path written with forward slashes in source is correct on Windows — the reverse of PHP, where
`DIRECTORY_SEPARATOR` string-building is the portability burden.

Constant: `Path::SEPARATOR` (replacing `DIRECTORY_SEPARATOR`). `Path::normalize` is **not** a launderer:
`../` removal is not path-traversal safety, because the base directory is not part of the input. The
launderer is `Core\IO::within`, in § 14, which is where the base is known.

## 9. Collections: `Core\ObjectMap`, `Core\ObjectSet`, `Core\Heap`

`array<T>` is list, stack, queue and dictionary, so `SplStack`, `SplQueue`, `SplDoublyLinkedList`,
`SplFixedArray`, `ArrayObject` and `ArrayIterator` are dropped as restatements of it. These three survive
because an insertion-ordered `string`-keyed hash cannot express them
(`rule:types/arrays`).

| Type | Members | Replaces |
|---|---|---|
| `ObjectMap<K, V>` | `set`, `get`, `has`, `remove`, `count`, `isEmpty`, `keys`, `values`, `clear`; `Iterable` | `SplObjectStorage` used as a map, `spl_object_id` side tables |
| `ObjectSet<T>` | `add`, `has`, `remove`, `count`, `isEmpty`, `union`, `intersect`, `diff`, `clear`; `Iterable` | `SplObjectStorage` used as a set |
| `Heap<T>` | `push`, `peek`, `pop`, `count`, `isEmpty`; `Iterable` | `SplPriorityQueue`, `SplMinHeap`, `SplMaxHeap` |

**`ObjectMap::get` returns `?V`**, not a throwing read: these types have no subscript
(`rule:iteration/two-interfaces` rejects `ArrayAccess`), so they cannot offer the
`$a[$k]` / `$a[$k] ?? $d` pair that `array<T>` does, and R5 bans a `getOrNull` twin. `diff` is spelled as
it is on `Core\Arr` rather than `difference`, because one operation gets one name.

`ObjectMap`/`ObjectSet` key on identity. `Heap` orders by `rule:classes/comparable`'s
`Comparable`, or by a comparator given at construction. These are the only mutable `Core` types, because a
persistent structure would give up the O(log n) that justifies their existence at all; they are objects,
so `rule:types/declaration`'s reference semantics apply and nothing about R3 is in
question.

**A `foreach` yields the one thing each collection has to say**: a map's *keys*, as `SplObjectStorage`
does and because a key hands `get` back its value while a value hands nothing back; a set's members; and a
heap's elements in `pop` order. All three are non-destructive and re-iterable — PHP's heap iteration
empties the heap, and this one does not — because the loop walks a snapshot taken when it began, so a
mutation inside the body cannot disturb the walk it is inside. `Heap` is `Iterable` for that reason
alone: without it a heap's contents are unreachable except by emptying it, which is exactly the PHP
behaviour these three replace.

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
  │    ├─ TimeoutError        // a deadline passed
  │    └─ RecursionError      // the call stack passed its soft depth (`rule:errors/on-limit`)
  └─ ArithmeticError          // overflow (`rule:types/declaration`), division by zero
```

Every one is constructed the same way — `new RuntimeError("could not reach the host", {previous: $e})` —
one required message and one options shape, which is R2 applied to a constructor like any other member.
Members are readonly properties, not `getX()` accessors: `$e->message`, `$e->previous`, `$e->backtrace`,
`$e->location`. `ParseError` and `Core\Db\DbError` carry one more — `issues: array<Core\Issue>`, where
`type Core\Issue = {path: string, message: string};` — so a failed decode reports **every** offending field
at once rather than the first, each located by a dotted path (`"address.city"`, `"tags.3"`) or, for a row, by
its column name ([ADR 0071 § 5](../decisions/0071.md)). It is empty on any error that has no field
list to report. There is no `getCode()`: an `int` code with no declared meaning is what a user-defined
subclass with a typed property does properly. `Throwable`'s message is a `secret` sink
(`rule:security/secret-qualifier`). Resource-limit reports are **not**
`Throwable` at all and never reach a `catch` (`rule:errors/escalation-ladder`).
`RecursionError` is not a counter-example to that: `rule:errors/on-limit` puts a **soft** depth above the call-stack
limit precisely so a recursive-descent parser over untrusted-depth input can degrade, and the limit itself —
the `FATAL` at the true ceiling — is still not `Throwable` and still reaches no `catch`.
Domain-specific errors are user-defined classes; `Core` does not attempt to enumerate them. The
exceptions are `Core\Db\DbError` and `Core\Db\RolledBack` (§ 18) and `Core\Cli\NotInteractive` (§ 15),
all three extending `RuntimeError`: a driver failure, a deliberate rollback and a prompt with no
controlling terminal to read (`rule:tooling/a-prompt-is-a-core-member`) have no
user-defined home.

## 11. `Core\Random`, `Core\Uuid`, `Core\Hash`

`Core\Random` is a CSPRNG, always — PHP's `rand`/`mt_rand`/`shuffle`/`str_shuffle`/`array_rand` and its
newer `Random\Randomizer` OOP twin all collapse into it, and none of the insecure generators survives
under any name.

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `Random::int` | `int(int $min, int $max): int` | `rand`, `mt_rand`, `random_int` | neutral |
| `Random::float` | `float(): float` | `lcg_value`, `mt_rand()/mt_getrandmax()` — uniform in `[0, 1)` | neutral |
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
| `Uuid::tryParse` | `tryParse(string $s): ?Uuid` | `uuid_is_valid`, userland `isValid` helpers | |
| `Uuid::fromBytes` | `fromBytes(bytes $b): Uuid` | userland `fromBytes`, the `hex2bin` round trip | |
| `$uuid->toString` | `$uuid->toString(): string` | `(string)` on a userland UUID object | neutral |
| `$uuid->toBytes` | `$uuid->toBytes(): bytes` | userland `getBytes`, `hex2bin(str_replace("-", "", …))` | neutral |
| `Hash::of` | `of(bytes\|string $data, Digest $digest): bytes` | `hash`, `md5`, `sha1`, `crc32`, `openssl_digest` | neutral |
| `Hash::hmac` | `hmac(bytes\|string $data, secret bytes $key, StrongDigest $digest): bytes` | `hash_hmac` | neutral |
| `Hash::equals` | `equals(bytes $a, bytes $b): bool` | `hash_equals` — constant-time | neutral |
| `Hash::stream` | `stream(Digest $digest): Hash\Stream` | `hash_init`, `HashContext` | |
| `$stream->update` | `$stream->update(bytes\|string $data): void` | `hash_update` | |
| `$stream->finish` | `$stream->finish(): bytes` | `hash_final` | |

A `Core\Uuid` is an opaque 128-bit **value**, not a string that has been checked once: `toString` renders
RFC 9562's canonical lower-case `8-4-4-4-12` form and is the only way text comes back out, which is what
lets a route segment (`rule:routing/routes-are-compiled-not-registered`) and a database column
(`rule:core-classes/db-statement-members`) state that they take one. `toBytes` and `fromBytes` carry the
same 128 bits as sixteen octets, most significant first — the form a native `UUID` column and a binary
protocol take, where the canonical text would be 36 bytes spelling bits the caller is already holding.
A length is the whole of what `fromBytes` refuses, because every 128-bit pattern is a UUID.

**Asking whether text is a UUID is `Uuid::tryParse($s) != null`** — `parse` with `null` where it throws,
one of the two `tryParse`s `rule:core-api/shape-rules` R5 admits
(`rule:expressions/try-parse`). `$s as ?Uuid` does **not** compile; `as`
never targets a class. There is no `Uuid::isValid`, because it was exactly `tryParse` asked a second time
and R17 keeps one — the same argument, and the same CVE, that § 12 gives for `Uri`. `v7` is time-ordered across
milliseconds and random inside one — the property that makes it the right primary key and the wrong public
identifier, since a `v7` handed to a stranger tells them when the row was created.

`Digest` carries every algorithm including `Md5`, `Sha1` and `Crc32`, because checksum interop genuinely
needs them. `StrongDigest` is the closed subset (`rule:types/literal-types`)
that the HMAC and signature members declare, so `Hash::hmac($m, $k, Digest::Md5)` is a compile error naming
the reason. Password hashing takes no algorithm argument at all and is in § 16.

**This table is `Digest`'s roster and its only home.** The **S** column is membership of `StrongDigest`.

| Case | Octets | S | Notes |
|---|---|---|---|
| `Digest::Crc32` | 4 | | CRC-32/ISO-HDLC, PHP's `crc32b`. A checksum: accidental corruption only |
| `Digest::Crc32c` | 4 | | CRC-32C/Castagnoli — the checksum S3 and GCS stamp objects with |
| `Digest::Md5` | 16 | | Collision-broken since 2004. Interop only |
| `Digest::Sha1` | 20 | | Collision-broken since 2017. Interop only |
| `Digest::Sha224` | 28 | ✓ | |
| `Digest::Sha256` | 32 | ✓ | The default to reach for |
| `Digest::Sha384` | 48 | ✓ | |
| `Digest::Sha512` | 64 | ✓ | Faster than SHA-256 on 64-bit hardware |
| `Digest::Sha512_224` | 28 | ✓ | SHA-512 truncated, own IV (FIPS 180-4 § 5.3.6) |
| `Digest::Sha512_256` | 32 | ✓ | SHA-512's speed at SHA-256's width, and length-extension-proof |
| `Digest::Sha3_224` | 28 | ✓ | FIPS 202's Keccak sponge — an independent construction, not a wider SHA-2 |
| `Digest::Sha3_256` | 32 | ✓ | |
| `Digest::Sha3_384` | 48 | ✓ | |
| `Digest::Sha3_512` | 64 | ✓ | |
| `Digest::Blake3` | 32 | | The fastest here, and the one algorithm PHP cannot compute |

`Blake3` is outside `StrongDigest` for a different reason than `Sha1` is: HMAC-BLAKE3 is a construction
nobody uses, because BLAKE3 is keyed natively, so it waits for a keyed member designed as one rather than
being bent into `hmac`. The underscore spellings are PHP's `sha512/256` and `sha3-256` written as
identifiers; the digit after it is the output width in bits, and `Sha512_256` is not `Sha512` truncated by
the caller — it is a different IV and a different function.

**A case's integer is ABI and this list is append-only.** `Hash::stream` stores the chosen case in a slot
and reads it back, so reordering these is a behaviour change rather than a cosmetic one; widening
`StrongDigest` is backward compatible and narrowing it is not.

A `Hash\Stream` is **consumed by its own `finish`**: the digest is final, so a second `finish`, or any
`update` after one, throws rather than continuing from where the first left off. PHP's `HashContext` says
the same thing by making `hash_final` invalidate the context, and a program that wants two digests of one
input opens two streams.

## 12. `Core\Uri`, `Core\Validate`, `Core\Csv`, `Core\Out`

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `Uri::parse` | `parse(string $uri): Uri` | `parse_url` | |
| `Uri::tryParse` | `tryParse(string $uri): ?Uri` | `filter_var(…, FILTER_VALIDATE_URL)` | |
| `Uri::encodeComponent` / `decodeComponent` | `encodeComponent(string $s): string`, `decodeComponent(string $s): bytes` | `rawurlencode`, `rawurldecode` | |
| `Uri::encodeFormValue` / `decodeFormValue` | `encodeFormValue(string $s): string`, `decodeFormValue(string $s): bytes` | `urlencode`, `urldecode` (the `+`-for-space variant) | |
| `Uri::parseQuery` | `parseQuery(string $query): array<mixed>` | `parse_str` — returns, never populates variables | |
| `Uri::buildQuery` | `buildQuery(array<mixed> $parameters): string` | `http_build_query` | |
| `$uri->scheme` / `$uri->userInfo` / `$uri->host` / `$uri->port` | `$uri->scheme(): ?string`, `$uri->userInfo(): ?string`, `$uri->host(): ?string`, `$uri->port(): ?int` | `parse_url`'s array keys | neutral |
| `$uri->path` / `$uri->query` / `$uri->fragment` / `$uri->toString` | `$uri->path(): string`, `$uri->query(): ?string`, `$uri->fragment(): ?string`, `$uri->toString(): string` | `parse_url`'s array keys, and reassembly by hand | neutral |
| `$uri->with` | `$uri->with({scheme?, host?, port?, path?, query?, fragment?}): Uri` | manual reassembly | |
| `$uri->queryParameter` | `$uri->queryParameter(string $name): mixed` | `parse_str` over `parse_url`'s `query` key, then an array read | neutral |
| `$uri->withQueryParameter` | `$uri->withQueryParameter(string $name, mixed $value): Uri` | `parse_str`, an array edit and `http_build_query` written out at every call site | |
| `$uri->resolve` | `$uri->resolve(string $reference): Uri` | nothing | |
| `$uri->compareTo` | `$uri->compareTo(Uri $other): int` | nothing — PHP compares `parse_url` arrays by hand | |
| `$uri->sign` | `$uri->sign({keys: array<secret bytes>, until: ?Time\Instant} $settings): Uri` | nothing — Laravel's `URL::signedRoute`, Symfony's `UriSigner` | |
| `$uri->verifySignature` | `$uri->verifySignature(array<secret bytes> $keys): void` | nothing | neutral |

`Uri::parse` is an `rule:expressions/intrinsic-literals` intrinsic. Note what is **not** here: `Core\Uri` never decides whether a URL
may be *fetched* — that is `Core\Http::allowUrl` in § 16, the SSRF launderer
(`rule:http-server/allow-url-pins-the-address`).

**`parse` reads RFC 3986 and reports rather than normalizes.** It takes a URI *reference*, so
`Uri::parse("/a?b")` answers a `Uri` whose `scheme()` is `null`; every component comes back exactly as it
was written, still percent-encoded and still in its own case, and dot segments are removed only by
`$uri->resolve`, where RFC 3986 § 5.2.4 asks for it. The WHATWG URL Standard is the other specification a
`Uri` could have read, and it is the wrong one here: it rewrites its input on the way through, so a program
comparing `$uri->host()` against an allowlist would be comparing against text no client sent.

**Asking whether two references are the same URI is `$a->compareTo($b) == 0`**, and `Uri` implements
`Comparable` (`rule:classes/comparable`) to say so. That is where the normalizing
`parse` refuses to do happens, and it is the whole of RFC 3986 § 6.2.2 and no more: the scheme and host
fold to lower case, every `%XX` escape's digits fold to upper case, an escape spelling an *unreserved*
character becomes that character, and dot segments are removed from an **absolute** path. It stops short
of § 6.2.3's scheme-based normalization, so `http://h:80/` and `http://h/` are two URIs — knowing that
`80` is `http`'s default is knowledge about a scheme, and a member carrying a table of them would answer
differently as the table grew. `==` on two `Uri`s is still object identity, which
`rule:expressions/object-identity-equality` fixes for every
class; `compareTo` is that ADR's own named answer for content equality, and it gives an order as well —
component-lexicographic, absent before present.

**`$uri->sign` signs exactly what `compareTo` normalizes**, and that is the whole reason the two signing
members sit on this class rather than beside `Core\Crypto`: a signature computed over assembled URL
*text* is the canonicalization bug every framework in this space carries, and reusing the § 6.2.2
normalization above is what makes it impossible for the signing and verifying sides to drift. The
reserved `_sig` parameter carries the tag and the lifetime together, every other component present is
covered — so appending a parameter invalidates — and the fragment is never signed, because the server
never receives one. `rule:core-api/signing-is-over-a-payload`
owns all of it, including why `verifySignature` answers nothing and throws rather than returning a
`bool` a caller can drop. **These two land with M8** rather than with the rest of Part I: they need
`Core\Crypto`'s construction, which is the same split `Core\Router` already carries.

**Asking whether text is a URI is `Uri::tryParse($s) != null`** — `parse` with `null` where it throws, and
the one spelling `rule:core-api/shape-rules` R5 admits `try…` for
(`rule:expressions/try-parse`). `$s as ?Uri` does **not** compile: `as`
never targets a class, which is § 3's row without exceptions. There is no `Uri::isValid` either, by R17:
a validator written as a *separate* implementation from the parser is how
`filter_var(FILTER_VALIDATE_URL)` came to accept user-info that `parse_url` read differently
(CVE-2024-5458), and `tryParse` cannot drift from `parse` because it *is* `parse`. The narrower question
`FILTER_VALIDATE_URL` is actually asked — "is this an **absolute** URI" — is one reader call further on,
`Uri::tryParse($s)?->scheme() != null`, and it launders nothing either way: whether a URL may be *fetched*
is `Core\Http::allowUrl` at § 16.

The eight readers are what `parse_url`'s array keys become, with two differences that array cannot express:
`host()` is `null` where no authority was written and `""` where an empty one was (`file:///tmp`), and
`userInfo()` is one reader rather than a `user` and a `pass` key, because RFC 3986 § 3.2.1 deprecates the
`user:password` form and a member that split it would be one that recommended writing it.

**The two decoders answer `bytes`, and the two encoders answer `string`.** Percent-decoding is defined over
octets and a client may send any of them, so `decodeComponent("%FF")` has an answer and `%ff%fe%fd` is the
three octets it spells. A caller who wants text writes `as string`, which is
`rule:types/conversion`'s checked conversion and throws in exactly the place a
`string`-returning decoder would have — so nothing is denied an answer it could have used, and the asymmetry
is the honest one: an encoder takes text and escapes it, a decoder is handed a wire and cannot promise what
is on it. The encoders are this class's two laundering rows and are unchanged.

**`parseQuery` reads PHP's bracket convention in full**, and `buildQuery` writes it: `a[]=1&a[]=2` builds
a list under the key `"a"`, `a[b]=c` builds a map, and the two nest to arbitrary depth. It is not a URL-spec
feature, but it is how every PHP form posts, and reproducing it here is what lets `Core\Request::query`
(§ 15) return the same shape from the same code rather than answering the question a second time. The
cost is the return type: every value is a `bytes` **or** a nested `array<mixed>`, which is why the row above
is `array<mixed>` rather than `array<bytes>` — the one place in Part I where a member's element type is not
statable more precisely. A **name** is the array key the pair is placed under, so it is a `string` and a name
whose escapes decode outside UTF-8 is refused; a value has no such refusal. What § 15 shares is the bracket
walk and not the element type — `Core\Request::query` reads a served request's parameters as text at the
door and keeps that refusal for a value too. A repeated key without brackets (`a=1&a=2`) keeps the last
value, as PHP does.

`Core\Validate` is what survives of `filter`: the genuine validators only. Its *sanitizing* filters are
dropped, because half-escaping produces exactly the false confidence
`rule:security/tainted-qualifier` exists to prevent — **no `Validate` member
launders anything.**

`isEmail`, `isIp(string $s, {version?: 4|6})`, `isMac`, `isDomain`, `isAscii`, `isPrintable` — all
`(subject, …): bool`, all neutral. Replaces `filter_var`'s validate half and its 20 `FILTER_*` constants.

Three members that were here are gone as duplicates, each with a one-line rewrite: `isUrl` is
`Uri::tryParse($s)?->scheme() != null`, `oneOf($value, $allowed)` is `Arr::contains($allowed, $value)` — the same operation with
PHP's argument order, which R10 exists to stop — and `isIpV4`/`isIpV6` are `{version: 4}`/`{version: 6}`,
a closed literal set (`rule:types/literal-types`) rather than two more names.

**There is no `isInteger`, `isFloat` or `isBoolean`**: each is `$s as ?int`/`?float`/`?bool != null`
(`rule:expressions/nullable-conversion`), and R17 forbids the second spelling. Every
member that remains names a *format*; only a *type* has an `as`, which is why the roster looks uneven and
is not.

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `Csv::parse` | `parse(string $text, {separator?, quote?, escape?, header?: bool}): array<array<string>>` | `str_getcsv`, the parsing half of `fgetcsv` | |
| `Csv::format` | `format(array<array<string>> $rows, {separator?, quote?, header?: array<string>}): string` | `fputcsv`'s formatting half | |
| `Csv::rows` | `rows(Core\IO\File $file, {separator?, quote?, escape?, header?: bool}): Core\Csv\Rows` | the `while (fgetcsv($handle))` loop | |
| `Out::capture` | `capture(callable(): mixed $fn, {through?: callable}): Sink` | `ob_start`/`ob_get_clean`, `ob_start($callback)` | |

`Core\Out` has exactly this one member. A buffer is scoped to a closure and nests by call nesting, so
PHP's global `ob_*` stack — start in one function, end in another, ten functions to inspect the stack — has
no equivalent, and neither does implicit flushing. `capture` always **swallows**: `{through: $filter}`
transforms what was captured, and re-emitting it is a visible `echo Out::capture(…)` rather than
`ob_start($callback)`'s invisible pass-through.

`Sink` above is not a type name — it is **the carrier of the sink in force**, `Core\Html\Markup` under an
HTTP request and `Cli\Text` in every other context
(`rule:tooling/echo-always-has-a-sink` and `rule:security/capture-answers-the-carrier`). It is not a plain
`string`, because the captured bytes have already been through the sink and re-emitting them as a string
would escape them a second time. `{through:}` therefore takes and returns that same carrier.

`Csv::parse`'s `{header: true}` consumes the first row as column names and keys every returned row by
them — the return type is unchanged, because every array key is a `string` already — and the header row is
not itself returned. `Csv::format`'s `{header: [...]}` writes those names as the first row.

`Csv::rows` is the same read over a file the program has open, a record at a time: it takes the same
dialect and the same `{header: true}`, keys a record exactly as `parse` does, and holds one record rather
than the document, so a file larger than memory reads. It reads forward from wherever the handle is and
is taken once — the walk ends where the file does, and winding the handle back is what reads the document
twice. Taking a handle rather than a path is `rule:core-api/a-lifetime-is-an-object`'s admitted shape: the
capability was checked at `IO::open`, and this member neither opens nor closes what it reads.

## 13. Compiler-facing surfaces

These are `Core` classes whose semantics live in their own ADRs; this file fixes only that they follow the
same shape rules.

| Class | Owns | ADR |
|---|---|---|
| `Core\Reflect` | `forClass`, `forObject`, `typeOf(mixed): TypeKind`, and the visibility-respecting walk. Replaces `ReflectionClass` **and** `get_class`, `get_object_vars`, `get_class_methods`, `method_exists`, `property_exists`, `class_exists`, `is_a`, `is_subclass_of`, `class_implements`, `spl_object_id` | [0019](../decisions/0019.md) |
| `Core\Ast` | `parse(string): Node` returning inert typed data | [0019](../decisions/0019.md) |
| `Core\Attributes` | `get<T>`, `all<T>` — structural, not a `Reflect` walk | [0046](../decisions/0046.md) |
| `Core\Program` | `implementing<T>()`, and `implementingWith<I, T>(string $member = "")` — the same enumeration with one attribute retrieval joined per class, each row holding the instance and its matching payload or null | [0061](../decisions/0061.md), [0212](../decisions/0212.md) |
| `Core\Router` | `match(Http\Method $method, tainted string $path): ?Router\Match`, `methodsFor(tainted string $path): array<Http\Method>` (empty ⇒ 404, else 405 + `Allow:`), `url(string $name, array<string, mixed> $params): string` (**launder**, URL path), `urlAbsolute(...)` (**launder**, URL — prepends the mount's configured `origin`) and `urlSigned(..., {keys: array<secret bytes>, until: ?Time\Instant} $settings): string` (`url` with a `_sig` parameter over the route's *name* and `$params`, so a link survives a remount — [0146](../decisions/0146.md)), over a table built while compiling from `#[Route]`. The server matches once per request and `Core\Request::route()` is that match. A duplicate route, a `{param}` with no matching method parameter, an unknown literal `url()` name and a `url()` key that is neither a capture nor a declared `#[Query]` parameter are compile errors | [0077](../decisions/0077.md), [0102](../decisions/0102.md) |
| `Core\Command` | `run(): uint`, `help(?string): Cli\Text` and `completions(Cli\Shell): string`, over a table built while compiling from `#[Command]`/`#[Option]`/`#[Argument]`. A duplicate command name, two options sharing a spelling and an `#[Option]` on a parameter with no conversion from `string` are compile errors. Unlike `Core\Router` it dispatches, because a CLI has one entry point and no middleware question | [0086](../decisions/0086.md) |
| `Core\Decimal`, `Core\BigInt` | the non-operator members of the `decimal` scalar and arbitrary-precision integers. Replaces `bcmath`, `gmp` | [0054](../decisions/0054.md) |
| `Core\Serialize` | `encode(mixed $value): bytes` and `decode(bytes $data): mixed` — the user-facing half of the one graph-copy operation the `spawn` boundary already runs. `decode` is a **`tainted` sink**. Replaces `serialize`, `unserialize` | [0023](../decisions/0023.md) |
| `Core\Test` | the assertion roster — `assertSame`/`assertEquals<T>`/`assertEqualsDeep`, `assertTrue`, `assertNull`, `assertCount`, `assertContains`, `assertThrows`, `assertDoesNotThrow`, `expectFailure` — plus `double<T>`/`partial<T>`, `assertCalled`/`assertNeverCalled`, `advance`, `scriptAnswers`, `assertCompletes`, `assertMatchesInline` and `request(Http\Method $method, string $path, {headers?: array<string>, body?: string\|bytes, mount?: string} $options): Core\Test\Response` — one in-process request against the program's own `#[Route]` table. Every one is subject-first and generic where it compares | [0079](../decisions/0079.md) |

`Core\Reflect::typeOf` is the single replacement for PHP's 14 `is_*` predicates plus `gettype`: they are
only meaningful on a `mixed`, and the checker already knows every other case.

**`Core\Test` is a surface plus a compiler feature**, which is why it sits here rather than in Part I. Its
members are ordinary `Core` members obeying `rule:core-api/shape-rules`; what is not
ordinary is that `#[Test]`, `#[Fixture]`, `#[TestWith]`, `#[Property]` and `#[Bench]` are read while
compiling, that `assertEquals<T>` makes a type-mismatched comparison a compile error, and that each test
runs in its own isolate. `rule:testing/test-attribute` owns all of that,
including which milestone each piece lands in; this file fixes only the roster's shape.

**`Core\Serialize::decode` is a `tainted` sink**, which is the whole reason the class is worth having
rather than deferring to `Core\Json`. `unserialize()` on attacker-controlled bytes is PHP's most
productive remote-code-execution class; Novis has already removed its gadget machinery — no `__wakeup`, no
`__destruct`, no `__toString` hook (`rule:classes/no-magic-methods`) — and
refusing the qualifier closes the input side structurally rather than by advice. What remains without the
sink is not code execution but **type confusion** — a payload that reconstructs a `User` with
`isAdmin: true`, bypassing the constructor — which is why contagion would be the wrong classification: the
danger is the object graph itself, not a string that later reaches an output sink. Bytes the program
serialized and stored are not `tainted` and decode normally; bytes that arrived from outside are refused,
and no launderer exists for them today. The format is versioned, self-describing and Novis's own; it is not
compatible with PHP's, and there is no hook to customise it
(`rule:classes/two-copy-depths`).

---

# Part II — the capability-bearing half (M8)

Everything below needs a capability grant, the reactor, an open handle or a driver. It is listed at one
line per member because the semantics are owned by `rule:core-api/tier-placement`'s
roster and by each subsystem's own ADR; the shape rules are identical to Part I's. Three entries land
before M8 — see *Milestones* above for which and why.

## 14. `Core\IO`

Every member is an `rule:security/tainted-qualifier` **path sink** and requires
an `fs.read` or `fs.write` capability. `resource` is never exposed — an open file is a `Core\IO\File`
object (R14).

- **Whole-file:** `read(string $path): bytes`, `readText(string $path, {charset?}): string`,
  `write(string $path, bytes|string $data)`, `append(…)`, `lines(string $path): Iterable<string>` —
  replacing `file_get_contents`, `file_put_contents`, `file`, `readfile`, `fpassthru`.
- **Streaming write:** `writeStream(string $path, Iterable<bytes> $src, {max?, overwrite?})` — any stream to
  disk in one member: an upload part, `Core\Request::bodyStream()`, a decompressed archive. `overwrite`
  defaults to `false`, and a write that fails mid-stream removes the partial file
  (`rule:core-classes/io-write-stream`).
- **Metadata:** `exists`, `isFile`, `isDir`, `isReadable`, `isWritable`, `size`, `modifiedAt`, `stat` —
  replacing `file_exists`, `is_file`, `is_dir`, `filesize`, `filemtime`, `fileperms`, `stat`, `lstat`.
- **Manipulation:** `copy`, `move`, `remove`, `makeDir`, `removeDir`, `list(string $path): array<string>`,
  `walk(string $path): Iterable<string>`, `temporaryDir` — replacing `copy`, `rename`,
  `unlink`, `mkdir`, `rmdir`, `scandir`, `glob`, `opendir`/`readdir`/`closedir`, `tempnam`, `tmpfile`,
  `sys_get_temp_dir`, and the `DirectoryIterator` family. There is no `temporaryFile` — a program that
  needs one temporary file needs somewhere to put the second — and what `temporaryDir` creates is deleted
  by the runtime when the script ends
  (`rule:core-classes/temporary-dir-sweep`).
- **Resolution:** `canonicalize(string $path): string` (`realpath`), and `within(string $base, tainted
  string $path): string` — **the path-traversal launderer**: it resolves and then proves containment,
  which is the check `Core\Path::normalize` structurally cannot make.
- **Handles:** `open(string $path, FileMode $mode): File`; `$file->read`, `->readLine`, `->write`,
  `->seek`, `->tell`, `->truncate`, `->flush`, `->lock`, `->close`. `FileMode` is an enum, never a
  mode string (R11) — replacing `fopen`'s `"r+b"` grammar and the whole `fread`/`fgets`/`fwrite`/`fseek`/
  `ftell`/`feof`/`flock`/`fstat` family plus `SplFileObject`.
- **Standard input:** `stdin(): tainted string` — everything the invoker attached to the program,
  read to end of input in one call. It is
  a value and not a `File`: a descriptor the process was handed has no position to seek, no length to
  truncate, and a `close` on it would take the stream away from the whole process. There is **no**
  `stdout()` or `stderr()` — standard output and standard error are
  `rule:tooling/terminal-output-is-a-sink`'s sink, whose one door is
  `Core\Cli::write`, and a raw handle onto either would be a hole in it.

No stream wrappers, no `php://`, no `phar://`, no user-registered protocols
(`rule:security/closed-doors`).

## 15. Request-facing: `Core\Server`, `Core\Request`, `Core\Response`, `Core\Session`, `Core\Env`, `Core\Cap`, `Core\Cli`

These replace PHP's superglobals (`rule:statements/no-host-populated-variables`); every value they return that
originates outside the process is `tainted` (`rule:security/tainted-qualifier`).

- `Core\Request`: `method`, `path`, `query`, `queryAs`, `post`, `postAs`, `body`, `bytes`, `json`, `jsonAs`, `bodyStream`,
  `header`, `headers`, `cookie`,
  `files`, `clientIp`, `scheme`, `host`, `mount`, `route`, `isHead` — replacing `$_GET`, `$_POST`, `$_FILES`,
  `$_COOKIE`, `$_REQUEST`, `filter_input`. `path` is the request path with the matched mount's prefix
  **removed** and `mount(): Request\Mount` answers what was removed — a `prefix(): string` and that mount's
  glob `captures(): array<tainted string>` — which is how a host-mounted deployment learns which tenant it
  serves, and never `null`, because a request that reached a program reached it through some mount;
  `route(): ?Router\Match` is the match the server made once before the handler, and is what the CSRF check
  and the `route` metric label read ([0102](../decisions/0102.md));
  `method` reports `Get` for a `HEAD` request so a
  `Get`-only route table still matches, with `isHead` carrying the truth; `clientIp` and `scheme` are
  resolved from the socket peer unless a peer in `[server] trusted_proxies` asserted otherwise;
  `files(): Iterable<Part>` is the **only** way to receive an uploaded file and yields parts lazily, each
  answering `name()`, a `filename()` that is never a path and a `contentType()` — all three `tainted` —
  and consumed by `readAll({max?}): tainted bytes`, by iterating `content(): Iterable<bytes>`, or by
  `saveTo(string $path, {max?, overwrite?})` — there is no temp path, no `move_uploaded_file` and no `size`
  (`rule:http-server/an-upload-is-received-only-through-files`); a
  multipart form's non-file parts are buffered into `post()` as usual;
  `post(string $name): mixed` reads one submitted field by name under `query`'s bracket convention, over
  those buffered parts or over a urlencoded body, and reads to the **end** of the body, which is what
  makes it answer every field rather than the ones that arrived before the part a `files` walk stopped
  on, so a handler wanting the uploads too takes `files()` first; and `bodyStream(): Iterable<tainted bytes>`
  is the raw-body alternative to `body`, carrying the qualifier `body` puts on the same octets.
  `json({maxDepth?: uint}): mixed` is `Core\Json::decode` over those same octets, carrying that
  member's bag and its default of 512; it consults no `Content-Type`, on `post`'s reasoning, and a
  body that is absent or empty is a `ParseError` like a malformed one, because `mixed` cannot tell
  "no body" from the document `null`. `jsonAs<T>({maxDepth?: uint}): T` is `Core\Json::decodeAs` over
  those same octets, refusing the same bodies for the same reasons; it keeps nothing of what it
  built, so every call hydrates a fresh `T` and two callers are never handed one object.
  `bytes(): tainted bytes` is `body`'s reading for a payload that is not text — named after
  `Core\Response::bytes`, buffering over the same hold, and answering the octets a `string` cannot
  hold. A body that is not UTF-8 is a `ParseError` from `body`, naming this member, rather than a
  repaired string: a lossy decode would silently change a webhook's signed payload
  (`rule:types/string-is-utf8`).
  `queryAs<T>({name?: string}): T` and `postAs<T>({name?: string}): T` are `Core\Arr::shapeAs` over what
  `query` and `post` parse: with no `name` the whole parameter set is the subject — the only way a program
  reaches it, since neither `$_GET` nor `$_POST` has a public array spelling here — and with one it is the
  subtree that name reaches under the bracket convention. Each field converts with `as`, so a form's text
  fits a declared `int`, and a key `T` does not name is left behind. `body`,
  `bytes`, `post`, `postAs`, `json` and `jsonAs` are **buffering** readers, which keep what they read and so may follow
  one another;
  `bodyStream` and `files` are **streaming** readers, each of which consumes the body and may only be the
  first reader of it, which is why a `post()` reading the fields a walk buffered is ordinary rather than
  an exception
  (`rule:http-server/a-mount-table-expands-at-boot`, `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`, `rule:http-server/head-runs-as-get`, `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it` and `rule:http-server/the-body-is-read-on-demand-under-two-caps`).
- `Core\Response`: `setStatus`, `setHeader`, `addCookie`, `redirect`, and the body members
  `html(Core\Html\Markup)`, `json(mixed)`, `text(string)`, `bytes(bytes, string $contentType)`,
  `sendFile(string $path)`, `stream(string $contentType)` — replacing `header`, `headers_sent`, `setcookie`,
  `setrawcookie`, `http_response_code`.
  `setHeader` is a header **sink** and overrides a policy-owned header on one response; `addCookie`'s
  options shape defaults every field from `[http.cookies]`, so a cookie written with no options is
  `Secure; HttpOnly; SameSite=Lax; Path=/` and `SameSite` is an enum, never a string
  (`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`). **One body member per shape, each setting its
  own `Content-Type`**, replacing a single `write`: `json` serializes the value itself so a tainted one is
  safe, `text` accepts tainted because `nosniff` is on by default, and `bytes`' content type is a sink.
  `sendFile` takes the path alone and the path is a sink: a download name is `Content-Disposition`
  through `setHeader`, and what the bytes are called is the static-file policy's media-type table
  ([0186](../decisions/0186.md) § 4).
  `stream` is the one that writes its body over time: it is told a media type, on `bytes`' terms and for
  `bytes`' reason, and answers a `Core\Response\Stream` whose `write(bytes|string $chunk)` is a union
  carrying no classification and so refuses a tainted argument outright. An event stream is
  `Core\Sse::stream` (§ 16) and never this one. `echo` is the HTML-only path, and mixing it with any body
  member on one response is a compile error (`rule:security/response-body-is-one-typed-member`).
- `Core\Server`: the request's own environment — replacing `$_SERVER` — plus `traceId(): string`, which is
  present on every request whether or not the trace is sampled and is Novis's only request identifier
  (`rule:observability/the-runtime-exports-what-it-already-measures`), and `isDraining(): bool`, true once graceful shutdown
  has begun (`rule:http-server/the-server-block-is-boot-class`).
- `Core\Session`: `start`, `get`, `set`, `remove`, `clear`, `regenerate`, `destroy`, `setSecret`,
  `getSecret` — replacing all ~25
  `session_*` functions. `start` is the one that reaches the store, and a member called before it throws
  (`rule:core-classes/session-is-started-explicitly`). Where the record lives is `[session] backend`, which
  names the shared cache tier or the database and refuses the in-process ones
  (`rule:http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot`, enforcing
  `rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent`). A user's own secret — the access and
  refresh token an application holds for them — goes through
  `setSecret(string $key, secret string $value, array<secret bytes> $keys): void` and
  `getSecret(string $key, array<secret bytes> $keys): ?secret string`, sealed under the ring with the
  session door's own domain byte and carrying no lifetime of its own, because a session value lives as long
  as its session. `set` still refuses a `secret` and `get` answers `null` for a sealed value
  (`rule:http-server/a-session-holds-a-secret-only-sealed`).
- `Core\Env`: `get(string): ?tainted string`, `all()`, `mode(): Env\Mode`, and the constants `EOL`, `OS`,
  `VERSION`; its one enum is `Env\Mode` — `Production`, `Development`. Read-only —
  `putenv` has no equivalent, because a process-global mutation is unsound across cores. `mode` reads the
  run mode (`rule:config/two-modes-and-the-default-is-production`); it is set
  through `Core\Config`, like every other directive, and no environment variable is consulted for it.
- `Core\Cap`: `has(string $capability): bool`, and nothing else. Reports whether the **calling namespace**
  holds a capability at this point in the request — the grant table narrowed by anything the request or an
  enclosing isolate already dropped. It is how a package that declared a capability *optional* degrades
  instead of failing a build, so the argument is a roster name and an unknown one is a compile error
  (`rule:security/optional-capability-degrades` and `rule:security/capability-roster-is-closed`). It grants nothing and
  needs no capability of its own; there is no `Core\Cap::drop`, because dropping a capability is
  `Core\Config::set` and stays there.
- `Core\Cli`: the terminal surface, owned by `rule:tooling/terminal-output-is-a-sink` —
  `arguments(): array<tainted string>`, `write`, `escape`, `isTty(Cli\Stream)`, `width`, `height`,
  `colorDepth`, `displayWidth`; the prompts `ask`, `confirm`, `select<T>`, `multiSelect<T>` and
  `secret(): secret tainted string`; and the scoped regions `live<T>` and `progress<T>`. Its value types
  are `Cli\Text` — built by `plain`/`styled`, composed with `+`, and read back by `text(): string` —
  `Cli\Style` and `Cli\Color`, its enums `Cli\Stream`, `Cli\ColorDepth` and `Cli\Shell`.
  Replaces `$argv`, `$argc`, `readline`, `mb_strwidth`, `posix_isatty`. **Terminal output is a `tainted`
  sink** that substitutes every control byte with a visible glyph; `Cli\Text` is the only thing that writes
  raw. The byte-and-line side of standard input is `Core\IO` (§ 14), not here; process exit is the `exit`
  keyword (`rule:statements/nvs-is-the-only-open-tag`).

## 16. Network, data and crypto

| Class | Surface | ADR |
|---|---|---|
| `Core\Http\Client` | `get`, `post`, `put`, `patch`, `delete`, `head`, plus `request(Core\Http\Method, …)` for a verb chosen at run time and `stream(Core\Http\Method, …)` for a body read as it arrives — each over `string \| Core\Http\Target`, and there is no `Core\Http\Request` class. `Core\Http::allowUrl` is the SSRF launderer, and the `Target` it answers carries every approved address. Replaces all ~30 `curl_*` functions and their handle. One trailing `Core\Http\Options` shape — the bounds `{deadline?, connectTimeout?, idle?, maxDuration?}`, the retry keys `{retryAttempts?, retryBackoff?, retryIdempotencyKey?}`, `{headers?, followRedirects?}`, at most one body of `{json?, form?, body? + contentType?, multipart?}`, and the connection keys `{identity?, connectTo?, redirectToHttp?, tlsCa?, tlsPin?, tlsVerifyHost?, tlsVerify?, tlsMinVersion?}` — with **no spelling for an unbounded wait**: `deadline` covers every attempt and every hop, `idle` and `maxDuration` bound a streamed body, retry is jittered, and two body keys, a body on `get`, or `post`/`patch` with `retryAttempts` and no `retryIdempotencyKey`, are compile errors. A reply answers `status`, `text`, `bytes`, `header`, `headers`, `jsonAs<T>` and `tls`, all of it `tainted`; a `Core\Http\Stream` answers the head and then `events`, `lines`, `chunks` or `saveTo` — one of them, once. `Http\Part::file`/`::bytes` are a body sent without being held, `Http\Identity::read` is a client certificate, and `Core\Test::answerHttp`/`::sentHttp` answer outbound calls from a table so a test needs no network | [0058](../decisions/0058.md), [0074](../decisions/0074.md), [0180](../decisions/0180.md) |
| `Core\RateLimit` | `consume(tainted string $key, uint $limit, Duration $per, {burst?, cost?}): RateLimit\Decision` over the shared store, and `shed(…)` per core and approximate. `Decision` is readonly `allowed: bool`, `limit: uint`, `remaining: uint`, `retryAfter: ?Duration`. Both **neutral**; a `secret` key is refused; an unreachable store throws | [0075](../decisions/0075.md) |
| `Core\Metrics` | `increment(string $name, {by?, labels?})`, `observe(string $name, float $value, {labels?})`, `gauge(…)`. A `labels` **value** is a `tainted` sink with no launderer — label by an enum, an `as`-converted scalar or a route name | [0076](../decisions/0076.md) |
| `Core\Task` | § 19 below — `all`, `map`, `afterResponse` | [0072](../decisions/0072.md) |
| `Core\Net` | TCP/UDP/Unix sockets over the runtime's own reactor. Replaces `socket_*`, `stream_socket_*`, `fsockopen` — three PHP APIs for one job | [0051](../decisions/0051.md) |
| `Core\Sse`, `Core\Sse\Message` | server-sent events, as two doors onto one type (`rule:concurrency/two-doors-one-isolate`). On the class: `upgrade(entry, args)` opens the connection isolate that outlives the request, `stream()` writes an event stream that ends with it, and `current()` answers whichever handle is open. On the handle: `send(mixed $data, ?string $event, ?string $id)` — a `string` goes out raw and anything else is JSON-encoded — `retry(Duration $after)`, and `receive(): ?Core\Sse\Message`, which waits on this stream's own subscriptions and throws a `LogicError` on the request-scoped door, there being no isolate for a wait to park in. `$data` accepts `tainted`; `$event` and `$id` are **sinks**, and a `\n`, `\r` or NUL in either is refused, as is an empty `$data` and a `setStatus` on a path that opens a stream. `Message` is `topic(): string` and `value(): mixed`. There is no replay buffer: resumption is the client's `Last-Event-ID`, read off the request by the handler and passed through `args` | [0083](../decisions/0083.md), [0177](../decisions/0177.md) |
| `Core\Db` | the full surface is § 18 below — the one subsystem in Part II too large for a row. Replaces `PDO` **and** the procedural `mysqli`/`pgsql`/`sqlite3` APIs | [0067](../decisions/0067.md) |
| `Core\Crypto`, `Core\Crypto\PublicKey`, `Core\Crypto\KeyPair` | AEAD only, no ECB, no unauthenticated CBC, no cipher-name-as-string — and no algorithm argument with a default. `generateKey`, `seal`/`open` over a required `Crypto\Cipher` (`XChaCha20Poly1305`, `Aes256Gcm`), `deriveKey` (PBKDF2-HMAC-SHA256, a required iteration count bounded at 100,000 and 2,000,000), `expandKey` (HKDF-SHA256), `generateKeyPair`/`agree` over `Crypto\KeyKind`, and `sign`/`verify`, whose scheme is the key's kind. On the two key types: `read`/`write` over `Crypto\KeyFormat` (`Raw`, `Spki`, `Jwk`) and PKCS#8, with every public key validated at `read` and no RSA pair ever generated. AES-GCM seals as `nonce(12) ‖ ciphertext ‖ tag(16)`, which is what WebCrypto answers. Replaces `openssl_*`'s primitive half and `sodium_*` | [0051](../decisions/0051.md), [0179](../decisions/0179.md) |
| `Core\Password` | `hash(secret string): string`, `verify(secret string, string): bool`, `needsRehash(string): bool` — **no algorithm argument**. `verify` and `needsRehash` also read a PHP-stored bcrypt hash (`verify` verifies it, `needsRehash` answers `true`); `hash` writes only Argon2id. Replaces `password_hash`, `password_verify`, `crypt` | [0063](../decisions/0063.md), [0129](../decisions/0129.md) |
| `Core\Jwt`, `Core\Jwe`, `Core\Csrf`, `Core\Totp`, `Core\SignedCookie`, `Core\Signature` | the closed roster; a JWT's algorithm comes from the key, never the token. `Jwt::sign` widens to `secret bytes\|Crypto\KeyPair` (HS256, RS256, PS256, ES256 or EdDSA by the key's kind), `Jwt::signObject` signs a shape or deriving class under a pair alone, and `Jwt::verifyIssued<T>` verifies a token another party issued against a `Jwt\KeySet` key found by `kid`, answering its claims as a `tainted` shape ([0179](../decisions/0179.md)). `Core\Jwe` is compact JWE with `A256GCM` alone, its key-management algorithm picked by which `Jwe\Key` static built the key — `shared` (`dir`), `password` (`PBES2-HS256+A128KW`), `recipient`/`own` (`ECDH-ES`) — and its payload answered **`tainted`** ([0179](../decisions/0179.md)). `Core\Signature::sign(array<string> $payload, {keys: array<secret bytes>, until: ?Time\Instant} $settings): string` and `::verify(string $token, array<secret bytes> $keys): array<tainted string>` are the detached pair — a canonical payload map, never assembled text; `until` is a required key whose `null` is the forever spelling; `verify` answers a **`tainted`** payload or throws ([0146](../decisions/0146.md)). A payload value is text at both halves for the reason a JWT claim is: `tainted` is defined over `string` and `bytes`, so an `array<mixed>` could not have carried the qualifier a program reaches | [0060](../decisions/0060.md) |
| `Core\Process` | `run`, `spawn` — argv only, never a shell string | [0044](../decisions/0044.md) |
| `Core\Mail` | an SMTP client with structured headers. Replaces `mail()` | [0051](../decisions/0051.md) |
| `Core\Cache` | `local(): Cache\Store`, `process(): Cache\Store`, `shared(): Cache\Store` — a member per tier, none of them taking an argument. `process()` is one store per serving process, coherent across its cores, in memory only and bounded by `[cache.process] max_size`. On the store: `put(string $key, mixed $value, {ttl?: Duration}): void` / `get(string $key): mixed` — copy-in/copy-out, and a miss answers `null` rather than throwing — plus `forget(string $key): void`, all three tiers gaining a lifetime and a `forget` together. A secret meets a cache only through `putSecret(string $key, secret string $value, Duration $ttl, array<secret bytes> $keys): void` and `getSecret(string $key, array<secret bytes> $keys, {fill?: callable(): Cache\SecretEntry, wait?: Duration}): ?secret string`, which store XChaCha20-Poly1305 **ciphertext** bound to the app, the entry's name and its expiry — so no `secret` crosses the boundary, `put` still refuses one and `get` answers `null` for a sealed entry. A sealed entry that does not open, under any key of the ring or past its sealed expiry, is a **miss**; on a miss `fill` runs in exactly one caller per process while every other waits for at most its `wait` (`[cache.process] fill_wait`) and then throws `TimeoutError`, a throwing `fill` releases the waiters with nothing, and an entry inside the last fifth of its lifetime is answered to everyone while one caller replaces it. `Cache\SecretEntry::of(secret string $value, Duration $ttl)` is what a `fill` answers. Sealing hides a secret from code that knows an entry's name but not the ring and keeps it out of the shared store in the clear; it does not protect it from a compromised process. Replaces `apcu_*`, `memcached` for the local case | [0059](../decisions/0059.md), [0181](../decisions/0181.md) |
| `Core\Log`, `Core\Fatal` | structured logging. `write(Log\Level, string $message, array<string, mixed> $fields = [])`, where `Log\Level` is `Debug`, `Info`, `Warn`, `Error`, `Critical`. Both refuse a `secret` argument at the call site — a sink on the [0033](../decisions/0033.md) axis; on the `tainted` axis neither is a sink, because a field is *data* ([0088](../decisions/0088.md) § 7) and logging tainted input is the point | [0020](../decisions/0020.md), [0092](../decisions/0092.md) |
| `Core\Debug` | `dump(mixed ...$values)` and `render(mixed)` — replacing `var_dump`, `print_r`, `var_export`, `debug_zval_refcount` — plus the coverage, tracing and profiling controls | `dump`/`render`: [0092](../decisions/0092.md); the probes: [0018](../decisions/0018.md) |
| `Core\Signal` | graceful shutdown only. What remains of `pcntl_*` after `fork` is refused | [0051](../decisions/0051.md) |
| `Core\Script` | `onExit(callable(Script\ExitReport): mixed $hook): void` — FIFO end-of-script hooks, run once as the last user code at every non-fatal ending (normal, `exit`, uncaught throw, `finish`); each receives a readonly `Script\ExitReport` — `reason: Script\ExitReason` (`Normal`, `ExitCall`, `UncaughtThrow`, `Finish`), `status: int`, `error: ?Throwable` — and may declare no parameter. Never fires on a `FATAL` or a cancellation, and observes the ending rather than changing it. Replaces `register_shutdown_function`'s non-fatal uses. `finish(): void` ends the script where it is called and does not return: every `finally` between the call and the root runs, then that queue drains with `reason: Finish` at status `0`, which is the ending `exit` is not. The value it raises is a second, parentless root of the exception tree, so no `catch` arm admits it and an arm naming that class is refused at compile time | [0127](../decisions/0127.md), [0178](../decisions/0178.md) |
| `Core\Os` | host and process facts (`pid`, `hostname`, `cpuCount`, `residentBytes`, `loadAverage`). Replaces `posix_*` minus fork, `php_uname`, `getrusage`, `sys_getloadavg`. A **request's** memory is `Core\Budget`'s and never here — `residentBytes` is the process's resident set, which is the question `Core\Os` is entitled to answer | [0051](../decisions/0051.md), [0148](../decisions/0148.md) |
| `Core\Budget` | `memoryHeld(): int`, `memoryPeak(): int`, `memoryLimit(): int` — this request's held bytes, its high-water mark and the ceiling both are measured against. Three members and no `$real_usage` boolean; an uncapped request answers `0` from `memoryLimit`. Replaces `memory_get_usage` and `memory_get_peak_usage` | [0148](../decisions/0148.md), [0005](../decisions/0005.md) |
| `Core\Config` | `set(string, string): bool`, `get(string): ?string`, `restore(string): void`, `all(): array<string, string>` — the request-local overlay over `nvs.toml`. Replaces `ini_set`, `ini_get`, `ini_restore`, `ini_get_all`, `set_time_limit`. String-in/string-out because the directive name is dynamic; the registry parses with the same parser the boot path uses | [0005](../decisions/0005.md), [0064](../decisions/0064.md) |

## 17. Documents and formats

| Class | Surface | Note |
|---|---|---|
| `Core\Html` | `escape` (the auto-applied launderer), `sanitize`, `Markup`, and the WHATWG HTML parser — never-failing, producing `Core\Xml`'s tree: one node family, two front doors | `rule:security/tainted-qualifier` owns both launderers; the parser and the shared tree are `rule:core-classes/html-parsing` |
| `Core\Xml` | one API replacing DOM, SimpleXML, XMLReader, XMLWriter, `xml_parser_*` and XSLTProcessor. Its **tree** API and its **streaming** reader/writer are different jobs, not twins — the tree materialises, the stream does not, and no operation is available through both | the one place in this file where two shapes of the same subsystem coexist, stated explicitly so it is not read as an exception to R17. The tree is also what `Core\Html`'s parser produces (`rule:core-classes/html-parsing`), and lands with it |
| `Core\Compress` | `compress(bytes\|string $data, Codec $codec): bytes` and `decompress(bytes $data, Codec $codec, uint $maxBytes = 67108864, uint $maxRatio = 1000): bytes` — one API replacing `gzopen` handles, `deflate_init` contexts and `zlib.*` stream filters. `Core\Codec` is five cases, not four: `Gzip`, `Zlib` and `Deflate` are one stream under three headers, which is what PHP spelled as three function names, plus `Brotli` and `Zstd`. The incremental half is `compressor(Codec $codec): Compress\Compressor` and `decompressor(Codec $codec, uint $maxBytes = 67108864, uint $maxRatio = 1000): Compress\Decompressor`, each an object with `add` and `finish`, replacing `deflate_init`/`deflate_add` and `inflate_init`/`inflate_add` — two classes because a `Compress\Decompressor`'s `finish` answers `tainted bytes` and a `Compress\Compressor`'s answers `bytes` | the bound is `rule:core-classes/decompression-bound`, [0166](../decisions/0166.md), and a decompressing stream carries it once for the whole stream rather than once per chunk; `crates/nvs-stdlib/src/compress.rs` owns why the streams hold their chunks and what that spends |
| `Core\Zip` | `entries(bytes $archive): array<tainted string>`, `read(bytes $archive, string $name, uint $maxBytes = 67108864, uint $maxRatio = 1000): tainted bytes` and `extract(bytes $archive, string $destination, uint $maxBytes = 67108864, uint $maxRatio = 1000): uint` — replacing `zip_*`, `ZipArchive` and the checking every caller of `extractTo` was supposed to remember. Core rather than an extension because `../` entries, absolute-path entries, symlink entries and decompression bombs are *policy*, and policy must be non-optional. **The refusals are the reader's**, so a program that walks the entries itself has nothing left it could have checked; the bomb half is `Core\Compress`'s bound applied per entry and across the archive, not a second rule | [ADR 0051 § 3](../decisions/0051.md), [0166](../decisions/0166.md); the names are tainted for `rule:security/tainted-sources`' reason, and what is on disk is `crates/nvs-stdlib/src/zip.rs`'s known gaps |
| `Core\Mime` | `detect(bytes $data): Mime\Type` and `mediaType(Mime\Type $type): string` — type detection by magic bytes, from a fixed table of literal signatures rather than libmagic's rule interpreter, and never from a file extension: a type read off a name is a type an attacker chose. `Core\Mime\Type` is closed and its zero case is `Unknown`, so a caller compares against a case rather than against a spelling that may never occur, and a format serialized as text answers `Unknown` rather than a guess | detection launders nothing (`rule:security/launderers-are-sink-named`): bytes that detect as `image/png` are still `tainted`. The table's shape, and what deliberately has no case, are `crates/nvs-stdlib/src/mime.rs`'s module doc |

## 18. `Core\Db`

Semantics are `rule:core-classes/db-one-api` — connection naming and memoization, the capability split,
why there is no `prepare`, the transaction shape, the coercion rule and the full SQL type map. This section
owns the signatures only. Every member needs `db.connect` or `db.open`, and a connection's credentials live
in a root-owned `[db.<name>]` block (`rule:config/the-file-is-nvs-toml-and-it-is-toml`).

### Entry points on `Core\Db`

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `connect` | `connect(string $name, {shared?: bool, timeout?: Duration}): Db\Connection` | `new PDO`, `mysqli_connect`, `pg_connect`, `new SQLite3` | **sink** (name) |
| `open` | `open(Db\Settings $settings, {shared?: bool}): Db\Connection` | a runtime-built DSN | **sink** (host) |
| `inList` | `inList(array<mixed> $values): Db\InList` | `implode(",", array_fill(0, n, "?"))` | |
| `quoteIdentifier` | `quoteIdentifier(tainted string $name): string` | `mysqli_real_escape_string` on a table/column name | **launder** (identifier) |

`PDO::quote`, `mysqli_real_escape_string` and `pg_escape_string` have **no equivalent**: binding is the
mechanism (`rule:security/sink-predicate`).

### `Core\Db\Queryable` — the interface both a connection and a transaction satisfy

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `query` | `$q->query(string $sql, array<mixed> $params, {timeout?: Duration}): Db\Rows<Row>` | `PDO::query`/`prepare`+`execute`, `mysqli_query` | **sink** (sql) |
| `queryAs` | `$q->queryAs<T>(string $sql, array<mixed> $params, {timeout?}): Db\Rows<T>` | `PDO::FETCH_CLASS`, hand-written hydration | **sink** (sql) |
| `execute` | `$q->execute(string $sql, array<mixed> $params, {timeout?}): Db\Write` | `PDO::exec`, `PDOStatement::execute`, `lastInsertId` | **sink** (sql) |
| `executeMany` | `$q->executeMany(string $sql, array<array<mixed>> $sets, {timeout?}): uint` | a loop around `PDOStatement::execute` | **sink** (sql) |
| `stream` | `$q->stream(string $sql, array<mixed> $params, {timeout?, chunk?: uint}): Iterable<Db\Row>` | `MYSQLI_USE_RESULT`, `PDO::CURSOR_*`, `pg_query` + `pg_fetch_row` | **sink** (sql) |
| `streamAs` | `$q->streamAs<T>(string $sql, array<mixed> $params, {timeout?, chunk?: uint}): Iterable<T>` | — | **sink** (sql) |
| `transaction` | `$q->transaction(callable(Transaction): T $fn, {isolation?: Isolation, readOnly?: bool, retries?: uint}): T` | `beginTransaction`/`commit`/`rollBack`, `SAVEPOINT` | |

`Core\Db\Connection` implements it; `Core\Db\Transaction implements Queryable by $connection`
(`rule:classes/no-traits`), so the surface is
declared once. A nested `transaction` is a savepoint. Params are one `array<mixed>`: list-keyed for `?`,
string-keyed for `:name`, mixing throws.

| Type | Members beyond `Queryable` |
|---|---|
| `Connection` | `$c->close(): void`; `->driver(): Driver`, `->serverVersion(): string`, `->isOpen(): bool` |
| `Transaction` | `$t->rollBack(string $reason): void` — sets the rollback-only flag and throws `Db\RolledBack` |

There is no `commit`, no connection-level `rollBack`, no `inTransaction`, no explicit savepoint member and
no `lastInsertId` — `rule:core-classes/db-transactions` and `rule:core-classes/db-one-api` say why each is absent.

`queryAs<T>`/`streamAs<T>` take an inline shape or a class implementing `Core\Db\Codec`
(`static fromRow(Db\Row): static`), whose body is generated from the class's own declared properties by the
opt-in `#[Db\Derive]` attribute — `type Core\Db\Derive = {};` on a class,
`type Core\Db\Field = {name?: string, skip?: bool};` on a property, both governed by
`rule:core-classes/derive-attribute`. There is no `toRow` and no generated `INSERT`: a write is an
explicit statement with bound parameters. Because a row is always `tainted`
(`rule:core-classes/db-column-types`), a `T` whose text-carrying fields are unqualified is a diagnostic at
the call site naming the field.

### Results

| Type | Members | Replaces |
|---|---|---|
| `Rows<T>` | `->all(): array<T>`, `->first(): ?T`, `->value(): mixed`, `->column(int\|string $key): array<mixed>`, `->count(): uint`, `->columns(): array<Column>`; `Iterable<T>` | `fetchAll`, `fetch`, `fetchColumn`, `rowCount` on a select, `getColumnMeta`, `FETCH_CLASS`, `fetchObject` |
| `Row` | `->has(string $name): bool` *(neutral)*, `->get(string $name): mixed`, `->toArray(): array<string, mixed>`, and the typed readers below | `FETCH_ASSOC`, `FETCH_NUM`, `FETCH_OBJ` |
| `Write` | `->affected(): uint`, `->changed(): ?uint`, `->lastId(): ?uint` | `rowCount` on a write, `lastInsertId`, `mysqli_info` |
| `Column` | `->name(): string`, `->type(): ColumnType`, `->nullable(): bool` | `getColumnMeta`, `mysqli_fetch_field` |
| `InList` | opaque; produced by `Db::inList`, accepted only as a bound parameter | — |

`Rows` is one generic class and not two: `query` answers `Rows<Row>`, `queryAs<T>` the same class at the
`T` its call site wrote, and a program that declares the type writes the argument like every other `Core`
generic — there is no bare `Rows` spelling that leaves it open.

`Row`'s typed readers each take `(string $name)` and return `?T`, a `null` being a NULL column: `string`,
`bytes`, `int`, `uint`, `float`, `bool`, `decimal`, `instant`, `date`, `time`, `uuid`. The requested type
drives a lossless conversion or throws (`rule:core-classes/db-column-types`); `->string` accepts
text-family columns only, and the universal path is `->get()` plus `as`. An unknown column name throws.

A class participates in `queryAs<T>` by implementing `Core\Db\Codec`, which declares
`static fromRow(Db\Row $row): static` — read-only by design, since writing rows from objects is an ORM
concern and not `Core`'s (`rule:core-api/tier-placement` test 6). The other accepted `T`
is an inline shape (`rule:types/object-top`), validated per row.

### Enums, settings and errors

```
Driver     { MySql, MariaDb, Postgres, Sqlite, SqlServer }   // detected from the handshake
Isolation  { ReadUncommitted, ReadCommitted, RepeatableRead, Snapshot, Serializable }
Tls        { Disabled, Required, VerifyCa, VerifyFull }      // VerifyFull is the default over TCP
ColumnType { Int, Uint, Float, Decimal, Text, Bytes, Bool, Date, Time, DateTime, Instant, Uuid, Json, Other }
ErrorKind  { UniqueViolation, ForeignKeyViolation, NotNullViolation, CheckViolation, Deadlock,
             SerializationFailure, ConnectionLost, Timeout, Syntax, Permission, Other }
```

`Db\Settings` is a discriminated union over `rule:types/literal-types`'s
enum-case types — a `host` on a SQLite literal is a compile error:

```
{driver: Driver::MySql|Driver::MariaDb|Driver::Postgres|Driver::SqlServer,
 host: string, port?: uint, database: tainted string, user: tainted string,
 password: secret tainted string, tls?: Tls, timeZone?: Core\Time\Zone,
 timeout?: Duration, statementCache?: uint}
| {driver: Driver::Sqlite, path: string, timeZone?: Core\Time\Zone, timeout?: Duration}
```

Two `Throwable`s join § 10's tree, both under `Core\Db`:

- `DbError extends RuntimeError` — readonly `kind: ErrorKind`, `sqlState: ?string`, `driverCode: ?int`,
  `constraint: ?string`, `sql: ?string`. Bound parameter values never appear on it
  (`rule:security/secret-qualifier`).
- `RolledBack extends RuntimeError` — readonly `reason: string`; thrown by `Transaction::rollBack` and
  propagated out of the owning `transaction()` call.

## 19. `Core\Task`

Semantics are `rule:concurrency/one-scheduler` — what cancellation does and does
not run, why control never leaves a call with work still running, and the `[deferred]` bound. This section
owns the signatures only. Nothing here needs a capability; it needs the M5 scheduler, which is why it sits
in Part II and lands at **M5** rather than M8.

| Member | Signature | Replaces | Q |
|---|---|---|---|
| `all` | `all({name: callable, …} $tasks, {limit?: uint, deadline?: Duration}): {name: T, …}` | — | |
| `map` | `map(array<T> $items, callable(T, string): U $fn, {limit?: uint, deadline?: Duration}): array<U>` | `curl_multi_*` | |
| `afterResponse` | `afterResponse(callable(): mixed $fn, {deadline?: Duration}): void` | `fastcgi_finish_request` | |

`all` takes a shape literal of zero-argument closures and returns a shape with the same field names, each
carrying **that closure's own declared return type**. Every field must be a written `fn` literal — a
`callable`-typed variable is a compile error naming the field, pending
`rule:types/grammar`'s typed `callable` signatures. `map` preserves its
input's keys and order regardless of completion order, and its callback receives `($value, $key)` like
every other callback here (R9).

The first throw cancels every sibling and propagates after they are gone; a `deadline` cancels everything
and throws `TimeoutError`. A cancelled task runs **no user code** — no `catch`, no cleanup — while native
teardown (arena, refcounts, an open transaction's rollback) still runs. `afterResponse` work is charged to
the request tree, may not touch `Core\Response`, and throws at the call site past
`[deferred] max_concurrent` rather than queueing. It is **not a durable queue**: nothing retries and a lost
process loses the work.

`Core\Task\Channel` and the `spawn`/`await` keywords are the concurrency *language* surface and belong to
[docs/spec/00-overview.md](00-overview.md) § 2 with M5, not to this file.

---

## Counting the result

| | PHP | Novis |
|---|---|---|
| Global functions / `Core` members | ~1,900 | ~450 |
| Sort functions | 11 + `array_multisort` | 2 |
| `array_diff`/`array_intersect` variants | 12 | 2 |
| Array-combining rules | 5, chosen by a key's type | 4, chosen by the member's name |
| `strpos` variants | 12 | 4 |
| `printf` variants | 9 | 2 |
| Date APIs | ~40 procedural + 2 mutable/immutable class trees | 1 immutable object tree |
| XML APIs | 6 | 1 |
| Socket APIs | 3 | 1 |
| Ways to run a program | 7 | 1 |
| Ways to hash | 3 | 1 |
| Database APIs | 4 (`PDO`, `mysqli` ×2, `pgsql`, `sqlite3`) | 1 |
| Exception classes in the stdlib | 13 SPL + 8 `Error` | 9 |

The reduction is entirely in restatements. The library covers strictly more than PHP's: an HTTP client,
SMTP, cache, CSV, UUID, a test surface, `rule:security/protocol-roster`'s
protocol roster, and typed date arithmetic are all things PHP leaves to userland.
