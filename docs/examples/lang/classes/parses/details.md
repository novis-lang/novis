The two methods are `public static function parse(tainted string $s): static` and
`public function format(): tainted string`. `tainted` text comes from outside your program, such as
a web request or a command line.

Novis needs this interface everywhere text from outside becomes a value: a piece of a route path, a
`#[Core\Query]` parameter, a command argument and a command option. A parameter typed at a class that
does not implement `Parses` does not compile, and the error says so.

In a route, `parse` runs before `Core\Router\Gate::allows`. So `allows` can check the object with
`$route->parameter('name')`, and this works the same for a path capture and a query value. If
`parse` throws, the gate's `refused` method gets `Core\Router\Refusal::BadCapture`, and the default
status is `400`.

The return type `static` means the class the call named, so a subclass that inherits `parse` gets an
object of the subclass back. In a `final` class, `static` is that class. There, `parse` may return an
object that already exists, such as a record it loads by its id.

`parse` runs for every request that matches the route, before `allows`. It runs whether the visitor
is signed in or not. Sometimes even the time of a lookup must not show whether a record exists. Then
declare the parameter as `string` and load the record after `allows`.

`Core\Router::url` puts the text that `format` returns into its own path segment and percent-encodes
it. A `/`, `?` or `#` in that text cannot reach another route or start the query string.

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
