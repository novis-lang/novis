The `namespace` declaration must come before every class, interface, enum and type alias in the
file. A second `namespace`, or one below a declaration, does not compile (`E0243`). A `use` import
covers the code below it. A class above the `use` cannot use the short name, so put every `use` at
the top. A `use` inside a method body does not compile (`E0233`).

A name that starts with `\`, such as `\Core\Str`, does not compile (`E0240`). A name with a `\` in it
already starts from the top namespace. `use Core\Str as S;` does not compile (`E0212`), and neither
does `use Core\{Str, Json};` (`E0238`). `namespace Core;` does not compile (`E0217`). A class of your
own may still be called `Str`, outside any namespace or inside one.

A class or an interface is declared at the top level of a file. A class inside an `if` or inside
another class does not compile (`E0233`). An enum or a type alias may also be declared inside a class
or an interface body. It has no `public` or other modifier in front of it (`E0133`).
