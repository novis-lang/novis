The error a program throws when input does not match the format it should have: text that is not
valid JSON, a field with the wrong type, or a document that is nested too deeply.

`ParseError` is a kind of `RuntimeError`, because bad input comes from outside the program.

A `ParseError` has a `message` like every error, and it also has `issues`. `issues` is a list with
one entry for each problem that was found. Each entry has a `path`, which is the name of the field,
and a `message`, which says what is wrong with that field. A form with three bad fields throws one
`ParseError` with three entries. Your reply can then tell the person about all three fields at once.

**Good to know:** when the whole document is broken, `issues` has one entry and its `path` is an
empty string.

**The examples below** show a form with two bad fields that are reported together, a document that
is not valid JSON, and a JSON reply that names each wrong field of a signup.
