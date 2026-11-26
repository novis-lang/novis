Returns the email address of a `Core\Mail\Address`, such as `ada@example.org`, without the name.

You meet a `Core\Mail\Address` in two places. `Core\Mail\Address::of` makes one for a mail that your
program sends. `from()`, `to()`, `cc()` and `replyTo()` of a received `Core\Mail\Message` return
the addresses that its headers name. `name()` returns the name that is shown for the address.

`address()` returns a `tainted string`. Tainted means that the value can come from outside your
program. The address of a received message is written by its sender, so it does not prove who sent
the message. Upper and lower case are kept as the sender wrote them.

**The examples below** show the addresses of a received message, an address your program made,
and how to find a customer by the address of a message.
