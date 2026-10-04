`Core\Task::map(array<T> $items, callable $fn, {limit?, deadline?}): array<U>` runs one callback per
element concurrently. Subject first, the callback receiving `($value, $key)` like every other
callback in the library, and the answer typed `array<U>` from that one callback's declared return
type.

**The result preserves the input's keys and its order, whatever order the children finished in.**
Completion order is a scheduling detail and is never observable in the answer.

`map` and `rule:concurrency/all-answers-a-typed-shape` are two members because they are two jobs — a
fixed set of differently-typed things, and one operation over many same-typed things — and not two
spellings of one. An anonymous object cannot express "one per element of a runtime array", and an array
cannot carry a per-element type. Each refuses the other's subject rather than coercing it.
