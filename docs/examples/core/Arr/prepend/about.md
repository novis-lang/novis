Adds one or more values to the front of an array. It replaces PHP's `array_unshift`.

`Core\Arr::prepend` never changes the array you give it. It returns a new array: first each value
you added, in the order you wrote them, then every value of your array.

The result is always a list. Its keys are numbers again, counting from `"0"`, and the keys your
array had are gone. A value put in front of the first entry has no key of its own, and `"0"` may
already be in use, so Novis numbers the whole result instead of choosing a key for you. This is the
one place where adding at the front and adding at the end differ.

If you pass no value, the result is your array with its keys renumbered.
