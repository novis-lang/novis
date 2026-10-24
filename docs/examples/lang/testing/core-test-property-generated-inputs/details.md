A parameter can have any of these types: a number, `bool`, `decimal`, `string`, `bytes`, an enum, `mixed`,
`?T`, `array<T>`, a shape, a union, and a class of your program with a `public` constructor. A class
is built with `new`, so its constructor runs. A function, an interface, an abstract class, a `Core`
class and a `secret` value are `E0890`.

`gen:` takes a range for a parameter, written as `"low..high"`. Both ends are included. On a
number, the range is the value. On a `string`, `bytes` or `array<T>`, the range is the length, up
to 10000. A string's length counts characters as a person reads them. On `?T`, the range is for
`T`. A range that names no parameter, or that does not fit the type, is `E0890`.

The options are those of `#[Test]`, without `retries:` and `because:`. `runs:` is from 1 to 100000.
A property never uses a `#[TestWith]` row or a fixture.

The seed also sets the test's own random numbers. Without `seed:`, every run has a new seed.

The runner tries at most 1000 smaller inputs. It also stops at the test's time limit. One call's
inputs have at most 1048576 elements, bytes and characters together. A property that needs more
fails before its first call.

In the JSON report, a failed property has a `counterexample` field. A subclass runs every property
it inherits, the same way it runs an inherited `#[Test]`.
