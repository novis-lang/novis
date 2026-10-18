`switch` compares the value with each label using `==`. A label whose type can never be equal to the
value is the error `E0466`. An `int` value with the label `"5"` is one such case.

A body falls through into the body written after it, even when that is `default`. `default` is
still tried last, after every `case`. When it matches, the bodies after it run too, until a
`break`.
