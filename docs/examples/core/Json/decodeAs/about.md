Reads a JSON object and returns a new instance of one of your classes. You write the class between
`<` and `>`: `Core\Json::decodeAs<User>($text)`. The class needs the `#[Core\Json\Derive]`
attribute, and each public field reads the key with the same name. Write `array<User>` to read a
JSON array that has one object for each element.

The result already has the type of your class, so you do not convert anything with `as`.

If the text is not valid JSON, or a field is missing or has the wrong type, `decodeAs` throws a
`ParseError`. Its `issues` list has one entry for each wrong field, so you can show every problem at
once. The constructor runs only when all fields are correct.

A field with a default value is optional. If its key is missing, the field gets the default. For
example, `?string $city = null` makes `city` optional, and a missing key gives `null`. The type
`?string` also allows the key to be present with the value `null`.

**The examples below** read one object into a class, show the error for a document with two wrong
fields, and read a list of orders from a shop.
