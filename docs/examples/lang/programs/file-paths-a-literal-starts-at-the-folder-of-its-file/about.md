A file path written as a string literal starts at the folder of the file that contains it.

`Core\IO::read("data/prices.txt")` reads the `data` folder next to your program. It reads the same
file when you start the program from another folder, under `nvs serve` and as a service. A full
path, such as `/srv/shop/prices.txt`, is used as you wrote it. Add `#[Core\Path]` to a `string`
parameter of your own method, and a literal passed to it works the same way.

**Good to know:** only a literal written in the call counts. A relative path you build while the
program runs, such as `"data/" . $name`, throws a `RuntimeError` when a file method gets it. Build
it with `Core\Path::join` from a full folder path instead.

**The examples below** read a file next to the program, pass a path to a method of your own, and
build a path from a name that is only known while the program runs.
