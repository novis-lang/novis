Returns every value of a query-string parameter that has square brackets. You give the name
without the brackets: for `tag[]=red&tag[]=new`, the name is `tag`, and the result is
`["red", "new"]`. Each value is a `tainted string`, which is text that came from the client.

Empty brackets give a list. Brackets with a key give that key: `filter[color]=red` gives
`["color" => "red"]`. A name without brackets gives a list with its one value. A name that is not
in the address gives an empty array, so you can loop over the result without a check for `null`.

A name with two pairs of brackets, such as `filter[color][dark]=navy`, throws a `ParseError`. Read
those values with `Core\Request::queryAs` and a type that describes them. A command-line program
has no request, so `queryArray` throws a `LogicError` there.
**The examples below** filter a product list, read a list and a missing name, and show the error
for two pairs of brackets.
