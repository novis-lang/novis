A relative file path written in quotes starts at the folder of the file that contains it.

`Core\IO::read("data/prices.txt")` reads the `data` folder next to your program. It reads the same
file when you start the program from another folder, under `nvs serve` and as a service. A full
path, such as `/srv/shop/prices.txt`, is used as you wrote it. Add `#[Core\Path]` to a `string`
parameter of your own method, and a path in quotes passed to it works the same way.

**Good to know:** this works only for a path written in quotes in the call itself. A path such as
`'data/' . $name` starts with a relative path, so it does not compile. Start it with the folder of your file:
`Core\Path::thisDir('data') . '/' . $name`. `Core\Path::join` also adds a name to a full folder path.
A relative path in a variable throws a `RuntimeError` when a file method gets it.

**The examples below** read a file next to the program, pass a path to a method of your own, and
build a path from a name that is only known while the program runs.
