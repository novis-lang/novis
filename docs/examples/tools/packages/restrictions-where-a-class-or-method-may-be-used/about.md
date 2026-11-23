A `[[restrict]]` table in `package.toml` says where a class, a method or a namespace may be used. A
framework uses it so that every module calls the framework's helper, and not the method the helper
wraps.

Each entry has four keys. `use` names the declaration. `allow` lists the namespaces, classes or
methods that may use it, and `[]` allows no place. `note` is the text the error shows. `replace` is
the code to write instead, and the editor offers it as a fix.

The check happens when the program compiles. A use outside the allowed places is error `E0664`. A
call through `mixed` and a reflective call (a call made by name at run time) throw a `LogicError`
when they run.

Novis reads the table from your application's `package.toml` and from each package your program
loads. No file can remove a restriction. When several entries restrict the same declaration, a use
must be allowed by each of them. A package can restrict only its own code and the code of `Core`.
