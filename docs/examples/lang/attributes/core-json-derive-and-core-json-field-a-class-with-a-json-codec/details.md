The fields are the instance properties that the class itself declares. A `private` or `protected`
property is a field too. A property promoted in the `constructor` is a field in the place where it is
written. A `static` property is not a field, and neither is a property the class inherits. A
`readonly` property is a field. A class with no fields at all is `E0758`.

A field may be a number, a `string`, a `bool`, an enum, an inline shape, an `array<T>` of these, or
`?T` of these. It may also be another class with `#[Core\Json\Derive]`. A class without the attribute
is `E0756`. An enum case is written in JSON as its number. A `float` is written with a fraction, so
`2` in a `float` field encodes as `2.0`.

A `secret` property cannot be a field, and is `E0462`. A `lateinit` property is `E0463`. Both work
with `#[Core\Json\Field(skip: true)]`. A field with no constructor parameter of the same name is
`E0460`. A parameter of another type is `E0461`. `#[Core\Json\Field]` has the options `name` and
`skip` only, and any other option is `E0464`. `skip: false` is the same as no `skip`.

A constructor parameter that is not a field is `E0820` at the `Core\Json::decodeAs` call. This is
true even when the parameter has a default value. `Core\Json::decodeAs` on a class without the
attribute does not compile, and is `E0821`. `Core\Json::encode` of such an object throws a
`LogicError` when it runs.

A `?T` field is always required in the document. A missing key is the issue `required field
missing`, as for any other field. Only its value may be `null`.

A decode checks each value's type and does not convert it. `1.5` and `2.0` in an `int` field are
issues, and so is `-1` in a `uint` field. A number in a `string` or `bool` field is an issue, and
so is a string in a `float` field. An `int` in a `float` field is accepted. An enum field takes the
number of a case. Any other number, or a string, is the issue `expected a declared enum case`.

The path of an issue joins the keys with `.`. A field of a nested object is `address.city`, and an
array element ends with its index, as `tags.1`. `Core\Json::decodeAs<array<User>>` decodes a list,
and its paths start with the index, as `1.address.city`.

Text that is not valid JSON gives one issue with an empty path. Its message names the line and the
column. A document that is a number, a string or `null` also gives one issue with an empty path.
Its message says that the class decodes from a JSON object.

The decode calls your constructor after the checks pass. An error the constructor throws reaches
your code unchanged.
