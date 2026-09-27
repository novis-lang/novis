Returns one field from a form that a browser sent with `POST`. You give the name of the field. The
result is its value, or `null` when the form has no field of that name. A checkbox that is not
ticked is not sent, so its result is `null`. A field that was sent with no text gives an empty
string.

The name must match exactly, and upper and lower case are different. A `+` in the value is
returned as a space, and a percent escape such as `%40` is decoded. A name with square brackets
gives an array: for `item[]=pen`, you read the name `item`.

`post` reads both kinds of form: `application/x-www-form-urlencoded` and `multipart/form-data`. If
you also need the uploaded files, call `Core\Request::files()` first.

The result has the type `mixed`, so you convert it with `as`. A command-line program answers no
request, so `post` throws a `LogicError` there. When an escape in the form is not valid UTF-8
text, `post` throws a `RuntimeError`.

This replaces PHP's `$_POST`.

**The examples below** read a contact form, show a field that is missing or empty, and read an
order form with several products.
