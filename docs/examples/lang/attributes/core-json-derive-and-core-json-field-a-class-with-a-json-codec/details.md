The document has one key for each instance property of the class. A `private` or `protected`
property has a key too. A property promoted in the `constructor` has its key in the place where it
is written. A `static` property has no key. A `readonly` property has a key. A class with no such
property is `E0758`.

A subclass also has a key for each property of its parent classes. The keys of the top parent come
first, then the keys of each class below it, and the subclass's own keys come last. A parent does
not need `#[Core\Json\Derive]`. A `#[Core\Json\Field]` on a parent's property renames or skips that
key in every subclass. The subclass's constructor needs a parameter for each inherited key, as for
its own properties. If one is missing, the error is reported at the subclass's attribute. A
property that a subclass declares again is one key, at the parent's position. A parent from `Core`
gives no keys.

A property may be a number, a `string`, a `bool`, `mixed`, an enum or a set of allowed values. It
may also be an inline shape, an `array<T>` of these, or `?T` of these. Another class with
`#[Core\Json\Derive]` is allowed too. A class without the attribute is `E0756`. An enum case is written in JSON as
its number. A `float` is written with a fraction, so `2` in a `float` property encodes as `2.0`.

A `mixed` property takes any JSON value: an object, an array, a string, a number, a `bool` or `null`.
An encode writes the value back as it is.

A set of allowed values is a type such as `"draft"|"live"` or `1|2|3`. All values are strings, or
all are numbers. It is written in JSON as the plain string or number. A decode checks that the value
is in the set. `"gone"` gives the issue `` `"gone"` is not one of `"draft"`, `"live"` ``. A long
value is cut to 64 characters in the message. `?"pinned"` and `"a"|"b"|null` also accept `null`.

Any other type with `|` is `E0756`. Examples are `string|int`, a mix of string and number values,
and two classes. Use `mixed` for a value that can have more than one type.

A `secret` property is decoded like any other property. Encoding its class does not compile, and is
`E0462`. A `lateinit` property is `E0463`. `#[Core\Json\Field(skip: true)]` leaves either one out. A property with no constructor parameter of the same name is
`E0460`. A parameter of another type is `E0461`. `#[Core\Json\Field]` has the options `name` and
`skip` only, and any other option is `E0464`. `skip: false` is the same as no `skip`.

A constructor parameter that does not match a property in the document is `E0820` at the `Core\Json::decodeAs` call. This is
true even when the parameter has a default value. `Core\Json::decodeAs` on a class without the
attribute does not compile, and is `E0821`. `Core\Json::encode` of such an object throws a
`LogicError` when it runs.

A `?T` property that is `null` is written as `null`. It is never left out of the document. A `?T`
property is always required in a decode too. A missing key is the issue `required field missing`, as
for any other property. Only its value may be `null`.

A key in the document that the class does not have is ignored.

`#[Core\Json\KeepKeyOrder(reason: '...')]` on the class says that somebody reads the keys in their
order. An example is a client that reads the `type` key first. The encoder does not change: it
always writes the keys in the order the properties are written. The attribute changes what
`nvs code order` and the editor's *Order class members* action do. They keep the instance properties
of this class in their written order. `reason` is required, and says who needs the order. The
attribute on a class without `#[Core\Json\Derive]` is `E0892`. Without `reason`, it is `E0401`.

The `ParseError` has every problem in its `issues`. Each issue has a `path` and a `message`.

A decode checks each value's type and does not convert it. `1.5` and `2.0` in an `int` property are
issues, and so is `-1` in a `uint` property. A number in a `string` or `bool` property is an issue,
and so is a string in a `float` property. An `int` in a `float` property is accepted. An enum
property takes the
number of a case. Any other number, or a string, is the issue `expected a declared enum case`.

The path of an issue joins the keys with `.`. A field of a nested object is `address.city`, and an
array element ends with its index, as `tags.1`. `Core\Json::decodeAs<array<User>>` decodes a list,
and its paths start with the index, as `1.address.city`.

Text that is not valid JSON gives one issue with an empty path. Its message names the line and the
column. A document that is a number, a string or `null` also gives one issue with an empty path.
Its message says that the class decodes from a JSON object.

The decode calls your constructor after the checks pass. An error the constructor throws reaches
your code unchanged.
