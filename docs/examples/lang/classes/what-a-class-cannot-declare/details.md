A method whose name starts with `__` is the error `E0111`. Its help names the method without the
underscores, such as `get` for `__get`. `__construct` is the error `E0114`, and its help names
`constructor`. A property whose name starts with `_` is the error `E0112`.

`new class { … }` is the error `E0244`.

An object cannot be called like a function. `$obj()` is the error `E0420`. Call a method by its
name, or use a method reference such as `$obj->run(...)`.

A method called `destructor` is an ordinary method. It runs only when your code calls it.
