`Core\Arr::from` turns a sequence into a plain array.

It takes whatever a `foreach` loop takes: an array, a generator, an object you can walk, or a `Core`
collection. The result is an `array<T>` numbered from `0`, holding the values in the order the
sequence gave them. Use it when you need the whole sequence at once, to count it, to sort it, or to
read it more than once. A generator is walked once and is then spent, so keep the array you get
back.

**Good to know:** the `limit` option stops the walk after the number of values you name. A sequence
with no end needs it, or the call never returns.

**The examples below** show a generator collected into an array, then `limit` on a sequence with no
end, then a `Core` collection sorted.
