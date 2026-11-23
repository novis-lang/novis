`use` names a class, an interface or an enum, such as `'Shop\Vault'`. It may also name a method, a property, a class constant or an enum case, such as `'Shop\Vault::open'`. Novis has no functions outside a class, so the only callable that `use` names is a method.

`use` may also name a namespace. `'Shop'` means the declarations of `Shop` itself. `'Shop\*'` also includes every namespace below `Shop`.

An entry of `allow` is a namespace, a class or one method of a class. A class entry allows that class only, and not a namespace with the same name. Code inside the restricted declaration, its class or its namespace may always use it.

These are uses: a call, a method reference, `new`, a property read or write, a class constant and an enum case. A call through `parent::` or a subclass is a use too. `$object->$key` is a use of every public property that `$key` may name.

`static::NAME` is a use of the constant `NAME`. A variable can have a union type, such as `Shop\Vault|Shop\Safe`. A call on it is a use of that method on each class of the union. A link to a route is a use of the method it names. A constant or an enum case in a default value or in an attribute's arguments is a use too.

A call through an interface or a parent class is a use of that interface's or parent's method. So an entry on the class `Shop\Vault` does not apply to a call on a `Shop\Lockable` variable. Restrict `Shop\Lockable::open` to check those calls too.

A type, an `is` test and `catch` are not uses. Allowed code can pass a restricted object to other code through typed parameters.

A restriction on a method also applies to every method that overrides or implements it. So a `use` must not name an overriding method: that is error `E0665`. Restrict the parent's method instead.

`replace` can use the restricted method's parameters and `$this`. At each use, they are the values of that call. A `replace` that does not compile is error `E0666`.

A package's `use` must name its own code or code in `Core`. A package cannot restrict another package or your application, and such an entry is error `E0665`. A package's `allow` entries must all be inside the package's own namespace. A package's entry allows only the files inside the package's folder. A file outside it that declares the package's namespace is still not allowed.

A package does not have to come from a registry. A folder with a `package.toml` that has a `[package] name` is a package. So a framework folder beside your modules can restrict what every module uses.

Your application's file cannot remove or widen an entry of a package. A package that adds a restriction makes a breaking change, and `nvs pkg diff` says so.
