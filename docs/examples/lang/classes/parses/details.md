A class that implements `Parses` must have two methods: `parse` and `format`. A class without one of
them is the error `E0449`. In a class that is not `final`, a `parse` method with a return type other
than `static` is the error `E0404`. A `final` class may write its own class name there.

`parse` is an ordinary static method, so your code can call it too: `Slug::parse("hello")`. It may
throw any error. `ParseError` is the usual one.

`format` returns the text that `parse` reads. `Core\Router::url` and `Core\Uri::buildQuery` call it
to write the value into a link. Its return type is `tainted string`. A class whose text is not
tainted may write `string`.

The text keeps its `tainted` qualifier. A property that stores it must have the type
`tainted string`. A `string` property is the error `E0401`. The object that `parse` returns is not
tainted.

A route capture or a `#[Core\Query]` parameter accepts these types: `string`, `int`, `uint`,
`decimal`, `Core\Uuid`, `Date`, `TimeOfDay`, `Instant`, an enum, a set of allowed values such as
`'a'|'b'`, and a class that implements `Parses`. A `#[Core\Query]` parameter also accepts `bool` and
`DateTime`. Any other class is the error `E0752`.

A path capture fails in two ways. The router converts some types itself while it matches the path:
`int`, `uint`, `decimal`, `Core\Uuid` and a set of allowed values. A segment that does not convert is
no match. If no other route matches, the response is `404`. For any other `Parses` class, the router matches the path first.
Then it calls `parse`. If `parse` throws, the response is `400`, and the method does not run.
