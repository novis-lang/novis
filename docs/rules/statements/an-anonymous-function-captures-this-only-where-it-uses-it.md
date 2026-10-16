An anonymous function captures `$this` only when its body uses it, decided by the compiler.
`static function () {}` and `static fn() => …` are refused with `E0210`, which says that `static` is
not a modifier an anonymous function takes and that the keyword can simply be dropped.

`static fn` is an **assertion** that an anonymous function does not capture `$this`, and there is no
assertion syntax here — everything else in the language is a declaration the compiler enforces. The
property it asserted is obtained by making it true instead.

The effect is the one the keyword existed to produce: a callable that never mentions `$this` cannot
extend the enclosing object's lifetime. The cost is the corner — a `$this`-free anonymous function
built inside a method and then bound to a *different* object gives a callable that ignores the
binding, because `bindTo()` and `bind()` return an equivalent callable rather than rebinding anything.
