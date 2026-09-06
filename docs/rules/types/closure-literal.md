`fn` is the only closure literal, in two body shapes:

```php
fn($x) => $x + 1                            // expression body, implicit return
fn($x) => { $y = $x + 1; return $y * 2; }   // block body, explicit `return` required
fn(int $x): int => $x + 1                   // typed either way
```

`function (...) {...}` and `function (...) use (...) {...}` do not parse; the diagnostic names `fn`.
A block-bodied `fn` still declares its return type (`E0450`) — inferring one would be whole-body
return-type inference, which this language does not do. `static fn` is diagnosed as a `function`
closure.

This removes a second spelling rather than adding a capability: first-class callable syntax and the
expression-bodied literal already produce exactly the value a block-bodied closure does
(`rule:types/callable-is-a-closure`). Capture is never written (`rule:types/implicit-capture`), and a
closure that needs to call itself carries a self-name instead (`rule:types/closure-self-name`).
