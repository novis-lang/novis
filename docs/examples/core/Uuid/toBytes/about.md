Returns the sixteen bytes of a `Uuid`. The bytes are in the same order as the digits of the UUID's
text, so `f9168c5e-…` gives the bytes `f9 16 8c 5e …`. Use it to store a UUID in a binary database
column, or to send it in a binary message. The text of the same UUID is 36 bytes long.

`toBytes` always works, and it always returns exactly sixteen bytes. `Core\Uuid::fromBytes` does
the opposite: it reads the bytes back as the same `Uuid`.

**The examples below** print the bytes as hexadecimal, read them back to the same UUID, and write
several UUIDs into one binary message.
