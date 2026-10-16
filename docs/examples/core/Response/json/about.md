Converts a value to JSON and sends it as the response. It also adds the header `Content-Type:
application/json`, so the client knows that the body is JSON. Use it for an API that a program or
a script in the browser reads.

It uses the same encoder as `Core\Json::encode`. The JSON is on one line, with no spaces. The value
can be a shape, an array, a number, a string, `null`, or an object of a class with
`#[Core\Json\Derive]`. Text from a visitor is allowed anywhere inside the value. Each string is
written as a JSON string, so its quotes are escaped and it cannot change the JSON around it.

If the value contains something JSON cannot write, such as a `float` that is NaN, `json` throws a
`LogicError` and sends nothing.

The examples show a small object, a value that JSON cannot write, and a list of products for an
API.
