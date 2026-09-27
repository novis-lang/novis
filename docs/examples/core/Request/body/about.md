Returns the whole body of the request as one string: every byte the client sent after the headers.

Use it when you need the body exactly as it arrived. For example, you can check a signature, or
store a document without changing it. The result is empty when the request has no body. You can
call `body` as many times as you like, and `post` and `json` still work after it. After
`bodyStream` or `files`, `body` throws a `LogicError`, because those two do not keep the body. A
command-line program answers no request, so `body` throws a `LogicError` there too.

**Good to know:** the result is tainted, which means it came from outside your program. You cannot
use it as the pattern of a regular expression or as other text that Novis parses. You can still
print it, hash it, compare it and store it.

This replaces PHP's `file_get_contents('php://input')`.
