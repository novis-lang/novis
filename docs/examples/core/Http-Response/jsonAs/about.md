Decodes the body of a reply as JSON into the type you name, for example a shape or a class.

You write the type between `<` and `>`: `$response->jsonAs<User>()`. The result has that type, so
you read its fields with no further checks. Write `array<User>` when the reply is a JSON array. If
the body is not valid JSON, is not UTF-8 text, or does not fit the type, `jsonAs` throws a
`ParseError`. The message says what did not fit. The option `maxDepth` limits how deeply the JSON may
be nested.

The text comes from another server, so every text field of the type must be `tainted`.

**Good to know:** the examples use `Core\Test::answerHttp` to give fixed replies, so they run
without a network. They read one user, show three replies that do not fit, and add up a list of
orders from a shop.
