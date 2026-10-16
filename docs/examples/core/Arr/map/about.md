Makes a new array by replacing every value with what a function returns for it.

You give `Core\Arr::map` an array and a function. The function receives the value and the key, and
returns the new value. A function that needs only the value declares only that one parameter.

The keys stay as they are, and the result has exactly as many entries as the array you gave. The type
of the result follows the function: a function returning `string` over an `array<int>` gives you an
`array<string>`.

The function runs once for each value, in order. If it throws an error, `map` stops and throws the
same error.

**Good to know:** to change the keys instead of the values, use `Core\Arr::mapKeys`. To leave entries
out, use `Core\Arr::filter`. To combine all values into one, use `Core\Arr::reduce`.

related: Core\Arr::mapKeys, Core\Arr::filter, Core\Arr::reduce
