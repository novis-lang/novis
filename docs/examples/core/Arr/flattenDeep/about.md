Takes every array nested inside an array, however deep they go, and gives you all their values as one
flat array. It replaces the recursive walk you would otherwise write yourself.

A value that is not an array is kept where it was met, so the order you read is the order the values
sit in. The keys are gone at every level, and the result is numbered from 0.
`Core\Arr::flatten` is the member that removes one level only.

**Good to know:** storing an array inside itself stores a copy of it, so the nesting is always
finite and this member always finishes.
