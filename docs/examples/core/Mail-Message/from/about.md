Returns the sender of a received message: the first address of its `From` header.

The result is a `Core\Mail\Address`. Its `address()` is the email address, and its `name()` is the
name before the address, such as `Ada Lovelace`. A name in another alphabet is sent as an encoded
word (the way a header writes non-ASCII text), and `from()` decodes it. A message without a `From`
address returns `null`.

**Good to know:** the sender writes the `From` header, and nothing checks it. Anybody can write
`From: ceo@example.com`. So do not trust the address to decide who may do something.

Both the address and the name are tainted. Tainted means that the value comes from outside your
program, so you must check or escape it before you use it. Escape it with `Core\Html::escape`
before you put it on a page.

**The examples below** read the sender and the name, read a name in another alphabet, and show
a sender on a web page.
