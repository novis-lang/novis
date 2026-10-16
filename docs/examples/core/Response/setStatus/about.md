Sets the HTTP status of the response. The status is a number that tells the client how the request
went: `200` means it worked, `201` means something was created, `404` means the page does not exist,
and `500` means the server had an error. If your program does not set a status, it is `200`.

`setStatus` does not write a body. You can use it together with `echo` or with a body method such as
`Core\Response::json`. If you call it more than once, the last call sets the status. If the request
fails with an error, the status is `500`.

A status is between 100 and 599. For any other number, `setStatus` throws a `LogicError` and the
status does not change.

The examples show a page that does not exist, a number that is not a status, and an API that
checks its input.
