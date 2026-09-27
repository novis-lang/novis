Returns one parameter from the query string. The query string is the part of the address after
the `?`, such as `q=shoes&page=2`. You give the name of the parameter. The result is its value, or
`null` when the address has no parameter of that name.

The name must match exactly, and upper and lower case are different. A `+` in the value is
returned as a space, and a percent escape such as `%21` is decoded.

A name with square brackets gives an array. For `filter[color]=red`, you read the name `filter`,
and the result is an array with the key `color`.

The result has the type `mixed`, so you convert it with `as`, for example `as int`. A command-line
program answers no request, so `query` throws a `LogicError` there. It also throws one when an
escape in the query string is not valid UTF-8 text.

This replaces PHP's `$_GET`.

**The examples below** show a search page, a parameter that is missing or is not a number, and a
product list filtered with bracket names.
