Reads the body of the request as one JSON document and returns the decoded value. A JSON object
becomes an array with string keys, and a JSON list becomes a list. The result has the type `mixed`,
so you convert each part with `as` to the type you need. Every string in it is tainted, which means
it came from outside your program.

The body must be one complete JSON document. If it is not valid JSON, or if it is empty, `json`
throws a `ParseError`. A web program usually answers that with the status `400`. The `Content-Type`
header of the request is not checked.

`maxDepth` limits how deeply the document may nest. The default is 512. Every call returns the same
value, so you can call `json` more than once.

This replaces `json_decode(file_get_contents('php://input'), true)` in PHP.
