Iteration has exactly two interfaces, both reserved names the compiler owns:

```
interface Iterator<T> {
    public function advance(): bool;   // move to the next element; false once exhausted
    public function current(): T;      // the element advance() just moved to
}

interface Iterable<T> {
    public function iterate(): Iterator<T>;
}
```

`Iterator<T>` is a **single-pass cursor**; `Iterable<T>` is a thing that can hand out a fresh one, so
draining an `Iterable<T>` twice walks it twice and draining an `Iterator<T>` twice does not. The pair
is `advance`-then-`current` rather than one `next(): ?T`, because a single method cannot tell "the
sequence ended" from "the next element is `null`", and Novis has nullable types.

Both members are declared without a body, and a class is held to every one of them. A cursor carries
**no key half** — nothing in `Iterator<T>` produces one — which is why a `foreach` over a cursor may
bind a value and not a key (`rule:iteration/foreach-subjects`).
