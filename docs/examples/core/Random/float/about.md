Returns a random `float` that is at least 0 and less than 1.

Every value in that range is equally likely. The result can be `0.0`, but it is never `1.0`. This
makes it easy to scale the result into your own range: multiply it by 10, and the result is at
least 0 and less than 10. The number comes from the same cryptographic generator
as every other `Core\Random` method.

**Good to know:** for a whole number in a range, such as a dice roll, use `Core\Random::int`. It
gives an exact result, and you do not need to round anything.
