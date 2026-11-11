`#[Core\Json\Derive]` on a class gives it a JSON encoder and a JSON decoder. `Core\Json::encode` then
accepts instances of that class, and `Core\Json::decodeAs` builds one from JSON text.

The fields of the document are the class's properties, in the order they are written. A subclass
also has the properties of its parent classes, and they come first. Each property is also a
constructor parameter of the same name and type, because a decode calls your constructor.
`#[Core\Json\Field]` renames one property's key or leaves it out.

A property can have a set of allowed values, such as `"draft"|"live"`. A `mixed` property takes any
JSON value.

A decode checks the whole document before it builds anything. If a field is missing or has the
wrong type, the decode throws a `ParseError`. The error lists every bad field at once.

The examples show an encode, a renamed key, a request body and each check of a decode. They also
show an enum, a fixed key order, a subclass, a set of allowed values and a `mixed` property.
