`get` with two matching attributes is `E0728`. `all` has no such limit.

The type between `<` and `>` must be a shape, written out as `{ label: string }` or as a `type` alias
for one. `get<int>` is `E0729`, and `get` with no type is `E0442`.

Every class has a constructor, even when you do not write one. So `Users::constructor` always
names the class.

The first argument must be a method reference written in the call. A variable that contains one is
`E0730`. A method that is `private` is `E0471` outside its class. A static method is named the same
way, as `Orders::show`.

A property or parameter name written in quotes is checked. A name that the class or the method does
not have is `E0798`. For the parameters of the constructor, use `Users::constructor` and the
parameter's name. For a promoted constructor parameter, you get the attributes on the parameter and
on the property together.

A subclass does not get its parent's attributes. `B::constructor` finds nothing when only the
parent class `A` has the attribute. `B::show` also finds nothing when `show` is declared in `A`,
even if `B` does not override it. An interface method's attributes are read with `I::m`.

`all<{}>` returns every attribute on the declaration, also the attributes Novis reads itself, such as
`#[Core\Json\Derive]`.

A class constant, an enum case or `Foo::class` in an attribute is read back as its value. The value
comes from the place where the attribute is written. In a file with `namespace Shop;`, `K::class` is
read back as `"Shop\K"`. A class constant whose own value is another class's constant is `E0792` in
an attribute. When `get` or `all` matches that attribute, the call is `E0731` too.

Each call gives a new copy of the value. A change to the copy does not change what the next call
returns.
