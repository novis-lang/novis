`get` on a declaration with two `Tag` attributes is `E0728`. `all` has no such limit.

The name between `<` and `>` must be an attribute declaration. An attribute declared inside a class
or an interface is written with its owner, as in `get<Job::Meta>`. A shape such as `{ label: string }`,
a `type` alias, a class and `int` are each `E1008`. `get` with no type is `E0442`.

The retrieval finds only attributes written with the name you give. Two declarations with the same
fields are still two different attributes, and `get` for one never returns the other.

You can also name an attribute that Novis reads itself, such as `Core\Route` or `Core\Command`. The
result is the fields written in that attribute. A field the attribute does not need may be missing,
so read it with `??`, as in `$command->about ?? ''`.

Every class has a constructor, even when you do not write one. So `Users::constructor` always
names the class.

The first argument must be a method reference written in the call. A variable that contains one is
`E0730`. A method that is `private` is `E0471` outside its class. A static method is named the same
way, as `Orders::show`.

A property or parameter name written in quotes is checked. A name that the class or the method does
not have is `E0798`. For the parameters of the constructor, use `Users::constructor` and the
parameter's name. For a promoted constructor parameter, you get the attributes on the parameter and
on the property together.

A subclass does not get its parent's class attributes. `B::constructor` finds nothing when only the
parent class `A` has the attribute. A method is read where it is declared. When `B` does not
override `show`, `B::show` is the method `A` declares, and you get the attributes `A` wrote on it.
When `B` overrides `show`, you get only the attributes on `B`'s method. An interface method's
attributes are read with `I::m`.

A class constant, an enum case or `Foo::class` in an attribute is read back as its value. The value
comes from the place where the attribute is written. In a file with `namespace Shop;`, `K::class` is
read back as `"Shop\K"`. A class constant whose own value is another class's constant is `E0792` in
an attribute. When `get` or `all` matches that attribute, the call is `E0731` too.

Each call gives a new copy of the value. A change to the copy does not change what the next call
returns.
