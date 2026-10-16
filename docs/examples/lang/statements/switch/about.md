`switch (value) { … }` compares one value with a list of labels and runs the body of the first label
that is equal to it. Each label is written as `case something:`, and `default:` is what runs when no
label matched. Nothing happens when no label matched and there is no `default`.

A body runs on into the next body until a `break` stops it. That is called falling through, and it
lets several labels share one body: write them one under the other, with nothing in between. A body
that ends with `return` also leaves the `switch`.

The labels are tried in the order you wrote them, and `default` is tried last wherever you put it.
`switch (true)` compares against `true`, so every label is a condition, and the first condition that
is true is the one that runs.

**The examples below** show a choice between three answers, then labels that share a body, then
`switch (true)` sorting a number into bands.
