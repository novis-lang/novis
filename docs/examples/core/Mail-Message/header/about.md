Returns every value of one header of a received message, such as `List-Id` or `Received`.

The result is an array with one string for each line of that header, in the order of the
message. A header the message does not have gives an empty array. Upper and lower case in the name
are the same, so `List-Id` and `list-id` return the same values. An encoded value, such as
`=?UTF-8?B?...?=`, is decoded.

Only the headers of the message itself are read, not the headers of its parts. The first 1,000
header lines are read.

The sender wrote every header, so each value is a `tainted string`. Tainted means that the value
comes from outside your program. Most headers also have a method of their own, such as `subject()`
or `references()`.

**The examples below** show a header with one and with several lines, a missing header, and an
encoded value.
