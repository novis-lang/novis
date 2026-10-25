Returns an attribute of the method of the route that matched. You write the shape of the attribute
between `<` and `>`. For a method with `#[{ seconds: 60 }]`, `$route->attribute<{ seconds: int }>()`
returns `{ seconds: 60 }`. If the method has no attribute with every field of the shape, the result is
`null`.

An attribute may have more fields than the shape. So one method can have a cache setting and a rate
limit, and you read each one with its own shape. A gate uses this to read a setting for one route.

The type argument must be a shape. If two attributes of one route method match the shape, the
program does not compile. The compiler checks every route method, because any route can match.

Each call builds one object for each route method that has an attribute of the shape. A program with
more such routes does more work per call.

The examples show a setting for each route, two settings on one method, and a gate that reads a
cache setting.
