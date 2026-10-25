The name in front of the brackets is checked first. A name that nothing declares is `E0303`. A class,
or a `type` alias for something that is not a shape, such as `type Id = int;`, is `E0726`.

Your values are then checked against the shape. A missing field or a value of the wrong type is
`E0401`, and the help line names the missing field. Fields the shape does not have are allowed.
`#[Name]` with no brackets has no fields, so it is `E0401` for a shape that has any.

Each value needs its field name. `#[Tag(5)]` is a syntax error, and the same field written twice is
`E0494`.

A value may be a number (also a negative one), a string, `true`, `false` or `null`. It may also be an
anonymous object, an array, a class constant, an enum case or `Foo::class`. A variable, a call, `new`, arithmetic such as
`1 + 2` and a string with `{$name}` in it are each `E0725`. A class constant that is `secret` is
`E0727`.

An array in an attribute has the type `array<mixed>`. A field typed `array<mixed>` takes it. A field
typed `array<int>` or `array<string>` does not, and the attribute is `E0401`.

An attribute may also stand in front of an enum case and a property hook. In front of a statement or
a local variable, it is `E0101`.

You can write an attribute on an enum, an enum case or a class constant, and the program compiles.
`Core\Attributes` cannot read it back, because only a class, an interface, a method, a property and a
parameter have a way to name them.

An attribute may have no name, as in `#[{ inject: "repo" }]`. Nothing checks its fields.

Several attributes can share one bracket, with commas between them: `#[A(n: 1), B(s: "x")]`. This is
the same as writing each one in its own bracket.

You can write the same attribute any number of times on one declaration. Novis keeps every copy, in
the order you wrote them.
