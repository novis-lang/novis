Returns a new array with copies of a value added in front, until the array has the
length you asked for.

`Core\Arr::padStart` never changes the array you give it. The result is always a list: its keys are
numbers counting from `"0"`, and the keys your array had are gone. The copies come first, then every
value of your array.

An array that is already that long, or longer, is not padded. You get its values back as a list.

**Good to know:** the number you pass is the length of the whole result. It is not the number of
copies to add.
