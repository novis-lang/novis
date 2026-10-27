A path must start with `/`, or it is `E0750`. A `{name?}` or `{name...}` capture that is not the last
segment is `E0750` too. A route with no `method:` is `E0747`, and a `method:` that is not a case of
`Core\Http\Method` is `E0401`. A route with no `name:` still matches a request. Its `name()` is
`null`, and `Core\Router::url` cannot build a link to it.

Two routes with the same path and the same HTTP method are `E0748`. This is true when they are in two
classes, and when their captures only have different names, such as `/users/{id}` and
`/users/{key}`. One method may have several `#[Core\Route]` lines, one for each HTTP method. They
must all have the same path. Two different paths on one method are `E0891`.

A capture parameter may be a `string`, `int`, `uint`, `decimal`, `bool` or an enum. It may also be a
set of allowed values such as `"en"|"de"`, or a class that implements `Parses`.

A capture with no parameter of the same name is `E0751`. A `float` parameter is `E0752`, and so is a
`{name...}` parameter that is not a `string`. A `{name?}` parameter with no default value is `E0753`.
A `#[Core\Query]` parameter takes the same types as a capture, so a `?string` one is `E0752`.
`#[Core\Query]` on a method with no route is `E0765`.

`Core\Router::dispatch` reads the query string key with the same name as the parameter. A parameter
with a default value makes its key optional, and a missing key gives the default. These requests
are a `400`, and the route method does not run:

- a value that does not convert to the parameter's type,
- a missing key for a parameter with no default value,
- a key that is sent twice,
- a key with brackets, such as `page[]=1`.

For each `400`, `refused` gets `Core\Router\Refusal::BadCapture` and the match. A path capture that
does not convert is a `404`, because then no route matches the request.

A route with no `#[Core\Access]` is `E0762`. `#[Core\Access]` on a method with no route is `E0788`.
Two `#[Core\Access]` lines on one method are `E0763`, and one with no `allow:` is `E0760`. The
`allow:` value must be an enum case or a class constant. A number is `E0761`. `csrf: false` on a
method whose routes are all `Head` or `Options` is `E0764`. It is allowed on `Get`, because a
WebSocket upgrade arrives as a `Get`.

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

`$match->param("id")` returns `uint|int|null|decimal|tainted string|Core\Uuid|Parses`, so you convert
it with `as`. A `string` capture gives a `tainted string`. A name that is not a capture gives `null`.
`access()` returns the full name of the `allow:` value, such as `Role::Staff` or `Users::ADMIN`.
`accessAs<Core\Audience>()` on a route with `allow: Role::Staff` returns `null`.

`Core\Router::url` with an unknown name is `E0754`. A params array that leaves out a capture is
`E0755`. A key that is neither a capture nor a `#[Core\Query]` parameter is `E0759`. A value outside a
closed set, such as `"fr"` for `"en"|"de"`, is `E0772`. These checks need the name and the array
written directly in the code. A name in a variable throws a `RuntimeError`. A params array in a
variable that leaves out a capture throws a `RuntimeError` too.

The compiler does not check the type of a value. `"x"` for a `uint` capture gives the link `/x`, and
that link does not match the route. A value of a class that implements `Parses` is given as text,
such as `"cup-01"`. An object throws a `RuntimeError`.

A capture value is encoded for one segment, so `x/y?z#` becomes `x%2Fy%3Fz%23`. An empty string gives
an empty segment. A space is `%20` in the path and `+` in the query string, and `&` in a query value is
`%26`. A `{name?}` key that you leave out drops its segment, so the link is `/u`. A `null` value for it
gives `/u/`, and the route matches that link too. A `null` value for a `#[Core\Query]` key leaves
that key out of the query string.

`Core\Router::urlAbsolute` with no `origin` setting throws a `RuntimeError`. An `origin` that ends in
`/` does not give two `/` characters in the link.
