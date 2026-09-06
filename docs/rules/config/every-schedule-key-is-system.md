Nothing in a `[[schedule]]` entry is changeable from inside a request, and there is no
`Core\Config::set` path to any of it. The whole block is `System`-class: read at boot, moved only by
a reload of the root-owned file, and refused as a `System` set from a program.

A running request adding, removing or retiming a scheduled job would be process-global mutable state
under another name (`rule:statements/static-is-a-member-modifier`), established by whichever request
happened to run the registering code first. And a request *narrowing* one — the direction every
other tightenable directive allows — would silently disable a job for everyone on that host, which is
why not even `RuntimeTighten` applies. This is the same class and the same reasoning that keeps
`opcache.validate` out of a request's reach.

`[queue]`, the durable job queue beside the schedule, is `System` throughout for the same reason: work
a request could redirect is work a request could redirect into a database it was never granted.
