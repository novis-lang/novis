`Core\Reflect\EnumInfo::of(Status::class)` reports an enum's shape as ordinary structural metadata —
the type's name and its closed case list — the same way `ClassInfo` reports a class's shape.

The name is the one a program writes to reach the enum from anywhere. For a file-scope enum it is the
qualified name, `Shop\Status`. For an enum declared in a class or interface body it is the owner's
qualified name, `::` and the member's name, `Shop\Order::Status` (`rule:enums/class-scoped-enum`), and
`EnumInfo::of(Order::Status::class)` finds it by that name. A diagnostic, `nvs meta` and the
error a route capture throws print the same string, so one enum has one printed name everywhere.

It grants an enum nothing it does not already have: no method dispatch, no interface, no case
identity, and nothing that acts on a value. It is a second description of the same closed integer
type (`rule:enums/closed-integer-type`), not a reopening of `rule:enums/no-class-machinery` — a
description of a case list is not a `::cases()` the language does not have.
