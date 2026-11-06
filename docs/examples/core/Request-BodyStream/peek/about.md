Returns the first bytes of the request body, up to `max` bytes. The bytes stay in the body, so a
`foreach` loop over the same value still gives every byte from the start.

Use it to check the start of an upload before you read or save the rest. A body shorter than `max`
is returned whole. A second call with a larger `max` reads further. The result is `tainted bytes`.

Call `peek` before the loop starts. After that, it throws a `LogicError`. A `max` larger than the
request's memory limit also throws a `LogicError`, and nothing is read. To find the file type of an
upload, use `Core\Mime::detect` with the body stream.

The examples show how to check a CSV header before the file is saved, how to reject an old file
version, and that the loop still reads every byte.
