`Core\Reflect\EnumInfo::of(Status::class)` reports an enum's shape as ordinary structural metadata —
the type's name and its closed case list — the same way `ClassInfo` reports a class's shape.

It grants an enum nothing it does not already have: no method dispatch, no interface, no case
identity, and nothing that acts on a value. It is a second description of the same closed integer
type (`rule:enums/closed-integer-type`), not a reopening of `rule:enums/no-class-machinery` — a
description of a case list is not a `::cases()` the language does not have.
