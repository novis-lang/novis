Creates a new random UUID, such as `f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4`. A UUID is a 128-bit
identifier that you can create anywhere without asking a database or another server for the next
number. 122 of the bits come from a secure random number generator, so two calls never give the same
UUID in practice, and nobody can guess the next one.

The other six bits are fixed. The first digit of the third group is always `4`, which says that this
is a version 4 UUID. A version 4 UUID contains no date or time, so it is safe to show to other
people. When you need IDs that sort by the time they were created, use `Core\Uuid::v7` instead.

**The examples below** show the parts of a new UUID that never change, check that many new UUIDs are
all different, and give each new order its own ID.
