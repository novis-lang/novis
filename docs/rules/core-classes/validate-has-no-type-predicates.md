`Core\Validate` carries no predicate whose job is to ask whether a string names a value of some type.
`Validate::isInteger($s)` and `$s as ?int != null` are the same predicate, and one operation gets one
spelling — so `isInteger`, `isFloat` and `isBoolean` do not exist, and neither does a `ctype_digit`
equivalent, which is `$s as ?uint != null`.

One implementation now exists because there is one operation, not because two were required to agree.
The conversion table is the definition of what parses; a second table maintained beside it is a
second table to drift.

What survives is the roster that names no type: `isEmail`, `isIp`, `isMac`, `isDomain`, `isAscii` and
`isPrintable`. None has an `as` equivalent, because none names a type
(`rule:expressions/nullable-conversion`).
