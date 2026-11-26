Reads a raw email message, with its headers and its body, into a `Core\Mail\Message`.

You pass the whole message as a `string` or as `bytes`. This is the content of an `.eml` file, or the
raw message that a mail provider posts to your webhook. The other methods of the message then return
its sender, its recipients, its subject, its text and its attachments.

A broken message is not an error. A header that is missing is `null` or an empty array, and a part
that cannot be read is skipped. So a program that reads many messages reaches all of them.

**Good to know:** every value that the message returns is tainted. A tainted value came from outside
your program. The sender wrote every byte, so escape a value before you put it on a page.

Only 16 levels of nested parts and 1,000 parts are read. The parts after that limit are skipped.

**The examples below** read the message that a webhook posts, read an `.eml` file, and read a list
of messages that has a broken one in it.
