A closure binds `$this` only when its body uses it, decided by the compiler. `static function () {}` and
`static fn() => …` are refused: *`static` is not a closure modifier; a closure captures `$this` only if it
uses it, so drop the keyword.*

`static fn` is an **assertion** that a closure does not capture `$this`, and there is no assertion syntax
here — everything else in the language is a declaration the compiler enforces. The property it asserted is
obtained by making it true instead.

The effect is the one the keyword existed to produce: a closure that never mentions `$this` cannot extend
the enclosing object's lifetime. The cost is the corner — a `$this`-free closure built inside a method and
then bound to a *different* object gets a closure that ignores the binding, because `bindTo()` and `bind()`
return an equivalent closure rather than rebinding anything.
