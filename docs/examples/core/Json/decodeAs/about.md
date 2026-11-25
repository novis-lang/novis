`Core\Json::decodeAs<User>($text)` reads a JSON object and returns a new instance of your class
`User`. The result already has the type of the class, so you need no `as`.
Write `array<User>` to read a JSON array that has one object for each element.

The class needs the `#[Core\Json\Derive]` attribute. Each public property reads the key with the
same name. A class used inside an object type such as `{user: User}`, or inside an array type, needs
the attribute too. The compiler checks this.

A property with a default value is optional. With `?string $city = null`, a missing `city` key gives
`null`.

A property can also be a `Core\Time\Date`, `TimeOfDay`, `DateTime`, `Instant`, `Duration`, `Zone`
or `Core\Uuid`. Its JSON value is a string. A date or a time is its ISO text, such as
`"2026-05-14"`, and a `DateTime` needs its time zone, as in
`"2026-05-14T08:30:00+02:00[Europe/Vienna]"`. A `Duration` is text such as `"1h30m"`, and a `Zone`
is its name.

If the text is not valid JSON, or a field is missing or has the wrong type, `decodeAs` throws a
`ParseError`. Its `issues` list has one entry for each wrong field, and at most 64 entries. When
more fields are wrong, a last entry says that more follow. The constructor runs only when all fields
are correct.

**The examples below** read one object, show the error for two wrong fields, and read a list of
orders. The last one reads a date, a duration and an id from strings.
