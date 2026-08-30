---
summary: a map keyed by object identity — the typed replacement for `SplObjectStorage` used as a map and for every `spl_object_id` side table
keywords: SplObjectStorage, spl_object_id, WeakMap, identity, object key, side table, map, dictionary, lookup
---

`Core\ObjectMap<K, V>` associates a value with an object, matched by **identity**: the one object is one
key however many variables hold it, and two objects that agree on every property are two keys. `array<T>`
cannot key on an object, so this is where a per-object side table lives. It is built with
`new Core\ObjectMap<K, V>()`, `get` answers `?V` rather than throwing — there is no `$m[$k]` subscript on it —
and `has` is the one member that tells an absent key from a stored `null`. The map is `Iterable`: a
`foreach` yields its **keys** in insertion order, walking a snapshot taken when the loop began, so the body
may `remove` what it is walking.

```nvs
<?nvs
class Tag {
    public function constructor(public string $name) {}
}

var $red = new Tag("red");
var $blue = new Tag("blue");

var $weights = new Core\ObjectMap<Tag, int>();
$weights->set($red, 3);
$weights->set($blue, 7);
$weights->set($red, 4);

echo "red=", $weights->get($red) ?? 0, "\n";
echo "other red=", $weights->get(new Tag("red")) ?? -1, "\n";
echo "count=", $weights->count(), "\n";

foreach ($weights as Tag $tag) {
    echo $tag->name, " ";
}
echo "\n";

$weights->remove($blue);
echo $weights->has($blue) ? "blue kept" : "blue gone", "\n";
echo "keys=", Core\Arr::count($weights->keys()), " values=", Core\Arr::count($weights->values()), "\n";
```
```output
red=4
other red=-1
count=2
red blue
blue gone
keys=1 values=1
```
