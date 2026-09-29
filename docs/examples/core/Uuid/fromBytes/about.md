Reads sixteen bytes as a `Uuid`. The bytes are in the same order as the digits of the UUID's text,
so the bytes `f9 16 8c 5e …` give the UUID `f9168c5e-…`. Many databases store a UUID in a binary
column in this form, and binary protocols send it this way.

Any sixteen bytes are a valid UUID, so only the length is checked. When the bytes are not exactly
sixteen long, `fromBytes` throws a `RuntimeError`. The message gives the length it got.

`$uuid->toBytes()` does the opposite. It returns the sixteen bytes of a `Uuid`.

**The examples below** read one UUID, show the error for bytes of the wrong length, and read a list
of UUIDs from one binary message.
