`ClassName::CONST_NAME`, written where a type is expected, resolves at compile time to the constant's
own value — exactly as long as that value is a `string` or `int` compile-time constant.

```nvs
class Foo {
    public const string TYPE_A = "a";
    public const string TYPE_B = "b";
}

function handle(Foo::TYPE_A|Foo::TYPE_B $type) { … }   // exactly "a"|"b"
```

This is safe precisely because a scalar `const` is not a distinct nominal type: `Foo::TYPE_A`
genuinely *is* the string `"a"`, so folding it to that single-value type changes nothing a caller could
observe — passing the bare `"a"` is exactly as valid.

A constant backed by a non-scalar type — an `array`, an object, a `float` — is **not eligible**, and
using one this way is a diagnostic naming the eligible types. An enum case is not folded either, and
for the opposite reason: it carries its enum's nominal type and stays a narrowed view of it
(`rule:types/enum-case-type`).
