Returns the part of a string that comes after a separator, without the separator itself.

`Core\Str::after` looks for the first place the separator appears and returns everything to its
right. If the separator is not in the string, the result is `null`. If the separator is at the very
end, the result is an empty string. So you can always tell "not found" apart from "found, with
nothing after it". With the option `{last: true}`, it cuts at the last place the separator appears.

The search is case-sensitive: `"Name"` does not find `"name"`. `Core\Str::before` does the same for
the part on the left.

**The examples below** read a value out of a line, take a file's extension with `{last: true}`, and
read the value out of an HTTP header.
