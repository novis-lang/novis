An enum body is a comma list of `Name` or `Name = <integer literal>`, and the optional `: int` or
`: uint` before it names the backing type (`rule:enums/one-backing-type`):

```
enum Status { Active, Banned }
enum Permission: uint { Read = 0b001, Write = 0b010, Admin = 0b100 }
```

The first case with no written value is `0`; every later case with no written value is the previous
case's value plus one, whether that previous value was written or counted. An explicit value resets
the counter for everything after it. A value may be negative or zero, and auto-increment continues
from a negative one.

Two cases may carry the same value. Nothing forbids the alias, because equality over an enum is
value equality and there is no identity for two names to collide on.
