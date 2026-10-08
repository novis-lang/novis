Returns one parameter from the query string as text. The query string is the part of the address
after the `?`, such as `q=shoes&page=2`. You give the name of the parameter. The result is its value
as a `tainted string`, or `null` when the address has no parameter of that name. Tainted text came
from the client, so Novis does not allow it in an SQL query or in HTML until you escape or check it.

The name must match exactly, and upper and lower case are different. A `+` in the value is
returned as a space, and a percent escape such as `%21` is decoded.

A name with square brackets, such as `tag[]=red`, gives `null`. Read those values with
`Core\Request::queryArray`.

To get a number, convert the text with `as`, for example `as ?int`. A command-line program has no
request, so `query` throws a `LogicError` there. When an escape in the query string is not valid
UTF-8 text, `query` throws a `RuntimeError`.
**The examples below** show a search page, a parameter that is missing or is not a number, and a
name with brackets.
