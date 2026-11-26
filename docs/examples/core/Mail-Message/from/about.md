Returns the sender of a received message: the first address of its `From` header.

The result is a `Core\Mail\Address`. Its `address()` is the email address, and its `name()` is the
name in front of it, such as `Ada Lovelace`. A name that the sender encoded for another alphabet is
decoded. A message without a `From` address returns `null`.

**Good to know:** the sender writes the `From` header, and nothing checks it. Anybody can write
`From: ceo@example.com`. So do not trust the address to decide who may do something. Both the
address and the name are tainted, so escape them before you put them on a page.

**The examples below** read the sender and the name, read a name in another alphabet, and show
a sender on a web page.
