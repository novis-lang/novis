---
summary: structured concurrency — run a fixed set or a whole array of callables as child tasks and get every result back before the call returns
keywords: curl_multi_*, structured concurrency, parallel, concurrent, child task, fan-out, limit, deadline, TimeoutError, cancellation, all, map
---

`Core\Task` runs callables as concurrent child tasks and never returns while one is still running.
`all` takes a shape whose every field is a zero-argument callable and answers a shape with the same
names, each field typed by that field's declared return — a field whose callable declares no return
type answers `mixed`, and only that field. `map` calls one callable per element and answers the results under the subject's own keys, in the
subject's order, whatever order the children finished in. Both take the same options: `limit` caps how many
children run at once (the rest wait, nothing is refused) and `deadline` bounds the **whole call** — when it
expires every child is cancelled and the call throws `TimeoutError`. The first child to throw cancels its
siblings and its throw propagates once they are gone; a cancelled child runs no more of its own code, not
even a `catch`.

```nvs
<?nvs
var $page = Core\Task::all({
    rows:  fn(): array<int> => [1, 2, 3],
    label: fn(): string     => "all",
});
echo $page->label, "=", Core\Arr::count($page->rows), "\n";

var $lengths = Core\Task::map(["fig" => "fig", "pear" => "pear"], fn(string $word): uint => Core\Str::length($word), {limit: 1});
foreach ($lengths as string $key => uint $length) {
    echo $key, ":", $length, "\n";
}

try {
    Core\Task::map([1, 2], fn(int $n): int => {
        Core\Time::sleep(200ms);
        return $n;
    }, {deadline: 20ms});
    echo "finished in time", "\n";
} catch (TimeoutError $late) {
    echo "deadline hit", "\n";
}
```
```output
all=3
fig:3
pear:4
deadline hit
```
