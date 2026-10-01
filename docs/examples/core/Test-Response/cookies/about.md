`cookies()` returns the cookies that the program set in the answer to a request that
`Core\Test::request()` sent. The result is an `array<string>`. Each key is a cookie name, and each
value is the cookie value, exactly as the program set it.

The result has only the name and the value of each cookie. To check an attribute such as `Path` or
`HttpOnly`, read the whole line from `headers()["set-cookie"]`.

If the program set the same name twice, the result has the last value. This is the value a browser
keeps. A cookie that the program deleted is in the result with an empty value. If the program set
no cookie, the result is an empty array.

**The examples below** show how to test a login cookie, a cookie that is set twice, and a cookie
that is deleted.
