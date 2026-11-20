---
summary: a map keyed by object identity — a typed side table that keeps a value for each object
keywords: identity, object identity, object key, per-object data, side table, map, dictionary, lookup
---

`Core\ObjectMap<K, V>` associates a value with an object, matched by **identity**: the one object is one
key however many variables hold it, and two objects that agree on every property are two keys. `array<T>`
cannot key on an object, so this is where a per-object side table lives. It is built with
`new Core\ObjectMap<K, V>()`, `get` answers `?V` rather than throwing — there is no `$m[$k]` subscript on it —
and `has` is the one member that tells an absent key from a stored `null`.

A `foreach` loop walks the map in the order the keys were added. With a key variable,
`foreach ($map as var $key => var $value)` gives each key with its value. With only a value variable,
the loop gives the keys. A `foreach` does not copy the map. If the body changes the map, the loop still
walks the entries the map had when it began. So the body may `remove` the entry it is on.

```nvs
<?nvs
class Tag
{
    public function constructor(public string $name)
    {}
}

var $red = new Tag('red');
var $blue = new Tag('blue');

var $weights = new Core\ObjectMap<Tag, int>();
$weights->set($red, 3);
$weights->set($blue, 7);
$weights->set($red, 4);

echo 'red=', $weights->get($red) ?? 0, "\n";
echo 'other red=', $weights->get(new Tag('red')) ?? -1, "\n";
echo 'count=', $weights->count(), "\n";

foreach ($weights as Tag $tag => int $weight) {
    echo $tag->name, '=', $weight, ' ';
}
echo "\n";

$weights->remove($blue);
echo $weights->has($blue) ? 'blue kept' : 'blue gone', "\n";
echo 'keys=', Core\Arr::count($weights->keys()), ' values=', Core\Arr::count($weights->values()), "\n";
```
```output
red=4
other red=-1
count=2
red=4 blue=7
blue gone
keys=1 values=1
```
