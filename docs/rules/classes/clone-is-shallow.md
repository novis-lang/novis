`clone $x` produces a new instance of `$x`'s class and copies its declared storage one level deep. A
scalar or `array<T>` property is copied by the value
semantics it already has, so the two sides diverge on the first write. An object-typed property — held
directly or reached through a cloned array — keeps pointing at the same instance, host handles
included; `clone` never crosses a heap, so nothing is asked to leave the arena it is in. The copy
answers `is` for every type the original did (`rule:types/type-test`), so it is not a fresh
construction of the declared type.

Storage is written through the privileged path construction already uses, not through ordinary
property assignment. Two things follow: a `readonly` property survives the copy without throwing, and
a declared `PropertyObserver` is told nothing about it
(`rule:classes/property-observer-pipeline` observes assignments, and the copy makes none).

No `__clone` runs, and no class can declare one. A class needing a duplicated nested collection
exposes an explicit method and calls it, rather than overloading what `clone` means.
