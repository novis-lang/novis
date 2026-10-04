```php
function Core\Attributes::get<T>(callable $target, string $member = ""): ?T;
function Core\Attributes::all<T>(callable $target, string $member = ""): array<T>;
```

`T` is a shape type — an inline `{...}`, or a `type` alias naming one. Retrieval is **structural**:
every attached payload object satisfying `T` under width subtyping is an answer, whether it was written
bare or under a name, and whatever that name was. An attach-time name checks the payload object where it is
written and is never part of how a caller asks for it, so there is no namespace of attribute-kind names
for unrelated libraries to collide in.

`$target` names a declaration through a first-class-callable reference, read syntactically and never
evaluated. A method is its own reference, `Foo::bar(...)`. A class or interface has none of its own and
is named by its `constructor`'s, `Foo::constructor(...)`, which every class definitely has. A parameter
is the owning method's reference plus its name as `$member`; a property is the owning class's
`constructor` reference plus its name. Those two overlap deliberately: for a promoted constructor
parameter they are the same declaration, so both rosters are consulted and their attributes joined.

A written `$member` is checked against the target's real declarations and one naming neither roster is
`E0798` — the answer a misspelling would otherwise fold to is the answer a correct retrieval of an
absent attribute gives, and nothing downstream could tell them apart. A `$member` that is not a string
literal has no name to check and folds to an empty result.
