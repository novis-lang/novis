Compares two cases of one enum.

`==` is true when both sides are the same case, and `!=` is true when they are different. Both sides
must belong to the same enum. A case compared against a plain number, or against a case of another
enum, does not compile: the two types have no value in common, so the comparison could only ever be
false.

There is no `<` or `>` between cases. Write `$case as int` on both sides and compare the numbers,
which is also how you subtract or add them. That keeps the order you get explicit, because the order
of the cases is the order of the numbers you gave them.

**The examples below** show `==` and `!=` between cases, then the number comparison that puts two
cases in order, then a page that decides what a visitor may do from a role.
