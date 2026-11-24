The HTTP methods a route can be declared with, and a request can arrive with. There are exactly
eight of them.

`Get`, `Head`, `Options` and `Trace` only read something. `Post`, `Put`, `Patch` and `Delete` may
change it, and the cross-site request check covers those four. You use a case when you declare a
route. The router also returns cases when you check which methods a path has routes for.

The list of cases is fixed, so a method is a value the compiler knows. You do not write it as text.
A misspelled method does not compile, so it cannot become a route that nobody can reach.

**Good to know:** there is no `CONNECT` case. A `CONNECT` request tells a proxy to open a tunnel,
and a route in your program never handles that.
