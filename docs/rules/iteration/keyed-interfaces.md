Iteration over keys and values has two more interfaces, both reserved names the compiler owns beside
`Iterable` and `Iterator`:

```
interface KeyedIterator<K, V> {
    public function advance(): bool;   // move to the next entry; false once exhausted
    public function key(): K;          // the key of the entry advance() just moved to
    public function current(): V;      // the value of that entry
}

interface KeyedIterable<K, V> {
    public function iterateKeyed(): KeyedIterator<K, V>;
}
```

They keep the value pair's nouns (`rule:iteration/two-interfaces`), and a keyed cursor shares
`advance` and `current` with `Iterator<T>`. One class may implement both cursors when `T` is `V`; with
another `T` its `current()` cannot satisfy both, which is the ordinary override mismatch. `key()` and
`current()` follow `rule:iteration/cursor-out-of-range`: called outside the protocol, each throws the
same `LogicError`. A user class implements either at concrete types only, as
`implements KeyedIterable<string, int>` (`rule:iteration/concrete-generic-implements`).
`Core\ObjectMap<K, V>` is a `KeyedIterable<K, V>` and an `Iterable<K>` at once. `Core\ObjectSet`,
`Core\Heap` and `Core\Task\Channel` are not keyed: none has a value apart from its element.

**The method is `iterateKeyed`, not `iterate`.** The map implements both pairs, and Novis has no
overloading, so one class cannot have `iterate(): Iterator<K>` and `iterate(): KeyedIterator<K, V>`.

**The binding form decides which pair a loop drives**, so no existing program changes:

- A loop with a key binding drives the keyed pair when the subject reaches it. `KeyedIterable` wins
  over `KeyedIterator` when a class reaches both, because a fresh cursor is the safer of the two. A
  subject that reaches only `Iterable`/`Iterator` is still `E0444`, and an array keeps its own key
  rules (`rule:iteration/foreach-subjects`).
- A loop with a value binding alone drives `Iterable`/`Iterator` when the subject reaches either, so a
  map still gives its keys. A subject that reaches only the keyed pair is walked through it and binds
  `V`, as a value-only loop over an array binds the element.

**The key binding takes `K`.** `var $k` takes the subject's `K` with its qualifier, so a
`tainted string` key binds `tainted string` (`rule:types/var-inference`). A written key type is checked
as a value binding is: it must be assignable from `K`, so a wider type such as `object` is accepted. A
plain `string` over a `tainted string` key is `E0851`, the refusal an outside array key gets
(`rule:security/taint-propagation`), and any other mismatch is `E0401`. `inout` over the keyed pair is
`E0490`, as over any cursor: the program writes with `$map->set($key, ...)`.

**The loop sees the map as it was when it began**, as the value-only loop does. An entry the body adds
is not visited, one it removes is still visited with the value it had, and a `set` on an entry not yet
reached does not change the value the loop gives. The keyed cursor walks the store the map shares with
the loop, with no copy, so a body that writes copies the store once (`nvs_stdlib::cursor`'s
*What it spends*).

**What it spends.** Memory: one cursor object per loop and one reference on the map's store, held for
the loop and charged to its request, as the value-only loop holds. Speed: the loop grows linearly with
no allocation per entry. Each entry costs three dispatched calls, `advance`, `key` and `current`, where
an array loop pays none, and it saves the `get` lookup a program that walks the keys writes. The bench
of `lang:iteration/keys-and-values-from-a-map` records the measured figures beside an array loop over
the same entries.

Out of scope, each staying as it is: a keyed `yield` (`rule:iteration/one-way-only`), so a generator is
still an `Iterator<T>`; and `Core\Arr::from($map)`, which still gives the keys, since its parameter is
`array|Iterable|Iterator`.

**Declined:**
- **`Iterable<K, V>` with a second type argument.** A name's arity is one fact on the reserved roster,
  and the map would have to be `Iterable<K>` and `Iterable<K, V>` at once.
- **A `key()` member on `Iterator<T>`.** Every cursor and generator would owe a key it does not have,
  which is the counter `rule:iteration/foreach-subjects` refuses to invent.
- **`entries()` as the method name.** A user class is likelier to have an `entries` method of its own.
