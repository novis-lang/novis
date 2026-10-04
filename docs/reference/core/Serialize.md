---
summary: a value graph — scalars, arrays, objects, cycles included — copied into Novis's own byte format and rebuilt from it
keywords: serialize, unserialize, __serialize, __sleep, __wakeup, persistence, byte format, graph copy
---

`Core\Serialize::encode` copies the whole value graph under a value into `bytes` in Novis's own
closed format, and `decode` rebuilds it: an object comes back as an instance of the class this
program declares, its declared properties assigned directly — no constructor runs, and there is no
`__serialize`, `__sleep` or `__wakeup` hook. A cycle is preserved as a cycle. `decode` answers
`mixed`, so narrow it with `as`; it refuses foreign bytes, another format version, and a class whose
declared properties no longer match with `ParseError`, and `encode` throws `LogicError` on a callable.

```nvs
<?nvs
class Slot {
    public function constructor(public int $n) {}
}
class Ring {
    public int $id = 0;
    public ?Ring $self;
    public function constructor(int $id) { $this->id = $id; $this->self = null; }
}

bytes $blob = Core\Serialize::encode(new Slot(7));
Slot $back = Core\Serialize::decode($blob) as Slot;
echo "n=", $back->n, "\n";

var $ring = new Ring(1);
$ring->self = $ring;
Ring $copy = Core\Serialize::decode(Core\Serialize::encode($ring)) as Ring;
echo "cycle=", ($copy->self?->id ?? 0), "\n";

try {
    Core\Serialize::decode("O:4:\"Slot\":1:{s:1:\"n\";i:7;}" as bytes);
} catch (ParseError $e) {
    echo "refused\n";
}
```
```output
n=7
cycle=1
refused
```
