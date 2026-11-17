`use` names a class, an interface or an enum, such as `'Shop\Vault'`. It may also name a method, a property or a class constant, such as `'Shop\Vault::open'`. It may name a function, or a namespace: `'Shop'` for its own declarations, and `'Shop\*'` for every namespace below it too.

An entry of `allow` is a namespace, a class or one method of a class. Code inside the restricted declaration, its class or its namespace may always use it.

These are uses: a call, a method reference, `new`, a property read or write, a class constant and an enum case. A call through `parent::`, a subclass or an interface is a use too. `$object->$key` is a use of every public property that `$key` may name.

A type, an `is` test and `catch` are not uses. Allowed code can pass a restricted object to other code through typed parameters.

A restriction on a method holds for every method that overrides or implements it. A `use` that names an overriding method is error `E0665`.

In `replace`, the method's parameters and `$this` are the values of the call. A `replace` that does not compile is error `E0666`.

A package's `allow` entries must all be inside the package's own namespace. A package's entry allows only the files inside the package's folder. A file outside it that declares the package's namespace is still not allowed.

A package does not have to come from a registry. A folder with a `package.toml` that has a `[package] name` is a package. So a framework folder beside your modules can restrict what every module uses.

Your application's file cannot remove or widen an entry of a package. A package that adds a restriction makes a breaking change, and `nvs pkg diff` says so.
