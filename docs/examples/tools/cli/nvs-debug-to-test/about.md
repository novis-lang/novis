`nvs debug to-test` prints a test made from one request that your server recorded.

The test sends the same request to your program with `Core\Test::request`. It checks the status,
the `Content-Type` header, the `Location` header of a redirect, and the body. A JSON body is
compared as data, so spaces in it do not matter. Save the test in your project and run it with
`nvs test .`. With `--write <file>`, the command writes the test to a new file.

The request must come from a development server with `[debug] store = true` in `nvs.toml`. Give
the trace id that the debug viewer shows for the request.

**Good to know:** a header that was removed when the request was recorded, such as
`Authorization`, gets the value `'redacted'`. Replace it with a credential your tests may use.

**The examples below** show the options of the command, a test for a JSON page, and a test for a
request with a removed `Authorization` header.
