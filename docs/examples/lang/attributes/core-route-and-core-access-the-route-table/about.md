`#[Core\Route]` on a method gives it a path and an HTTP method, and `#[Core\Access]` on the same
method says who may reach it. A route needs both. Novis collects every route in the program into one
table while it compiles, checks that table, and builds links from it with `Core\Router::url`.

A path is made of literal segments and captures. `{id}` matches one segment, `{id?}` matches one or
none and comes last, and `{path...}` matches everything that is left. Each capture binds to the
parameter of the same name, and that parameter's type is what the segment converts to. A parameter
marked `#[Core\Query]` comes from the query string instead.

A link is checked while your program compiles. The route name is a string written at the call, and an
unknown name, a missing capture or a key that belongs to no parameter are all errors before the
program runs. Each value is percent-encoded into its own segment, so a value cannot add a segment of
its own.

**Good to know:** this build has no server. The table is built, checked and used for links, and
nothing sends a request to a route yet.

The examples show one route and its link, the three kinds of capture, and the links a product list
page needs.
