Compares two strings and tells you which one comes first when you sort them.

`Core\Str::compare` returns `-1` when the first string comes first, `0` when the two are equal,
and `1` when the second string comes first. You can use it as the comparison function of a sort.

By default, the comparison is character by character and case-sensitive. All upper case letters
come before all lower case letters, so `"Zebra"` comes before `"apple"`. Two options change this.
With `{caseInsensitive: true}`, upper and lower case letters are equal. With `{natural: true}`,
digits inside the strings are compared as numbers, so `"file2"` comes before `"file10"`. You can
use both options together.

This replaces PHP's `strcmp`, `strcasecmp`, `strnatcmp` and `strnatcasecmp`.

**The examples below** compare two words, sort file names with numbers in them, and sort a list of
customer names without caring about upper and lower case.
