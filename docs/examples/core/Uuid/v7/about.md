Creates a new UUID that starts with the current time, such as
`0190d1e6-8e3c-7c2a-9a4b-1f2e3d4c5b6a`. The first twelve digits are the number of milliseconds
since 1 January 1970. Most of the other digits are random, so two calls never give the same UUID
in practice.

Because the time comes first, a UUID created later sorts after one created earlier. This makes it
a good key for a database table: new rows are added at the end of the index. Two UUIDs created in
the same millisecond sort in a random order.

Anybody who has the UUID can read the time it was created. For an ID that other people see, use
`Core\Uuid::v4` instead.

**The examples below** show the parts of a new UUID that never change, check that a newer UUID
sorts after an older one, and read the time a UUID was created.
