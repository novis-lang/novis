Returns the name of the form field that an uploaded file came from. This is the `name` attribute
of the `<input type="file">` in the form, such as `avatar` or `photos[]`. A field that accepts more
than one file gives the same name to each of its files.

The result is a `tainted string`, which means it came from outside your program. The client can send
any field name, including one that your form does not have. Compare the name with the names you
expect, and do not use it as a path or as part of a query.

The examples show the field of each file, how to count the files of one field, and how to handle each
field in its own way.
