`json()` reads the body of the answer to a request that `Core\Test::request()` sent, as one JSON
document, and returns the decoded value. A JSON object becomes an array with string keys. It
works like `Core\Request::json`, which reads the body of a request.

If the body is empty or is not valid JSON, `json()` throws a `ParseError`. The `Content-Type`
header is not checked. The `maxDepth` option sets how deep the document may nest. The default is
512, and the value must be from 1 to 1024. Any other value throws a `LogicError`.

You can call `json()` as often as you like. Each call decodes the body again.

**The examples below** show how to check values in a JSON answer, what happens with a body that is
not JSON, and how `maxDepth` limits the nesting.
