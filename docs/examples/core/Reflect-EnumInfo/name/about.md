Returns the full name of the enum that a `Core\Reflect\EnumInfo` describes.

You get a description with `Core\Reflect\EnumInfo::of`, and you pass it the name of an enum.
`name()` returns that same name as a string, with its namespace. This is useful in a function that
gets only the description, for example to print a heading or a log line. You can also store the
name and pass it to `of` again later. The result is a description of the same enum.

**The examples below** print the name, use it to describe the enum again, and write a heading for
each enum in the documentation of an API.
