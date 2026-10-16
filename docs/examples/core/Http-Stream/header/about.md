Returns one header of a streamed reply, or `null` when the server did not send it.

The name is matched without regard to case, so `Content-Type` and `content-type` find the same
header. When the server sent the same header on several lines, `header()` joins the values with a
comma and a space. The value came from another server, so it is `tainted`.

The headers arrive before the body. So you can read them before you decide how to read the body,
or whether to read it at all. Reading a header does not read the body, and you can still read
headers after the body is read.

**Good to know:** `Set-Cookie` cannot be joined safely, so `header("set-cookie")` throws a
`LogicError`. Use `headers()` for it, which returns one value for each line.

**The examples below** read one header, show a header sent twice and one that is missing, and
check the size of a download before it starts.
