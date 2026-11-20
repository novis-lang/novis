A declaration is written `attribute Name on <places> { fields }`. The places are `class`, `method`,
`property` and `parameter`, and the list needs at least one of them. Any other word in the list is
`E1004`. A field is written `name: T`, or `name?: T` when a use may leave it out.

The name after `#[` is checked first. A name that nothing declares is `E0303`. A class, an interface,
an enum or a `type` alias is `E1005`, because only an attribute declaration may follow `#[`. An
attribute with no name, such as `#[{ inject: "repo" }]`, is `E0140`.

Then the place is checked. An attribute written where its `on` list does not allow it is `E0874`, and
the message names the places it allows.

An attribute declared inside a class or an interface is used only on the classes that extend or
implement it, and on their members. A use anywhere else is `E1006`. That includes the class or
interface that declares it.

An attribute may be written once on each declaration. Write `repeatable` before `attribute` to allow
it more than once. A second use without it is `E0763`. Novis keeps every copy of a repeatable
attribute, in the order you wrote them.

Your values are then checked against the fields. A missing field or a value of the wrong type is
`E0401`, and the help line names the missing field. Fields the declaration does not have are allowed.
`#[Name]` with no brackets gives no fields, so it is `E0401` when the declaration has a required field.

Each value needs its field name. `#[Tag(5)]` is a syntax error, and the same field written twice is
`E0494`.

A value may be a number (also a negative one), a string, `true`, `false` or `null`. It may also be an
anonymous object, an array, a class constant, an enum case or `Foo::class`. A variable, a call, `new`,
arithmetic such as `1 + 2` and a string with `{$name}` in it are each `E0725`. A class constant that is
`secret` is `E0727`.

An array is checked like any other value. `[1, 2]` fits a field typed `array<int>`, and `['a']` does
not.

In front of a statement or a local variable, an attribute is `E0101`.

Several attributes can share one bracket, with commas between them: `#[A(n: 1), B(s: "x")]`. This is
the same as writing each one in its own bracket.
