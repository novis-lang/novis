A path must start with `/`, or it is `E0750`. A `{name?}` or `{name...}` capture that is not the last
segment is `E0750` too. A route with no `method:` is `E0747`, and a `method:` that is not a case of
`Core\Http\Method` is `E0401`.

Two routes with the same path and the same HTTP method are `E0748`. This is true when they are in two
classes, and when their captures only have different names, such as `/users/{id}` and
`/users/{key}`. One method may have several `#[Core\Route]` lines, one for each HTTP method. They
must all have the same path. Two different paths on one method are `E0891`.

A capture parameter may be a `string`, `int`, `uint`, `decimal` or an enum. It may also be a set of
allowed values such as `"en"|"de"`, or a class that implements `Parses`.

A capture with no parameter of the same name is `E0751`. A `float` parameter is `E0752`, and so is a
`{name...}` parameter that is not a `string`. A `{name?}` parameter with no default value is `E0753`.
A `#[Core\Query]` parameter takes the same types as a capture. It may also be `?T $x = null` for
any of those types `T`. A `?T` with no default, or with a default that is not `null`, is `E0752`.
A `bool` path capture does not compile, but a `bool` query value does. `#[Core\Query]` on a method
with no route is `E0765`.

`Core\Router::dispatch` reads the query string key with the same name as the parameter. A parameter
with a default value makes its key optional, and a missing key gives the default. These requests
are a `400`, and the route method does not run:

- a value that does not convert to the parameter's type, also for a `?T $x = null` parameter,
- a `bool` value that is not `1` or `0`, such as `true`, `on`, an empty value or a key with no `=`,
- a missing key for a parameter with no default value,
- a key that is sent twice,
- a key with brackets, such as `page[]=1`.

For each `400`, `refused` gets `Core\Router\Refusal::BadCapture` and the match. A path capture that
does not convert is a `404`, because then no route matches the request.

A program can also give a default for a bad value, and then the request is not a `400`. Declare the
parameter as a `string` with a default, such as `string $page = ''`. Any valid UTF-8 text converts to
a `string`. Then convert it in the route method, such as `($page as ?uint) ?? 1`. Use this only when a wrong link should still show a
page. A typed parameter is the better choice in most cases, because it checks the value for you.

A route with no `#[Core\Access]` is `E0762`. `#[Core\Access]` on a method with no route is `E0788`.
Two `#[Core\Access]` lines on one method are `E0763`, and one with no `allow:` is `E0760`. The
`allow:` value must be an enum case or a class constant. A number is `E0761`. `csrf: false` on a
method whose routes are all `Head` or `Options` is `E0764`. It is allowed on `Get`, because a
WebSocket upgrade arrives as a `Get`.

The server checks the `Origin` header before it looks for a route. A `POST`, `PUT`, `PATCH` or
`DELETE` from another origin gets `Core\Router\Refusal::Csrf` and the status `403`. This is also true
when no route has its path. So a request gets `NotFound` only when it comes from the same origin,
has no `Origin`, or uses `Get`, `Head` or `Options`.

A fixed segment is tried before a capture, so `/users/new` matches the route `/users/new` and not
`/users/{id}`. The order of the two routes in the code does not matter.

A fixed segment is compared exactly, so `/U` does not match `/u`. A path with an extra `/` at the
end, such as `/u/`, does not match the route `/u`. An `int` capture matches `-4`, `+4` and `04`. It
does not match a number too large for an `int`. A `uint` capture does not match a negative number.
A `decimal` capture matches `1.50` and not `x`. A `{name...}` capture needs at least one segment, so
`/files` does not match `/files/{path...}`.

An enum capture matches the name of a case exactly, such as `De`, and not `de` or `1`. When every case
of the enum has a written value, the capture matches that value instead. For `enum Lang { En = 3,
De = 7 }`, `/7` matches and `/De` does not.

A capture of a class that implements `Parses` matches any segment. The class's `parse` method runs
when `Core\Router::match` returns the result. If `parse` throws an error, that error comes out of
`Core\Router::match`.

`$match->parameter("id")` returns `uint|int|null|decimal|tainted string|Core\Uuid|Parses`, so you convert
it with `as`. A `string` capture gives a `tainted string`. A name that is not a capture gives `null`.
Inside `Core\Router::dispatch`, the match also has each `#[Core\Query]` value under its parameter's
name, so `allows` can read it. The match from `Core\Request::route()` has no query values.
`access()` returns the full name of the `allow:` value, such as `Role::Staff` or `Users::ADMIN`.
`accessAs<Core\Audience>()` on a route with `allow: Role::Staff` returns `null`.

`Core\Router::url` with a method that has no `#[Core\Route]` is `E0754`. A method the class does not
have is `E0309`. A params array that leaves out a capture is `E0755`. A key that is neither a capture
nor a `#[Core\Query]` parameter is `E0759`. A value outside a closed set, such as `"fr"` for
`"en"|"de"`, is `E0772`. These checks need the method reference and the array written directly in
the code.

A method reference written in the call works for a static method and an instance method. The link
does not call the method. You can also pass a method reference in a variable, such as an
`array<callable>` for a menu. A static method goes in as `Shop::products`. An instance method goes
in as `$shop->order(...)`. A `callable` with no route throws a `RuntimeError` when the program runs.
A params array in a variable that leaves out a capture throws a `RuntimeError` too.

The compiler checks the type of each value written in the params array. A value whose type the
parameter cannot take is `E1010`, such as a `Date` for a `uint` capture. A `string` value always
compiles. `"x"` for a `uint` capture gives the link `/x`, and that link does not match the route.
A value of a class that implements `Parses` is written as the text its `format` method returns. Any
other object throws a `RuntimeError`.

A capture value is encoded for one segment, so `x/y?z#` becomes `x%2Fy%3Fz%23`. An empty string gives
an empty segment. A space is `%20` in the path and `+` in the query string, and `&` in a query value is
`%26`. A `{name?}` key that you leave out drops its segment, so the link is `/u`. A `null` value for it
gives `/u/`, and the route matches that link too. A `null` value for a `#[Core\Query]` key leaves
that key out of the query string.

`Core\Router::urlAbsolute` with no `origin` setting throws a `RuntimeError`. An `origin` that ends in
`/` does not give two `/` characters in the link.
