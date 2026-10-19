`mixed` is the one type Novis does not check. A `mixed` variable can contain a value of any type.

Input from outside the program has the type `mixed`: a query string, a form post, decoded JSON.
Convert each value to the type you need with `as`. If that is not possible, `as` throws an error.
`is` tests what the value is and does not convert it.

A `mixed` value is never converted to a narrower type for you. Passing one to a `string` parameter
does not compile until you write `as string`.

**Good to know:** an operation on a `mixed` value is slower than on a declared type. Convert the
value once, close to where it arrives.

**The examples below** read a page number from a query string, describe each field of a decoded
payload, and convert an order line. The last two use a `mixed` value directly and show the compile
error.
