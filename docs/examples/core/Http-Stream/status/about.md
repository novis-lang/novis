Returns the status code of a streamed reply, such as `200` or `404`.

The status code is the three-digit number at the start of every HTTP reply. It tells your program
whether the request worked. `status()` returns it as an `int` as soon as the start of the reply has
arrived, so you can check it before you read the body.

A code such as `404` or `503` does not throw an error. Your program gets the reply and decides
what to do with it. You can call `status()` as often as you want, also after the body was read.

**The examples below** check the code before reading the body, show that a `404` is returned like
any other reply, and try a second server when the first one answers that it is busy.
