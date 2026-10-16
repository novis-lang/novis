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

```nvs
class Counter { public int $value = 0; }
$count = new Counter();
$increment = fn() => $count->value++;   // both capture $count by value...
$get       = fn() => $count->value;     // ...but $count is a heap object, so they share it
```

There is no `Core\Ref<T>` or boxed-cell builtin for this, and an anonymous object
(`rule:types/anonymous-object`) is the lightweight way to write the carrier.

A written capture list exists to make the free variables visible where the function is written.
Capturing what the body reads makes the same set visible from the body itself, and cannot drift from
it. Dropping by-reference capture also removes a known trap: anonymous functions made inside a loop
that each capture the loop variable by reference, and all see its last value.

- **Memory:** evaluating an anonymous function allocates one object with one field per captured
  variable. One that captures nothing holds no fields.

**Declined:**
- **`use ($y)` kept as documentation.** Capture is already implicit, so the list would be unchecked
  decoration that drifts from the body.
- **`use (&$y)` kept and the by-value form dropped.** By value is the common, safe case; by reference
  is the trap above.
- **A builtin boxed cell such as `Core\Ref<T>`.** No real code has shown the object carrier above is
  a tax. If it does, the answer is a narrow named type, never by-reference capture again.
