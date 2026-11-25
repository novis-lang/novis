`Core\Json::decodeAs<User>($text)` reads a JSON object and returns a new instance of your class
`User`. Write `array<User>` to read a JSON array of objects.

The class needs the `#[Core\Json\Derive]` attribute, and each public property reads the key with the
same name. A class inside an object type such as `{user: User}`, or inside an array type, needs the
attribute too. The compiler checks this. A property with a default value is optional: with
`?string $city = null`, a missing `city` key gives `null`.

A property can also be a time value or a `Core\Uuid`, written as a JSON string. A date is its ISO
text, such as `"2026-05-14"`. A `DateTime` needs its time zone, as in
`"2026-05-14T08:30:00+02:00[Europe/Vienna]"`.

Text that is not valid JSON, or a missing or wrong field, throws a `ParseError`. Its `issues` list
has one entry for each wrong field, up to 64. The constructor runs only when all fields are correct.

**The examples below** read one object, show the error for two wrong fields, read a list of orders,
and read time values and an id.
