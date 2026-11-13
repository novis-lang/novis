`autoload` tells Novis which folder your classes are in, so your program uses a class by its name
and never loads a file.

One declaration connects the start of a namespace to a folder. With `autoload 'App' from './src';`,
the class `App\Text\Greeter` is read from `./src/Text/Greeter.nvs`. `App` is removed, the middle
parts are folders, and the last part is the file name. One prefix can have several folders, and
they are searched in the order you wrote them.

`autoload discover 'Site' from '../../*/src';` uses every folder the pattern matches. The folder
`Shop/src` is the namespace `Site\Shop`.

In `autoload 'Site\{..}' from '../src';`, `{..}` is replaced by the name of the folder that `..`
points to. In `Blog/public`, the prefix is `Site\Blog`.

`autoload links` reads a folder only for its routes. Your program can link to them but cannot call
their classes. Both forms take it, as in `autoload links discover 'Site' from '../../*/src';`.

**Good to know:** every `autoload` declaration is read when the program is compiled. A class that
cannot be found stops the compile with an error. `nvs check --autoload-map` shows which folders
each prefix uses.
