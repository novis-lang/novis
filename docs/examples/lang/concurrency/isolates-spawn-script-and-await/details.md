`spawn script` gives a `Core\Script\Handle`. The handle has no properties, so `$h->ok` is `E0405`.
`await` needs a handle. `await $n` on an `int` is `E0401`.

The entry is a path or a static method. An anonymous function is `E0802`.

`with(…)` takes the options `args`, `limits`, `grants`, `output` and `on`. Another name is `E0101`, and
its help lists the five.

`on:` must be the word `"worker"` or `"here"`, written at the spawn. Another word is `E0818`, and so is
a variable. To choose the core while the program runs, write two spawns in an `if`.

`output:` may be a variable. A value that is not `"capture"` or `"inherit"` throws `LogicError` with
the message ``` `spawn script`'s `output:` is `'capture'` or `'inherit'`, got `'keep'` ```.

The type of `output` in the result is `Core\Html\Markup|Core\Cli\Text`, not `string`. So `string $s =
$r->output;` is `E0401`. `echo $r->output` works, and `$r->output as string` gives the text.

With `output: "inherit"`, the child's output is printed when you `await` its handle. It comes after
everything the parent printed before that `await`. The `output` field is then empty.

A handle that is never awaited prints nothing. The program ends when the parent ends, even while the
child still runs.

`value` is `mixed`, so `int $v = $r->value;` is `E0401`. `$r->value as string` gives a `tainted
string`. A child with no `return` gives `null`. A child that calls `exit` also gives `null`, and `ok`
is `true`.

A child cannot return an object. When it does, `ok` is `false`, and the class in `error` is `Error`.
This is true for an anonymous object too, and for a class the parent also declares.

`args:` copies its value at the spawn. A change the parent makes later is not seen by the child.
Without `args:`, `Core\Script::args()` returns `null`. A file cannot receive an object in `args:`. The
spawn then throws `LogicError` in the parent.

A static method gets each entry of `args:` as the argument with the same name. Write `args:` as an
array with string keys, such as `["n" => 5]`. Here an object can be passed. A method with parameters
and no `args:` is `E0402`, and a key that names no parameter is `E0486`. When an entry is missing
while the program runs, the spawn throws `LogicError`. When an entry has the wrong type, the child
fails with a `LogicError`, and `ok` is `false`.

A path built while the program runs must be absolute. For a relative one, the message is ``
`spawn script` needs an absolute path, and `child.nvs` is relative ``. A second line names
`Core\Path::join` and `Core\Path::fromCwd`.

`limits: { memory: "1M" }` gives the child a memory limit. A child that goes over it ends with `ok`
`false` and a `RuntimeError`. `grants: []` gives the child no capabilities, so a `spawn script` inside
it throws `RuntimeError`.

A child can start its own child. A task in `Core\Task::all` can start a child and `await` it.
