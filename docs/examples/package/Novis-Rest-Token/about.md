`Novis\Rest\Token` is an access token from an OAuth 2 server, with its type, its scope, the time it
expires and the refresh token when the server sent one.

`Token::fromResponse()` reads the reply of a token endpoint. If the reply is an error or is not a
valid token, it throws a `RuntimeError`. The message has the status and the server's error code. The
token, the refresh token and the scope may only contain the characters the OAuth 2 standard allows.
So a server cannot add a line break or a second header to your requests.

`isExpired()` returns `true` when the token expires within 30 seconds, or within the margin you give.
Get a new token before a request when it returns `true`. `authorization()` returns the value of the
`Authorization` header, such as `Bearer at-7f3c`.

**Good to know:** the access token and the refresh token are secret. Your program cannot print them
or write them to a log by mistake.

**The examples below** read a token reply, decide when a token needs to be renewed, and handle an
error reply.
