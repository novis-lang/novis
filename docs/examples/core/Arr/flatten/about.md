Takes the arrays inside an array and gives you all their values as one flat array. It replaces the
loop over the inner arrays you would otherwise write yourself.

Only one level is removed. The values come out in the order they were met, and an array inside an
inner array stays an array. `Core\Arr::flattenDeep` is the member that removes every level.

The keys are gone, and the result is numbered from 0. Two inner arrays can hold the same key, so only
one of them could have kept it. Every entry of the array has to be an array, and an inner array with
no entries adds nothing to the result.
