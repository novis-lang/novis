---
summary: every array function as a pure member — filter, map, sort, search, reshape, combine and aggregate, always answering a new array
keywords: count, sizeof, array_filter, array_map, array_reduce, array_keys, array_values, array_key_first, array_key_last, reset, end, in_array, array_search, array_slice, array_splice, array_chunk, array_push, array_pop, array_shift, array_unshift, array_pad, array_reverse, array_flip, array_fill, array_fill_keys, range, array_combine, iterator_to_array, array_column, array_merge, array_replace, array_replace_recursive, array_diff, array_intersect, array_unique, array_count_values, array_find, array_any, array_all, array_sum, array_product, min, max, sort, rsort, asort, arsort, usort, uasort, ksort, krsort, uksort, natsort, array_multisort, array_is_list, array_key_exists, isset, array + array, group by, copy-on-write, immutable
---

`Core\Arr` is every PHP array function as a static member that answers a **new** array and never
changes its argument: there is no `sort($a)` by reference — `Core\Arr::sort` returns the sorted
array, and `append` returns the longer one. Keys are always `string`: `$a[0]` and `$a["0"]` name
the same entry, `keys` answers `array<string>`, and a callback's key parameter is `string`. A
callback receives `($value, $key)` and may declare fewer parameters; `reduce` puts the carry first.
Absence is `null` — `first`, `last`, `find`, `keyOf`, `min` over an empty array — and `count`
answers a `uint`. `$a + $b` does not compile: it is `underlay`; `array_merge` is `overlay` over
maps and `appendAll` over lists.

`shapeAs<T>` is the one member that answers something other than an array. It reads its subject as the
type written at the call site — an inline shape, `{name: string, score: int}`, or a class carrying
`#[Core\Json\Derive]` — converting every named field with `as` and leaving every key the type does not
name behind, so a form carrying a CSRF token does not break a handler reading two fields out of it.
That is the whole story for loose input: an `array<T>` is homogeneous and there is no per-key-typed
one, so a form, a query string or a decoded document is converted **once, at the boundary**, where a
failure is still a `400` — and nothing downstream is left holding a `mixed` to check. A field that is
absent, or that holds a value `as` refuses for its declared type, throws `ParseError` carrying every
field that failed at its own dotted path, rather than the first of them. `{name?: T}` marks a key that
may be absent, and `??` and `isset` read one without throwing. `Core\Request::queryAs` and `postAs`
are this member over what a request carried.

```nvs
<?nvs
array<int> $scores = ["ada" => 92, "bo" => 67, "cy" => 85, "di" => 74];
var $passed = Core\Arr::filter($scores, fn(int $n): bool => $n >= 70);
echo Core\Json::encode($passed), "\n";
var $ranked = Core\Arr::sort($passed, {order: Core\Order::Desc, preserveKeys: true});
echo Core\Json::encode(Core\Arr::keys($ranked)), "\n";
echo Core\Arr::first($ranked) ?? 0, " ", Core\Arr::firstKey($ranked) ?? "-", "\n";
var $labels = Core\Arr::map($scores, fn(int $n, string $k): string => $k . ":" . ($n as string));
echo Core\Str::join(Core\Arr::values($labels), " "), "\n";
echo Core\Arr::sum($scores), " ", Core\Arr::count($scores), " ", Core\Arr::average($scores) ?? 0, "\n";
int $total = Core\Arr::reduce($scores, fn(int $carry, int $n): int => $carry + $n, 0);
echo $total, " ", Core\Arr::max($scores) ?? 0, "\n";
array<int> $evens = Core\Arr::filter(Core\Arr::range(1, 10), fn(int $n): bool => $n % 2 == 0);
echo Core\Json::encode(Core\Arr::values($evens)), " ", Core\Arr::contains($evens, 4) ? "yes" : "no", "\n";
var $byLength = Core\Arr::groupBy(["fig", "pear", "kiwi", "plum"], fn(string $w): int => Core\Str::length($w) as int);
echo Core\Json::encode($byLength), "\n";
echo Core\Json::encode($scores), "\n";
var $entry = Core\Arr::shapeAs<{name: string, score: int, note?: string}>(["name" => "ada", "score" => "92", "csrf" => "t0ken"]);
echo $entry->name, " ", $entry->score as string, " ", $entry->note ?? "no note", "\n";
```
```output
{"ada":92,"cy":85,"di":74}
["ada","cy","di"]
92 ada
ada:92 bo:67 cy:85 di:74
318 4 79.5
318 92
[2,4,6,8,10] yes
{"3":["fig"],"4":{"1":"pear","2":"kiwi","3":"plum"}}
{"ada":92,"bo":67,"cy":85,"di":74}
ada 92 no note
```
