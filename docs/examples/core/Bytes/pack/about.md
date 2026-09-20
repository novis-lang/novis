Writes numbers and text into a buffer of bytes, in the layout you describe. This replaces PHP's
`pack`.

The first argument is the format. Each letter in it describes one field: how wide the field is, and
for a number, which end of it comes first. The values after the format fill those fields in order.
`N` writes a whole number in four bytes with the largest byte first, and `V` writes the same number
with the smallest byte first. `a`, `A` and `Z` write text or bytes into a field of the width you
give, and fill the rest of it with padding.

Every field states its own width, so two machines that agree on the format read the same bytes. A
value that is too large for its field throws an error. It is never cut short and never wrapped
around.

**Good to know:** the format and the values must match exactly. A field with no value left for it,
and a value no field writes, both throw an error.

**The examples below** show the number fields first, then text fields and padding, then a complete
message ready to send.
