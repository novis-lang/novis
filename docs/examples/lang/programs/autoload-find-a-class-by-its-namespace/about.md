`autoload` tells Novis which folder your classes are in, so your program uses a class by its name
and never loads a file.

One declaration connects the start of a namespace to a folder. With `autoload 'App' from './src';`,
the class `App\Text\Greeter` is read from `./src/Text/Greeter.nvs`. `App` is removed, the middle
parts are folders, and the last part is the file name. One prefix can have several folders, and
they are searched in the order you wrote them.

`autoload discover` takes a folder pattern with a `*`. Every folder that matches is used for the
namespace named by the part that `*` matched.

A prefix can also contain `{..}`. In `autoload 'Site\{..}' from '../src';`, `{..}` is replaced by
the name of the folder that `..` points to from this file. If the file is in `Blog/public`, the
prefix is `Site\Blog`.

**Good to know:** every `autoload` declaration is read when the program is compiled. A class that
cannot be found stops the compile, and the error lists the folders that were searched.
