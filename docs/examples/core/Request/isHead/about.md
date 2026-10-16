Checks whether the request is a `HEAD` request. The result is `true` for `HEAD` and `false` for
every other method.

A client sends `HEAD` when it wants the headers of a response but not its body. A link checker does
this to see whether a page exists. Novis runs a `HEAD` request the same way as a `GET` request, and
the server does not send the body. So `Core\Request::method` returns `Get` for both, and `isHead`
is the only way to tell them apart.

You do not need to check for `HEAD` for your program to be correct. Check it when building the body
is expensive and you want to skip that work, or when a `HEAD` request should not count as a visit.

A command-line program answers no request, so `isHead` throws a `LogicError` there.
