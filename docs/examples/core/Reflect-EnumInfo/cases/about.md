Returns the name of every case an enum declares, as a list of strings.

You get the description of an enum with `Core\Reflect\EnumInfo::of`. `cases()` then returns one
string for each case. The list is sorted by the value of each case, from the smallest to the
largest. It is not sorted in the order of the declaration. Two cases can have the same value, and
then the one whose name comes first in the alphabet is listed first. An enum with no case gives an
empty list. It replaces PHP's `cases()`, but it returns names and not case objects.

**Good to know:** every name in this list works with `valueOf`, which returns the value of the case.

**The examples below** print the cases of an enum, show two cases with the same value, and check a
filter from a web address against the list of cases.
