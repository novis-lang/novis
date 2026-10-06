Replaces several different texts in a string with one call.

`Core\Str::replaceAll` takes a string and an array of pairs. Each key of the array is a text to
find, and its value is the text that replaces it: `replaceAll("{name} is here", ["{name}" =>
"Anna"])` gives `"Anna is here"`. If nothing matches, the string is returned unchanged.

The string is read once, from left to right. The new text is not searched again, so
`["left" => "right", "right" => "left"]` makes the two words change places. Where several keys
match at the same place, the longest key is used. The order of the pairs does not change the
result. An empty key is ignored.

`caseInsensitive: true` lets upper-case and lower-case letters match each other.

To replace one text, use `Core\Str::replace`.
