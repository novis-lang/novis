`fn` is the only way to write an anonymous function, in two body shapes:

```php
fn($x) => $x + 1                            // expression body, implicit return
fn($x) => { $y = $x + 1; return $y * 2; }   // block body, explicit `return` required
fn(int $x): int => $x + 1                   // typed either way
```

`function (...) {...}` and `function (...) use (...) {...}` do not parse; the diagnostic names `fn`.
A block-bodied `fn` still declares its return type (`E0450`) — inferring one would be whole-body
return-type inference, which this language does not do. `static fn` is diagnosed as a `function`
anonymous function.

This removes a second spelling rather than adding a capability: a method reference and the
expression-bodied literal already produce exactly the value a block-bodied anonymous function does
(`rule:types/callable-values`). Capture is never written (`rule:types/implicit-capture`), and an
anonymous function that needs to call itself carries a self-name instead
(`rule:types/anonymous-function-self-name`).
