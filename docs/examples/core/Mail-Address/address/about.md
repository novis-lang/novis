Returns the email address of a `Core\Mail\Address`, such as `ada@example.org`, without the name.

You meet a `Core\Mail\Address` in two places. `Core\Mail\Address::of` makes one for a mail that your
program sends. For a received `Core\Mail\Message`, the methods `from()`, `to()`, `cc()` and
`replyTo()` return addresses. `name()` returns the name that is shown for the address.

`address()` returns a `tainted string`. Tainted means that the value comes from outside your
program, so you must check or escape it before you use it. Escape it with `Core\Html::escape`
before you put it on a page. It cannot be part of a file path or an SQL query without a check.

The sender of a received message writes its addresses, so an address does not prove who sent the
message. Upper and lower case are kept as the sender wrote them.

**The examples below** show the addresses of a received message, an address your program made,
and how to find a customer by the address of a message.
