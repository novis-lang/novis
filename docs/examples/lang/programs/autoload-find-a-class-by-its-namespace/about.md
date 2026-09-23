`autoload` tells Novis where your classes live, so your program names a class and never a file.

One declaration maps a namespace prefix to a folder. With `autoload 'App' from './src';` the class
`App\Text\Greeter` is read from `./src/Text/Greeter.nvs`: the prefix is dropped, what is left becomes
folders, and the last part becomes the file name. A prefix may name several folders, which are
searched in the order you wrote them, so a folder of your own fixes can sit in front of one you
installed. `autoload discover` takes a folder pattern instead, and makes every folder it matches the
home of the namespace named by the part the `*` stands for.

One part of a prefix can be the name of a folder. In `autoload 'Site\{..}' from '../src';`, `{..}` is
replaced by the name of the folder that `..` points to from this file. If the file is in
`Blog/public`, the prefix is `Site\Blog`. Many modules can then use the same start file.

**Good to know:** the map is built while your program is compiled, from every `autoload` in every
file it reaches. There is nothing to register while the program runs, and a class that cannot be
found stops the build with the folders that were searched.
