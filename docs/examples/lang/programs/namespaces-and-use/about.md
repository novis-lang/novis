A namespace is a prefix on every name a file declares, so two classes called `Greeter` in two parts
of a program never get in each other's way.

A file may open with one `namespace` declaration, and any number of `use` imports after it. Both
belong at the top of the file and never inside a class or a function. A name you write with a `\` in
it is read from the very beginning: `Core\Str` means `Core\Str` wherever you write it. A name with
no `\` is looked for in the file's imports first, then in the file's own namespace, and nowhere
else.

**Good to know:** there is no falling back to the outermost level, so inside `namespace App;` a
plain `Helper` means `App\Helper` and never a `Helper` declared outside any namespace. An import
cannot be renamed and cannot name a group of names, so write one `use` per name. `Core` is reserved
for the classes Novis ships, and nothing you write may live under it.
