`autoload` tells Novis where your classes live, so your program names a class and never a file.

One declaration maps a namespace prefix to a folder. With `autoload 'App' from './src';` the class
`App\Text\Greeter` is read from `./src/Text/Greeter.nvs`: the prefix is dropped, what is left becomes
folders, and the last part becomes the file name. A prefix may name several folders, which are
searched in the order you wrote them, so a folder of your own fixes can sit in front of one you
installed. `autoload discover` takes a folder pattern instead, and makes every folder it matches the
home of the namespace named by the part the `*` stands for.

**Good to know:** the map is built while your program is compiled, from every `autoload` in every
file it reaches. There is nothing to register while the program runs, and a class that cannot be
found stops the build with the folders that were searched.
