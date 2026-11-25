The first type argument of `array<K, T>` is optional. When you write it, it must be an enum. A key
type such as `string` or `int` is the error `E1017`. The type `array<T>` with one argument keeps
its usual keys.

A key of an `array<Status, T>` must be a case of `Status`. The compiler checks the key when you
write a value, when you read a value and in an array literal. A key of type `int`, `string` or `mixed` is the error
`E0434`. A case of another enum is `E0434` too. The help for `E0434` shows how to convert the key
with `as Status`.

`$array[] = $value` is the error `E1018`. An array keyed by an enum has no next key to add a value
under.

In a `foreach` over an `array<Status, T>`, the key has the type `Status`. A callback that
`Core\Arr::filter` or `Core\Arr::map` calls with a key also gets a `Status`.

`Core\Arr::keys` returns an `array<Status>`. `Core\Arr::firstKey`, `Core\Arr::lastKey`,
`Core\Arr::keyOf` and `Core\Arr::findKey` return a `Status`, or `null` when there is no such key.
`Core\Arr::filter`, `Core\Arr::map` and the other functions that keep the keys return an array that
is also keyed by `Status`. `Core\Arr::groupBy` with a callback that returns a `Status` returns an
array keyed by `Status`.

You can pass an `array<Status, T>` to a parameter of type `array<T>`. To pass an `array<T>` where
an `array<Status, T>` is needed, convert it with `as array<Status, T>`.

`$value as array<Status, T>` checks every key and every value. A key that is not the number of a
case of `Status` throws a `RuntimeError`. With `as ?array<Status, T>`, the result is `null` instead.
Use this conversion for an array from `Core\Json::decode`. Its keys are strings such as `"2"`, and
each one must be the number of a case.

`Core\Json::encode` writes each key as the number of its case, as a JSON string such as `"2"`.

An array keyed by an enum is stored like any other array. It uses the same memory as an `array<T>` with the same values.
