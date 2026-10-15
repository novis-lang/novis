Splits a string with holes into its text and its holes.

`Core\Str::parts` takes a string with holes, such as a `string<propertyPath<Order>>`, and returns an
array. The array starts with text and ends with text, and text and holes take turns. So a hole is
always at an odd index. A text piece can be the empty string, for example when the string starts
with a hole.

In a text piece, `{{` is already one `{` and `}}` is already one `}`. Your code does not need to
handle them again. The compiler checked each hole before the call, so every hole is a valid value.

**Good to know:** if the string is `tainted`, every text piece is `tainted` too. The holes are not
tainted, because each one is a value the program declared.

**The examples below** show the pieces of a condition, a small query builder that turns each hole
into a column name, and a message template with a fixed list of holes.
