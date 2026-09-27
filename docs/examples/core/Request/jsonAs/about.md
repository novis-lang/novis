Reads the body of the request as one JSON object and returns a new instance of one of your
classes. You write the class between `<` and `>`: `Core\Request::jsonAs<NewUser>()`. The class needs
the `#[Core\Json\Derive]` attribute, and each public field reads the key with the same name. Write
`array<NewUser>` to read a JSON array that has one object for each element. The body comes from the
client, so every string field must be `tainted string`.

If the body is not valid JSON, or a field is missing or has the wrong type, `jsonAs` throws a
`ParseError`. Its `issues` list has one entry for each wrong field. A web program usually answers
that with the status `422`. The `Content-Type` header is not checked.

Every call creates new objects, so two parts of your program never share one.

**The examples below** read one user, answer a body with wrong fields, and import a list of
products.
