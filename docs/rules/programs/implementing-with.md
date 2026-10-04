```php
function Core\Program::implementingWith<I, T>(string $member = ""): array<{instance: I, attribute: ?T}>;
```

`rule:programs/implementing`'s enumeration joined with `rule:attributes/structural-retrieval`'s retrieval
in one expansion: one row per class the enumeration would instantiate, in its order, each carrying the
instance and what `Core\Attributes::get<T>` answers for that class's own `$member` — the class itself for
an empty name, its method for a method name, its property or constructor parameter otherwise, so a
promoted parameter is read once. `attribute` is the one attached payload object satisfying `T`, or `null`.

`I` is an interface or a class, selected exactly as `implementing<I>` selects it.

It exists because the two cannot be composed by a program. A retrieval names its target where it is
written, and inside a loop over the enumeration the variable is typed as `I`, so no listed class is
written anywhere; a framework could enumerate its classes or read their attributes, and not both. The
compiler holds both lists at the same moment, and the join is one retrieval fold per class — nothing
about what a retrieval target may be changes, and no reflection table reaches the compiled unit.

The retrieval's refusals are this member's, each naming the class it was made on: a `$member` some
implementor does not declare is `E0798`, two matching payload objects on one class are `E0728`, and a `T` that
is not a shape is `E0729`. A `$member` that is not a string literal names no roster, and every row's
`attribute` is `null`, as the retrieval's computed member folds. The enumeration's own two refusals
hold unchanged.
