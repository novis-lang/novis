Returns every value of a form field that has square brackets. You give the name without the
brackets: for `item[]=notebook&item[]=pen`, the name is `item`, and the result is
`["notebook", "pen"]`. Each value is a `tainted string`, which is text that came from the client.

Empty brackets give a list. Brackets with a key give that key: `size[shirt]=M` gives
`["shirt" => "M"]`. A name without brackets gives a list with its one value. A field that was not
sent gives an empty array, so you can loop over the result without a check for `null`.

`postArray` reads the same forms as `Core\Request::post`. If you also need the uploaded files, call
`Core\Request::files()` first. A name with two pairs of brackets throws a `ParseError`. Read those
values with `Core\Request::postAs`. A command-line program has no request, so `postArray` throws a
`LogicError` there.
**The examples below** read an order form, read fields with a key in the brackets, and show a
single value and a missing field.
