Returns the `Date` header of a received message, as a `Core\Time\DateTime`.

The date is in the time zone that the sender wrote. A message sent at `14:05 +0100` gives 14:05 with
an offset of `+01:00`. To compare messages from different time zones, call `toInstant()` on each
date. It returns the same moment in UTC. To show that moment on your own clock, call `in()` on it
with your time zone.

A message without a `Date` header, or with a date that cannot be read, returns `null`.

**Good to know:** the sender's computer writes this date, and its clock can be wrong. For the time
that a message arrived, use the time when your program received it.

**The examples below** read a date with its offset, show a date that cannot be read, and sort an
inbox by the moment each message was sent.
