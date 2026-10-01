`jsonAs<T>()` reads the body of the answer to a request that `Core\Test::request()` sent, as one
JSON object, and returns a new instance of the class `T`. The class needs the
`#[Core\Json\Derive]` attribute. Write `array<T>` to read a JSON array with one object for each
element. It works like `Core\Request::jsonAs`, which reads the body of a request.

If a field is missing or has the wrong type, `jsonAs` throws a `ParseError`. The error has one
entry in `issues` for each wrong field. An empty body throws a `ParseError` too. The `maxDepth`
option sets how deep the document may nest. The default is 512.

The body is text that your own program wrote. So a `string` field of `T` does not need to be
`tainted`. Each call creates new objects.

**The examples below** show how to read an answer into a class, how to read a list, and what
happens when a field has the wrong type.
