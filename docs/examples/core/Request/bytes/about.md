Returns the whole body of the request as bytes: every byte the client sent after the headers,
unchanged.

Use it when the body is a file or other data that is not text, for example an uploaded image or
PDF. `body` returns a string, and a string must be valid UTF-8 text. When the body is not valid
text, `body` throws an error and `bytes` still returns it. The result is empty when the request has
no body. You can call `bytes`, `body`, `post` and `json` in any order, as many times as you like.
After `bodyStream` or `files`, `bytes` throws a `LogicError`, because those two do not keep the
body. A command-line program answers no request, so `bytes` throws a `LogicError` there too.

**Good to know:** the result is tainted, which means it came from outside your program. You can
still measure it, hash it, compare it and store it.
