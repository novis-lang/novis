Returns the value of one case of an enum, found by the name of the case.

Every case of an enum has a whole number as its value. You get the description of the enum with
`Core\Reflect\EnumInfo::of`, and then `valueOf("Paid")` returns the value of the case `Paid`. The
name is case-sensitive. The result is a `uint` for an enum declared with `: uint`, and an `int`
for every other enum. `isUnsigned()` says which one you get. If the enum has no case with that
name, `valueOf` throws a `LogicError`.

**Good to know:** `cases()` returns every name that `valueOf` accepts. Check a name against that
list first when it comes from a user.

**The examples below** print the value of each case, catch the error for a name that does not
exist, and turn a choice from a form into the number saved in a database.
