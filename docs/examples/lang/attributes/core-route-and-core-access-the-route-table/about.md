`#[Core\Route]` on a method gives it a path and an HTTP method. `#[Core\Access]` on the same method
says who may use it. A route needs both attributes.

A path has fixed segments and captures. `{id}` matches one segment. `{id?}` matches one segment or
none, and must come last. `{path...}` matches all remaining segments. Each capture gives its value
to the parameter with the same name, converted to the type of that parameter.

Novis collects every route into one table when the program is compiled. `Core\Router::url` builds a
link from the name of a route, and the link is checked at the same time. An unknown route name or a
missing capture is a compile error.

**Good to know:** the server matches each request to a route before your program runs. Your entry
file calls `Core\Router::dispatch`, which calls the method of that route. A gate class of your own
checks each request first and writes the error pages. A program can also read the match with
`Core\Request::route()` and call the method itself.

**The examples below** show one route and its link, the three kinds of capture, and the links for a
product list page. Then they show routes in another file, a program that calls the route method
itself, and a route with no `#[Core\Access]`, which does not compile.
