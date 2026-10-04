A callable of arity *n* satisfies `callable(T₁..Tₘ): R` when **`n ≤ m`**, and only the first *n*
parameter types are compared. An arity greater than *m* is refused where it is written.

```php
map(array<T> $a, callable(T, string): U $fn): array<U>

Core\Arr::map($users, fn($u) => $u->name);               // 1 ≤ 2 — $u is User
Core\Arr::map($users, fn($u, $k) => "{$k}: {$u->name}"); // 2 ≤ 2 — $k is string
Core\Arr::map($users, fn($u, $k, $x) => …);              // refused
```

A **call** through such a type passes *m* arguments, not *n*: what the value holds is the type's
business and not the call site's, and the runtime hands that callable only the leading arguments it
declares. A site passing fewer would leave one of the callable's own parameters unfilled, which is a
fault below the language rather than a throw, so it is refused where it is written (`E0809`).

This is not tolerance invented for convenience: the runtime already hands a callee only the arguments
it declares, which is what lets a one-parameter callback satisfy the `Core` convention that every
callback is offered value *and* key. The type system describes that behaviour. An exact-arity rule
would instead grow an unused `$key` parameter across every callback ever written.
