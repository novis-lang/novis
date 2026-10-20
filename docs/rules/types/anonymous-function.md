`fn` is the only way to write an anonymous function, in two body shapes:

```nvs
fn($x) => $x + 1                            // expression body, implicit return
fn($x) => { $y = $x + 1; return $y * 2; }   // block body, explicit `return` required
fn(int $x): int => $x + 1                   // typed either way
```

`function (...) {...}` and `function (...) use (...) {...}` do not parse; the diagnostic names `fn`.
A block-bodied `fn` still declares its return type (`E0450`) — inferring one would be whole-body
return-type inference, which this language does not do. `static fn` is diagnosed as a `function`
anonymous function.

Each parameter of a `fn` takes exactly one argument. A `...` or a default on one is `E0877` at the
parameter: a call through `callable` passes one argument for each declared parameter and checks only
that there are enough, so a variadic parameter would collect no list and a default would never be read.
A function that takes many values declares an `array<T>` parameter.

This removes a second spelling rather than adding a capability: a method reference and the
expression-bodied anonymous function already produce exactly the value a block-bodied anonymous function does
(`rule:types/callable-values`). Capture is never written (`rule:types/implicit-capture`), and an
anonymous function that needs to call itself carries a self-name instead
(`rule:types/anonymous-function-self-name`). The block body is not a new form: PHP's arrow function
never grew one only because it was added after the `function` literal, and nothing requires `fn` to
keep that restriction.

**Declined:**
- **Keeping both `function (...) {...}` and `fn`.** Two spellings of one value
  (`rule:statements/nothing-gets-a-second-name`).
- **A bare `($x) => ...` without `fn`.** A parenthesised list before `=>` cannot be told apart from a
  parenthesised expression without a cover grammar; `fn` decides it from one token.
- **Filling a missing argument from a parameter's default, or collecting a `...` into a list.** A
  `callable` carries its arity and its parameter tags, never its defaults, so each call through it
  would have to read the declaration at run time. One argument for each parameter keeps the call as it
  is.
- **Blocks that are expressions, returning their last value.** That decides statements against
  expressions for the whole language to save one `return`.
