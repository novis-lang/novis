Returns an attribute of the method of the route that matched. You write the name of an `attribute`
declaration between `<` and `>`. For `attribute Cache on method { seconds: int }` and a method with
`#[Cache(seconds: 60)]`, `$route->attribute<Cache>()` returns `{ seconds: 60 }`. If the method has no
`Cache` attribute, the result is `null`.

Each kind of attribute has its own declaration. So one method can have a cache setting and a rate
limit, and you read each one by its name. A gate uses this to read a setting for one route.

The type argument must be the name of an attribute declaration. If one route method has two
attributes of that kind, the program does not compile. The compiler checks every route method,
because any route can match.

Each call builds one object for each route method that has an attribute of that kind. A program with
more such routes does more work per call.

The examples show a setting for each route, two settings on one method, and a gate that reads a
cache setting.
