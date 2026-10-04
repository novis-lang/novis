An anonymous function captures exactly the outer variables its body reads, snapshotted **by value**
at the point the anonymous function is evaluated. There is no syntax to opt a variable in or out, and no
by-reference capture: `use ($y)` and `use (&$y)` are both diagnostics, the second with its own wording
where the intent was mutation visible outside the anonymous function. `$this` is captured like any
other binding when the body names it.

Dropping `use (&$y)` has one real consequence, and it is deliberate: **sharing one mutable cell
between two independent callables** has no builtin replacement. It is written in user code with an
ordinary object, because capturing an object by value still shares the same heap object — only
rebinding a bare scalar or a copy-on-write `array<T>` local from inside an anonymous function is
actually lost.

```php
class Counter { public int $value = 0; }
$count = new Counter();
$increment = fn() => $count->value++;   // both capture $count by value...
$get       = fn() => $count->value;     // ...but $count is a heap object, so they share it
```

There is no `Core\Ref<T>` or boxed-cell builtin for this, and an anonymous object
(`rule:types/anonymous-object`) is the lightweight way to write the carrier.
