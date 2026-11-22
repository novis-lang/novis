Reads a JSON object and returns a new instance of one of your classes. You write the class between
`<` and `>`: `Core\Json::decodeAs<User>($text)`. The class needs the `#[Core\Json\Derive]`
attribute, and each public property reads the key with the same name. Write `array<User>` to read a
JSON array that has one object for each element. A class inside a shape or an array needs the
attribute too, however deep it is. The compiler checks this.

The result already has the type of your class, so you do not convert anything with `as`.

If the text is not valid JSON, or a field is missing or has the wrong type, `decodeAs` throws a
`ParseError`. Its `issues` list has one entry for each wrong field. The constructor runs only when
all fields are correct.

A property with a default value is optional. If its key is missing, the property gets the default. For
example, `?string $city = null` makes `city` optional, and a missing key gives `null`.

**The examples below** read one object into a class and show the error for two wrong fields. The
last one reads a list of orders from a shop.
