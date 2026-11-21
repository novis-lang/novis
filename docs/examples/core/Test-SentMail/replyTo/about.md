`replyTo()` returns the address that gets the answer when somebody replies to a message. In a
test, `Core\Test::sentMail()` returns one `Core\Test\SentMail` for each message, and `replyTo()`
is one of its methods.

The result is the `replyTo` address that your program gave to `Core\Mail::send`. When the program
gave no `replyTo` address, the result is `null`. A reply then goes to the `from` address of the
mail block.

Use `replyTo()` to check where answers go. A mail from an address that nobody reads needs a
`replyTo` address, or the answers are lost.

**The examples below** show a notice whose replies go to the support desk, an alert with no
`replyTo` address, and a message between two users that a request sends.
