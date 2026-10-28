Returns the HTTP status code that the push service sent back for one message, as an `int`.

`201` means that the push service accepted the message. A code from 200 to 299 means that the
send worked. `429` means "too many requests", so wait before you send to that browser again. A
code of 400 or more is an error, but `Core\WebPush::send` does not throw an error for it. Your
program reads `status()` and decides what to do. A redirect code such as `301` is also returned
as it is, and `send` does not follow it.

**Good to know:** the examples use `Core\Test::answerHttp` to give fixed replies, so they run
without a network. They read the status of one message, tell an accepted message from one that
was sent too often, and count the failed sends to several browsers.
