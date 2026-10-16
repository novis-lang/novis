Returns a new array with copies of a value added at the end, until the array has the
length you asked for.

`Core\Arr::padEnd` never changes the array you give it. The result is always a list: its keys are
numbers counting from `"0"`, and the keys your array had are gone. Every value of your array comes
first, then the copies.

An array that is already that long, or longer, is not padded. You get its values back as a list.

**Good to know:** the number you pass is the length of the whole result. It is not the number of
copies to add.
