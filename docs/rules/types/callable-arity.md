A closure of arity *n* satisfies `callable(T₁..Tₘ): R` when **`n ≤ m`**, and only the first *n*
parameter types are compared. An arity greater than *m* is refused where it is written.

```php
map(array<T> $a, callable(T, string): U $fn): array<U>

Core\Arr::map($users, fn($u) => $u->name);               // 1 ≤ 2 — $u is User
Core\Arr::map($users, fn($u, $k) => "{$k}: {$u->name}"); // 2 ≤ 2 — $k is string
Core\Arr::map($users, fn($u, $k, $x) => …);              // refused
```

This is not tolerance invented for convenience: the runtime already hands a callee only the arguments
it declares, which is what lets a one-parameter callback satisfy the `Core` convention that every
callback is offered value *and* key. The type system describes that behaviour. An exact-arity rule
would instead grow an unused `$key` parameter across every callback ever written.
