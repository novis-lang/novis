Returns one field from a form that a browser sent with `POST`. You give the name of the field. The
result is its value as a `tainted string`, which is text that came from the client. The result is
`null` when the form has no field of that name. A checkbox that is not ticked is not sent, so its
result is `null`. A field that was sent with no text gives an empty string.

The name must match exactly, and upper and lower case are different. A `+` in the value is
returned as a space, and a percent escape such as `%40` is decoded. A name with square brackets,
such as `item[]=pen`, gives `null`. Read those values with `Core\Request::postArray`.

`post` reads both kinds of form: `application/x-www-form-urlencoded` and `multipart/form-data`. If
you also need the uploaded files, call `Core\Request::files()` first.

A command-line program has no request, so `post` throws a `LogicError` there. When an escape in the
form is not valid UTF-8 text, `post` throws a `RuntimeError`.
**The examples below** read a contact form, show a field that is missing or empty, and show a field
with brackets.
