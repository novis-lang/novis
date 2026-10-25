---
summary: every array function as a pure member — filter, map, sort, search, reshape, combine and aggregate, always answering a new array
keywords: array, list, dictionary, count, filter, map, reduce, keys, values, first, last, search, slice, chunk, push, pop, append, prepend, pad, reverse, flip, fill, range, combine, column, merge, overlay, underlay, diff, intersect, unique, sum, product, average, min, max, sort, natural sort, stable sort, isset, array + array, group by, copy-on-write, immutable
---

`Core\Arr` has the array functions as static methods. Each one returns a **new** array and never
changes its argument. `Core\Arr::sort` returns the sorted array, and `append` returns the longer one.
Keys are always `string`: `$a[0]` and `$a["0"]` are the same entry, `keys` returns `array<string>`,
and a callback's key parameter is `string`. A callback receives `($value, $key)` and may declare
fewer parameters. `reduce` passes the carry first. A missing value is `null`: `first`, `last`,
`find`, `keyOf` and `min` return `null` for an empty array. `count` returns a `uint`. `$a + $b` does
not compile. `overlay` and `underlay` combine maps, and `appendAll` joins lists.

`shapeAs<T>` is the one member that answers something other than an array. It reads its subject as the
type written at the call site — an inline shape, `{ name: string, score: int }`, or a class carrying
`#[Core\Json\Derive]` — converting every named field with `as` and leaving every key the type does not
name behind, so a form carrying a CSRF token does not break a handler reading two fields out of it.
That is the whole story for loose input: an `array<T>` is homogeneous and there is no per-key-typed
one, so a form, a query string or a decoded document is converted **once, at the boundary**, where a
failure is still a `400` — and nothing downstream is left holding a `mixed` to check. A field that is
absent, or that holds a value `as` refuses for its declared type, throws `ParseError` carrying every
field that failed at its own dotted path, rather than the first of them. `{ name?: T }` marks a key that
may be absent, and `??` and `isset` read one without throwing. `Core\Request::queryAs` and `postAs`
are this member over what a request carried. Text that came from outside the program, such as a form
or an `array<mixed>`, is `tainted`. A text field it fills must then be declared `tainted` too.

```nvs
<?nvs
array<int> $scores = ['ada' => 92, 'bo' => 67, 'cy' => 85, 'di' => 74];
var $passed = Core\Arr::filter($scores, fn(int $n): bool => $n >= 70);
echo Core\Json::encode($passed), "\n";
var $ranked = Core\Arr::sort($passed, { order: Core\Arr::Order::Desc, preserveKeys: true });
echo Core\Json::encode(Core\Arr::keys($ranked)), "\n";
echo Core\Arr::first($ranked) ?? 0, ' ', Core\Arr::firstKey($ranked) ?? '-', "\n";
var $labels = Core\Arr::map($scores, fn(int $n, string $k): string => $k . ':' . ($n as string));
echo Core\Str::join(Core\Arr::values($labels), ' '), "\n";
echo Core\Arr::sum($scores), ' ', Core\Arr::count($scores), ' ', Core\Arr::average($scores) ?? 0, "\n";
int $total = Core\Arr::reduce($scores, fn(int $carry, int $n): int => $carry + $n, 0);
echo $total, ' ', Core\Arr::max($scores) ?? 0, "\n";
array<int> $evens = Core\Arr::filter(Core\Arr::range(1, 10), fn(int $n): bool => $n % 2 == 0);
echo Core\Json::encode(Core\Arr::values($evens)), ' ', Core\Arr::contains($evens, 4) ? 'yes' : 'no', "\n";
var $byLength = Core\Arr::groupBy(['fig', 'pear', 'kiwi', 'plum'], fn(string $w): int => Core\Str::length($w) as int);
echo Core\Json::encode($byLength), "\n";
echo Core\Json::encode($scores), "\n";
var $entry = Core\Arr::shapeAs<{ name: string, score: int, note?: string }>(['name' => 'ada', 'score' => '92', 'csrf' => 't0ken']);
echo $entry->name, ' ', $entry->score as string, ' ', $entry->note ?? 'no note', "\n";
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
