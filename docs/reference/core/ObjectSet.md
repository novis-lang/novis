---
summary: a set of objects held by identity — the typed replacement for `SplObjectStorage` used as a set
keywords: SplObjectStorage, spl_object_id, identity, set, membership, union, intersect, diff, deduplicate objects
---

`Core\ObjectSet<T>` holds each object at most once, matched by **identity**: adding the same object twice
leaves one member, and a second object equal in every property is a second member. It is built with
`new Core\ObjectSet<T>()`; `add`, `has` and `remove` are the membership members, and the set is `Iterable`
— a `foreach` yields its members in insertion order, over a snapshot, so the body may `remove` what it is
walking. `union`, `intersect` and `diff` answer a new set and leave both operands untouched; the binary
types that new set as a bare `Core\ObjectSet`, so it can be counted, asked `has` and walked as `mixed`
(`foreach ($both as mixed $m)` and then `$m as Tag`), but a `foreach … as Tag $t` over it is refused.

```nvs
<?nvs
class Tag {
    public function constructor(public string $name) {}
}

var $red = new Tag("red");
var $blue = new Tag("blue");
var $green = new Tag("green");

var $seen = new Core\ObjectSet<Tag>();
$seen->add($red);
$seen->add($blue);
$seen->add($red);
echo "count=", $seen->count(), "\n";
echo $seen->has($red) ? "has red" : "no red", "\n";
echo $seen->has(new Tag("red")) ? "has other red" : "no other red", "\n";

var $wanted = new Core\ObjectSet<Tag>();
$wanted->add($blue);
$wanted->add($green);
echo "union=", $seen->union($wanted)->count(), "\n";
echo "both=", $seen->intersect($wanted)->count(), "\n";
echo "only seen=", $seen->diff($wanted)->count(), "\n";

$seen->remove($red);
foreach ($seen as Tag $tag) {
    echo $tag->name, "\n";
}
echo $seen->isEmpty() ? "empty" : "not empty", "\n";
```
```output
count=2
has red
no other red
union=3
both=1
only seen=1
blue
not empty
```
