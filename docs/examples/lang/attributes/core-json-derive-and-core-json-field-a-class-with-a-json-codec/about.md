`#[Core\Json\Derive]` on a class gives it a JSON encoder and a JSON decoder. `Core\Json::encode` then
accepts instances of that class, and `Core\Json::decodeAs` builds one from JSON text. A class without
the attribute is not allowed at either call.

The fields of the document are the class's declared properties, in the order they are written. Each
field is also a constructor parameter of the same name and type, because a decode calls your
constructor. `#[Core\Json\Field]` changes one property. `name:` writes it under a different key, and
`skip: true` leaves it out of the document.

A decode checks the whole document before it builds anything. If a field is missing, has the wrong
type, or is `null` where that is not allowed, the decode throws a `ParseError`. The error lists every
bad field at once.

The examples show a class encoded as JSON, a renamed key and a skipped property, and a request body
read into an object.
